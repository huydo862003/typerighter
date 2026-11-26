//! Cursor context shared by LSP services: the request resolved to file, offset and node once

use lsp_types::{Position, Uri};
use ropey::Rope;
use typedown_lang::db::TypedownDatabase;
use typedown_lang::db::derived::parse_file::parse_file;
use typedown_lang::db::types::{File, Project};
use typedown_lang::syntax::red::RedNode;

use crate::core::analysis::Analysis;
use crate::core::utils::ast::node_at_offset;
use crate::core::utils::position::lsp_position_to_text_offset;
use crate::core::utils::uri::uri_to_path;

pub struct CursorContext<'a> {
  pub db: &'a TypedownDatabase,
  pub project: Project,
  pub file: File,
  pub rope: Rope,
  /// Cursor offset in chars
  pub offset: usize,
  /// Innermost node at the char just before the cursor, so a just-typed token resolves to itself
  pub node: RedNode,
}

impl<'a> CursorContext<'a> {
  pub fn new(analysis: &'a Analysis, uri: &Uri, position: Position) -> Option<Self> {
    let db = &analysis.db;
    let project = analysis.project;

    let path = uri_to_path(uri)?;
    let rope = analysis.file_rope(&path)?;
    let offset = lsp_position_to_text_offset(&rope, position)?;

    let file = *project.files(db).get(&path)?;
    let root = parse_file(db, project, file).ast(db).node.clone();
    let node = node_at_offset(root, offset.saturating_sub(1))?;

    Some(Self {
      db,
      project,
      file,
      rope,
      offset,
      node,
    })
  }
}
