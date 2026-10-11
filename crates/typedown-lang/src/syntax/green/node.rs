use std::alloc::{Layout, alloc, dealloc};
use std::fmt::{self, Debug};
use std::hash::{Hash, Hasher};

use smallvec::SmallVec;
use std::sync::atomic::{AtomicUsize, Ordering};

use typedown_incremental::{QueryDatabase, StableCompare};

use super::GreenNode;
use crate::syntax::syntax_kind::SyntaxKind;

pub(super) struct NodeHeader {
  pub(super) ref_count: AtomicUsize,
  pub(super) kind: SyntaxKind,
  pub(super) text_len: usize,
  pub(super) n_children: u32,
}

/// An interior node in the green tree.
pub struct SyntaxNode(pub(super) *const NodeHeader);

impl SyntaxNode {
  fn layout(n: usize) -> (Layout, usize) {
    Layout::new::<NodeHeader>()
      .extend(Layout::array::<GreenNode>(n).unwrap())
      .unwrap()
  }

  pub(super) fn from_raw_parts(kind: SyntaxKind, children: &[GreenNode]) -> Self {
    let n = children.len();
    let text_len = children.iter().map(|c| c.text_len()).sum();
    let (layout, children_offset) = Self::layout(n);

    unsafe {
      let base = alloc(layout);
      assert!(!base.is_null(), "allocation failed");

      (base as *mut NodeHeader).write(NodeHeader {
        ref_count: AtomicUsize::new(1),
        kind,
        text_len,
        n_children: n as u32,
      });

      // Clone each child (bumps ref-count) and write into the allocation
      let children_ptr = base.add(children_offset) as *mut GreenNode;
      for (i, child) in children.iter().enumerate() {
        children_ptr.add(i).write(child.clone());
      }

      Self(base as *const NodeHeader)
    }
  }

  pub fn kind(&self) -> SyntaxKind {
    unsafe { (*self.0).kind }
  }

  pub fn text_len(&self) -> usize {
    unsafe { (*self.0).text_len }
  }

  pub fn chars(&self) -> impl Iterator<Item = char> {
    self.children().iter().flat_map(|c| c.chars())
  }

  pub fn n_children(&self) -> u32 {
    unsafe { (*self.0).n_children }
  }

  /// Returns a slice of this node's children.
  pub fn children(&self) -> &[GreenNode] {
    unsafe {
      let n = (*self.0).n_children as usize;
      let (_, offset) = Self::layout(n);
      let ptr = (self.0 as *const u8).add(offset) as *const GreenNode;
      std::slice::from_raw_parts(ptr, n)
    }
  }
}

impl Clone for SyntaxNode {
  /// The clone is very cheap
  /// Suggest to use clone instead of &
  fn clone(&self) -> Self {
    unsafe { (*self.0).ref_count.fetch_add(1, Ordering::AcqRel) };
    Self(self.0)
  }
}

impl Drop for SyntaxNode {
  fn drop(&mut self) {
    let prev = unsafe { (*self.0).ref_count.fetch_sub(1, Ordering::AcqRel) };
    if prev != 1 {
      return;
    }
    unsafe {
      let n = (*self.0).n_children as usize;
      let (layout, offset) = Self::layout(n);
      let children_ptr = (self.0 as *mut u8).add(offset) as *mut GreenNode;
      for i in 0..n {
        std::ptr::drop_in_place(children_ptr.add(i));
      }
      dealloc(self.0 as *mut u8, layout);
    }
  }
}

impl PartialEq for SyntaxNode {
  fn eq(&self, other: &Self) -> bool {
    self.0 == other.0 || (self.kind() == other.kind() && self.children() == other.children())
  }
}

impl Eq for SyntaxNode {}

impl Debug for SyntaxNode {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    f.debug_struct("SyntaxNode")
      .field("kind", &self.kind())
      .field("children", &self.children())
      .finish()
  }
}

impl Hash for SyntaxNode {
  fn hash<H: Hasher>(&self, state: &mut H) {
    let children = self.children();
    let mut buf: SmallVec<[u64; 64]> = SmallVec::with_capacity(1 + children.len());
    buf.push(((self.kind() as u64) << 32) | self.text_len() as u64);
    for child in children {
      buf.push(((child.kind() as u64) << 32) | child.text_len() as u64);
    }
    state.write(unsafe { std::slice::from_raw_parts(buf.as_ptr().cast::<u8>(), buf.len() * 8) });
  }

  fn hash_slice<H: Hasher>(data: &[Self], state: &mut H)
  where
    Self: Sized,
  {
    let total = data.iter().map(|n| 1 + n.n_children() as usize).sum();
    let mut buf: SmallVec<[u64; 64]> = SmallVec::with_capacity(total);
    for node in data {
      buf.push(((node.kind() as u64) << 32) | node.text_len() as u64);
      for child in node.children() {
        buf.push(((child.kind() as u64) << 32) | child.text_len() as u64);
      }
    }
    state.write_usize(data.len());
    state.write(unsafe { std::slice::from_raw_parts(buf.as_ptr().cast::<u8>(), buf.len() * 8) });
  }
}

unsafe impl Send for SyntaxNode {}
unsafe impl Sync for SyntaxNode {}

impl StableCompare for SyntaxNode {
  fn stable_cmp<DB: QueryDatabase + ?Sized>(&self, db: &DB, other: &Self) -> std::cmp::Ordering {
    std::cmp::Ordering::Equal
      .then_with(|| self.kind().stable_cmp(db, &other.kind()))
      .then_with(|| self.text_len().stable_cmp(db, &other.text_len()))
      .then_with(|| self.n_children().stable_cmp(db, &other.n_children()))
      .then_with(|| self.children().stable_cmp(db, other.children()))
  }
}
