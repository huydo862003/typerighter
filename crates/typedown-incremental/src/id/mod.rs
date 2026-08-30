//! Base Id trait for all query id types

mod derived;
mod input;
mod interned;

pub use derived::*;
pub use input::*;
pub use interned::*;

/// All query ids wrap a usize. This trait exposes it in an object-safe way.
/// Returns (type_tag, id) where type_tag is the ingredient start index
/// (unique per type) and id is the instance id within that type.
pub type DepId = (usize, usize);

// Entry ID for evicted or deleted derived structs
pub const TOMBSTONE_ENTRY_ID: usize = usize::MAX;

pub trait Id {
  fn as_id(&self) -> DepId;
}
