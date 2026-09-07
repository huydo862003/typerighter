//! Interned id for the incremental database

/// A fast id for an interned state
pub trait InternedId: super::Id + From<u32> + Into<u32> {
  /// Marker used by macros to verify a type implements InternedId at compile time.
  #[cfg(debug_assertions)]
  #[doc(hidden)]
  const __TYPEDOWN_INTERNED_ID: () = ();

  fn iter<DB: crate::QueryDatabase + ?Sized>(db: &DB) -> Vec<Self>
  where
    Self: Sized;
}
