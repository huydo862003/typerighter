//! Dependency graph types for cache persistence.

use std::hash::Hasher;

use rustc_stable_hash::{FromStableHash, SipHasher128Hash, StableSipHasher128};

use super::StableHasher;

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
