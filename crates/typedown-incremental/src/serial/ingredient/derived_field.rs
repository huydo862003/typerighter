use std::sync::atomic::Ordering;

use crate::{Decodable, DepId, Encodable, Fingerprint, LazyFingerprint, StableHash};
use crate::{DerivedFieldIngredientStore, Ingredient, StampedDerivedField};

use crate::serial::Codec;
use crate::serial::format::binary_files::dep_graph::{DepNode, DepNodeIndex};
use crate::serial::ingredient::serde::{DeserializeContext, SerializeContext, UnresolvedDepNode};

impl<T: StableHash + std::fmt::Debug + Encodable + Decodable + Send + Sync + 'static> Codec
  for DerivedFieldIngredientStore<T>
{
  fn serialize(&self, ctx: &mut SerializeContext, entry_id: crate::EntryId) {
    let Some(entry) = self.data.get(&entry_id) else {
      return;
    };

    let dep_id = self.ingredient_id.with_entry(entry_id);
    let node_index = ctx.encoder.add_dep_id(dep_id);
    ctx.dep_graph.set(
      node_index,
      UnresolvedDepNode::DerivedField {
        name: self.name_fingerprint(),
        field_index: self.field_index,
        entry_id,
        value: if self.no_hash_flag {
          Fingerprint::SKIPPED
        } else {
          self
            .value_fingerprint(ctx.encoder.db(), entry_id)
            .expect("entry must have a fingerprint at serialize time")
        },
        changed_at: entry.changed_at,
      },
    );

    // Write field value blob
    let mut buffer = vec![];
    entry.value.encode(&mut buffer, &mut ctx.encoder);
    ctx.query_cache.set(node_index, &buffer);
  }

  fn deserialize(&self, ctx: &DeserializeContext, node_index: DepNodeIndex) -> Option<DepId> {
    if let Some(dep_id) = ctx.decoder.get_dep_node_id(node_index) {
      return Some(dep_id);
    }
    let node = &ctx.serialized.dep_graph.nodes[node_index as usize];
    let DepNode::DerivedField {
      name,
      field_index,
      entry_id: serialized_entry_id,
      changed_at,
      value: stored_value_fingerprint,
    } = node
    else {
      return None;
    };
    // Dispatch must route to the ingredient matching this field_index
    debug_assert_eq!(
      *field_index, self.field_index,
      "deserialize dispatched to wrong field ingredient: node field_index={field_index} self.field_index={}",
      self.field_index
    );

    // Allocate a session-local entry_id, shared across sibling fields
    let entry_id = *ctx
      .serialized_entry_id_to_session_local_map
      .entry((*name, *serialized_entry_id))
      .or_insert_with(|| self.id_counter.fetch_add(1, Ordering::Relaxed));

    let blob = ctx.serialized.query_cache.get(node_index)?;
    let mut data = blob;
    let value = T::decode(&mut data, &ctx.decoder);
    // Use the stored dep node fingerprint instead of recomputing via stable_hash
    // Recomputing needs sub-fields that may not be loaded yet (lazy deserialization)
    let fingerprint = if *stored_value_fingerprint == Fingerprint::SKIPPED {
      LazyFingerprint::new()
    } else {
      LazyFingerprint::from_stored(*stored_value_fingerprint)
    };
    self.data.insert(
      entry_id,
      StampedDerivedField {
        value,
        changed_at: *changed_at,
        fingerprint,
      },
    );

    let dep_id = self.ingredient_id.with_entry(entry_id);
    ctx.decoder.set_dep_node_id(node_index, dep_id);

    // Trigger deserialization of sibling fields so the whole struct is populated
    let group_key = (*name, *serialized_entry_id);
    if let Some(field_group) = ctx.derived_groups.get(&group_key) {
      for &(_, sibling_node_index) in &field_group.fields {
        ctx
          .decoder
          .get_or_deserialize_dep_node_id(sibling_node_index);
      }
    }

    Some(dep_id)
  }

  // Fields follow their parent queries, no independent LRU cap
  fn promote_cached(&self, ctx: &DeserializeContext) {
    let Some(indices) = ctx.fingerprint_reverse_map.get(&self.name_fingerprint()) else {
      return;
    };
    for &node_index in indices.iter() {
      let node = &ctx.serialized.dep_graph.nodes[node_index as usize];
      if let DepNode::DerivedField { field_index, .. } = node
        && *field_index == self.field_index
        && ctx.decoder.get_dep_node_id(node_index).is_none()
      {
        self.deserialize(ctx, node_index);
      }
    }
  }

  fn no_hash(&self) -> bool {
    self.no_hash_flag
  }
}
