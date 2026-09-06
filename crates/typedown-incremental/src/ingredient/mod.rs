mod derived;
mod input;
mod interned;
mod inventory;

use std::any::Any;

pub use derived::*;
pub use input::*;
pub use interned::*;
pub use inventory::*;

use crate::persist::serialized::dep_graph::DepNodeIndex;
use crate::{DepId, DeserializeContext, Fingerprint, QueryDatabase, SerializeContext};

/// Shared base trait for all ingredient kinds
pub trait Ingredient: std::fmt::Debug + Any + Send + Sync {
  #[cfg(debug_assertions)]
  fn readable_name(&self) -> String;

  fn name_fingerprint(&self) -> Fingerprint;

  fn entry_ids(&self) -> Box<dyn Iterator<Item = u32> + '_>;

  #[cfg(debug_assertions)]
  fn recompute_count(&self) -> usize;
}

/// Input field ingredient (leaf node, ground truth)
pub trait InputIngredient: Ingredient {
  fn green_check(&self, arg_id: u32, last_changed_at: u32) -> bool;

  fn value_fingerprint(&self, db: &dyn QueryDatabase, entry_id: u32) -> Option<Fingerprint>;

  fn serialize(&self, ctx: &mut SerializeContext, entry_id: u32);

  fn deserialize(&self, ctx: &DeserializeContext, node_index: DepNodeIndex) -> Option<DepId>;

  fn field_index(&self) -> u8;
}

/// Interned ingredient (leaf node, never changes)
pub trait InternedIngredient: Ingredient {
  fn value_fingerprint(&self, db: &dyn QueryDatabase, entry_id: u32) -> Option<Fingerprint>;

  fn serialize(&self, ctx: &mut SerializeContext, entry_id: u32);

  fn deserialize(&self, ctx: &DeserializeContext, node_index: DepNodeIndex) -> Option<DepId>;
}

/// Derived query ingredient (has deps, green check, recomputation)
pub trait DerivedQueryIngredient: Ingredient {
  fn reset_for_new_revision(&self);

  fn green_check(&self, db: &dyn QueryDatabase, arg_id: u32, last_changed_at: u32) -> bool;

  fn re_execute(&self, db: &dyn QueryDatabase, arg_id: u32);

  fn value_fingerprint(&self, db: &dyn QueryDatabase, entry_id: u32) -> Option<Fingerprint>;

  fn serialize(&self, ctx: &mut SerializeContext, entry_id: u32);

  fn deserialize(&self, ctx: &DeserializeContext, node_index: DepNodeIndex) -> Option<DepId>;

  fn no_hash(&self) -> bool {
    false
  }
}

/// Derived field ingredient (set by parent query, not independently recomputed)
pub trait DerivedFieldIngredient: Ingredient {
  fn remove_entry(&self, entry_id: u32);

  fn green_check(&self, arg_id: u32, last_changed_at: u32) -> bool;

  fn value_fingerprint(&self, db: &dyn QueryDatabase, entry_id: u32) -> Option<Fingerprint>;

  fn serialize(&self, ctx: &mut SerializeContext, entry_id: u32);

  fn deserialize(&self, ctx: &DeserializeContext, node_index: DepNodeIndex) -> Option<DepId>;

  fn field_index(&self) -> u8;

  fn no_hash(&self) -> bool {
    false
  }
}
