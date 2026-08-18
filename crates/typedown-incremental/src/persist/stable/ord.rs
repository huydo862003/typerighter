use std::collections::{HashMap, HashSet};

use typedown_types::either::Either::{self};

use crate::QueryDatabase;

/// https://github.com/rust-lang/rust/blob/63f05e3635171e7ac3f9ca78bad6c71052cda5a3/compiler/rustc_data_structures/src/stable_hash.rs#L117-L132
/// Their original comment:
/// '''
/// Trait for marking a type as having a sort order that is
/// stable across compilation session boundaries. More formally:
///
/// ```txt
/// Ord::cmp(a1, b1) == Ord::cmp(a2, b2)
///    where a2 = decode(encode(a1, context1), context2)
///          b2 = decode(encode(b1, context1), context2)
/// ```
///
/// i.e. the result of `Ord::cmp` is not influenced by encoding
/// the values in one session and then decoding them in another
/// session.
///
/// This is trivially true for types where encoding and decoding
/// don't change the bytes of the values that are used during
/// comparison and comparison only depends on these bytes (as
/// opposed to some non-local state). Examples are u32, String,
/// Path, etc.
///
/// But it is not true for:
///  - `*const T` and `*mut T` because the values of these pointers
///    will change between sessions.
///  - `DefIndex`, `CrateNum`, `LocalDefId`, because their concrete
///    values depend on state that might be different between
///    compilation sessions.
///    '''
pub trait StableOrd: Ord {}

/// TIL: Ordering of a reference is exactly that of the referent
/// This is not the case for raw pointers though
impl<T: StableOrd> StableOrd for &T {}

// https://github.com/rust-lang/rust/blob/63f05e3635171e7ac3f9ca78bad6c71052cda5a3/compiler/rustc_data_structures/src/stable_hash.rs#L144-L148
/// Their original comment:
/// '''
/// This is a companion trait to `StableOrd`. Some types like `Symbol` can be
/// compared in a cross-session stable way, but their `Ord` implementation is
/// not stable. In such cases, a `StableOrd` implementation can be provided
/// to offer a lightweight way for stable sorting. (The more heavyweight option
/// is to sort via `ToStableHashKey`, but then sorting needs to have access to
/// a stable hashing context and `ToStableHashKey` can also be expensive as in
/// the case of `Symbol` where it has to allocate a `String`.)
///
/// See the documentation of [StableOrd] for how stable sort order is defined.
/// The same definition applies here. Be careful when implementing this trait.
/// '''
/// So StableCompare is weaker
pub trait StableCompare {
  fn stable_cmp<DB: QueryDatabase + ?Sized>(&self, db: &DB, other: &Self) -> std::cmp::Ordering;
}

impl<T: StableCompare> StableCompare for Box<T> {
  fn stable_cmp<DB: QueryDatabase + ?Sized>(&self, db: &DB, other: &Self) -> std::cmp::Ordering {
    (**self).stable_cmp(db, other)
  }
}

impl<T: StableCompare> StableCompare for Option<T> {
  fn stable_cmp<DB: QueryDatabase + ?Sized>(&self, db: &DB, other: &Self) -> std::cmp::Ordering {
    match (self, other) {
      (None, None) => std::cmp::Ordering::Equal,
      (None, Some(_)) => std::cmp::Ordering::Less,
      (Some(_), None) => std::cmp::Ordering::Greater,
      (Some(l), Some(r)) => l.stable_cmp(db, r),
    }
  }
}

impl<L: StableCompare, R: StableCompare> StableCompare for Either<L, R> {
  fn stable_cmp<DB: QueryDatabase + ?Sized>(&self, db: &DB, other: &Self) -> std::cmp::Ordering {
    match (self, other) {
      (Either::Left(_), Either::Right(_)) => std::cmp::Ordering::Less,
      (Either::Right(_), Either::Left(_)) => std::cmp::Ordering::Greater,
      (Either::Left(s), Either::Left(o)) => s.stable_cmp(db, o),
      (Either::Right(s), Either::Right(o)) => s.stable_cmp(db, o),
    }
  }
}

impl<K: StableCompare, V: StableCompare> StableCompare for HashMap<K, V> {
  fn stable_cmp<DB: QueryDatabase + ?Sized>(&self, db: &DB, other: &Self) -> std::cmp::Ordering {
    self.len().cmp(&other.len()).then_with(|| {
      let mut self_entries: Vec<_> = self.iter().collect();
      let mut other_entries: Vec<_> = other.iter().collect();
      self_entries.sort_by(|(k1, _), (k2, _)| k1.stable_cmp(db, k2));
      other_entries.sort_by(|(k1, _), (k2, _)| k1.stable_cmp(db, k2));
      self_entries
        .iter()
        .zip(other_entries.iter())
        .map(|((k1, v1), (k2, v2))| k1.stable_cmp(db, k2).then_with(|| v1.stable_cmp(db, v2)))
        .find(|o| o.is_ne())
        .unwrap_or(std::cmp::Ordering::Equal)
    })
  }
}

impl<V: StableCompare> StableCompare for HashSet<V> {
  fn stable_cmp<DB: QueryDatabase + ?Sized>(&self, db: &DB, other: &Self) -> std::cmp::Ordering {
    self.len().cmp(&other.len()).then_with(|| {
      let mut self_entries: Vec<_> = self.iter().collect();
      let mut other_entries: Vec<_> = other.iter().collect();
      self_entries.sort_by(|a, b| a.stable_cmp(db, b));
      other_entries.sort_by(|a, b| a.stable_cmp(db, b));
      self_entries
        .iter()
        .zip(other_entries.iter())
        .map(|(a, b)| a.stable_cmp(db, b))
        .find(|o| o.is_ne())
        .unwrap_or(std::cmp::Ordering::Equal)
    })
  }
}

impl<T: StableCompare> StableCompare for [T] {
  fn stable_cmp<DB: QueryDatabase + ?Sized>(&self, db: &DB, other: &Self) -> std::cmp::Ordering {
    let l1 = self.len();
    let l2 = other.len();
    for i in 0..l1.min(l2) {
      let ord = self[i].stable_cmp(db, &other[i]);
      if ord.is_ne() {
        return ord;
      }
    }
    l1.cmp(&l2)
  }
}

impl<T: StableCompare> StableCompare for Vec<T> {
  fn stable_cmp<DB: QueryDatabase + ?Sized>(&self, db: &DB, other: &Self) -> std::cmp::Ordering {
    self.as_slice().stable_cmp(db, other.as_slice())
  }
}

macro_rules! impl_stable_compare_via_ord {
  ($($ty:ty),*) => {
    $(
      impl StableCompare for $ty {
        fn stable_cmp<DB: QueryDatabase + ?Sized>(&self, _db: &DB, other: &Self) -> std::cmp::Ordering {
          self.cmp(other)
        }
      }
    )*
  };
}

impl_stable_compare_via_ord!(
  i8,
  i16,
  i32,
  i64,
  i128,
  isize,
  u8,
  u16,
  u32,
  u64,
  u128,
  usize,
  char,
  (),
  bool,
  String,
  std::path::PathBuf,
  std::time::SystemTime
);

impl StableCompare for &str {
  fn stable_cmp<DB: QueryDatabase + ?Sized>(&self, _db: &DB, other: &Self) -> std::cmp::Ordering {
    self.cmp(other)
  }
}

impl StableOrd for i8 {}
impl StableOrd for i16 {}
impl StableOrd for i32 {}
impl StableOrd for i64 {}
impl StableOrd for i128 {}
impl StableOrd for isize {}

impl StableOrd for u8 {}
impl StableOrd for u16 {}
impl StableOrd for u32 {}
impl StableOrd for u64 {}
impl StableOrd for u128 {}
impl StableOrd for usize {}

impl StableOrd for char {}
impl StableOrd for () {}
impl StableOrd for bool {}

impl<T: StableCompare> StableCompare for (T,) {
  fn stable_cmp<DB: QueryDatabase + ?Sized>(&self, db: &DB, other: &Self) -> std::cmp::Ordering {
    self.0.stable_cmp(db, &other.0)
  }
}

impl<T1: StableCompare, T2: StableCompare> StableCompare for (T1, T2) {
  fn stable_cmp<DB: QueryDatabase + ?Sized>(&self, db: &DB, other: &Self) -> std::cmp::Ordering {
    self
      .0
      .stable_cmp(db, &other.0)
      .then_with(|| self.1.stable_cmp(db, &other.1))
  }
}

impl<T1: StableCompare, T2: StableCompare, T3: StableCompare> StableCompare for (T1, T2, T3) {
  fn stable_cmp<DB: QueryDatabase + ?Sized>(&self, db: &DB, other: &Self) -> std::cmp::Ordering {
    self
      .0
      .stable_cmp(db, &other.0)
      .then_with(|| self.1.stable_cmp(db, &other.1))
      .then_with(|| self.2.stable_cmp(db, &other.2))
  }
}

impl<T1: StableCompare, T2: StableCompare, T3: StableCompare, T4: StableCompare> StableCompare
  for (T1, T2, T3, T4)
{
  fn stable_cmp<DB: QueryDatabase + ?Sized>(&self, db: &DB, other: &Self) -> std::cmp::Ordering {
    self
      .0
      .stable_cmp(db, &other.0)
      .then_with(|| self.1.stable_cmp(db, &other.1))
      .then_with(|| self.2.stable_cmp(db, &other.2))
      .then_with(|| self.3.stable_cmp(db, &other.3))
  }
}

impl StableCompare for f32 {
  fn stable_cmp<DB: QueryDatabase + ?Sized>(&self, _db: &DB, other: &Self) -> std::cmp::Ordering {
    self.total_cmp(other)
  }
}

impl StableCompare for f64 {
  fn stable_cmp<DB: QueryDatabase + ?Sized>(&self, _db: &DB, other: &Self) -> std::cmp::Ordering {
    self.total_cmp(other)
  }
}

impl StableOrd for str {}

impl<T: StableOrd> StableOrd for &[T] {}

impl StableOrd for String {}

impl StableOrd for std::ffi::OsStr {}
impl StableOrd for std::path::Path {}
impl StableOrd for std::path::PathBuf {}

impl StableOrd for std::time::SystemTime {}
