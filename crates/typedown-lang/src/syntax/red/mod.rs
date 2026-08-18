//! The higher layer over green nodes
//! Which gives green node identity and the child nodes now contain a back pointers to their
//! parents

use std::{
  fmt::{self, Debug},
  hash::{Hash, Hasher},
  ops::Deref,
};

use typedown_incremental::{QueryDatabase, StableCompare};

use crate::syntax::green::{GreenNode, node::SyntaxNode};
use crate::syntax::syntax_kind::SyntaxKind;

#[derive(Clone)]
pub struct RedNodeData {
  /// The start offset of this red node in the source code
  offset: usize,
  /// The parent pointer
  parent: Option<Box<RedNode>>,
  /// The underlying green child
  green: GreenNode,
}

impl PartialEq for RedNodeData {
  fn eq(&self, other: &Self) -> bool {
    self.offset == other.offset && self.green == other.green
  }
}

impl Eq for RedNodeData {}

impl Hash for RedNodeData {
  fn hash<H: Hasher>(&self, state: &mut H) {
    self.offset.hash(state);
    self.green.hash(state);
  }
}

#[derive(Clone, Eq, PartialEq)]
pub struct RedNode(RedNodeData);

impl Hash for RedNode {
  fn hash<H: Hasher>(&self, state: &mut H) {
    self.0.hash(state);
  }
}

impl Debug for RedNode {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    f.debug_struct("RedNode")
      .field("offset", &self.0.offset)
      .field("green", &self.0.green)
      .finish()
  }
}

impl RedNode {
  pub fn new_root(root: SyntaxNode) -> RedNode {
    RedNode(RedNodeData {
      offset: 0,
      parent: None,
      green: GreenNode::from_node(root),
    })
  }

  pub fn from_green(offset: usize, green: GreenNode) -> RedNode {
    RedNode(RedNodeData {
      offset,
      parent: None,
      green,
    })
  }

  /// Walk up the parent chain to find the root node.
  pub fn root(&self) -> RedNode {
    let mut current = self.clone();
    while let Some(parent) = current.parent() {
      current = parent;
    }
    current
  }

  /// Find a descendant node at the given offset. Walks depth-first.
  /// Returns the deepest node whose offset matches.
  pub fn find_at_offset(&self, target_offset: usize) -> Option<RedNode> {
    if self.offset() == target_offset {
      return Some(self.clone());
    }
    for child in self.children() {
      if target_offset >= child.offset() && target_offset < child.offset() + child.text_len() {
        return child.find_at_offset(target_offset);
      }
    }
    None
  }

  pub fn kind(&self) -> SyntaxKind {
    self.0.green.kind()
  }

  pub fn parent(&self) -> Option<RedNode> {
    self.0.parent.as_deref().cloned()
  }

  /// Collect all token text under this node into a String.
  pub fn text(&self) -> String {
    match self.0.green.as_token() {
      Some(token) => token.text().unwrap_or("").to_string(),
      None => self.children().map(|child| child.text()).collect(),
    }
  }

  pub fn offset(&self) -> usize {
    self.0.offset
  }

  pub fn text_len(&self) -> usize {
    self.0.green.text_len()
  }

  /// Leading trivia nodes (whitespace, indent, comment before content)
  pub fn leading_trivia(&self) -> Vec<RedNode> {
    self
      .children()
      .take_while(|c| c.kind().is_trivia())
      .collect()
  }

  /// Trailing trivia nodes (whitespace, newline, comment after content)
  pub fn trailing_trivia(&self) -> Vec<RedNode> {
    let children: Vec<_> = self.children().collect();
    children
      .into_iter()
      .rev()
      .take_while(|c| c.kind().is_trivia())
      .collect::<Vec<_>>()
      .into_iter()
      .rev()
      .collect()
  }

  /// Offset and length excluding leading/trailing trivia tokens
  pub fn trimmed_range(&self) -> (usize, usize) {
    let children: Vec<_> = self.children().collect();
    // Find first non-trivia children
    let start = children
      .iter()
      .find(|c| !c.kind().is_trivia())
      .map(|c| c.offset())
      .unwrap_or(self.offset());
    // Find last non-trivia children
    let end = children
      .iter()
      .rfind(|c| !c.kind().is_trivia())
      .map(|c| c.offset() + c.text_len())
      .unwrap_or(self.offset() + self.text_len());
    (start, end - start)
  }

  pub fn children(&self) -> RedNodeChildren {
    let green_node = self.0.green.as_node();
    RedNodeChildren {
      parent: self.clone(),
      green_node: green_node.cloned(),
      index: 0,
      offset: self.0.offset,
    }
  }
}

impl Deref for RedNode {
  type Target = GreenNode;

  fn deref(&self) -> &Self::Target {
    &self.0.green
  }
}

/// A lazy iterator over a RedNode's children.
pub struct RedNodeChildren {
  parent: RedNode,
  green_node: Option<SyntaxNode>,
  index: usize,
  offset: usize,
}

impl Iterator for RedNodeChildren {
  type Item = RedNode;

  fn next(&mut self) -> Option<RedNode> {
    let children = self.green_node.as_ref()?.children();
    let child = children.get(self.index)?;
    let child_offset = self.offset;
    self.offset += child.text_len();
    self.index += 1;
    Some(RedNode(RedNodeData {
      offset: child_offset,
      parent: Some(Box::new(self.parent.clone())),
      green: child.clone(),
    }))
  }
}

impl StableCompare for RedNode {
  fn stable_cmp<DB: QueryDatabase + ?Sized>(&self, db: &DB, other: &Self) -> std::cmp::Ordering {
    self
      .0
      .offset
      .stable_cmp(db, &other.0.offset)
      .then_with(|| self.0.green.stable_cmp(db, &other.0.green))
  }
}
