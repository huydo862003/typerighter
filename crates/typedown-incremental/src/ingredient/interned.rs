// TIL: We use DashMap to support high-performance concurrent reads, which fits the workload of IDEs
use std::hash::Hash;
use std::sync::Arc;
use std::sync::atomic::AtomicU32;

use super::IdDashMap;
use dashmap::DashMap;

use crate::{
  Decodable, DepId, Encodable, EntryId, Fingerprint, LazyFingerprint, QueryDatabase, StableHash,
};

use super::{Ingredient, InternedIngredient};

pub struct StampedInternedValue<T> {
  pub value: T,
  pub fingerprint: LazyFingerprint,
}

/// An ingredient for an interned struct
#[derive(Clone)]
#[doc(hidden)]
pub struct InternedIngredientStore<T: 'static> {
  pub(crate) ingredient_id: DepId, // A DepId with entry_id = 0, identifies this ingredient
  pub(crate) name: &'static str,
  pub(crate) id_counter: &'static AtomicU32,
  pub(crate) intern_map: &'static DashMap<T, EntryId>,
  #[doc(hidden)]
  pub data: Arc<IdDashMap<StampedInternedValue<T>>>,
}

impl<T: 'static> std::fmt::Debug for InternedIngredientStore<T> {
  fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    formatter
      .debug_struct("InternedIngredientStore")
      .field("name", &self.name)
      .finish_non_exhaustive()
  }
}

impl<T: 'static> InternedIngredientStore<T> {
  #[cfg(debug_assertions)]
  #[doc(hidden)]
  pub const __TYPEDOWN_INTERNED_INGREDIENT: () = ();

  pub fn new(
    ingredient_id: DepId, // A DepId with entry_id = 0, identifies this ingredient
    name: &'static str,
    id_counter: &'static AtomicU32,
    intern_map: &'static DashMap<T, u32>,
  ) -> Self {
    Self {
      ingredient_id,
      name,
      id_counter,
      intern_map,
      data: Arc::new(IdDashMap::default()),
    }
  }
}

impl<
  T: StableHash + std::fmt::Debug + Encodable + Decodable + Eq + Hash + Clone + Send + Sync + 'static,
> Ingredient for InternedIngredientStore<T>
{
  fn readable_name(&self) -> String {
    self.name.to_string()
  }

  fn name_fingerprint(&self) -> Fingerprint {
    Fingerprint::from_name(self.name)
  }

  fn entry_ids(&self) -> Box<dyn Iterator<Item = u32> + '_> {
    Box::new(self.data.iter().map(|entry| *entry.key()))
  }

  fn value_fingerprint(&self, db: &dyn QueryDatabase, entry_id: u32) -> Option<Fingerprint> {
    self
      .data
      .get(&entry_id)
      .map(|entry| entry.fingerprint.get_or_compute(db, &entry.value))
  }

  #[cfg(debug_assertions)]
  fn recompute_count(&self) -> usize {
    0
  }
}

impl<
  T: StableHash + std::fmt::Debug + Encodable + Decodable + Eq + Hash + Clone + Send + Sync + 'static,
> InternedIngredient for InternedIngredientStore<T>
{
}
