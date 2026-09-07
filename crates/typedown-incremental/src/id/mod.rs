//! Base Id trait and composite DepId for the incremental engine

mod derived;
mod input;
mod interned;

pub use derived::*;
pub use input::*;
pub use interned::*;

/// Kind of ingredient, encoded in the top 2 bits of DepId
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum IngredientKind {
  Input = 0,
  Interned = 1,
  Query = 2,
  Field = 3,
}

impl IngredientKind {
  pub const fn from_u8(v: u8) -> Self {
    match v {
      0 => Self::Input,
      1 => Self::Interned,
      2 => Self::Query,
      3 => Self::Field,
      _ => panic!("invalid IngredientKind"),
    }
  }
}

/// Composite dependency ID: 2-bit kind + 30-bit ingredient index + 32-bit entry id
///
/// Layout: [kind: 2] [ingredient_index: 30] [entry_id: 32]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct DepId(u64);

impl DepId {
  const KIND_SHIFT: u32 = 62;
  const INDEX_SHIFT: u32 = 32;
  const INDEX_MASK: u64 = 0x3FFF_FFFF; // 30 bits
  const ENTRY_MASK: u64 = 0xFFFF_FFFF; // 32 bits

  pub const fn new(kind: IngredientKind, ingredient_index: u32, entry_id: u32) -> Self {
    Self(
      ((kind as u64) << Self::KIND_SHIFT)
        | ((ingredient_index as u64) << Self::INDEX_SHIFT)
        | entry_id as u64,
    )
  }

  pub const fn from_prefix(prefix: u64, entry_id: u32) -> Self {
    Self(prefix | entry_id as u64)
  }

  pub const fn kind(self) -> IngredientKind {
    IngredientKind::from_u8((self.0 >> Self::KIND_SHIFT) as u8)
  }

  pub const fn ingredient_index(self) -> u32 {
    ((self.0 >> Self::INDEX_SHIFT) & Self::INDEX_MASK) as u32
  }

  pub const fn entry_id(self) -> u32 {
    (self.0 & Self::ENTRY_MASK) as u32
  }

  pub const fn prefix(kind: IngredientKind, ingredient_index: u32) -> u64 {
    ((kind as u64) << Self::KIND_SHIFT) | ((ingredient_index as u64) << Self::INDEX_SHIFT)
  }

}

pub type EntryId = u32;
pub type Revision = u32;

// Entry ID for evicted or deleted derived structs
pub const TOMBSTONE_ENTRY_ID: EntryId = EntryId::MAX;

pub trait Id {
  fn as_id(&self) -> DepId;
}
