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
pub struct VaultConfigResult<'db> {
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
pub struct FileAstResult<'db> {
  #[id]
  handle: FileHandle,
  project: Project,
  file: File,
  ast: RedNode,
  diagnostics: Vec<Diagnostic>,
}

#[query_derived]
pub struct SchemaAstResults<'db> {
  files: HashMap<PathBuf, FileAstResult<'db>>,
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
