//! Syntax Token: The leaf node in the AST
//! Use the same SyntaxKind as Syntax Node

use std::hash::{Hash, Hasher};
use std::sync::atomic::{AtomicUsize, Ordering};

use typedown_incremental::{QueryDatabase, StableCompare};

use crate::syntax::green::cache::Cache;
use crate::syntax::syntax_kind::SyntaxKind;

pub(super) struct TokenBody {
  pub(super) ref_count: AtomicUsize,
  pub(super) kind: SyntaxKind,
  pub(super) bytes: Vec<u8>,
}

/// The leaf node in the green tree.
pub struct SyntaxToken(pub(super) *const TokenBody);

impl SyntaxToken {
  pub(crate) fn new(cache: &mut Cache, kind: SyntaxKind, text: &[u8]) -> Self {
    cache.token(kind, text)
  }

  pub(crate) fn from_raw_parts(kind: SyntaxKind, bytes: Vec<u8>) -> Self {
    let body = Box::new(TokenBody {
      ref_count: AtomicUsize::new(1),
      kind,
      bytes,
    });
    Self(Box::into_raw(body))
  }

  pub fn kind(&self) -> SyntaxKind {
    unsafe { (*self.0).kind }
  }

  pub fn chars(&self) -> impl Iterator<Item = char> {
    let bytes = unsafe { &(*self.0).bytes };
    bytes
      .iter()
      .cloned()
      .map(u32::from)
      .map(char::from_u32)
      .map(|maybe_char| maybe_char.unwrap_or('\u{FFFD}'))
  }

  pub fn text(&self) -> Option<&str> {
    unsafe { str::from_utf8(&(*self.0).bytes).ok() }
  }

  pub fn bytes(&self) -> &[u8] {
    unsafe { &(*self.0).bytes }
  }

  pub fn text_len(&self) -> usize {
    unsafe { (*self.0).bytes.len() }
  }
}

impl Clone for SyntaxToken {
  /// The clone is very cheap
  /// Suggest to use clone instead of &
  fn clone(&self) -> Self {
    // Currently use AcqRel for extra safety
    unsafe { (*self.0).ref_count.fetch_add(1, Ordering::AcqRel) };
    Self(self.0)
  }
}

impl Drop for SyntaxToken {
  fn drop(&mut self) {
    // Currently use AcqRel for extra safety
    let prev = unsafe { (*self.0).ref_count.fetch_sub(1, Ordering::AcqRel) };
    if prev != 1 {
      return;
    }
    unsafe { drop(Box::from_raw(self.0 as *mut TokenBody)) };
  }
}

impl PartialEq for SyntaxToken {
  fn eq(&self, other: &Self) -> bool {
    let self_bytes = unsafe { &(*self.0).bytes };
    let other_bytes = unsafe { &(*other.0).bytes };

    self.0 == other.0
      || self.kind() == other.kind()
        && self_bytes.len() == other_bytes.len()
        && self_bytes == other_bytes
  }
}

impl Eq for SyntaxToken {}

impl std::fmt::Debug for SyntaxToken {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    let text: String = self.chars().collect();
    write!(f, "{:?}({:?})", self.kind(), text)
  }
}

impl Hash for SyntaxToken {
  fn hash<H: Hasher>(&self, state: &mut H) {
    self.kind().hash(state);
    self.bytes().hash(state);
  }
}

unsafe impl Send for SyntaxToken {}
unsafe impl Sync for SyntaxToken {}

impl StableCompare for SyntaxToken {
  fn stable_cmp<DB: QueryDatabase + ?Sized>(&self, db: &DB, other: &Self) -> std::cmp::Ordering {
    std::cmp::Ordering::Equal
      .then_with(|| self.kind().stable_cmp(db, &other.kind()))
      .then_with(|| self.text_len().stable_cmp(db, &other.text_len()))
      .then_with(|| self.bytes().stable_cmp(db, other.bytes()))
  }
}
