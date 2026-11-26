//! Members of the left-hand type after `lhs.`

use lsp_types::{CompletionItem, CompletionItemKind};
use typedown_lang::db::derived::typechecker::actual_node_typ::actual_node_typ;
use typedown_lang::db::types::{ProjectRedNode, TdStaticTyp};
use typedown_lang::syntax::ast::{AstNode, BinaryExpr};

use crate::lsp::service::utils::cursor::CursorContext;

pub(super) fn complete(ctx: &CursorContext, binary: &BinaryExpr) -> Option<Vec<CompletionItem>> {
  let mut items = Vec::new();

  let Some(lhs) = binary.left() else {
    return Some(items);
  };
  let project_node = ProjectRedNode::new(ctx.project, ctx.file, lhs.syntax().clone());
  let Some(lhs_typ) = actual_node_typ(ctx.db, project_node).typ(ctx.db) else {
    return Some(items);
  };

  for (name, _) in lhs_typ.get_fields(ctx.db) {
    // Skip internal fields
    if name.starts_with('_') {
      continue;
    }
    items.push(CompletionItem {
      label: name,
      kind: Some(CompletionItemKind::FIELD),
      ..Default::default()
    });
  }

  for (name, _) in lhs_typ.static_vtable(ctx.db) {
    items.push(CompletionItem {
      label: name,
      kind: Some(CompletionItemKind::METHOD),
      ..Default::default()
    });
  }

  Some(items)
}
