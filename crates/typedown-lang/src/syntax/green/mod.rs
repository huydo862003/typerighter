// Inspired by rust-analyzer

pub mod cache;
pub mod node;
pub mod token;

use std::fmt::{self, Debug};
use std::hash::{Hash, Hasher};

use smallvec::SmallVec;
use typedown_incremental::{QueryDatabase, StableCompare};
use typedown_types::either::Either;

use crate::syntax::syntax_kind::SyntaxKind;

pub use node::SyntaxNode;
pub use token::SyntaxToken;

/// GreenNode: A thread-safe readonly tagged pointer to either a Node or a Token
// Tag bit 0 = node, tag bit 1 = token
pub struct GreenNode(usize);

impl GreenNode {
  /// Take ownership of a Node and store it as a tagged pointer
  pub fn from_node(node: SyntaxNode) -> Self {
    let ptr = node.0 as usize;
    debug_assert!(ptr & 1 == 0, "Node pointer not aligned");
    std::mem::forget(node);
    Self(ptr)
  }

  /// Take ownership of a Token and store it as a tagged pointer
  pub fn from_token(token: SyntaxToken) -> Self {
    let ptr = token.0 as usize;
    debug_assert!(ptr & 1 == 0, "Token pointer not aligned");
    std::mem::forget(token);
    Self(ptr | 1)
  }

  /// Returns true if this child points to a Node
  pub fn is_node(&self) -> bool {
    self.0 & 1 == 0
  }

  /// Returns true if this child points to a Token
  pub fn is_token(&self) -> bool {
    self.0 & 1 == 1
  }

  /// Returns a borrowed handle to the inner Node
  /// Returns None if this child is a token
  pub fn as_node(&self) -> Option<&SyntaxNode> {
    if !self.is_node() {
      return None;
    }
    unsafe { Some(&*(&self.0 as *const usize as *const SyntaxNode)) }
  }

  /// Returns an owned handle to the inner Token (ref-count bumped via Token::clone)
  /// Returns None if this child is a node
  pub fn as_token(&self) -> Option<SyntaxToken> {
    if !self.is_token() {
      return None;
    }
    let tmp = SyntaxToken((self.0 & !1) as *const _);
    let cloned = tmp.clone();
    std::mem::forget(tmp);
    Some(cloned)
  }

  pub fn kind(&self) -> SyntaxKind {
    if self.is_token() {
      self.as_token().unwrap().kind()
    } else {
      self.as_node().unwrap().kind()
    }
  }

  /// Raw tagged pointer value, usable as a dedup key
  pub fn as_ptr(&self) -> usize {
    self.0
  }

  pub fn text_len(&self) -> usize {
    if self.is_token() {
      self.as_token().unwrap().text_len()
    } else {
      self.as_node().unwrap().text_len()
    }
  }

  // We need to use Box<dyn Iterator> here due to rust cannot resolve recursive opaque types
  // TIL: Rust iterator type is really opaque and hardly writable. It mirrors the operations you've performed on the iterator. I think this is used for optimization
  pub fn chars<'a>(
    &'a self,
  ) -> Either<impl Iterator<Item = char>, Box<dyn Iterator<Item = char> + 'a>> {
    if self.is_token() {
      let token = self.as_token().unwrap();
      // Collect bytes first so the iterator owns its data independent of token's lifetime
      let bytes = token.bytes().to_vec();
      Either::Left(
        bytes
          .into_iter()
          .map(|byte| char::from_u32(u32::from(byte)).unwrap_or('\u{FFFD}')),
      )
    } else {
      Either::Right(Box::new(self.as_node().unwrap().chars()))
    }
  }
}

impl Clone for GreenNode {
  fn clone(&self) -> Self {
    if self.is_node() {
      let tmp = SyntaxNode(self.0 as *const _);
      let cloned = tmp.clone();
      std::mem::forget(tmp);
      Self::from_node(cloned)
    } else {
      let tmp = SyntaxToken((self.0 & !1) as *const _);
      let cloned = tmp.clone();
      std::mem::forget(tmp);
      Self::from_token(cloned)
    }
  }
}

impl Drop for GreenNode {
  fn drop(&mut self) {
    if self.is_node() {
      drop(SyntaxNode(self.0 as *const _));
    } else {
      drop(SyntaxToken((self.0 & !1) as *const _));
    }
  }
}

impl PartialEq for GreenNode {
  fn eq(&self, other: &Self) -> bool {
    self.0 == other.0 || {
      match (self.is_node(), other.is_node()) {
        (true, true) => self.as_node().unwrap() == other.as_node().unwrap(),
        (false, false) => self.as_token().unwrap() == other.as_token().unwrap(),
        _ => false,
      }
    }
  }
}

impl Eq for GreenNode {}

impl Debug for GreenNode {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    if self.is_node() {
      self.as_node().unwrap().fmt(f)
    } else {
      self.as_token().unwrap().fmt(f)
    }
  }
}

impl Hash for GreenNode {
  fn hash<H: Hasher>(&self, state: &mut H) {
    if self.is_node() {
      let node = self.as_node().unwrap();
      let children = node.children();
      let mut buffer: SmallVec<[u64; 64]> = SmallVec::with_capacity(1 + children.len());
      buffer.push(((node.kind() as u64) << 32) | node.text_len() as u64);
      for child in children {
        buffer.push(((child.kind() as u64) << 32) | child.text_len() as u64);
      }
      state.write(unsafe {
        std::slice::from_raw_parts(buffer.as_ptr().cast::<u8>(), buffer.len() * 8)
      });
    } else {
      let token = self.as_token().unwrap();
      // bit 63 = 1 (token), bits 0-31 = kind
      state.write_u64((1u64 << 63) | token.kind() as u64);
      state.write(token.bytes());
    }
  }

  fn hash_slice<H: Hasher>(data: &[Self], state: &mut H)
  where
    Self: Sized,
  {
    // Pack (discriminant | kind | text_len) per element into one write
    let mut buf: SmallVec<[u64; 64]> = SmallVec::with_capacity(data.len());
    for green in data {
      if green.is_node() {
        let node = green.as_node().unwrap();
        buf.push(((node.kind() as u64) << 32) | node.text_len() as u64);
      } else {
        let token = green.as_token().unwrap();
        buf.push((1u64 << 63) | ((token.kind() as u64) << 32) | token.text_len() as u64);
      }
    }
    state.write_usize(data.len());
    state.write(unsafe { std::slice::from_raw_parts(buf.as_ptr().cast::<u8>(), buf.len() * 8) });
  }
}

unsafe impl Send for GreenNode {}
unsafe impl Sync for GreenNode {}

impl StableCompare for GreenNode {
  fn stable_cmp<DB: QueryDatabase + ?Sized>(&self, db: &DB, other: &Self) -> std::cmp::Ordering {
    match (self.is_node(), other.is_node()) {
      (true, false) => std::cmp::Ordering::Less,
      (false, true) => std::cmp::Ordering::Greater,
      (true, true) => self
        .as_node()
        .unwrap()
        .stable_cmp(db, other.as_node().unwrap()),
      (false, false) => self
        .as_token()
        .unwrap()
        .stable_cmp(db, &other.as_token().unwrap()),
    }
  }
}
