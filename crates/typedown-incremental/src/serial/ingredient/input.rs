use std::sync::atomic::Ordering;

use crate::Ingredient;
use crate::serial::format::binary_files::dep_graph::{DepNode, DepNodeIndex};
use crate::serial::ingredient::serde::{DeserializeContext, SerializeContext, UnresolvedDepNode};
use crate::{Decodable, DepId, Encodable, StableHash};
use crate::{InputIngredientStore, StampedInputField};

use crate::serial::Codec;

impl<T: StableHash + std::fmt::Debug + Send + Sync + Encodable + Decodable + 'static> Codec
  for InputIngredientStore<T>
{
  fn serialize(&self, ctx: &mut SerializeContext, entry_id: crate::EntryId) {
    let Some(entry) = self.data.get(&entry_id) else {
      return;
    };

    let dep_id = self.ingredient_id.with_entry(entry_id);
    let node_index = ctx.encoder.add_dep_id(dep_id);
    ctx.dep_graph.set(
      node_index,
      UnresolvedDepNode::InputField {
        name: self.name_fingerprint(),
        field_index: self.field_index,
        entry_id,
        value: self
          .value_fingerprint(ctx.encoder.db(), entry_id)
          .expect("entry must have a fingerprint at serialize time"),
        changed_at: entry.changed_at,
      },
    );

    let mut buf = vec![];
    entry.value.encode(&mut buf, &mut ctx.encoder);
    ctx.query_cache.set(node_index, &buf);
  }

  fn deserialize(&self, ctx: &DeserializeContext, node_index: DepNodeIndex) -> Option<DepId> {
    if let Some(dep_id) = ctx.decoder.get_dep_node_id(node_index) {
      return Some(dep_id);
    }
    let node = &ctx.serialized.dep_graph.nodes[node_index as usize];
    let DepNode::InputField {
      name,
      entry_id: serialized_entry_id,
      changed_at,
      ..
    } = node
    else {
      return None;
    };

    let entry_id = *ctx
      .serialized_entry_id_to_session_local_map
      .entry((*name, *serialized_entry_id))
      .or_insert_with(|| self.id_counter.fetch_add(1, Ordering::Relaxed));

    let blob = ctx.serialized.query_cache.get(node_index)?;
    let mut data = blob;
    let value = T::decode(&mut data, &ctx.decoder);
    self.data.insert(
      entry_id,
      StampedInputField {
        value,
        changed_at: *changed_at,
      },
    );
    let dep_id = self.ingredient_id.with_entry(entry_id);
    ctx.decoder.set_dep_node_id(node_index, dep_id);
    Some(dep_id)
  }
}
