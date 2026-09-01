//! Shared utilities for locating and resolving symbols at the cursor position

use std::path::PathBuf;

use lsp_types::Range;
use ropey::Rope;
use typedown_incremental::QueryDatabase;
use typedown_lang::db::TypedownDatabase;
use typedown_lang::db::derived::evaluate::evaluate_resource::evaluate_resource;
use typedown_lang::db::derived::parse_file::parse_file;
use typedown_lang::db::types::{File, Project, Symbol, SymbolKind, TdRuntimeObject};
use typedown_lang::syntax::ast::{AstNode, CallExpr, IdentLit};
use typedown_lang::syntax::red::RedNode;
use typedown_lang::syntax::syntax_kind::SyntaxKind;

use crate::core::utils::ast::{containing_fref_expr, find_ancestor, node_at_offset};
use crate::core::utils::position::text_offset_to_lsp_position;

/// The kind of symbol reference found at the cursor position
pub enum CursorSymbol {
  Fref { call_node: CallExpr },
  Identifier { ident_node: IdentLit },
}

impl CursorSymbol {
  /// Return the LSP range of the referenceable span at the cursor
  pub fn get_range(&self, rope: &Rope) -> Range {
    let (offset, len) = match self {
      CursorSymbol::Fref { call_node } => {
        let arg = call_node.arg(0).expect("fref must have an argument");
        let content =
          str_content_node(arg.syntax()).expect("fref argument must be a string literal");
        content.trimmed_range()
      }
      CursorSymbol::Identifier { ident_node } => ident_node.syntax().trimmed_range(),
    };
    Range {
      start: text_offset_to_lsp_position(rope, offset),
      end: text_offset_to_lsp_position(rope, offset + len),
    }
  }
}

/// Find the fref or ident symbol reference at a given offset in a file
pub fn find_symbol_at_cursor(
  db: &TypedownDatabase,
  project: Project,
  file: File,
  offset: usize,
) -> Option<CursorSymbol> {
  let root = parse_file(db, project, file).ast(db);
  let node = node_at_offset(root, offset)?;

  if let Some(call_expr) = containing_fref_expr(&node) {
    // reject interpolated fref arguments, no single symbol to resolve
    let arg = call_expr.arg(0)?;
    if arg
      .syntax()
      .children()
      .any(|c| c.kind() == SyntaxKind::InterpFragment)
    {
      return None;
    }
    return Some(CursorSymbol::Fref {
      call_node: call_expr,
    });
  }

  find_ancestor(&node, SyntaxKind::IdentLit)
    .and_then(IdentLit::cast)
    .map(|ident_node| CursorSymbol::Identifier { ident_node })
}

/// Get the file path backing a user-defined symbol
pub fn symbol_file_path(db: &dyn QueryDatabase, symbol: Symbol) -> Option<PathBuf> {
  match symbol.kind(db) {
    SymbolKind::UserDefinedSchema(_, file)
    | SymbolKind::UserDefinedResource(_, file)
    | SymbolKind::Asset(_, _, file) => file.handle(db).path().cloned(),
    _ => None,
  }
}

/// Find the string content node inside a StrLit
pub fn str_content_node(str_lit: &RedNode) -> Option<RedNode> {
  str_lit.children().find(|c| {
    matches!(
      c.kind(),
      SyntaxKind::DqStrContent | SyntaxKind::SqStrContent
    )
  })
}

// Get the _label string from a resource symbol
pub fn get_resource_label(db: &TypedownDatabase, sym: Symbol) -> Option<String> {
  let obj = evaluate_resource(db, sym).value(db)?;
  let field = obj.get_builtin_field(db, "_label")?;
  let str_obj = field.as_td_str_obj()?;
  Some(str_obj.value(db))
}
