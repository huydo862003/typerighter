//! Dependency graph types for cache persistence.

use std::hash::Hasher;
use std::sync::OnceLock;

use rustc_stable_hash::{FromStableHash, SipHasher128Hash, StableSipHasher128};

use super::StableHasher;
use crate::QueryDatabase;
use crate::StableHash;

/// A stable 128-bit hash value, used for both query identity and result change detection.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct Fingerprint(pub [u8; 16]);

impl FromStableHash for Fingerprint {
  type Hash = SipHasher128Hash;

  fn from(hash: SipHasher128Hash) -> Self {
    let [lo, hi] = hash.0;
    let mut bytes = [0u8; 16];
    bytes[..8].copy_from_slice(&lo.to_le_bytes());
    bytes[8..].copy_from_slice(&hi.to_le_bytes());
    Fingerprint(bytes)
  }
}

// A fingerprint that is computed lazily on first access and cached
// Pre-populate with `from_stored` when restoring from serialized data
pub struct LazyFingerprint(OnceLock<Fingerprint>);

impl LazyFingerprint {
  pub fn new() -> Self {
    Self(OnceLock::new())
  }

  pub fn from_stored(fp: Fingerprint) -> Self {
    Self(OnceLock::from(fp))
  }

  pub fn get_or_compute<T: StableHash + ?Sized>(
    &self,
    db: &dyn QueryDatabase,
    value: &T,
  ) -> Fingerprint {
    *self.0.get_or_init(|| {
      let mut hasher = StableHasher::new();
      value.stable_hash(db, &mut hasher);
      Fingerprint::from_hasher(hasher)
    })
  }
}

impl Default for LazyFingerprint {
  fn default() -> Self {
    Self::new()
  }
}

impl Fingerprint {
  /// Sentinel for no_hash queries: signals "fingerprint not computed, must re-verify at runtime"
  pub const SKIPPED: Self = Fingerprint([0u8; 16]);

  pub fn from_hasher(hasher: StableHasher) -> Self {
    hasher.finish()
  }

  /// Compute a fingerprint from a stable name string.
  /// Used for ingredient identity across sessions.
  pub fn from_name(name: &str) -> Self {
    let mut hasher: StableHasher = StableSipHasher128::new();
    hasher.write(name.as_bytes());
    Self::from_hasher(hasher)
  }
}
