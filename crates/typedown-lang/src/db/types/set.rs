use std::collections::HashSet;
use std::collections::hash_set::{IntoIter, Iter};
use std::hash::Hash;
use std::ops::{Deref, DerefMut};
use typedown_incremental::{
  Decodable, Decoder, Encodable, Encoder, FieldDecodable, FieldEncodable, QueryDatabase,
  StableCompare, StableHash, StableHasher,
};

// Built in HashSet doesnt support hashing so we wrap
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Set<T: Eq + Hash>(pub HashSet<T>);

impl<T: Eq + Hash> Set<T> {
  pub fn new() -> Self {
    Set(HashSet::new())
  }
}

impl<T: Eq + Hash> Default for Set<T> {
  fn default() -> Self {
    Self::new()
  }
}

impl<T: Eq + Hash> Deref for Set<T> {
  type Target = HashSet<T>;
  fn deref(&self) -> &Self::Target {
    &self.0
  }
}

impl<T: Eq + Hash> DerefMut for Set<T> {
  fn deref_mut(&mut self) -> &mut Self::Target {
    &mut self.0
  }
}

impl<T: Eq + Hash> FromIterator<T> for Set<T> {
  fn from_iter<I: IntoIterator<Item = T>>(iter: I) -> Self {
    Set(HashSet::from_iter(iter))
  }
}

impl<T: Eq + Hash> IntoIterator for Set<T> {
  type Item = T;
  type IntoIter = IntoIter<T>;
  fn into_iter(self) -> Self::IntoIter {
    self.0.into_iter()
  }
}

impl<'a, T: Eq + Hash> IntoIterator for &'a Set<T> {
  type Item = &'a T;
  type IntoIter = Iter<'a, T>;
  fn into_iter(self) -> Self::IntoIter {
    self.0.iter()
  }
}

impl<T: Eq + Hash, const N: usize> From<[T; N]> for Set<T> {
  fn from(arr: [T; N]) -> Self {
    Set(HashSet::from(arr))
  }
}

impl<T: Eq + Hash> From<HashSet<T>> for Set<T> {
  fn from(set: HashSet<T>) -> Self {
    Set(set)
  }
}

impl<T: Hash + Eq> Hash for Set<T> {
  fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
    self.0.len().hash(state);
    let mut hash_sum: u64 = 0;
    for item in &self.0 {
      let mut h = std::collections::hash_map::DefaultHasher::new();
      item.hash(&mut h);
      hash_sum = hash_sum.wrapping_add(std::hash::Hasher::finish(&h));
    }
    hash_sum.hash(state);
  }
}

impl<T: FieldEncodable + StableCompare + Eq + Hash> Encodable for Set<T> {
  fn encode(&self, buf: &mut Vec<u8>, encoder: &mut Encoder) {
    self.0.encode(buf, encoder);
  }
}

impl<T: FieldDecodable + Eq + Hash> Decodable for Set<T> {
  fn decode(data: &mut &[u8], decoder: &Decoder) -> Self {
    Set(HashSet::decode(data, decoder))
  }
}

impl<T: StableHash + StableCompare + Eq + Hash> StableHash for Set<T> {
  fn stable_hash<DB: QueryDatabase + ?Sized>(&self, db: &DB, hasher: &mut StableHasher) {
    self.0.stable_hash(db, hasher);
  }
}

impl<T: StableCompare + Eq + Hash> StableCompare for Set<T> {
  const CAN_USE_UNSTABLE_SORT: bool = T::CAN_USE_UNSTABLE_SORT;

  fn stable_cmp<DB: QueryDatabase + ?Sized>(&self, db: &DB, other: &Self) -> std::cmp::Ordering {
    self.0.stable_cmp(db, &other.0)
  }
}
