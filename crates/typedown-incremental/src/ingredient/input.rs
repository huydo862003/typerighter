// TIL: We use DashMap to support high-performance concrruent reads, which fits the workload of IDEs
use std::sync::Arc;
use std::sync::atomic::AtomicUsize;

use dashmap::DashMap;

use crate::persist::serialized::dep_graph::{DepNode, DepNodeIndex};
use crate::{
  Decodable, DepId, DeserializeContext, Encodable, Fingerprint, QueryDatabase, SerializeContext,
  StableHash, StableHasher, UnresolvedDepNode,
};

use super::Ingredient;

pub struct StampedInputField<T> {
  pub value: T,
  pub changed_at: usize, // The last revision number this one changed
}

/// A field of an input ingredient, containing data for that input type
#[derive(Clone)]
#[doc(hidden)]
pub struct InputFieldIngredient<T> {
  ingredient_index: usize,
  field_index: u8,
  name: &'static str,
  pub id_counter: &'static AtomicUsize,
  #[doc(hidden)]
  pub data: Arc<DashMap<usize, StampedInputField<T>>>,
}

impl<T> InputFieldIngredient<T> {
  #[cfg(debug_assertions)]
  #[doc(hidden)]
  pub const __TYPEDOWN_INPUT_FIELD_INGREDIENT: () = ();

  pub fn new(
    ingredient_index: usize,
    name: &'static str,
    field_index: u8,
    id_counter: &'static AtomicUsize,
  ) -> Self {
    Self {
      ingredient_index,
      field_index,
      name,
      id_counter,
      data: Arc::new(DashMap::new()),
    }
  }
}

impl<T: StableHash + Send + Sync + Encodable + Decodable + 'static> Ingredient
  for InputFieldIngredient<T>
{
  #[cfg(debug_assertions)]
  fn readable_name(&self) -> String {
    self.name.to_string()
  }

  fn name_fingerprint(&self) -> Fingerprint {
    Fingerprint::from_name(self.name)
  }

  fn green_check(&self, _db: &dyn QueryDatabase, arg_id: usize, last_changed_at: usize) -> bool {
    self
      .data
      .get(&arg_id)
      .map(|entry| entry.changed_at <= last_changed_at)
      .unwrap_or(false)
  }

  fn re_execute(&self, _db: &dyn QueryDatabase, _arg_id: usize) {
    // Inputs are ground truth, nothing to recompute
  }

  fn entry_ids(&self) -> Box<dyn Iterator<Item = usize> + '_> {
    Box::new(self.data.iter().map(|entry| *entry.key()))
  }

  fn value_fingerprint(&self, db: &dyn QueryDatabase, entry_id: usize) -> Option<Fingerprint> {
    InputFieldIngredient::value_fingerprint(self, db, entry_id)
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

    // Look up or allocate a session-local entry_id shared across all fields of this input entry.
    let entry_id = *ctx
      .entry_id_map
      .entry((*name, *serialized_entry_id))
      .or_insert_with(|| {
        self
          .id_counter
          .fetch_add(1, std::sync::atomic::Ordering::Relaxed)
      });

    let blob = ctx.serialized.query_cache.get(node_index)?;
    let mut data = blob;
    let value = T::decode(&mut data, &ctx.decoder);
    self.data.insert(
      entry_id,
      StampedInputField {
        value,
        changed_at: *changed_at as usize,
      },
    );
    let dep_id = (self.ingredient_index, entry_id);
    ctx.decoder.set_dep_node_id(node_index, dep_id);
    Some(dep_id)
  }

  fn serialize(&self, ctx: &mut SerializeContext, entry_id: usize) {
    let entry = self.data.get(&entry_id);
    if entry.is_none() {
      return;
    }

    let entry = entry.expect("Entry must contain a value after the none check pass");

    // Add the dep node
    let dep_id = (self.ingredient_index, entry_id);
    let node_index = ctx.encoder.add_dep_id(dep_id);
    ctx.dep_graph.set(
      node_index,
      UnresolvedDepNode::InputField {
        name: self.name_fingerprint(),
        field_index: self.field_index,
        entry_id: entry_id as u64,
        value: self
          .value_fingerprint(ctx.db(), entry_id)
          .expect("Entry is available so there must be a fingerprint"),
        changed_at: entry.changed_at as u64,
      },
    );

    // Encode and write to query cache
    let mut buf = vec![];
    entry.value.encode(&mut buf, &mut ctx.encoder);
    ctx.query_cache.set(node_index, &buf);
  }

  // Input fields are ground truth, they are never recomputed
  #[cfg(debug_assertions)]
  fn recompute_count(&self) -> usize {
    0
  }
}

impl<T: StableHash + Send + Sync + 'static> InputFieldIngredient<T> {
  pub fn value_fingerprint(&self, db: &dyn QueryDatabase, arg_id: usize) -> Option<Fingerprint> {
    self.data.get(&arg_id).map(|entry| {
      let mut hasher: StableHasher = StableHasher::new();
      entry.value.stable_hash(db, &mut hasher);
      Fingerprint::from_hasher(hasher)
    })
  }
}
