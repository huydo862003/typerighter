//! Derived types for the incremental database

pub mod obj_system;
pub mod symbol;

pub use obj_system::*;
pub use symbol::*;

use std::collections::BTreeMap;
use std::path::PathBuf;

use crate::syntax::diagnostic::Diagnostic;
use typedown_macros::query_derived;

use crate::syntax::ast::{
  AstNode, DictLit, ListLit, SourceFile, YamlFrontmatter, YamlMapping, YamlSequence,
};
use crate::syntax::red::RedNode;
use typedown_incremental::{
  Decodable, Decoder, Encodable, Encoder, QueryDatabase, StableHash, StableHasher,
};

use super::inputs::{File, FileHandle, Project};

/// A RedNode bound to a file
/// Carries the file identity so StableHash can use O(1) hashing
/// (file mtime + offset + kind + text_len) instead of recursively walking the tree
#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub struct ProjectRedNode {
  pub project: Project,
  pub file: File,
  pub node: RedNode,
}

impl ProjectRedNode {
  pub fn new(project: Project, file: File, node: RedNode) -> Self {
    Self {
      project,
      file,
      node,
    }
  }

  pub fn mapping_entries(&self) -> Option<Vec<(String, ProjectRedNode)>> {
    let project = self.project;
    let file = self.file;
    let node = self.node.clone();

    if let Some(sf) = SourceFile::cast(node.clone()) {
      let fm_node = sf
        .frontmatter()
        .map(|fm| ProjectRedNode::new(project, file, fm.syntax().clone()))?;
      return fm_node.mapping_entries();
    }
    if let Some(fm) = YamlFrontmatter::cast(node.clone()) {
      let mapping = fm.mapping()?;
      return Some(
        mapping
          .entries()
          .map(|(k, v)| (k, ProjectRedNode::new(project, file, v.syntax().clone())))
          .collect(),
      );
    }
    if let Some(m) = YamlMapping::cast(node.clone()) {
      return Some(
        m.entries()
          .map(|(k, v)| (k, ProjectRedNode::new(project, file, v.syntax().clone())))
          .collect(),
      );
    }
    if let Some(dict) = DictLit::cast(node) {
      return Some(
        dict
          .entries()
          .filter_map(|e| {
            let k = e.key()?;
            let v = e.value()?;
            Some((k, ProjectRedNode::new(project, file, v.syntax().clone())))
          })
          .collect(),
      );
    }
    None
  }

  pub fn sequence_items(&self) -> Option<Vec<ProjectRedNode>> {
    let project = self.project;
    let file = self.file;
    let node = self.node.clone();

    if let Some(seq) = YamlSequence::cast(node.clone()) {
      return Some(
        seq
          .values()
          .map(|v| ProjectRedNode::new(project, file, v.syntax().clone()))
          .collect(),
      );
    }
    if let Some(list) = ListLit::cast(node) {
      return Some(
        list
          .items()
          .filter_map(|item| item.value())
          .map(|v| ProjectRedNode::new(project, file, v.syntax().clone()))
          .collect(),
      );
    }
    None
  }
}

impl std::ops::Deref for ProjectRedNode {
  type Target = RedNode;

  fn deref(&self) -> &RedNode {
    &self.node
  }
}

impl StableHash for ProjectRedNode {
  fn stable_hash<DB: QueryDatabase + ?Sized>(&self, db: &DB, hasher: &mut StableHasher) {
    self.project.stable_hash(db, hasher);
    self.file.stable_hash(db, hasher);
    (self.node.offset() as u64).stable_hash(db, hasher);
    self.node.kind().stable_hash(db, hasher);
    (self.node.text_len() as u64).stable_hash(db, hasher);
  }
}

impl Encodable for ProjectRedNode {
  fn encode(&self, buffer: &mut Vec<u8>, encoder: &mut Encoder) {
    self.project.field_encode(buffer, encoder);
    self.file.field_encode(buffer, encoder);
    self.node.encode(buffer, encoder);
  }
}

impl Decodable for ProjectRedNode {
  fn decode(data: &mut &[u8], decoder: &Decoder) -> Self {
    Self {
      project: Project::field_decode(data, decoder),
      file: File::field_decode(data, decoder),
      node: RedNode::decode(data, decoder),
    }
  }
}

impl typedown_incremental::StableCompare for ProjectRedNode {
  fn stable_cmp<DB: QueryDatabase + ?Sized>(&self, db: &DB, other: &Self) -> std::cmp::Ordering {
    self
      .file
      .stable_cmp(db, &other.file)
      .then_with(|| self.node.stable_cmp(db, &other.node))
  }
}

#[query_derived]
pub struct VaultConfigResult<'db> {
  version: String,
  root_dir: PathBuf,
  base_path: String,
  origin: Option<String>,
  lang: String,
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
  ast: ProjectRedNode,
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
pub struct TypResult<'db> {
  typ: Option<TdTypEnum<'db>>,
  diagnostics: Vec<Diagnostic>,
}

#[query_derived]
pub struct InstantiateResult<'db> {
  pub typ: TdTypEnum<'db>,
  pub diagnostics: Vec<Diagnostic>,
}

#[query_derived]
pub struct ResourceResult<'db> {
  pub value: Option<TdObjEnum<'db>>,
  pub diagnostics: Vec<Diagnostic>,
}
