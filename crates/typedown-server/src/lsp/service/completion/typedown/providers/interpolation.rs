//! Inside `${...}`: file references and the variables in scope

use lsp_types::CompletionItem;
use typedown_lang::db::derived::name_resolver::members::all_visible_members;
use typedown_lang::db::types::{Scope, ScopeKind};

use super::fref::collect_fref_wrapped_completions;
use crate::lsp::service::completion::typedown::items::scope_symbol_kind;
use crate::lsp::service::utils::cursor::CursorContext;

pub(super) fn complete(ctx: &CursorContext) -> Vec<CompletionItem> {
  let mut items = collect_fref_wrapped_completions(ctx.db, ctx.project, None);

  let file_scope = Scope::new(ctx.db, ScopeKind::File(ctx.project, ctx.file));
  for (name, symbol) in all_visible_members(ctx.db, file_scope) {
    if name.starts_with('_') {
      continue;
    }
    items.push(CompletionItem {
      label: name,
      kind: Some(scope_symbol_kind(ctx.db, symbol)),
      ..Default::default()
    });
  }
  items
}
