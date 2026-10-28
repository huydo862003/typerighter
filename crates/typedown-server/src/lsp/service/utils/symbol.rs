//! Shared utilities for locating and resolving symbols at the cursor position

use std::path::PathBuf;

use lsp_types::Range;
use ropey::Rope;
use typedown_incremental::QueryDatabase;
use typedown_lang::db::TypedownDatabase;
use typedown_lang::db::derived::evaluate::evaluate_resource::evaluate_resource;
use typedown_lang::db::derived::get_vault_config::get_vault_config;
use typedown_lang::db::derived::parse_file::parse_file;
use typedown_lang::db::types::{File, Project, Symbol, SymbolKind, TdRuntimeObj};
use typedown_lang::db::utils::{is_external_url, resolve_md_link_path};
use typedown_lang::syntax::ast::{AstNode, CallExpr, IdentLit, MdLink, MdMedia};
use typedown_lang::syntax::red::RedNode;
use typedown_lang::syntax::syntax_kind::SyntaxKind;

use crate::core::utils::ast::{containing_fref_expr, find_ancestor, node_at_offset};
use crate::core::utils::position::text_offset_to_lsp_position;

/// The kind of symbol reference found at the cursor position
pub enum CursorSymbol {
  Fref {
    call_node: CallExpr,
  },
  Identifier {
    ident_node: IdentLit,
  },
  /// Cursor is on a URL inside [text](url) or ![alt](url) that points to a vault file
  MdLinkUrl {
    url_node: RedNode,
    target_file: File,
  },
}

impl CursorSymbol {
  /// Return the LSP range of the referenceable span at the cursor
  pub fn get_range(&self, rope: &Rope) -> Range {
    let (offset, len) = match self {
      CursorSymbol::Fref { call_node } => {
        let arg = call_node.arg(0).expect("fref must have an argument");
        let content =
          get_string_content_node(arg.syntax()).expect("fref argument must be a string literal");
        content.trimmed_range()
      }
      CursorSymbol::Identifier { ident_node } => ident_node.syntax().trimmed_range(),
      CursorSymbol::MdLinkUrl { url_node, .. } => url_node.trimmed_range(),
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
  let root = parse_file(db, project, file).ast(db).node.clone();
  let node = node_at_offset(root, offset)?;

  if let Some(call_expr) = containing_fref_expr(&node) {
    // reject interpolated fref arguments, no single symbol to resolve
    let arg = call_expr.arg(0)?;
    if arg
      .syntax()
      .children()
      .any(|child| child.kind() == SyntaxKind::InterpFragment)
    {
      return None;
    }
    return Some(CursorSymbol::Fref {
      call_node: call_expr,
    });
  }

  // Check if cursor is on a markdown link/media URL that points to a vault file
  if let Some(cursor_symbol) = find_md_link_url_symbol(db, project, file, &node) {
    return Some(cursor_symbol);
  }

  find_ancestor(&node, SyntaxKind::IdentLit)
    .and_then(IdentLit::cast)
    .map(|ident_node| CursorSymbol::Identifier { ident_node })
}

/// Check if the cursor is inside the URL part of a [text](url) or ![alt](url) and the URL resolves to a vault file
fn find_md_link_url_symbol(
  db: &TypedownDatabase,
  project: Project,
  file: File,
  node: &RedNode,
) -> Option<CursorSymbol> {
  // Find enclosing MdLink or MdMedia
  let (url_text, url_node) = if let Some(link_node) = find_ancestor(node, SyntaxKind::MdLink) {
    let link = MdLink::cast(link_node)?;
    let url = link.url()?;
    (url.value(), url.syntax().clone())
  } else {
    let media_node = find_ancestor(node, SyntaxKind::MdMedia)?;
    let media = MdMedia::cast(media_node)?;
    let url = media.url()?;
    (url.value(), url.syntax().clone())
  };

  // Check that the cursor is actually inside the URL node, not the alt/text part
  let (url_start, url_len) = url_node.trimmed_range();
  let (node_start, _) = node.trimmed_range();
  if node_start < url_start || node_start >= url_start + url_len {
    return None;
  }

  // Skip external URLs and anchors
  if is_external_url(&url_text) || url_text.starts_with('#') {
    return None;
  }

  // Resolve to vault file path
  let config = get_vault_config(db, project);
  let root_dir = config.root_dir(db);
  let base_path = config.base_path(db);
  let file_dir = file.handle(db).path()?.parent()?.to_path_buf();
  let target_path = resolve_md_link_path(&url_text, &file_dir, &root_dir, &base_path);

  let target_file = project.files(db).get(&target_path).copied()?;
  Some(CursorSymbol::MdLinkUrl {
    url_node,
    target_file,
  })
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
pub fn get_string_content_node(string_lit: &RedNode) -> Option<RedNode> {
  string_lit.children().find(|child| {
    matches!(
      child.kind(),
      SyntaxKind::DqStrContent | SyntaxKind::SqStrContent
    )
  })
}

// Get the _label string from a resource symbol
pub fn get_resource_label(db: &TypedownDatabase, symbol: Symbol) -> Option<String> {
  let obj = evaluate_resource(db, symbol).value(db)?;
  let field = obj.get_builtin_field(db, "_label")?;
  let string_obj = field.as_td_string_obj()?;
  Some(string_obj.value(db))
}
