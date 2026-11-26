//! Field names for a mapping key or blank line, from the mapping's declared type

use lsp_types::{CompletionItem, CompletionItemKind, InsertTextFormat};
use ropey::Rope;
use typedown_lang::db::TypedownDatabase;
use typedown_lang::db::derived::evaluate::evaluate_typ::evaluate_typ;
use typedown_lang::db::derived::name_resolver::members::members;
use typedown_lang::db::derived::typechecker::expected_node_typ::expected_node_typ;
use typedown_lang::db::types::typecheck::is_nullable;
use typedown_lang::db::types::{
  File, Project, ProjectRedNode, Scope, ScopeKind, TdStaticTyp, TdTypEnum,
};
use typedown_lang::db::utils::get_mapping_schema_name;
use typedown_lang::syntax::ast::{AstNode, Expr};
use typedown_lang::syntax::red::RedNode;
use typedown_lang::syntax::syntax_kind::SyntaxKind;

use crate::core::utils::ast::{find_ancestor, is_in_mapping_value_position};
use crate::lsp::service::utils::cursor::CursorContext;
use crate::lsp::service::utils::expected_typ::{
  has_nullable_member, resolve_field_typ_from_schema,
};

pub(super) fn complete(ctx: &CursorContext) -> Option<Vec<CompletionItem>> {
  let (typ, mapping) = find_enclosing_mapping_typ(ctx.db, ctx.project, ctx.file, &ctx.node)?;

  // The cursor may sit on a blank line inside a nested product-typed value that has no mapping node yet
  if let Some(nested_typ) = find_product_typ_at_deeper_indent(
    ctx.db,
    ctx.project,
    ctx.file,
    &mapping,
    &ctx.rope,
    ctx.offset,
  ) {
    return Some(collect_field_completions_from_typ(ctx.db, &nested_typ, &[]));
  }

  let existing = collect_existing_keys(&mapping);
  Some(collect_field_completions_from_typ(ctx.db, &typ, &existing))
}

fn find_enclosing_mapping_typ<'db>(
  db: &'db TypedownDatabase,
  project: Project,
  file: File,
  node: &RedNode,
) -> Option<(TdTypEnum<'db>, RedNode)> {
  if is_in_mapping_value_position(node) {
    return None;
  }
  let mapping = find_ancestor(node, SyntaxKind::YamlMapping)?;

  // Explicit _typ in this mapping
  if let Some(schema_name) = get_mapping_schema_name(&mapping) {
    let scope = Scope::new(db, ScopeKind::Project(project));
    let symbol = *members(db, scope).members(db).get(&schema_name)?;
    let typ = evaluate_typ(db, symbol).typ(db)?;
    return Some((typ, mapping));
  }

  // No explicit _typ, resolve via the parent field's declared type
  let mapping_expr = Expr::cast(mapping.clone())?;
  let project_node = ProjectRedNode::new(project, file, mapping_expr.syntax().clone());
  let typ = expected_node_typ(db, project_node).typ(db)?;
  if typ.is_td_product_typ() || typ.is_td_schema_typ() {
    return Some((typ, mapping));
  }
  None
}

// When cursor is indented deeper than mapping entries, find the product type for the last entry whose key is at a lesser column than the cursor
fn find_product_typ_at_deeper_indent<'db>(
  db: &'db TypedownDatabase,
  project: Project,
  file: File,
  mapping: &RedNode,
  rope: &Rope,
  cursor_offset: usize,
) -> Option<TdTypEnum<'db>> {
  let cursor_line = rope.char_to_line(cursor_offset);
  let cursor_column = cursor_offset - rope.line_to_char(cursor_line);

  // Walk entries in order
  // Track the last one whose key column < cursor column and that starts before the cursor position
  let mut candidate_key: Option<String> = None;

  for entry in mapping
    .children()
    .filter(|child| child.kind() == SyntaxKind::YamlMappingEntry)
  {
    if entry.start_offset() >= cursor_offset {
      break;
    }
    let Some(key_node) = entry
      .children()
      .find(|child| child.kind() == SyntaxKind::YamlMappingEntryKey)
    else {
      continue;
    };
    let key_line = rope.char_to_line(key_node.start_offset());
    let key_column = key_node.start_offset() - rope.line_to_char(key_line);

    if key_column < cursor_column {
      candidate_key = Some(key_node.text().trim().to_string());
    }
  }

  let key = candidate_key?;
  let field_typ = resolve_field_typ_from_schema(db, project, file, mapping, &key)?;

  if field_typ.is_td_product_typ() {
    Some(field_typ)
  } else if has_nullable_member(db, &field_typ, TdTypEnum::is_td_product_typ) {
    field_typ.as_td_sum_typ().and_then(|sum| {
      sum
        .members(db)
        .iter()
        .find_map(|member| member.resolve(db).filter(|typ| typ.is_td_product_typ()))
    })
  } else {
    None
  }
}

// Collect existing key names from a mapping node
fn collect_existing_keys(mapping: &RedNode) -> Vec<String> {
  mapping
    .children()
    .filter(|child| child.kind() == SyntaxKind::YamlMappingEntry)
    .filter_map(|entry| {
      entry
        .children()
        .find(|child| child.kind() == SyntaxKind::YamlMappingEntryKey)
        .map(|key| key.text().trim().to_string())
    })
    .collect()
}

// Suggest field names from a resolved type, excluding already-present keys
fn collect_field_completions_from_typ(
  db: &TypedownDatabase,
  typ: &TdTypEnum,
  existing: &[String],
) -> Vec<CompletionItem> {
  let fields = typ.get_fields(db);
  let mut items: Vec<CompletionItem> = fields
    .iter()
    .filter(|(field, _)| !existing.iter().any(|key| key == *field))
    .map(|(field, lazy)| {
      let optional = lazy.resolve(db).is_some_and(|typ| is_nullable(db, &typ));
      let detail = if optional { "optional" } else { "required" };

      CompletionItem {
        label: field.clone(),
        kind: Some(CompletionItemKind::FIELD),
        detail: Some(detail.to_string()),
        sort_text: Some(format!("{}{}", if optional { "1" } else { "0" }, field)),
        ..Default::default()
      }
    })
    .collect();

  // Add _meta snippet if not already present
  if !existing.iter().any(|key| key == "_meta") {
    items.push(CompletionItem {
      label: "_meta".to_string(),
      kind: Some(CompletionItemKind::SNIPPET),
      detail: Some("SEO overrides".to_string()),
      insert_text: Some(
        r#"_meta:
  title: "$1"
  description: "$2""#
          .to_string(),
      ),
      insert_text_format: Some(InsertTextFormat::SNIPPET),
      sort_text: Some("2_meta".to_string()),
      ..Default::default()
    });
  }

  items
}
