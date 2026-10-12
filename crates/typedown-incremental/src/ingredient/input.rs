use std::sync::Arc;
use std::sync::atomic::AtomicU32;

use crate::{
  Decodable, DepId, Encodable, EntryId, Fingerprint, QueryDatabase, Revision, StableHash,
  StableHasher,
};

use super::{IdDashMap, Ingredient, InputIngredient};

pub struct StampedInputField<T> {
  pub value: T,
  pub changed_at: Revision,
}

/// A field of an input ingredient, containing data for that input type
#[derive(Clone)]
#[doc(hidden)]
pub struct InputIngredientStore<T> {
  pub(crate) ingredient_id: DepId, // A DepId with entry_id = 0, identifies this ingredient
  pub(crate) field_index: u8,
  pub(crate) name: &'static str,
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
      let mut hasher = StableHasher::new();
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
}
