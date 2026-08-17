use std::collections::{HashMap, HashSet};

use typedown_types::either::Either::{self, Left};

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
///
/// The associated constant `CAN_USE_UNSTABLE_SORT` denotes whether
/// unstable sorting can be used for this type. Set to true if and
/// only if `a == b` implies `a` and `b` are fully indistinguishable.
/// '''
pub trait StableOrd: Ord {
  const CAN_USE_UNSTABLE_SORT: bool;
}

/// TIL: Ordering of a reference is exactly that of the referent
/// This is not the case for raw pointers though
impl<T: StableOrd> StableOrd for &T {
  const CAN_USE_UNSTABLE_SORT: bool = T::CAN_USE_UNSTABLE_SORT;
}

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
  const CAN_USE_UNSTABLE_SORT: bool;

  fn stable_cmp<DB: QueryDatabase + ?Sized>(&self, db: &DB, other: &Self) -> std::cmp::Ordering;
}

impl<T: StableOrd> StableCompare for T {
  const CAN_USE_UNSTABLE_SORT: bool = T::CAN_USE_UNSTABLE_SORT;

  fn stable_cmp<DB: QueryDatabase + ?Sized>(&self, _db: &DB, other: &Self) -> std::cmp::Ordering {
    self.cmp(other)
  }
}

impl<L: StableCompare, R: StableCompare> StableCompare for Either<L, R> {
  const CAN_USE_UNSTABLE_SORT: bool = L::CAN_USE_UNSTABLE_SORT && R::CAN_USE_UNSTABLE_SORT;

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
  const CAN_USE_UNSTABLE_SORT: bool = K::CAN_USE_UNSTABLE_SORT && V::CAN_USE_UNSTABLE_SORT;

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
  const CAN_USE_UNSTABLE_SORT: bool = V::CAN_USE_UNSTABLE_SORT;

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

impl StableOrd for i8 {
  const CAN_USE_UNSTABLE_SORT: bool = true;
}
impl StableOrd for i16 {
  const CAN_USE_UNSTABLE_SORT: bool = true;
}
impl StableOrd for i32 {
  const CAN_USE_UNSTABLE_SORT: bool = true;
}
impl StableOrd for i64 {
  const CAN_USE_UNSTABLE_SORT: bool = true;
}
impl StableOrd for i128 {
  const CAN_USE_UNSTABLE_SORT: bool = true;
}
impl StableOrd for isize {
  const CAN_USE_UNSTABLE_SORT: bool = true;
}

impl StableOrd for u8 {
  const CAN_USE_UNSTABLE_SORT: bool = true;
}
impl StableOrd for u16 {
  const CAN_USE_UNSTABLE_SORT: bool = true;
}
impl StableOrd for u32 {
  const CAN_USE_UNSTABLE_SORT: bool = true;
}
impl StableOrd for u64 {
  const CAN_USE_UNSTABLE_SORT: bool = true;
}
impl StableOrd for u128 {
  const CAN_USE_UNSTABLE_SORT: bool = true;
}
impl StableOrd for usize {
  const CAN_USE_UNSTABLE_SORT: bool = true;
}

impl StableOrd for char {
  const CAN_USE_UNSTABLE_SORT: bool = true;
}
impl StableOrd for () {
  const CAN_USE_UNSTABLE_SORT: bool = true;
}
impl StableOrd for bool {
  const CAN_USE_UNSTABLE_SORT: bool = true;
}

impl<T: StableOrd> StableOrd for Option<T> {
  const CAN_USE_UNSTABLE_SORT: bool = T::CAN_USE_UNSTABLE_SORT;
}

impl<T: StableOrd> StableOrd for (T,) {
  const CAN_USE_UNSTABLE_SORT: bool = T::CAN_USE_UNSTABLE_SORT;
}

impl<T1: StableOrd, T2: StableOrd> StableOrd for (T1, T2) {
  const CAN_USE_UNSTABLE_SORT: bool = T1::CAN_USE_UNSTABLE_SORT && T2::CAN_USE_UNSTABLE_SORT;
}

impl<T1: StableOrd, T2: StableOrd, T3: StableOrd> StableOrd for (T1, T2, T3) {
  const CAN_USE_UNSTABLE_SORT: bool =
    T1::CAN_USE_UNSTABLE_SORT && T2::CAN_USE_UNSTABLE_SORT && T3::CAN_USE_UNSTABLE_SORT;
}

impl<T1: StableOrd, T2: StableOrd, T3: StableOrd, T4: StableOrd> StableOrd for (T1, T2, T3, T4) {
  const CAN_USE_UNSTABLE_SORT: bool = T1::CAN_USE_UNSTABLE_SORT
    && T2::CAN_USE_UNSTABLE_SORT
    && T3::CAN_USE_UNSTABLE_SORT
    && T4::CAN_USE_UNSTABLE_SORT;
}

impl StableOrd for str {
  const CAN_USE_UNSTABLE_SORT: bool = true;
}

impl StableOrd for String {
  const CAN_USE_UNSTABLE_SORT: bool = true;
}

impl StableOrd for std::ffi::OsStr {
  const CAN_USE_UNSTABLE_SORT: bool = true;
}
impl StableOrd for std::path::Path {
  const CAN_USE_UNSTABLE_SORT: bool = true;
}
impl StableOrd for std::path::PathBuf {
  const CAN_USE_UNSTABLE_SORT: bool = true;
}
