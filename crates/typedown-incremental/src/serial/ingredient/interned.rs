use std::hash::Hash;
use std::sync::atomic::Ordering;

use crate::Ingredient;
use crate::serial::format::binary_files::dep_graph::{DepNode, DepNodeIndex};
use crate::serial::ingredient::serde::{DeserializeContext, SerializeContext, UnresolvedDepNode};
use crate::{Decodable, DepId, Encodable, Fingerprint, LazyFingerprint, StableHash};
use crate::{InternedIngredientStore, StampedInternedValue};

use crate::serial::Codec;

impl<
  T: StableHash + std::fmt::Debug + Encodable + Decodable + Eq + Hash + Clone + Send + Sync + 'static,
> Codec for InternedIngredientStore<T>
{
  fn serialize(&self, ctx: &mut SerializeContext, entry_id: crate::EntryId) {
    let Some(entry) = self.data.get(&entry_id) else {
      return;
    };

    let mut buf = vec![];
    entry.value.encode(&mut buf, &mut ctx.encoder);
    let blob_index = ctx.encoder.intern_blob::<T>(buf, Some(entry_id));

    let dep_id = self.ingredient_id.with_entry(entry_id);
    let node_index = ctx.encoder.add_dep_id(dep_id);
    ctx.dep_graph.set(
      node_index,
      UnresolvedDepNode::Interned {
        name: self.name_fingerprint(),
        entry_id,
        blob_index,
      },
    );
  }

  fn deserialize(&self, ctx: &DeserializeContext, node_index: DepNodeIndex) -> Option<DepId> {
    if let Some(dep_id) = ctx.decoder.get_dep_node_id(node_index) {
      return Some(dep_id);
    }
    let node = &ctx.serialized.dep_graph.nodes[node_index as usize];
    let DepNode::Interned {
      entry_id: serialized_entry_id,
      blob_index,
      ..
    } = node
    else {
      return None;
    };
    let blob = ctx.decoder.get_intern_blob(*blob_index);
    let mut data = blob;
    let value = T::decode(&mut data, &ctx.decoder);
    let id_counter = self.id_counter;
    let entry_id = *self
      .intern_map
      .entry(value.clone())
      .or_insert_with(|| id_counter.fetch_add(1, Ordering::Relaxed));
    self.data.entry(entry_id).or_insert(StampedInternedValue {
      value,
      fingerprint: LazyFingerprint::new(),
    });
    let dep_id = self.ingredient_id.with_entry(entry_id);
    ctx.decoder.set_dep_node_id(node_index, dep_id);
    let name = Fingerprint::from_name(self.name);
    ctx
      .serialized_entry_id_to_session_local_map
      .entry((name, *serialized_entry_id))
      .or_insert(entry_id);
    Some(dep_id)
  }
}
