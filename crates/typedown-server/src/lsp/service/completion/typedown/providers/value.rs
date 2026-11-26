//! Values: keywords, variables in scope, and typed extras (enums, dates, lists, `fref("path")`)
//!
//! Offered after `key:`, in list items, dict values and call arguments
//! Each `push_*` is an independent contributor; the expected type only adds to the base set

use lsp_types::{CompletionItem, CompletionItemKind, InsertTextFormat};
use typedown_lang::db::TypedownDatabase;
use typedown_lang::db::derived::name_resolver::members::all_visible_members;
use typedown_lang::db::derived::name_resolver::scope::scope;
use typedown_lang::db::types::typecheck::is_nullable;
use typedown_lang::db::types::{
  LitValue, ProjectRedNode, Scope, ScopeKind, SymbolKind, TdStaticTyp, TdTypEnum,
};
use typedown_lang::syntax::ast::AstNode;
use typedown_lang::syntax::syntax_kind::SyntaxKind;

use super::fref::collect_fref_wrapped_completions;
use crate::core::utils::ast::{find_ancestor, get_nearest_expr_ancestor};
use crate::lsp::service::completion::typedown::items::scope_symbol_kind;
use crate::lsp::service::utils::cursor::CursorContext;
use crate::lsp::service::utils::expected_typ::{is_or_nullable, typ_at_value_position};

pub(super) fn complete(ctx: &CursorContext) -> Vec<CompletionItem> {
  let expected = typ_at_value_position(ctx.db, ctx.project, ctx.file, &ctx.node);
  let inside_str = find_ancestor(&ctx.node, SyntaxKind::StringLit).is_some();

  let mut items = vec![];

  // A schema-typed (or nullable schema) value outside a string can reference matching files
  if !inside_str
    && let Some(typ) = &expected
    && is_or_nullable(ctx.db, typ, TdTypEnum::is_td_schema_typ)
  {
    items.extend(collect_fref_wrapped_completions(
      ctx.db,
      ctx.project,
      Some(typ),
    ));
  }

  items.extend([make_keyword_item("true"), make_keyword_item("false")]);
  push_scope_symbols(ctx, inside_str, &mut items);
  if let Some(typ) = &expected {
    push_typed_values(ctx.db, typ, inside_str, &mut items);
  }
  items
}

/// Variables, builtins, schemas and files visible at the cursor
fn push_scope_symbols(ctx: &CursorContext, inside_str: bool, items: &mut Vec<CompletionItem>) {
  // Resolve the scope from the nearest expression so enclosing closure params are visible
  let current_scope = get_nearest_expr_ancestor(&ctx.node)
    .map(|expr| {
      let project_node = ProjectRedNode::new(ctx.project, ctx.file, expr.syntax().clone());
      scope(ctx.db, project_node)
    })
    .unwrap_or_else(|| Scope::new(ctx.db, ScopeKind::File(ctx.project, ctx.file)));

  for (name, symbol) in all_visible_members(ctx.db, current_scope) {
    // Assets are not valid expression values
    if name.starts_with('_') || matches!(symbol.kind(ctx.db), SymbolKind::Asset(..)) {
      continue;
    }
    // fref is usable in any expression position outside string literals
    if name == "fref" && inside_str {
      continue;
    }
    let kind = scope_symbol_kind(ctx.db, symbol);
    // fref inserts a snippet with quotes and the cursor inside
    if name == "fref" {
      items.push(CompletionItem {
        label: name,
        kind: Some(kind),
        insert_text: Some(r#"fref("$1")"#.to_string()),
        insert_text_format: Some(InsertTextFormat::SNIPPET),
        ..Default::default()
      });
    } else {
      items.push(CompletionItem {
        label: name,
        kind: Some(kind),
        ..Default::default()
      });
    }
  }
}

/// Extras that depend on the expected type: null, enum members, dates and times, list scaffold
fn push_typed_values<'db>(
  db: &'db TypedownDatabase,
  typ: &TdTypEnum<'db>,
  inside_str: bool,
  items: &mut Vec<CompletionItem>,
) {
  if is_nullable(db, typ) {
    items.push(make_keyword_item("null"));
  }
  collect_enum_items(db, typ, inside_str, items);

  if is_or_nullable(db, typ, TdTypEnum::is_td_date_typ) {
    push_temporal_items(
      items,
      inside_str,
      "today",
      current_iso_date_str(),
      "YYYY-MM-DD",
      "ISO 8601 date",
    );
  }
  if is_or_nullable(db, typ, TdTypEnum::is_td_time_typ) {
    push_temporal_items(
      items,
      inside_str,
      "now",
      current_iso_time_str(),
      "HH:MM:SS",
      "ISO 8601 time",
    );
  }
  if is_or_nullable(db, typ, TdTypEnum::is_td_date_time_typ) {
    push_temporal_items(
      items,
      inside_str,
      "now",
      current_iso_datetime_str(),
      "YYYY-MM-DDTHH:MM:SS",
      "ISO 8601 datetime",
    );
  }

  if is_or_nullable(db, typ, TdTypEnum::is_td_list_typ) {
    items.push(CompletionItem {
      label: "- ...".to_string(),
      insert_text: Some("\n  - $1".to_string()),
      insert_text_format: Some(InsertTextFormat::SNIPPET),
      detail: Some("List".to_string()),
      kind: Some(CompletionItemKind::SNIPPET),
      ..Default::default()
    });
  }
}

/// A quick-insert of the current value (`today`, `now`) plus a placeholder to type over
fn push_temporal_items(
  items: &mut Vec<CompletionItem>,
  inside_str: bool,
  quick_label: &str,
  current_value: String,
  placeholder: &str,
  placeholder_detail: &str,
) {
  let (quick_insert, placeholder_label, placeholder_insert) = if inside_str {
    (
      current_value.clone(),
      placeholder.to_string(),
      "$1".to_string(),
    )
  } else {
    (
      format!("\"{current_value}\""),
      format!("\"{placeholder}\""),
      "\"$1\"".to_string(),
    )
  };
  items.push(CompletionItem {
    label: quick_label.to_string(),
    insert_text: Some(quick_insert),
    insert_text_format: Some(InsertTextFormat::PLAIN_TEXT),
    detail: Some(current_value),
    kind: Some(CompletionItemKind::VALUE),
    ..Default::default()
  });
  items.push(CompletionItem {
    label: placeholder_label,
    insert_text: Some(placeholder_insert),
    insert_text_format: Some(InsertTextFormat::SNIPPET),
    detail: Some(placeholder_detail.to_string()),
    kind: Some(CompletionItemKind::VALUE),
    ..Default::default()
  });
}

// Collect literal values from a union type as completion items
// When inside_str is true, string values are inserted without surrounding quotes
fn collect_enum_items(
  db: &TypedownDatabase,
  typ: &TdTypEnum,
  inside_str: bool,
  items: &mut Vec<CompletionItem>,
) {
  let sum = if let Some(sum) = typ.as_td_sum_typ() {
    sum
  } else {
    return;
  };
  for member in sum.members(db) {
    let Some(resolved_member) = member.resolve(db) else {
      continue;
    };
    let Some(lit) = resolved_member.as_td_lit_typ() else {
      continue;
    };
    let (label, insert_text, detail) = match lit.value(db) {
      LitValue::String(string) => {
        let label = format!("\"{string}\"");
        let insert = if inside_str {
          string.clone()
        } else {
          label.clone()
        };
        (label, insert, "string".to_string())
      }
      LitValue::Number(number) => (number.clone(), number.clone(), "number".to_string()),
      LitValue::Bool(bool) => (bool.to_string(), bool.to_string(), "boolean".to_string()),
      LitValue::Typ(typ) => {
        let name = typ.display_name(db);
        (name.clone(), name, "type".to_string())
      }
    };
    items.push(CompletionItem {
      label,
      insert_text: Some(insert_text),
      detail: Some(detail),
      kind: Some(CompletionItemKind::ENUM_MEMBER),
      ..Default::default()
    });
  }
}

// Build a keyword completion item (true, false, null)
fn make_keyword_item(label: &str) -> CompletionItem {
  CompletionItem {
    label: label.to_string(),
    kind: Some(CompletionItemKind::KEYWORD),
    ..Default::default()
  }
}

// Returns current date in YYYY-MM-DD format
fn current_iso_date_str() -> String {
  chrono::Utc::now().format("%Y-%m-%d").to_string()
}

// Returns current time in HH:MM:SS format
fn current_iso_time_str() -> String {
  chrono::Utc::now().format("%H:%M:%S").to_string()
}

// Returns current datetime in YYYY-MM-DDTHH:MM:SS format
fn current_iso_datetime_str() -> String {
  chrono::Utc::now().format("%Y-%m-%dT%H:%M:%S").to_string()
}
