//! Derived types for the incremental database

pub mod hir;
pub mod object_system;
pub mod symbol;

pub use hir::*;
pub use object_system::*;
pub use symbol::*;

use std::collections::BTreeMap;
use std::path::PathBuf;

use crate::syntax::diagnostic::Diagnostic;
use typedown_macros::query_derived;

use crate::syntax::red::RedNode;
use typedown_incremental::{
  Decodable, Decoder, Encodable, Encoder, QueryDatabase, StableHash, StableHasher,
};

use super::inputs::{File, FileHandle, Project};

/// A RedNode bound to a file
/// Carries the file identity so StableHash can use O(1) hashing
/// (file mtime + offset + kind + text_len) instead of recursively walking the tree
#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub struct FileRedNode {
  pub owner_file: File,
  pub node: RedNode,
}

impl FileRedNode {
  pub fn new(owner_file: File, node: RedNode) -> Self {
    Self { owner_file, node }
  }
}

impl std::ops::Deref for FileRedNode {
  type Target = RedNode;

  fn deref(&self) -> &RedNode {
    &self.node
  }
}

impl StableHash for FileRedNode {
  fn stable_hash<DB: QueryDatabase + ?Sized>(&self, db: &DB, hasher: &mut StableHasher) {
    self.owner_file.stable_hash(db, hasher);
    (self.node.offset() as u64).stable_hash(db, hasher);
    self.node.kind().stable_hash(db, hasher);
    (self.node.text_len() as u64).stable_hash(db, hasher);
  }
}

impl Encodable for FileRedNode {
  fn encode(&self, buf: &mut Vec<u8>, encoder: &mut Encoder) {
    self.owner_file.field_encode(buf, encoder);
    self.node.encode(buf, encoder);
  }
}

impl Decodable for FileRedNode {
  fn decode(data: &mut &[u8], decoder: &Decoder) -> Self {
    Self {
      owner_file: File::field_decode(data, decoder),
      node: RedNode::decode(data, decoder),
    }
  }
}

impl typedown_incremental::StableCompare for FileRedNode {
  fn stable_cmp<DB: QueryDatabase + ?Sized>(&self, db: &DB, other: &Self) -> std::cmp::Ordering {
    self
      .owner_file
      .stable_cmp(db, &other.owner_file)
      .then_with(|| self.node.stable_cmp(db, &other.node))
  }
}

#[query_derived]
pub struct VaultConfigResult<'db> {
  version: String,
  root_dir: PathBuf,
  base_path: String,
  site_title: String,
  site_description: String,
  repo: Option<String>,
  author: Option<String>,
  license: Option<String>,
  public_dir: String,
  // (text, link, icon_name) tuples for StableHash/Encodable/Decodable compatibility
  nav_items: Vec<(String, String, Option<String>)>,
  diagnostics: Vec<Diagnostic>,
}

#[query_derived]
pub struct FileAstResult<'db> {
  #[id]
  handle: FileHandle,
  project: Project,
  file: File,
  ast: FileRedNode,
  diagnostics: Vec<Diagnostic>,
}

#[query_derived]
pub struct SchemaAstResults<'db> {
  files: BTreeMap<PathBuf, FileAstResult<'db>>,
}

#[query_derived]
pub struct TypecheckResult<'db> {
  diagnostics: Vec<Diagnostic>,
}

#[query_derived]
pub struct ResolveResult<'db> {
  diagnostics: Vec<Diagnostic>,
}

#[query_derived]
pub struct TypeResult<'db> {
  typ: Option<TdTypeEnum<'db>>,
  diagnostics: Vec<Diagnostic>,
}

#[query_derived]
pub struct InstResult<'db> {
  pub typ: TdTypeEnum<'db>,
  pub diagnostics: Vec<Diagnostic>,
}

#[query_derived]
pub struct ResourceResult<'db> {
  pub value: Option<TdObjectEnum<'db>>,
  pub diagnostics: Vec<Diagnostic>,
}
