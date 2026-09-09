use std::sync::Arc;
use std::sync::atomic::{AtomicU32, Ordering};

use super::IdDashMap;

use crate::persist::serialized::dep_graph::{DepNode, DepNodeIndex};
use crate::{
  Decodable, DepId, DeserializeContext, Encodable, EntryId, Fingerprint, QueryDatabase, Revision,
  SerializeContext, StableHash, StableHasher, UnresolvedDepNode,
};

use super::{Ingredient, InputIngredient};

pub struct StampedInputField<T> {
  pub value: T,
  pub changed_at: Revision, // The last revision number this one changed
}

/// A field of an input ingredient, containing data for that input type
#[derive(Clone)]
#[doc(hidden)]
pub struct InputIngredientStore<T> {
  ingredient_id: DepId, // DepId with entry_id=0, identifies this ingredient
  field_index: u8,
  name: &'static str,
  pub id_counter: &'static AtomicU32,
  #[doc(hidden)]
  pub data: Arc<IdDashMap<StampedInputField<T>>>,
}

impl<T> std::fmt::Debug for InputIngredientStore<T> {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    f.debug_struct("InputIngredientStore")
      .field("name", &self.name)
      .finish_non_exhaustive()
  }
}

impl<T> InputIngredientStore<T> {
  #[cfg(debug_assertions)]
  #[doc(hidden)]
  pub const __TYPEDOWN_INPUT_FIELD_INGREDIENT: () = ();

  pub fn new(
    ingredient_id: DepId,
    name: &'static str,
    field_index: u8,
    id_counter: &'static AtomicU32,
  ) -> Self {
    Self {
      ingredient_id,
      field_index,
      name,
      id_counter,
      data: Arc::new(IdDashMap::default()),
    }
  }
}

impl<T: StableHash + std::fmt::Debug + Send + Sync + Encodable + Decodable + 'static> Ingredient
  for InputIngredientStore<T>
{
  #[cfg(debug_assertions)]
  fn readable_name(&self) -> String {
    self.name.to_string()
  }

  fn name_fingerprint(&self) -> Fingerprint {
    Fingerprint::from_name(self.name)
  }

  fn entry_ids(&self) -> Box<dyn Iterator<Item = EntryId> + '_> {
    Box::new(self.data.iter().map(|entry| *entry.key()))
  }

  fn value_fingerprint(&self, db: &dyn QueryDatabase, entry_id: EntryId) -> Option<Fingerprint> {
    self.data.get(&entry_id).map(|entry| {
      let mut hasher: StableHasher = StableHasher::new();
      entry.value.stable_hash(db, &mut hasher);
      Fingerprint::from_hasher(hasher)
    })
  }

  // Input fields are ground truth, they are never recomputed
  #[cfg(debug_assertions)]
  fn recompute_count(&self) -> usize {
    0
  }
}

impl<T: StableHash + std::fmt::Debug + Send + Sync + Encodable + Decodable + 'static>
  InputIngredient for InputIngredientStore<T>
{
  fn green_check(&self, entry_id: EntryId, last_changed_at: Revision) -> bool {
    self
      .data
      .get(&entry_id)
      .map(|entry| entry.changed_at <= last_changed_at)
      .unwrap_or(false)
  }

  fn field_index(&self) -> u8 {
    self.field_index
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

    // Look up or allocate a session-local entry_id shared across all fields of this input entry
    let entry_id = *ctx
      .entry_id_map
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

  fn serialize(&self, ctx: &mut SerializeContext, entry_id: EntryId) {
    let entry = self.data.get(&entry_id);
    if entry.is_none() {
      return;
    }

    let entry = entry.expect("Entry must contain a value after the none check pass");

    // Add the dep node
    let dep_id = self.ingredient_id.with_entry(entry_id);
    let node_index = ctx.encoder.add_dep_id(dep_id);
    ctx.dep_graph.set(
      node_index,
      UnresolvedDepNode::InputField {
        name: self.name_fingerprint(),
        field_index: self.field_index,
        entry_id,
        value: self
          .value_fingerprint(ctx.db(), entry_id)
          .expect("Entry is available so there must be a fingerprint"),
        changed_at: entry.changed_at,
      },
    );

    // Encode and write to query cache
    let mut buf = vec![];
    entry.value.encode(&mut buf, &mut ctx.encoder);
    ctx.query_cache.set(node_index, &buf);
  }
}
