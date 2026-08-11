use std::hash::Hash;
use std::sync::Arc;
use std::sync::atomic::AtomicUsize;

use dashmap::DashMap;

use crate::persist::serialized::dep_graph::{DepNode, DepNodeIndex};
use crate::{
  Decodable, DepId, DeserializeContext, Encodable, Fingerprint, QueryDatabase, SerializeContext,
  StableHash, StableHasher, UnresolvedDepNode,
};

use super::Ingredient;

/// An ingredient for an interned struct
#[derive(Clone)]
#[doc(hidden)]
pub struct InternedIngredient<T: 'static> {
  ingredient_index: usize,
  name: &'static str,
  pub(crate) id_counter: &'static AtomicUsize,
  pub(crate) intern_map: &'static DashMap<T, usize>,
  #[doc(hidden)]
  pub data: Arc<DashMap<usize, T>>,
}

impl<T: 'static> InternedIngredient<T> {
  #[cfg(debug_assertions)]
  #[doc(hidden)]
  pub const __TYPEDOWN_INTERNED_INGREDIENT: () = ();

  pub fn new(
    ingredient_index: usize,
    name: &'static str,
    id_counter: &'static AtomicUsize,
    intern_map: &'static DashMap<T, usize>,
  ) -> Self {
    Self {
      ingredient_index,
      name,
      id_counter,
      intern_map,
      data: Arc::new(DashMap::new()),
    }
  }
}

impl<T: StableHash + Send + Sync + 'static> InternedIngredient<T> {
  pub fn value_fingerprint(&self, db: &dyn QueryDatabase, arg_id: usize) -> Option<Fingerprint> {
    self.data.get(&arg_id).map(|entry| {
      let mut hasher = StableHasher::new();
      entry.value().stable_hash(db, &mut hasher);
      Fingerprint::from_hasher(hasher)
    })
  }
}

impl<T: StableHash + Encodable + Decodable + Eq + Hash + Clone + Send + Sync + 'static> Ingredient
  for InternedIngredient<T>
{
  #[cfg(debug_assertions)]
  fn readable_name(&self) -> String {
    self.name.to_string()
  }

  fn name_fingerprint(&self) -> Fingerprint {
    Fingerprint::from_name(self.name)
  }

  fn green_check(&self, _db: &dyn QueryDatabase, _arg_id: usize, _last_changed_at: usize) -> bool {
    // Interned values never change, always green
    true
  }

  fn re_execute(&self, _db: &dyn QueryDatabase, _arg_id: usize) {
    // Interned values never change, nothing to recompute
  }

  fn entry_ids(&self) -> Box<dyn Iterator<Item = usize> + '_> {
    Box::new(self.data.iter().map(|entry| *entry.key()))
  }

  fn value_fingerprint(&self, db: &dyn QueryDatabase, entry_id: usize) -> Option<Fingerprint> {
    InternedIngredient::value_fingerprint(self, db, entry_id)
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
      .or_insert_with(|| id_counter.fetch_add(1, std::sync::atomic::Ordering::Relaxed));
    self.data.entry(entry_id).or_insert(value);
    let dep_id = (self.ingredient_index, entry_id);
    ctx.decoder.set_dep_node_id(node_index, dep_id);
    Some(dep_id)
  }

  fn serialize(&self, ctx: &mut SerializeContext, entry_id: usize) {
    let Some(entry) = self.data.get(&entry_id) else {
      return;
    };

    // Encode the value to register it in the encoder's intern table
    let mut buf = vec![];
    entry.value().encode(&mut buf, &mut ctx.encoder);
    let blob_index = ctx.encoder.intern_blob::<T>(buf, Some(entry_id));

    let dep_id = (self.ingredient_index, entry_id);
    let node_index = ctx.encoder.add_dep_id(dep_id);
    ctx.dep_graph.set(
      node_index,
      UnresolvedDepNode::Interned {
        name: self.name_fingerprint(),
        blob_index,
      },
    );
  }

  // Interned values are never recomputed
  #[cfg(debug_assertions)]
  fn recompute_count(&self) -> usize {
    0
  }
}
