//! Derived query engine for the incremental database

use super::Id;

/// A fast id for a derived state
/// Derived id is bound to a database's lifetime
pub trait DerivedId: Id + From<u32> + Into<u32> {
  /// Marker used by macros to verify a type implements DerivedId at compile time.
  #[cfg(debug_assertions)]
  #[doc(hidden)]
  const __TYPEDOWN_DERIVED_ID: () = ();
}
