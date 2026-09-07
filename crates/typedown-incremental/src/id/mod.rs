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

/// Composite dependency ID packing three concepts:
/// - `ingredient_kind` (2 bits): Input, Interned, Query, or Field
/// - `ingredient_id` (30 bits): index within the kind's array
/// - `entry_id` (32 bits): entry within the ingredient
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct DepId(u64);

impl DepId {
  const INGREDIENT_ID_BITS: u32 = 30;
  const ENTRY_ID_BITS: u32 = 32;

  const INGREDIENT_KIND_SHIFT: u32 = Self::INGREDIENT_ID_BITS + Self::ENTRY_ID_BITS;
  const INGREDIENT_ID_SHIFT: u32 = Self::ENTRY_ID_BITS;
  const INGREDIENT_ID_MASK: u64 = (1 << Self::INGREDIENT_ID_BITS) - 1;
  const ENTRY_ID_MASK: u64 = (1 << Self::ENTRY_ID_BITS) - 1;

  pub const fn new(kind: IngredientKind, ingredient_id: u32, entry_id: u32) -> Self {
    Self(
      ((kind as u64) << Self::INGREDIENT_KIND_SHIFT)
        | ((ingredient_id as u64) << Self::INGREDIENT_ID_SHIFT)
        | entry_id as u64,
    )
  }

  /// Construct a DepId that identifies an ingredient (entry_id=0)
  pub const fn ingredient(kind: IngredientKind, ingredient_id: u32) -> Self {
    Self(((kind as u64) << Self::INGREDIENT_KIND_SHIFT) | ((ingredient_id as u64) << Self::INGREDIENT_ID_SHIFT))
  }

  /// Derive a full DepId by combining this ingredient identity with an entry_id
  pub const fn with_entry(self, entry_id: u32) -> Self {
    Self((self.0 & !Self::ENTRY_ID_MASK) | entry_id as u64)
  }

  pub const fn kind(self) -> IngredientKind {
    IngredientKind::from_u8((self.0 >> Self::INGREDIENT_KIND_SHIFT) as u8)
  }

  pub const fn ingredient_id(self) -> u32 {
    ((self.0 >> Self::INGREDIENT_ID_SHIFT) & Self::INGREDIENT_ID_MASK) as u32
  }

  pub const fn entry_id(self) -> u32 {
    (self.0 & Self::ENTRY_ID_MASK) as u32
  }

  pub const fn as_u64(self) -> u64 {
    self.0
  }

  pub const fn from_u64(v: u64) -> Self {
    Self(v)
  }
}

pub type EntryId = u32;
pub type Revision = u32;

// Entry ID for evicted or deleted derived structs
pub const TOMBSTONE_ENTRY_ID: EntryId = EntryId::MAX;

pub trait Id {
  fn as_id(&self) -> DepId;
}
