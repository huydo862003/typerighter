use std::sync::Arc;
use std::sync::atomic::AtomicU32;

use crate::{
  Decodable, DepId, Encodable, EntryId, Fingerprint, LazyFingerprint, QueryDatabase, Revision,
  StableHash,
};

use super::{DerivedFieldIngredient, IdDashMap, Ingredient};

/// A stamped field value for a derived struct
pub struct StampedDerivedField<T> {
  pub value: T,
  pub changed_at: Revision,
  pub fingerprint: LazyFingerprint,
}

/// Ingredient for a derived struct field: maps entry id to stamped value
#[derive(Clone)]
#[doc(hidden)]
pub struct DerivedFieldIngredientStore<T> {
  pub(crate) ingredient_id: DepId,
  pub(crate) field_index: u8,
  // Parent struct name, shared across sibling fields so they map to the same entry ID
  pub(crate) struct_name: &'static str,
  pub id_counter: &'static AtomicU32,
  #[doc(hidden)]
  pub data: Arc<IdDashMap<StampedDerivedField<T>>>,
  pub no_hash_flag: bool,
}

impl<T> std::fmt::Debug for DerivedFieldIngredientStore<T> {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    f.debug_struct("DerivedFieldIngredientStore")
      .field("struct_name", &self.struct_name)
      .finish_non_exhaustive()
  }
}

impl<T> DerivedFieldIngredientStore<T> {
  #[cfg(debug_assertions)]
  #[doc(hidden)]
  pub const __TYPEDOWN_DERIVED_FIELD_INGREDIENT: () = ();

  pub fn new(
    ingredient_id: DepId,
    struct_name: &'static str,
    field_index: u8,
    id_counter: &'static AtomicU32,
  ) -> Self {
    Self {
      ingredient_id,
      field_index,
      struct_name,
      id_counter,
      data: Arc::new(IdDashMap::default()),
      no_hash_flag: false,
    }
  }
}

impl<T: StableHash + std::fmt::Debug + Encodable + Decodable + Send + Sync + 'static> Ingredient
  for DerivedFieldIngredientStore<T>
{
  fn readable_name(&self) -> String {
    self.struct_name.to_string()
  }

  fn name_fingerprint(&self) -> Fingerprint {
    Fingerprint::from_name(self.struct_name)
  }

  fn entry_ids(&self) -> Box<dyn Iterator<Item = EntryId> + '_> {
    Box::new(self.data.iter().map(|entry| *entry.key()))
  }

  fn value_fingerprint(&self, db: &dyn QueryDatabase, entry_id: EntryId) -> Option<Fingerprint> {
    if self.no_hash_flag {
      return None;
    }
    // Deserialized fields have fingerprint pre-populated from the dep node, get_or_compute is a no-op
    // Freshly computed fields compute stable_hash here with all deps in memory
    self
      .data
      .get(&entry_id)
      .map(|entry| entry.fingerprint.get_or_compute(db, &entry.value))
  }

  // Derived fields are set by their parent query, not independently recomputed
  #[cfg(debug_assertions)]
  fn recompute_count(&self) -> usize {
    0
  }
}

impl<T: StableHash + std::fmt::Debug + Encodable + Decodable + Send + Sync + 'static>
  DerivedFieldIngredient for DerivedFieldIngredientStore<T>
{
  fn remove_entry(&self, entry_id: EntryId) {
    self.data.remove(&entry_id);
  }

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
