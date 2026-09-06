use std::hash::Hash;
use std::sync::Arc;
use std::sync::atomic::{AtomicU32, Ordering};

use dashmap::DashMap;

use crate::persist::serialized::dep_graph::{DepNode, DepNodeIndex};
use crate::{
  Decodable, DepId, DeserializeContext, Encodable, Fingerprint, QueryDatabase, SerializeContext,
  StableHash, StableHasher, UnresolvedDepNode,
};

use super::{Ingredient, InternedIngredient};

/// An ingredient for an interned struct
#[derive(Clone)]
#[doc(hidden)]
pub struct InternedIngredientStore<T: 'static> {
  dep_id_prefix: u64,
  name: &'static str,
  pub(crate) id_counter: &'static AtomicU32,
  pub(crate) intern_map: &'static DashMap<T, u32>,
  #[doc(hidden)]
  pub data: Arc<DashMap<u32, T>>,
}

impl<T: 'static> std::fmt::Debug for InternedIngredientStore<T> {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    f.debug_struct("InternedIngredientStore")
      .field("name", &self.name)
      .finish_non_exhaustive()
  }
}

impl<T: 'static> InternedIngredientStore<T> {
  #[cfg(debug_assertions)]
  #[doc(hidden)]
  pub const __TYPEDOWN_INTERNED_INGREDIENT: () = ();

  pub fn new(
    dep_id_prefix: u64,
    name: &'static str,
    id_counter: &'static AtomicU32,
    intern_map: &'static DashMap<T, u32>,
  ) -> Self {
    Self {
      dep_id_prefix,
      name,
      id_counter,
      intern_map,
      data: Arc::new(DashMap::new()),
    }
  }
}


impl<
  T: StableHash
    + std::fmt::Debug
    + Encodable
    + Decodable
    + Eq
    + Hash
    + Clone
    + Send
    + Sync
    + 'static,
> Ingredient for InternedIngredientStore<T>
{
  #[cfg(debug_assertions)]
  fn readable_name(&self) -> String {
    self.name.to_string()
  }

  fn name_fingerprint(&self) -> Fingerprint {
    Fingerprint::from_name(self.name)
  }


  fn entry_ids(&self) -> Box<dyn Iterator<Item = u32> + '_> {
    Box::new(self.data.iter().map(|entry| *entry.key()))
  }

  #[cfg(debug_assertions)]
  fn recompute_count(&self) -> usize {
    0
  }
}

impl<
  T: StableHash
    + std::fmt::Debug
    + Encodable
    + Decodable
    + Eq
    + Hash
    + Clone
    + Send
    + Sync
    + 'static,
> InternedIngredient for InternedIngredientStore<T>
{
  fn value_fingerprint(&self, db: &dyn QueryDatabase, entry_id: u32) -> Option<Fingerprint> {
    self.data.get(&entry_id).map(|entry| {
      let mut hasher = StableHasher::new();
      entry.value().stable_hash(db, &mut hasher);
      Fingerprint::from_hasher(hasher)
    })
  }

  fn deserialize(&self, ctx: &DeserializeContext, node_index: DepNodeIndex) -> Option<DepId> {
    if let Some(dep_id) = ctx.decoder.get_dep_node_id(node_index) {
      return Some(dep_id);
    }
    let node = &ctx.serialized.dep_graph.nodes[node_index as usize];
    let DepNode::Interned { blob_index, .. } = node else {
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
    self.data.entry(entry_id).or_insert(value);
    let dep_id = DepId::from_prefix(self.dep_id_prefix, entry_id);
    ctx.decoder.set_dep_node_id(node_index, dep_id);
    Some(dep_id)
  }

  fn serialize(&self, ctx: &mut SerializeContext, entry_id: u32) {
    let Some(entry) = self.data.get(&entry_id) else {
      return;
    };

    let mut buf = vec![];
    entry.value().encode(&mut buf, &mut ctx.encoder);
    let blob_index = ctx.encoder.intern_blob::<T>(buf, Some(entry_id));

    let dep_id = DepId::from_prefix(self.dep_id_prefix, entry_id);
    let node_index = ctx.encoder.add_dep_id(dep_id);
    ctx.dep_graph.set(
      node_index,
      UnresolvedDepNode::Interned {
        name: self.name_fingerprint(),
        blob_index,
      },
    );
  }
}
