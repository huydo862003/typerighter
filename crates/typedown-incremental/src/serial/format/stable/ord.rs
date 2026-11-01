use std::{
  cmp::Ordering,
  collections::{BTreeMap, BTreeSet, HashMap, HashSet},
};

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
/// session
///
/// This is trivially true for types where encoding and decoding
/// don't change the bytes of the values that are used during
/// comparison and comparison only depends on these bytes (as
/// opposed to some non-local state). Examples are u32, String,
/// Path, etc
///
/// But it is not true for:
///  - `*const T` and `*mut T` because the values of these pointers
///    will change between sessions
///  - `DefIndex`, `CrateNum`, `LocalDefId`, because their concrete
///    values depend on state that might be different between
///    compilation sessions
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
/// See the documentation of [StableOrd] for how stable sort order is defined
/// The same definition applies here. Be careful when implementing this trait
/// '''
/// So StableCompare is weaker
pub trait StableCompare {
  fn stable_cmp<DB: QueryDatabase + ?Sized>(&self, db: &DB, other: &Self) -> Ordering;
}

impl<T: StableCompare> StableCompare for Box<T> {
  fn stable_cmp<DB: QueryDatabase + ?Sized>(&self, db: &DB, other: &Self) -> Ordering {
    (**self).stable_cmp(db, other)
  }
}

impl<T: StableCompare> StableCompare for Option<T> {
  fn stable_cmp<DB: QueryDatabase + ?Sized>(&self, db: &DB, other: &Self) -> Ordering {
    match (self, other) {
      (None, None) => Ordering::Equal,
      (None, Some(_)) => Ordering::Less,
      (Some(_), None) => Ordering::Greater,
      (Some(left), Some(right)) => left.stable_cmp(db, right),
    }
  }
}

impl<L: StableCompare, R: StableCompare> StableCompare for Either<L, R> {
  fn stable_cmp<DB: QueryDatabase + ?Sized>(&self, db: &DB, other: &Self) -> Ordering {
    match (self, other) {
      (Either::Left(_), Either::Right(_)) => Ordering::Less,
      (Either::Right(_), Either::Left(_)) => Ordering::Greater,
      (Either::Left(self_), Either::Left(other)) => self_.stable_cmp(db, other),
      (Either::Right(self_), Either::Right(other)) => self_.stable_cmp(db, other),
    }
  }
}

impl<K: StableCompare + Ord, V: StableCompare> StableCompare for BTreeMap<K, V> {
  fn stable_cmp<DB: QueryDatabase + ?Sized>(&self, db: &DB, other: &Self) -> Ordering {
    self.len().cmp(&other.len()).then_with(|| {
      self
        .iter()
        .zip(other.iter())
        .map(|((left_key, left_value), (k2, v2))| {
          left_key
            .stable_cmp(db, k2)
            .then_with(|| left_value.stable_cmp(db, v2))
        })
        .find(|obj| obj.is_ne())
        .unwrap_or(Ordering::Equal)
    })
  }
}

impl<V: StableCompare + Ord> StableCompare for BTreeSet<V> {
  fn stable_cmp<DB: QueryDatabase + ?Sized>(&self, db: &DB, other: &Self) -> Ordering {
    self.len().cmp(&other.len()).then_with(|| {
      self
        .iter()
        .zip(other.iter())
        .map(|(left, right)| left.stable_cmp(db, right))
        .find(|obj| obj.is_ne())
        .unwrap_or(Ordering::Equal)
    })
  }
}

impl<K: StableCompare, V: StableCompare> StableCompare for HashMap<K, V> {
  fn stable_cmp<DB: QueryDatabase + ?Sized>(&self, db: &DB, other: &Self) -> Ordering {
    self.len().cmp(&other.len()).then_with(|| {
      let mut self_entries: Vec<_> = self.iter().collect();
      let mut other_entries: Vec<_> = other.iter().collect();
      self_entries.sort_by(|(left_key, _), (right_key, _)| left_key.stable_cmp(db, right_key));
      other_entries.sort_by(|(left_key, _), (right_key, _)| left_key.stable_cmp(db, right_key));
      self_entries
        .iter()
        .zip(other_entries.iter())
        .map(|((left_key, left_value), (right_key, right_value))| {
          left_key
            .stable_cmp(db, right_key)
            .then_with(|| left_value.stable_cmp(db, right_value))
        })
        .find(|obj| obj.is_ne())
        .unwrap_or(Ordering::Equal)
    })
  }
}

impl<V: StableCompare> StableCompare for HashSet<V> {
  fn stable_cmp<DB: QueryDatabase + ?Sized>(&self, db: &DB, other: &Self) -> Ordering {
    self.len().cmp(&other.len()).then_with(|| {
      let mut self_entries: Vec<_> = self.iter().collect();
      let mut other_entries: Vec<_> = other.iter().collect();
      self_entries.sort_by(|left_entry, right_entry| left_entry.stable_cmp(db, right_entry));
      other_entries.sort_by(|left_entry, right_entry| left_entry.stable_cmp(db, right_entry));
      self_entries
        .iter()
        .zip(other_entries.iter())
        .map(|(left_entry, right_entry)| left_entry.stable_cmp(db, right_entry))
        .find(|obj| obj.is_ne())
        .unwrap_or(Ordering::Equal)
    })
  }
}

impl<T: StableCompare> StableCompare for [T] {
  fn stable_cmp<DB: QueryDatabase + ?Sized>(&self, db: &DB, other: &Self) -> Ordering {
    let left_length = self.len();
    let right_length = other.len();
    for index in 0..left_length.min(right_length) {
      let order = self[index].stable_cmp(db, &other[index]);
      if order.is_ne() {
        return order;
      }
    }
    left_length.cmp(&right_length)
  }
}

impl<T: StableCompare> StableCompare for Vec<T> {
  fn stable_cmp<DB: QueryDatabase + ?Sized>(&self, db: &DB, other: &Self) -> Ordering {
    self.as_slice().stable_cmp(db, other.as_slice())
  }
}

macro_rules! impl_stable_compare_via_ord {
  ($($ty:ty),*) => {
    $(
      impl StableCompare for $ty {
        fn stable_cmp<DB: QueryDatabase + ?Sized>(&self, _db: &DB, other: &Self) -> Ordering {
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
  std::time::SystemTime
);

impl StableCompare for &str {
  fn stable_cmp<DB: QueryDatabase + ?Sized>(&self, _db: &DB, other: &Self) -> Ordering {
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
  fn stable_cmp<DB: QueryDatabase + ?Sized>(&self, db: &DB, other: &Self) -> Ordering {
    self.0.stable_cmp(db, &other.0)
  }
}

impl<T1: StableCompare, T2: StableCompare> StableCompare for (T1, T2) {
  fn stable_cmp<DB: QueryDatabase + ?Sized>(&self, db: &DB, other: &Self) -> Ordering {
    self
      .0
      .stable_cmp(db, &other.0)
      .then_with(|| self.1.stable_cmp(db, &other.1))
  }
}

impl<T1: StableCompare, T2: StableCompare, T3: StableCompare> StableCompare for (T1, T2, T3) {
  fn stable_cmp<DB: QueryDatabase + ?Sized>(&self, db: &DB, other: &Self) -> Ordering {
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
  fn stable_cmp<DB: QueryDatabase + ?Sized>(&self, db: &DB, other: &Self) -> Ordering {
    self
      .0
      .stable_cmp(db, &other.0)
      .then_with(|| self.1.stable_cmp(db, &other.1))
      .then_with(|| self.2.stable_cmp(db, &other.2))
      .then_with(|| self.3.stable_cmp(db, &other.3))
  }
}

impl StableCompare for f32 {
  fn stable_cmp<DB: QueryDatabase + ?Sized>(&self, _db: &DB, other: &Self) -> Ordering {
    self.total_cmp(other)
  }
}

impl StableCompare for f64 {
  fn stable_cmp<DB: QueryDatabase + ?Sized>(&self, _db: &DB, other: &Self) -> Ordering {
    self.total_cmp(other)
  }
}

impl StableOrd for str {}

impl<T: StableOrd> StableOrd for &[T] {}

impl StableOrd for String {}

impl StableOrd for std::ffi::OsStr {}
impl StableOrd for std::path::Path {}
impl StableOrd for std::path::PathBuf {}

// Compare paths by raw bytes instead of component-by-component
impl StableCompare for std::path::PathBuf {
  fn stable_cmp<DB: QueryDatabase + ?Sized>(&self, _db: &DB, other: &Self) -> Ordering {
    self
      .as_os_str()
      .as_encoded_bytes()
      .cmp(other.as_os_str().as_encoded_bytes())
  }
}

impl StableOrd for std::time::SystemTime {}
