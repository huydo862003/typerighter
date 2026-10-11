use super::derived::symbol::Symbol;

/// A step in a static access path from an anchor to a nested field
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum PathStep {
  /// A field access by name (e.g. `address` in `address.street`)
  Field(String),
  /// A sequence index access
  Index,
}

/// A static access path rooted at an owner symbol
#[derive(Clone, Debug)]
pub struct StaticAccessPath {
  pub owner: Symbol,
  pub steps: Vec<PathStep>,
}
