mod derived;
mod input;
mod interned;
mod inventory;

use std::any::Any;
use std::hash::{BuildHasher, Hasher};

pub use derived::*;
pub use input::*;
pub use interned::*;
pub use inventory::*;

use crate::persist::serialized::dep_graph::DepNodeIndex;
use crate::{
  DepId, DeserializeContext, EntryId, Fingerprint, QueryDatabase, Revision, SerializeContext,
};

// Identity hasher for u32 keys, passes the value through as-is
#[derive(Default)]
pub struct IdHasher(u64);
impl Hasher for IdHasher {
  fn write(&mut self, _: &[u8]) {
    unreachable!()
  }
  fn write_u32(&mut self, i: u32) {
    self.0 = i as u64;
  }
  fn finish(&self) -> u64 {
    self.0
  }
}

#[derive(Default, Clone)]
pub struct IdBuildHasher;
impl BuildHasher for IdBuildHasher {
  type Hasher = IdHasher;
  fn build_hasher(&self) -> IdHasher {
    IdHasher(0)
  }
}

/// DashMap with identity hasher, constrained to u32 keys
pub struct IdDashMap<V>(dashmap::DashMap<u32, V, IdBuildHasher>);

impl<V> IdDashMap<V> {
  pub fn new() -> Self {
    Self(dashmap::DashMap::with_hasher(IdBuildHasher))
  }
}

impl<V> std::ops::Deref for IdDashMap<V> {
  type Target = dashmap::DashMap<u32, V, IdBuildHasher>;
  fn deref(&self) -> &Self::Target {
    &self.0
  }
}

impl<V> Default for IdDashMap<V> {
  fn default() -> Self {
    Self::new()
  }
}

// A mapped reference into a DashMap entry, derefs to a projected field
pub struct MappedRef<'a, V, T> {
  _guard: dashmap::mapref::one::Ref<'a, u32, V>,
  ptr: *const T,
}

impl<V, T> std::ops::Deref for MappedRef<'_, V, T> {
  type Target = T;
  fn deref(&self) -> &T {
    // Safety: ptr points into _guard which is alive
    unsafe { &*self.ptr }
  }
}

impl<'a, V, T> MappedRef<'a, V, T> {
  pub fn new(guard: dashmap::mapref::one::Ref<'a, u32, V>, f: impl FnOnce(&V) -> &T) -> Self {
    let ptr = f(&*guard) as *const T;
    Self { _guard: guard, ptr }
  }
}

/// Shared base trait for all ingredient kinds
pub trait Ingredient: std::fmt::Debug + Any + Send + Sync {
  fn readable_name(&self) -> String;

  fn name_fingerprint(&self) -> Fingerprint;

  fn entry_ids(&self) -> Box<dyn Iterator<Item = EntryId> + '_>;

  fn value_fingerprint(&self, db: &dyn QueryDatabase, entry_id: EntryId) -> Option<Fingerprint>;

  /// Number of times the query function was actually invoked (not served from cache)
  #[cfg(debug_assertions)]
  fn recompute_count(&self) -> usize;
}

/// Input field ingredient (leaf node, ground truth)
pub trait InputIngredient: Ingredient {
  fn green_check(&self, entry_id: EntryId, last_changed_at: Revision) -> bool;

  fn serialize(&self, ctx: &mut SerializeContext, entry_id: EntryId);

  /// Load a dep node into this ingredient's storage
  fn deserialize(&self, ctx: &DeserializeContext, node_index: DepNodeIndex) -> Option<DepId>;

  fn field_index(&self) -> u8;
}

/// Interned ingredient (leaf node, never changes)
pub trait InternedIngredient: Ingredient {
  fn serialize(&self, ctx: &mut SerializeContext, entry_id: EntryId);

  /// Load a dep node into this ingredient's storage
  fn deserialize(&self, ctx: &DeserializeContext, node_index: DepNodeIndex) -> Option<DepId>;
}

/// Derived query ingredient (has deps, green check, recomputation)
pub trait DerivedQueryIngredient: Ingredient {
  fn reset_for_new_revision(&self);

  fn green_check(
    &self,
    db: &dyn QueryDatabase,
    entry_id: EntryId,
    last_changed_at: Revision,
  ) -> bool;

  fn re_execute(&self, db: &dyn QueryDatabase, entry_id: EntryId);

  fn serialize(&self, ctx: &mut SerializeContext, entry_id: EntryId);

  /// Load a dep node into this ingredient's storage
  fn deserialize(&self, ctx: &DeserializeContext, node_index: DepNodeIndex) -> Option<DepId>;

  /// Skip fingerprint computation, always mark dirty on reload
  fn no_hash(&self) -> bool {
    false
  }
}

/// Derived field ingredient (set by parent query, not independently recomputed)
pub trait DerivedFieldIngredient: Ingredient {
  fn remove_entry(&self, entry_id: EntryId);

  fn green_check(&self, entry_id: EntryId, last_changed_at: Revision) -> bool;

  fn serialize(&self, ctx: &mut SerializeContext, entry_id: EntryId);

  /// Load a dep node into this ingredient's storage
  fn deserialize(&self, ctx: &DeserializeContext, node_index: DepNodeIndex) -> Option<DepId>;

  fn field_index(&self) -> u8;

  /// Skip fingerprint computation, always mark dirty on reload
  fn no_hash(&self) -> bool {
    false
  }
}
