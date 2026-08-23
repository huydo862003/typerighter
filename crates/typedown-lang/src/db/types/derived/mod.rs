//! Derived types for the incremental database

pub mod hir;
pub mod object_system;
pub mod symbol;

pub use hir::*;
pub use object_system::*;
pub use symbol::*;

use std::collections::HashMap;
use std::path::PathBuf;

use crate::syntax::diagnostic::Diagnostic;
use typedown_macros::query_derived;

use crate::syntax::red::RedNode;

use super::inputs::{File, FileHandle, Project};

#[query_derived]
pub struct VaultConfigResult {
  version: String,
  root_dir: PathBuf,
  base_path: String,
  site_title: String,
  site_description: String,
  repo: Option<String>,
  public_dir: String,
  diagnostics: Vec<Diagnostic>,
}

#[query_derived]
pub struct FileAstResult {
  #[id]
  handle: FileHandle,
  project: Project,
  file: File,
  ast: RedNode,
  diagnostics: Vec<Diagnostic>,
}

#[query_derived]
pub struct SchemaAstResults {
  files: HashMap<PathBuf, FileAstResult>,
}

#[query_derived]
pub struct TypecheckResult {
  diagnostics: Vec<Diagnostic>,
}

#[query_derived]
pub struct ResolveResult {
  diagnostics: Vec<Diagnostic>,
}

#[query_derived]
pub struct TypeResult {
  typ: Option<TdTypeEnum>,
  diagnostics: Vec<Diagnostic>,
}

#[query_derived]
pub struct InstResult {
  pub typ: TdTypeEnum,
  pub diagnostics: Vec<Diagnostic>,
}

#[query_derived]
pub struct ResourceResult {
  pub value: Option<TdObjectEnum>,
  pub diagnostics: Vec<Diagnostic>,
}
