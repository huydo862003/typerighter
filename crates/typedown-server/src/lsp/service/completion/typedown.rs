use typedown_incremental::StableCompare;
use typedown_lang::db::types::{Scope, ScopeKind};
use typedown_lang::db::utils::{is_content_file, is_type_file};

use crate::lsp::service::utils::symbol::get_resource_label;
use lsp_types::{
  CompletionItem, CompletionItemKind, CompletionParams, CompletionResponse, InsertTextFormat,
};
use typedown_lang::db::TypedownDatabase;
use typedown_lang::db::derived::evaluate::evaluate_type::evaluate_type;
use typedown_lang::db::derived::get_vault_config::get_vault_config;
use typedown_lang::db::derived::hir::lower_node;
use typedown_lang::db::derived::name_resolver::file_symbol::file_symbol;
use typedown_lang::db::derived::name_resolver::members::{all_visible_members, members};
use typedown_lang::db::derived::name_resolver::scope::scope;
use typedown_lang::db::derived::parse_file::parse_file;
use typedown_lang::db::derived::typechecker::actual_node_type::actual_node_type;
use typedown_lang::db::derived::typechecker::expected_node_type::expected_node_type;
use typedown_lang::db::derived::typechecker::get_symbol_type::get_symbol_type;
use typedown_lang::db::typecheck::utils::{is_nullable, is_subtype_of};
use typedown_lang::db::types::{
  File, FileRedNode, LazyType, LiteralValue, Project, SymbolKind, TdStaticType, TdTypeEnum,
};
use typedown_lang::db::utils::get_mapping_schema_name;
use typedown_lang::syntax::ast::{AstNode, BinaryExpr, Expr, YamlOpKind};
use typedown_lang::syntax::red::RedNode;
use typedown_lang::syntax::syntax_kind::SyntaxKind;

use crate::core::analysis::Analysis;
use crate::core::utils::ast::{
  find_ancestor, get_nearest_expr_ancestor, is_in_mapping_value_position, node_at_offset,
};
use crate::core::utils::position::lsp_position_to_text_offset;
use crate::core::utils::uri::uri_to_path;

pub fn completion(analysis: &Analysis, params: CompletionParams) -> Option<CompletionResponse> {
  let db = &analysis.db;
  let project = analysis.project;

  let path = uri_to_path(&params.text_document_position.text_document.uri)?;
  let rope = analysis.file_rope(&path)?;
  let offset = lsp_position_to_text_offset(&rope, params.text_document_position.position)?;

  let file = *project.files(db).get(&path)?;
  let root = parse_file(db, project, file).ast(db).node.clone();
  // Use offset-1 so the cursor position (between characters) resolves to the token just typed
  let lookup = offset.saturating_sub(1);
  let node = node_at_offset(root, lookup)?;

  // Cursor in a _type value: suggest schema names
  if is_type_value_position(&node) {
    return Some(CompletionResponse::Array(schema_completions(db, project)));
  }

  // Cursor after "." in a dot expression: suggest members of the LHS type
  if let Some(items) = dot_access_completions(db, project, file, &node) {
    return Some(CompletionResponse::Array(items));
  }

  // Cursor inside a fref() string argument: suggest file paths
  if is_fref_arg_position(&node) {
    return Some(CompletionResponse::Array(fref_completions(
      db, project, file, &node,
    )));
  }

  // Cursor inside ${} interpolation in markdown: suggest fref("path") completions
  if is_interp_position(&node) {
    return Some(CompletionResponse::Array(fref_wrapped_completions(
      db, project, None,
    )));
  }

  // Cursor in a field value whose type is a schema: suggest fref("path") completions
  if let Some(typ) = declared_field_type_at_value(db, project, file, &node)
    && (typ.is_td_schema_type() || has_nullable_member(db, &typ, TdTypeEnum::is_td_schema_type))
  {
    let expected = declared_field(db, project, file, &node);
    let items = fref_wrapped_completions(db, project, expected.as_ref());
    if !items.is_empty() {
      return Some(CompletionResponse::Array(items));
    }
  }

  // Cursor in a field value: suggest value completions (booleans, null for optional fields)
  if let Some(items) = value_completions(db, project, file, &node) {
    return Some(CompletionResponse::Array(items));
  }

  // Cursor in a mapping key or blank line: suggest field names from the declared type
  if let Some((typ, mapping)) = enclosing_mapping_type(db, project, file, &node) {
    let existing = existing_keys(&mapping);
    return Some(CompletionResponse::Array(field_completions_from_type(
      db, &typ, &existing,
    )));
  }

  None
}

// Returns true if the cursor is inside the value of a _type mapping entry
fn is_type_value_position(node: &RedNode) -> bool {
  let Some(entry) = find_ancestor(node, SyntaxKind::YamlMappingEntry) else {
    return false;
  };
  let Some(key) = entry
    .children()
    .find(|child| child.kind() == SyntaxKind::YamlMappingEntryKey)
  else {
    return false;
  };
  key.text().trim() == "_type"
}

// Suggest members of the LHS type after a "." in a binary expression
fn dot_access_completions(
  db: &TypedownDatabase,
  project: Project,
  file: File,
  node: &RedNode,
) -> Option<Vec<CompletionItem>> {
  let bin_node = find_ancestor(node, SyntaxKind::BinaryExpr)?;
  let bin = BinaryExpr::cast(bin_node)?;

  if !matches!(bin.op()?.kind()?, YamlOpKind::Dot) {
    return None;
  }

  let lhs = bin.left()?;
  let hir = lower_node(db, project, FileRedNode::new(file, lhs.syntax().clone()));
  let lhs_type = actual_node_type(db, hir).typ(db)?;

  let mut items = Vec::new();

  for (name, _) in lhs_type.get_fields(db) {
    items.push(CompletionItem {
      label: name,
      kind: Some(CompletionItemKind::FIELD),
      ..Default::default()
    });
  }

  for (name, _) in lhs_type.static_vtable(db) {
    items.push(CompletionItem {
      label: name,
      kind: Some(CompletionItemKind::METHOD),
      ..Default::default()
    });
  }

  if items.is_empty() { None } else { Some(items) }
}

// Returns true if the cursor is inside a ${} interpolation
fn is_interp_position(node: &RedNode) -> bool {
  find_ancestor(node, SyntaxKind::InterpFragment).is_some()
}

// Returns true if the cursor is inside the string argument of a fref() call
fn is_fref_arg_position(node: &RedNode) -> bool {
  // Walk up to find an enclosing StrLit, then a CallExpr above it
  let str_lit = find_ancestor(node, SyntaxKind::StrLit);
  let call = match str_lit {
    Some(ref lit) => find_ancestor(lit, SyntaxKind::CallExpr),
    None => find_ancestor(node, SyntaxKind::CallExpr),
  };
  let Some(call) = call else {
    return false;
  };
  // Check callee text is "fref"
  call
    .children()
    .next()
    .is_some_and(|callee| callee.text().trim() == "fref")
}

// A resolved fref candidate with display info
struct FrefCandidate {
  // Vault-relative path (e.g. "people/alice.td")
  path: String,
  // Human-readable label from _label field
  label: Option<String>,
  // Schema type name
  schema: Option<String>,
  // File basename without extension
  basename: String,
}

// Collect fref candidates compatible with an optional expected type
fn collect_fref_candidates(
  db: &TypedownDatabase,
  project: Project,
  expected_type: Option<&TdTypeEnum>,
) -> Vec<FrefCandidate> {
  let config = get_vault_config(db, project);
  let root_dir = config.root_dir(db);
  project
    .files(db)
    .iter()
    .filter(|(path, _)| path.starts_with(&root_dir) && is_content_file(path) && !is_type_file(path))
    .filter_map(|(path, target_file)| {
      let sym = file_symbol(db, project, *target_file).value(db)?;

      // Filter by expected type if provided
      if let Some(expected_typ) = expected_type {
        let file_type = get_symbol_type(db, sym).typ(db)?;
        if !is_subtype_of(db, &file_type, expected_typ) {
          return None;
        }
      }

      let rel = path.strip_prefix(&root_dir).ok()?;
      let rel_str = rel.to_string_lossy().into_owned();
      let label = get_resource_label(db, sym);
      let schema = get_symbol_type(db, sym).typ(db).map(|t| t.display_name(db));
      let basename = rel
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or_default()
        .to_string();

      Some(FrefCandidate {
        path: rel_str,
        label,
        schema,
        basename,
      })
    })
    .collect()
}

// Build a CompletionItem from a candidate, wrapping the path with the given template
fn candidate_to_completion(
  candidate: &FrefCandidate,
  insert_template: impl Fn(&str) -> String,
) -> CompletionItem {
  let display = candidate.label.as_deref().unwrap_or(&candidate.basename);

  // filterText includes path, basename, and label for fuzzy matching
  let mut filter_parts = vec![candidate.path.clone(), candidate.basename.clone()];
  if let Some(ref label) = candidate.label {
    filter_parts.push(label.clone());
  }

  CompletionItem {
    label: display.to_string(),
    insert_text: Some(insert_template(&candidate.path)),
    detail: Some(candidate.path.clone()),
    label_details: candidate
      .schema
      .as_ref()
      .map(|s| lsp_types::CompletionItemLabelDetails {
        detail: Some(s.clone()),
        description: None,
      }),
    filter_text: Some(filter_parts.join(" ")),
    kind: Some(CompletionItemKind::REFERENCE),
    ..Default::default()
  }
}

// Suggest file paths inside fref("") argument
fn fref_completions(
  db: &TypedownDatabase,
  project: Project,
  file: File,
  node: &RedNode,
) -> Vec<CompletionItem> {
  let expected_type = declared_field(db, project, file, node);
  let candidates = collect_fref_candidates(db, project, expected_type.as_ref());
  candidates
    .iter()
    .map(|c| candidate_to_completion(c, |path| path.to_string()))
    .collect()
}

// Resolve the declared field type at a value position or empty value after a colon
fn declared_field_type_at_value<'db>(
  db: &'db TypedownDatabase,
  project: Project,
  file: File,
  node: &RedNode,
) -> Option<TdTypeEnum<'db>> {
  if is_in_mapping_value_position(node) {
    return declared_field(db, project, file, node);
  }

  // Empty value: resolve from the entry key name
  let entry = find_ancestor(node, SyntaxKind::YamlMappingEntry)?;
  let key_text = entry_key_text(&entry)?;
  let mapping = entry
    .parent()
    .filter(|p| p.kind() == SyntaxKind::YamlMapping)?;
  resolve_field_type_from_schema(db, project, &mapping, &key_text)
}

// Check if a nullable type (T?) has a member satisfying the predicate
fn has_nullable_member<'db>(
  db: &'db TypedownDatabase,
  typ: &TdTypeEnum<'db>,
  predicate: fn(&TdTypeEnum<'db>) -> bool,
) -> bool {
  typ.as_td_sum_type().is_some_and(|sum| {
    sum
      .members(db)
      .iter()
      .any(|m| m.resolve(db).is_some_and(|t| predicate(&t)))
  })
}

// Suggest fref("path") completions, optionally filtered by an expected type
fn fref_wrapped_completions(
  db: &TypedownDatabase,
  project: Project,
  expected_type: Option<&TdTypeEnum>,
) -> Vec<CompletionItem> {
  let candidates = collect_fref_candidates(db, project, expected_type);
  candidates
    .iter()
    .map(|c| candidate_to_completion(c, |path| format!("fref(\"{path}\")")))
    .collect()
}

fn enclosing_mapping_type<'db>(
  db: &'db TypedownDatabase,
  project: Project,
  file: File,
  node: &RedNode,
) -> Option<(TdTypeEnum<'db>, RedNode)> {
  if is_in_mapping_value_position(node) {
    return None;
  }
  let mapping = find_ancestor(node, SyntaxKind::YamlMapping)?;

  // Explicit _type in this mapping
  if let Some(schema_name) = get_mapping_schema_name(&mapping) {
    let scope = Scope::new(db, ScopeKind::Project(project));
    let symbol = *members(db, scope).members(db).get(&schema_name)?;
    let typ = evaluate_type(db, symbol).typ(db)?;
    return Some((typ, mapping));
  }

  // No explicit _type, resolve via the parent field's declared type
  let mapping_expr = Expr::cast(mapping.clone())?;
  let hir = lower_node(
    db,
    project,
    FileRedNode::new(file, mapping_expr.syntax().clone()),
  );
  let typ = expected_node_type(db, hir).typ(db)?;
  if typ.is_td_product_type() || typ.is_td_schema_type() {
    return Some((typ, mapping));
  }
  None
}

// Suggest value completions based on the declared field type
fn value_completions(
  db: &TypedownDatabase,
  project: Project,
  file: File,
  node: &RedNode,
) -> Option<Vec<CompletionItem>> {
  if !is_in_mapping_value_position(node) {
    return None;
  }

  let mut items = vec![keyword_item("true"), keyword_item("false")];

  // Variables in scope (icon, fref, vault, file names, imports, closure params)
  // Find the nearest Expr ancestor to resolve scope (captures enclosing closures)
  let current_scope = get_nearest_expr_ancestor(node)
    .map(|expr| {
      let hir = lower_node(db, project, FileRedNode::new(file, expr.syntax().clone()));
      scope(db, hir)
    })
    .unwrap_or_else(|| Scope::new(db, ScopeKind::File(project, file)));

  for (name, sym) in all_visible_members(db, current_scope) {
    if name.starts_with('_') {
      continue;
    }
    let kind = match sym.kind(db) {
      SymbolKind::BuiltinMacro(_) => CompletionItemKind::FUNCTION,
      SymbolKind::BuiltinGlobal(_) => CompletionItemKind::VARIABLE,
      SymbolKind::UserDefinedSchema(..) => CompletionItemKind::CLASS,
      _ => CompletionItemKind::VARIABLE,
    };
    items.push(CompletionItem {
      label: name,
      kind: Some(kind),
      ..Default::default()
    });
  }

  // Type-specific completions from declared field type
  let Some(typ) = declared_field(db, project, file, node) else {
    return Some(items);
  };

  if is_nullable(db, &typ) {
    items.push(keyword_item("null"));
  }

  // Enum values from union of literals
  collect_enum_items(db, &typ, &mut items);

  // Date placeholder
  if typ.is_td_date_type() || has_nullable_member(db, &typ, TdTypeEnum::is_td_date_type) {
    items.push(CompletionItem {
      label: "\"YYYY-MM-DD\"".to_string(),
      insert_text: Some("\"$1\"".to_string()),
      insert_text_format: Some(InsertTextFormat::SNIPPET),
      detail: Some("ISO 8601 date".to_string()),
      kind: Some(CompletionItemKind::VALUE),
      ..Default::default()
    });
  }

  // List scaffold
  if typ.is_td_list_type() || has_nullable_member(db, &typ, TdTypeEnum::is_td_list_type) {
    items.push(CompletionItem {
      label: "- ...".to_string(),
      insert_text: Some("\n  - $1".to_string()),
      insert_text_format: Some(InsertTextFormat::SNIPPET),
      detail: Some("List".to_string()),
      kind: Some(CompletionItemKind::SNIPPET),
      ..Default::default()
    });
  }

  Some(items)
}

// Collect literal values from a union type as completion items
fn collect_enum_items(db: &TypedownDatabase, typ: &TdTypeEnum, items: &mut Vec<CompletionItem>) {
  let sum = if let Some(s) = typ.as_td_sum_type() {
    s
  } else {
    return;
  };
  for member in sum.members(db) {
    let Some(resolved) = member.resolve(db) else {
      continue;
    };
    let Some(lit) = resolved.as_td_literal_type() else {
      continue;
    };
    let (label, detail) = match lit.value(db) {
      LiteralValue::Str(s) => (format!("\"{s}\""), "string".to_string()),
      LiteralValue::Num(n) => (n.clone(), "number".to_string()),
      LiteralValue::Bool(b) => (b.to_string(), "boolean".to_string()),
    };
    items.push(CompletionItem {
      label,
      detail: Some(detail),
      kind: Some(CompletionItemKind::ENUM_MEMBER),
      ..Default::default()
    });
  }
}

// Resolve the declared type for the field whose value the cursor is in
fn declared_field<'db>(
  db: &'db TypedownDatabase,
  project: Project,
  file: File,
  node: &RedNode,
) -> Option<TdTypeEnum<'db>> {
  let entry_value = find_ancestor(node, SyntaxKind::YamlMappingEntryValue)?;

  // Try the value expression first
  if let Some(value_expr) = entry_value.children().find_map(Expr::cast) {
    let hir = lower_node(
      db,
      project,
      FileRedNode::new(file, value_expr.syntax().clone()),
    );
    if let Some(typ) = expected_node_type(db, hir).typ(db) {
      return Some(typ);
    }
  }

  // Fall back to looking up the field name in the enclosing schema
  let entry = entry_value.parent()?;
  let key_text = entry_key_text(&entry)?;
  let mapping = find_ancestor(&entry, SyntaxKind::YamlMapping)?;
  resolve_field_type_from_schema(db, project, &mapping, &key_text)
}

// Extract the key text from a YamlMappingEntry node
fn entry_key_text(entry: &RedNode) -> Option<String> {
  entry
    .children()
    .find(|child| child.kind() == SyntaxKind::YamlMappingEntryKey)
    .map(|key| key.text().trim().to_string())
}

// Look up a field's declared type from the enclosing schema
fn resolve_field_type_from_schema<'db>(
  db: &'db TypedownDatabase,
  project: Project,
  mapping: &RedNode,
  key: &str,
) -> Option<TdTypeEnum<'db>> {
  let schema_name = get_mapping_schema_name(mapping)?;
  let scope = Scope::new(db, ScopeKind::Project(project));
  let symbol = *members(db, scope).members(db).get(&schema_name)?;
  let typ = evaluate_type(db, symbol).typ(db)?;
  let schema = typ.as_td_schema_type()?;
  let prop = schema.fields(db).get(key)?.clone();
  prop.field_type.resolve(db)
}

// Build a keyword completion item (true, false, null)
fn keyword_item(label: &str) -> CompletionItem {
  CompletionItem {
    label: label.to_string(),
    kind: Some(CompletionItemKind::KEYWORD),
    ..Default::default()
  }
}

// Suggest all user-defined schema names visible in the project scope
fn schema_completions(db: &TypedownDatabase, project: Project) -> Vec<CompletionItem> {
  let scope = Scope::new(db, ScopeKind::Project(project));
  members(db, scope)
    .members(db)
    .iter()
    .filter(|(_, sym)| matches!(sym.kind(db), SymbolKind::UserDefinedSchema(..)))
    .map(|(name, sym)| {
      let snippet = build_schema_snippet(db, name, sym);

      CompletionItem {
        label: name.clone(),
        kind: Some(CompletionItemKind::CLASS),
        insert_text: Some(snippet),
        insert_text_format: Some(InsertTextFormat::SNIPPET),
        ..Default::default()
      }
    })
    .collect()
}

// Build a snippet with all schema fields as placeholders
fn build_schema_snippet(
  db: &TypedownDatabase,
  name: &str,
  sym: &typedown_lang::db::types::Symbol,
) -> String {
  let typ = evaluate_type(db, *sym).typ(db);
  let schema = typ.as_ref().and_then(|t| t.as_td_schema_type());

  let Some(schema) = schema else {
    return name.to_string();
  };

  let fields = schema.fields(db);
  let mut snippet = name.to_string();
  for (tab_stop, (field_name, prop_desc)) in fields.iter().enumerate() {
    let placeholder = lazy_placeholder(db, &prop_desc.field_type, 0);
    let idx = tab_stop + 1;

    snippet.push_str(&format!("\n{field_name}: ${{{idx}:{placeholder}}}"));
  }

  snippet
}

// Generate a placeholder string for a lazy type
fn lazy_placeholder(db: &TypedownDatabase, lazy: &LazyType, indent: usize) -> String {
  let Some(typ) = lazy.resolve(db) else {
    return "value".to_string();
  };
  match typ {
    TdTypeEnum::TdSumType(sum) => {
      let mut members: Vec<_> = sum.members(db).into_iter().collect();
      members.sort_by(|a, b| a.stable_cmp(db, b));
      // Optional type: use non-null member's placeholder
      let non_null: Vec<_> = members
        .iter()
        .filter(|m| m.resolve(db).is_none_or(|t| t.as_td_null_type().is_none()))
        .collect();
      if non_null.len() == 1 {
        return lazy_placeholder(db, non_null[0], indent);
      }
      // Enum: use first literal string option as default
      let first = members.iter().find_map(|m| {
        if let Some(TdTypeEnum::TdLiteralType(lit)) = m.resolve(db)
          && let LiteralValue::Str(s) = lit.value(db)
        {
          return Some(s);
        }
        None
      });
      first.unwrap_or_else(|| "value".to_string())
    }
    _ => simple_type_placeholder(db, &typ, indent),
  }
}

fn simple_type_placeholder(db: &TypedownDatabase, typ: &TdTypeEnum, indent: usize) -> String {
  match typ {
    TdTypeEnum::TdStrType(_) => "string".to_string(),
    TdTypeEnum::TdNumType(_) => "0".to_string(),
    TdTypeEnum::TdBoolType(_) => "true".to_string(),
    TdTypeEnum::TdDateType(_) => "date".to_string(),
    TdTypeEnum::TdDateTimeType(_) => "datetime".to_string(),
    TdTypeEnum::TdTimeType(_) => "time".to_string(),
    TdTypeEnum::TdListType(list) => {
      let inner = list
        .elem(db)
        .and_then(|elem| elem.resolve(db))
        .map(|elem| simple_type_placeholder(db, &elem, indent + 1))
        .unwrap_or_else(|| "value".to_string());
      let pad = "  ".repeat(indent);

      format!("\\n{pad}- {inner}")
    }
    TdTypeEnum::TdSchemaType(schema) => {
      format!("fref(\\\"{}\\\")", schema.name(db))
    }
    TdTypeEnum::TdProductType(product) => {
      let fields = product.get_fields(db);
      let pad = "  ".repeat(indent + 1);
      let mut nested = String::new();
      for (field_name, field_lazy) in &fields {
        let placeholder = lazy_placeholder(db, field_lazy, indent + 1);
        nested.push_str(&format!("\\n{pad}{field_name}: {placeholder}"));
      }
      nested
    }
    _ => "value".to_string(),
  }
}

// Collect existing key names from a mapping node
fn existing_keys(mapping: &RedNode) -> Vec<String> {
  mapping
    .children()
    .filter(|child| child.kind() == SyntaxKind::YamlMappingEntry)
    .filter_map(|entry| {
      entry
        .children()
        .find(|c| c.kind() == SyntaxKind::YamlMappingEntryKey)
        .map(|key| key.text().trim().to_string())
    })
    .collect()
}

// Suggest field names from a resolved type, excluding already-present keys
fn field_completions_from_type(
  db: &TypedownDatabase,
  typ: &TdTypeEnum,
  existing: &[String],
) -> Vec<CompletionItem> {
  typ
    .get_fields(db)
    .keys()
    .filter(|field| !existing.iter().any(|k| k == *field))
    .map(|field| CompletionItem {
      label: field.clone(),
      kind: Some(CompletionItemKind::FIELD),
      ..Default::default()
    })
    .collect()
}

#[cfg(test)]
mod tests {
  use std::collections::BTreeMap;
  use std::path::PathBuf;
  use std::sync::{Arc, Condvar, Mutex};

  use lsp_types::{
    CompletionParams, CompletionResponse, PartialResultParams, Position, TextDocumentIdentifier,
    TextDocumentPositionParams, Uri, WorkDoneProgressParams,
  };
  use ropey::Rope;
  use typedown_lang::db::types::{File, FileHandle, FileMetadata, Project};
  use typedown_lang::db::{QueryStorage, TypedownDatabase};

  use crate::core::analysis::Analysis;
  use crate::core::utils::uri::path_to_uri;

  use super::completion;

  const VAULT_CONFIG: &str = r#"version: "1"
vault:
  root_dir: "."
"#;
  const SCHEMA_PERSON: &str = r#"---
_type: schema
properties:
  name:
    type: string
  age:
    type: number
  verified:
    type: boolean
  nickname:
    type: string?
---
"#;
  const SCHEMA_EVENT: &str = r#"---
_type: schema
properties:
  name:
    type: string
  date:
    type: date
---
"#;

  const SCHEMA_TASK: &str = r#"---
_type: schema
properties:
  title:
    type: string
  status:
    type: ['todo', 'in_progress', 'done']
  assignee:
    type: Person?
---
"#;

  // Schema with a nested inline object field (no named type reference)
  const SCHEMA_PERSON_WITH_ADDRESS: &str = r#"---
_type: schema
properties:
  name:
    type: string
  address:
    type:
      street:
        type: string
      city:
        type: string
---
"#;

  /// Strip the `|` cursor marker from content and return its char offset.
  fn cursor(content: &str) -> (String, usize) {
    let offset = content
      .find('|')
      .expect("content must have a '|' cursor marker");
    (content.replacen('|', "", 1), offset)
  }

  /// Build CompletionParams from a URI, content string, and char offset.
  fn make_params(uri: Uri, content: &str, offset: usize) -> CompletionParams {
    let rope = Rope::from(content);
    let line = rope.char_to_line(offset);
    let character = offset - rope.line_to_char(line);
    CompletionParams {
      text_document_position: TextDocumentPositionParams {
        text_document: TextDocumentIdentifier { uri },
        position: Position {
          line: line as u32,
          character: character as u32,
        },
      },
      work_done_progress_params: WorkDoneProgressParams::default(),
      partial_result_params: PartialResultParams::default(),
      context: None,
    }
  }

  /// Build an in-memory vault with Person and Event schemas, plus the given content file.
  fn setup(content: &str) -> (Analysis, Uri) {
    let root = PathBuf::from(if cfg!(windows) { "C:\\vault" } else { "/vault" });

    let type_root = root.join("_types");

    let test_path = root.join("file.td");
    let uri = path_to_uri(&test_path, "file");

    let db = TypedownDatabase {
      storage: QueryStorage::default(),
    };

    let config_file = File::new(
      &db,
      FileHandle::Content(
        root.join("typedown.yaml"),
        VAULT_CONFIG.to_string(),
        FileMetadata::default(),
      ),
    );
    let person_file = File::new(
      &db,
      FileHandle::Content(
        type_root.join("Person.td"),
        SCHEMA_PERSON.to_string(),
        FileMetadata::default(),
      ),
    );
    let event_file = File::new(
      &db,
      FileHandle::Content(
        type_root.join("Event.td"),
        SCHEMA_EVENT.to_string(),
        FileMetadata::default(),
      ),
    );
    let task_file = File::new(
      &db,
      FileHandle::Content(
        type_root.join("Task.td"),
        SCHEMA_TASK.to_string(),
        FileMetadata::default(),
      ),
    );
    let person_with_address_file = File::new(
      &db,
      FileHandle::Content(
        type_root.join("PersonWithAddress.td"),
        SCHEMA_PERSON_WITH_ADDRESS.to_string(),
        FileMetadata::default(),
      ),
    );
    let test_file = File::new(
      &db,
      FileHandle::Content(
        test_path.clone(),
        content.to_string(),
        FileMetadata::default(),
      ),
    );

    let files = BTreeMap::from([
      (root.join("typedown.yaml"), config_file),
      (root.join("_types/Person.td"), person_file),
      (root.join("_types/Event.td"), event_file),
      (root.join("_types/Task.td"), task_file),
      (
        root.join("_types/PersonWithAddress.td"),
        person_with_address_file,
      ),
      (test_path, test_file),
    ]);

    let project = Project::new(&db, root, files);
    let analysis = Analysis::new(
      db,
      project,
      Arc::new(BTreeMap::new()),
      Arc::new(BTreeMap::new()),
      Arc::new((Mutex::new(1), Condvar::new())),
    );

    (analysis, uri)
  }

  #[test]
  fn schema_name_completion_in_type_value() {
    let (content, offset) = cursor(
      r#"---
_type: |
---
"#,
    );
    let (analysis, uri) = setup(&content);
    let params = make_params(uri, &content, offset);

    let response = completion(&analysis, params);
    let Some(CompletionResponse::Array(items)) = response else {
      panic!("expected completion items");
    };
    let labels: Vec<&str> = items.iter().map(|item| item.label.as_str()).collect();
    assert!(labels.contains(&"Person"), "should suggest Person schema");
    assert!(labels.contains(&"Event"), "should suggest Event schema");
  }

  #[test]
  fn schema_completion_generates_snippet_with_fields() {
    let (content, offset) = cursor(
      r#"---
_type: |
---
"#,
    );
    let (analysis, uri) = setup(&content);
    let params = make_params(uri, &content, offset);

    let response = completion(&analysis, params);
    let Some(CompletionResponse::Array(items)) = response else {
      panic!("expected completion items");
    };

    let person = items
      .iter()
      .find(|i| i.label == "Person")
      .expect("should have Person");
    let snippet = person
      .insert_text
      .as_ref()
      .expect("should have insert_text");

    assert!(
      snippet.starts_with("Person\n"),
      "snippet should start with schema name: {snippet}"
    );
    assert!(
      snippet.contains("name:"),
      "snippet should contain name field: {snippet}"
    );
    assert!(
      snippet.contains("age:"),
      "snippet should contain age field: {snippet}"
    );
    assert!(
      snippet.contains("string"),
      "string field should have string placeholder: {snippet}"
    );
    assert!(
      snippet.contains(":0}"),
      "number field should have 0 placeholder: {snippet}"
    );
    assert!(
      person.insert_text_format == Some(lsp_types::InsertTextFormat::SNIPPET),
      "should use snippet format"
    );
  }

  #[test]
  fn schema_snippet_includes_enum_and_relation() {
    let (content, offset) = cursor(
      r#"---
_type: |
---
"#,
    );
    let (analysis, uri) = setup(&content);
    let params = make_params(uri, &content, offset);

    let response = completion(&analysis, params);
    let Some(CompletionResponse::Array(items)) = response else {
      panic!("expected completion items");
    };

    let task = items
      .iter()
      .find(|i| i.label == "Task")
      .expect("should have Task");
    let snippet = task.insert_text.as_ref().expect("should have insert_text");

    assert!(
      snippet.contains("done") || snippet.contains("todo"),
      "enum field should have first option as placeholder: {snippet}"
    );
    assert!(
      snippet.contains("fref(\\\"Person\\\")"),
      "relation field should have fref placeholder: {snippet}"
    );
  }

  #[test]
  fn schema_name_completion_while_partially_typed() {
    // Cursor in the middle of a partially typed schema name
    let (content, offset) = cursor(
      r#"---
_type: Per|
---
"#,
    );
    let (analysis, uri) = setup(&content);
    let params = make_params(uri, &content, offset);

    let response = completion(&analysis, params);
    let Some(CompletionResponse::Array(items)) = response else {
      panic!("expected completion items");
    };
    let labels: Vec<&str> = items.iter().map(|item| item.label.as_str()).collect();
    assert!(labels.contains(&"Person"), "should still suggest Person");
  }

  #[test]
  fn field_completion_based_on_declared_type() {
    // Cursor after typing a partial key, _type already set
    let (content, offset) = cursor(
      r#"---
_type: Person
na|:
---
"#,
    );
    let (analysis, uri) = setup(&content);
    let params = make_params(uri, &content, offset);

    let response = completion(&analysis, params);
    let Some(CompletionResponse::Array(items)) = response else {
      panic!("expected field completions");
    };
    let labels: Vec<&str> = items.iter().map(|item| item.label.as_str()).collect();
    assert!(labels.contains(&"name"), "should suggest 'name' field");
    assert!(labels.contains(&"age"), "should suggest 'age' field");
  }

  #[test]
  fn field_completion_when_type_declared_after_other_fields() {
    // _type appears after the cursor position in the mapping
    let (content, offset) = cursor(
      r#"---
name: Alice
ag|:
_type: Person
---
"#,
    );
    let (analysis, uri) = setup(&content);
    let params = make_params(uri, &content, offset);

    let response = completion(&analysis, params);
    let Some(CompletionResponse::Array(items)) = response else {
      panic!("expected field completions");
    };
    let labels: Vec<&str> = items.iter().map(|item| item.label.as_str()).collect();
    assert!(
      !labels.contains(&"name"),
      "should not suggest 'name' (already exists)"
    );
    assert!(labels.contains(&"age"), "should suggest 'age' field");
    assert!(
      labels.contains(&"verified"),
      "should suggest 'verified' field"
    );
  }

  #[test]
  fn no_field_completion_without_type() {
    // No _type in mapping: no field completions expected
    let (content, offset) = cursor(
      r#"---
na|:
---
"#,
    );
    let (analysis, uri) = setup(&content);
    let params = make_params(uri, &content, offset);

    let response = completion(&analysis, params);
    let is_empty = response.is_none()
      || matches!(response, Some(CompletionResponse::Array(ref items)) if items.is_empty());
    assert!(is_empty, "should not suggest fields when _type is absent");
  }

  #[test]
  fn field_completion_on_blank_line_between_fields() {
    let (content, offset) = cursor(
      r#"---
_type: Person
name: Alice
|
age: 30
---
"#,
    );
    let (analysis, uri) = setup(&content);
    let params = make_params(uri, &content, offset);

    let response = completion(&analysis, params);
    let Some(CompletionResponse::Array(items)) = response else {
      panic!("expected field completions on blank line");
    };
    let labels: Vec<&str> = items.iter().map(|item| item.label.as_str()).collect();
    assert!(
      labels.contains(&"verified"),
      "should suggest 'verified' on blank line between fields"
    );
  }

  #[test]
  fn field_completion_on_last_line_of_mapping() {
    let (content, offset) = cursor(
      r#"---
_type: Person
name: Alice
|
---
"#,
    );
    let (analysis, uri) = setup(&content);
    let params = make_params(uri, &content, offset);

    let response = completion(&analysis, params);
    let Some(CompletionResponse::Array(items)) = response else {
      panic!("expected field completions on last line of mapping");
    };
    let labels: Vec<&str> = items.iter().map(|item| item.label.as_str()).collect();
    assert!(
      labels.contains(&"age"),
      "should suggest 'age' on last blank line"
    );
  }

  #[test]
  fn field_completion_excludes_existing_keys() {
    let (content, offset) = cursor(
      r#"---
_type: Person
name: Alice
age: 30
|
---
"#,
    );
    let (analysis, uri) = setup(&content);
    let params = make_params(uri, &content, offset);

    let response = completion(&analysis, params);
    let Some(CompletionResponse::Array(items)) = response else {
      panic!("expected field completions");
    };
    let labels: Vec<&str> = items.iter().map(|item| item.label.as_str()).collect();
    assert!(
      !labels.contains(&"name"),
      "should not suggest 'name' (already exists)"
    );
    assert!(
      !labels.contains(&"age"),
      "should not suggest 'age' (already exists)"
    );
    assert!(
      labels.contains(&"verified"),
      "should suggest 'verified' (not yet used)"
    );
    assert!(
      labels.contains(&"nickname"),
      "should suggest 'nickname' (not yet used)"
    );
  }

  #[test]
  fn field_completion_on_key_excludes_other_existing_keys() {
    let (content, offset) = cursor(
      r#"---
_type: Person
name: Alice
ver|:
---
"#,
    );
    let (analysis, uri) = setup(&content);
    let params = make_params(uri, &content, offset);

    let response = completion(&analysis, params);
    let Some(CompletionResponse::Array(items)) = response else {
      panic!("expected field completions");
    };
    let labels: Vec<&str> = items.iter().map(|item| item.label.as_str()).collect();
    assert!(
      !labels.contains(&"name"),
      "should not suggest 'name' (already exists)"
    );
    assert!(labels.contains(&"age"), "should suggest 'age'");
    assert!(labels.contains(&"verified"), "should suggest 'verified'");
  }

  #[test]
  fn no_completion_in_plain_markdown_body() {
    let (content, offset) = cursor(
      r#"---
_type: Person
name: Alice
---

Some bod|y text.
"#,
    );
    let (analysis, uri) = setup(&content);
    let params = make_params(uri, &content, offset);

    let response = completion(&analysis, params);
    let is_empty = response.is_none()
      || matches!(response, Some(CompletionResponse::Array(ref items)) if items.is_empty());
    assert!(
      is_empty,
      "should not suggest anything in plain markdown body"
    );
  }

  // Inside ${} interpolation, suggest fref("path") completions
  #[test]
  fn fref_completion_in_interpolation() {
    let (content, offset) = cursor(
      r#"---
_type: Person
name: Alice
---

Reference: ${|}
"#,
    );
    let (analysis, uri) = setup_with_content(&content);
    let params = make_params(uri, &content, offset);

    let response = completion(&analysis, params);
    let Some(CompletionResponse::Array(items)) = response else {
      panic!("expected fref completions inside interpolation");
    };

    assert!(
      !items.is_empty(),
      "should suggest fref completions inside ${{}}"
    );

    // Should have fref("path") as insert text
    let has_fref_insert = items.iter().any(|item| {
      item
        .insert_text
        .as_deref()
        .is_some_and(|t| t.starts_with("fref(\"") && t.ends_with("\")"))
    });
    assert!(
      has_fref_insert,
      "should wrap path in fref(): {:?}",
      items
        .iter()
        .map(|i| (&i.label, &i.insert_text))
        .collect::<Vec<_>>()
    );

    // Label should be the display name, not the path
    let alice_item = items
      .iter()
      .find(|item| item.label == "Alice Chen")
      .or_else(|| items.iter().find(|item| item.label.contains("alice")));
    assert!(
      alice_item.is_some(),
      "should have alice completion with human-readable label: {:?}",
      items.iter().map(|i| &i.label).collect::<Vec<_>>()
    );

    // Detail should be the path
    let alice = alice_item.unwrap();
    assert!(
      alice.detail.as_deref().is_some_and(|d| d.contains("alice")),
      "detail should contain the file path: {:?}",
      alice.detail
    );
  }

  #[test]
  fn boolean_keywords_suggested_in_any_value_position() {
    // true/false are keywords usable in any value position, not limited to boolean-typed fields
    let (content, offset) = cursor(
      r#"---
_type: Person
name: tru|
---
"#,
    );
    let (analysis, uri) = setup(&content);
    let params = make_params(uri, &content, offset);

    let response = completion(&analysis, params);
    let Some(CompletionResponse::Array(items)) = response else {
      panic!("expected value completions");
    };
    let labels: Vec<&str> = items.iter().map(|item| item.label.as_str()).collect();
    assert!(
      labels.contains(&"true"),
      "should suggest 'true' in any value position"
    );
    assert!(
      labels.contains(&"false"),
      "should suggest 'false' in any value position"
    );
    assert!(
      !labels.contains(&"null"),
      "non-optional field should not suggest 'null'"
    );
  }

  #[test]
  fn null_completion_for_optional_field() {
    // Cursor in the value of a nullable field: suggest null
    let (content, offset) = cursor(
      r#"---
_type: Person
nickname: nu|
---
"#,
    );
    let (analysis, uri) = setup(&content);
    let params = make_params(uri, &content, offset);

    let response = completion(&analysis, params);
    let Some(CompletionResponse::Array(items)) = response else {
      panic!("expected value completions");
    };
    let labels: Vec<&str> = items.iter().map(|item| item.label.as_str()).collect();
    assert!(
      labels.contains(&"null"),
      "optional field should suggest 'null'"
    );
  }

  // A schema with a field typed as another schema (Person)
  const SCHEMA_DIRECTORY: &str = r#"---
_type: schema
properties:
  featured:
    type: Person
---
"#;

  const SCHEMA_TASK_REF: &str = r#"---
_type: schema
properties:
  title:
    type: string
  assignee:
    type: Person?
---
"#;

  const CONTENT_ALICE: &str = r#"---
_type: Person
_label: "Alice Chen"
name: Alice
age: 30
---
"#;

  const CONTENT_BIRTHDAY: &str = r#"---
_type: Event
name: Birthday
date: 2024-01-01
---
"#;

  /// Build a vault with Person, Event, Directory schemas plus two content files.
  fn setup_with_content(content: &str) -> (Analysis, Uri) {
    let root = PathBuf::from(if cfg!(windows) { "C:\\vault" } else { "/vault" });

    let type_root = root.join("_types");

    let test_path = root.join("file.td");
    let uri = path_to_uri(&test_path, "file");

    let db = TypedownDatabase {
      storage: QueryStorage::default(),
    };

    let config_file = File::new(
      &db,
      FileHandle::Content(
        root.join("typedown.yaml"),
        VAULT_CONFIG.to_string(),
        FileMetadata::default(),
      ),
    );
    let person_file = File::new(
      &db,
      FileHandle::Content(
        type_root.join("Person.td"),
        SCHEMA_PERSON.to_string(),
        FileMetadata::default(),
      ),
    );
    let event_file = File::new(
      &db,
      FileHandle::Content(
        type_root.join("Event.td"),
        SCHEMA_EVENT.to_string(),
        FileMetadata::default(),
      ),
    );
    let directory_file = File::new(
      &db,
      FileHandle::Content(
        type_root.join("Directory.td"),
        SCHEMA_DIRECTORY.to_string(),
        FileMetadata::default(),
      ),
    );
    let task_ref_file = File::new(
      &db,
      FileHandle::Content(
        type_root.join("TaskRef.td"),
        SCHEMA_TASK_REF.to_string(),
        FileMetadata::default(),
      ),
    );
    let alice_file = File::new(
      &db,
      FileHandle::Content(
        root.join("alice.td"),
        CONTENT_ALICE.to_string(),
        FileMetadata::default(),
      ),
    );
    let birthday_file = File::new(
      &db,
      FileHandle::Content(
        root.join("birthday.td"),
        CONTENT_BIRTHDAY.to_string(),
        FileMetadata::default(),
      ),
    );
    let editing_file = File::new(
      &db,
      FileHandle::Content(
        test_path.clone(),
        content.to_string(),
        FileMetadata::default(),
      ),
    );

    let files = BTreeMap::from([
      (root.join("typedown.yaml"), config_file),
      (root.join("_types/Person.td"), person_file),
      (root.join("_types/Event.td"), event_file),
      (root.join("_types/Directory.td"), directory_file),
      (root.join("_types/TaskRef.td"), task_ref_file),
      (root.join("alice.td"), alice_file),
      (root.join("birthday.td"), birthday_file),
      (test_path, editing_file),
    ]);

    let project = Project::new(&db, root, files);
    let analysis = Analysis::new(
      db,
      project,
      Arc::new(BTreeMap::new()),
      Arc::new(BTreeMap::new()),
      Arc::new((Mutex::new(1), Condvar::new())),
    );

    (analysis, uri)
  }

  #[test]
  fn fref_completion_filters_by_declared_field_type() {
    // The 'featured' field on Directory expects type Person
    // Only content/alice.td (_type: Person) should be suggested, not content/birthday.td (_type: Event)
    let (content, offset) = cursor(
      r#"---
_type: Directory
featured: fref("|")
---
"#,
    );
    let (analysis, uri) = setup_with_content(&content);
    let params = make_params(uri, &content, offset);

    let response = completion(&analysis, params);
    let Some(CompletionResponse::Array(items)) = response else {
      panic!("expected fref completions");
    };
    // Detail contains the path, label is the display name
    let details: Vec<String> = items
      .iter()
      .filter_map(|item| item.detail.clone())
      .collect();
    assert!(
      details.iter().any(|d| d.contains("alice")),
      "should suggest alice.td (Person type), got: {:?}",
      details
    );
    assert!(
      !details.iter().any(|d| d.contains("birthday")),
      "should not suggest birthday.td (Event type), got: {:?}",
      details
    );
  }

  // fref completions should use vault-relative paths, not project-relative paths
  // When root_dir is "vault", fref("alice.td") not fref("vault/alice.td")
  #[test]
  fn fref_completion_uses_vault_relative_paths() {
    let project_root = PathBuf::from(if cfg!(windows) {
      "C:\\project"
    } else {
      "/project"
    });
    let vault_root = project_root.join("vault");
    let type_root = vault_root.join("_types");

    let (content, offset) = cursor(
      r#"---
_type: Directory
featured: fref("|")
---
"#,
    );

    let test_path = vault_root.join("file.td");
    let uri = path_to_uri(&test_path, "file");

    let nested_config = r#"version: "1"
vault:
  root_dir: "vault"
"#;

    let db = TypedownDatabase {
      storage: QueryStorage::default(),
    };

    let files = BTreeMap::from([
      (
        project_root.join("typedown.yaml"),
        File::new(
          &db,
          FileHandle::Content(
            project_root.join("typedown.yaml"),
            nested_config.to_string(),
            FileMetadata::default(),
          ),
        ),
      ),
      (
        type_root.join("Person.td"),
        File::new(
          &db,
          FileHandle::Content(
            type_root.join("Person.td"),
            SCHEMA_PERSON.to_string(),
            FileMetadata::default(),
          ),
        ),
      ),
      (
        type_root.join("Directory.td"),
        File::new(
          &db,
          FileHandle::Content(
            type_root.join("Directory.td"),
            SCHEMA_DIRECTORY.to_string(),
            FileMetadata::default(),
          ),
        ),
      ),
      (
        vault_root.join("alice.td"),
        File::new(
          &db,
          FileHandle::Content(
            vault_root.join("alice.td"),
            CONTENT_ALICE.to_string(),
            FileMetadata::default(),
          ),
        ),
      ),
      (
        test_path.clone(),
        File::new(
          &db,
          FileHandle::Content(test_path, content.clone(), FileMetadata::default()),
        ),
      ),
    ]);

    let project = Project::new(&db, project_root, files);
    let analysis = Analysis::new(
      db,
      project,
      Arc::new(BTreeMap::new()),
      Arc::new(BTreeMap::new()),
      Arc::new((Mutex::new(1), Condvar::new())),
    );

    let params = make_params(uri, &content, offset);
    let response = completion(&analysis, params);
    let Some(CompletionResponse::Array(items)) = response else {
      panic!("expected fref completions");
    };
    // Detail contains the path, should be vault-relative
    let details: Vec<String> = items
      .iter()
      .filter_map(|item| item.detail.clone())
      .collect();

    // Should suggest "alice.td", not "vault/alice.td"
    assert!(
      details.iter().any(|d| d == "alice.td"),
      "should suggest vault-relative path 'alice.td', got: {:?}",
      details
    );
    assert!(
      !details.iter().any(|d| d.contains("vault/")),
      "should not include vault dir prefix in path, got: {:?}",
      details
    );
  }

  // fref completions include _label as detail and basename in filterText
  #[test]
  fn fref_completion_includes_label_and_filter_text() {
    let (content, offset) = cursor(
      r#"---
_type: Directory
featured: fref("|")
---
"#,
    );
    let (analysis, uri) = setup_with_content(&content);
    let params = make_params(uri, &content, offset);

    let response = completion(&analysis, params);
    let Some(CompletionResponse::Array(items)) = response else {
      panic!("expected fref completions");
    };

    let alice_item = items
      .iter()
      .find(|item| item.label == "Alice Chen")
      .expect("should have alice completion with display name as label");

    // detail should contain the vault-relative path
    assert_eq!(
      alice_item.detail.as_deref(),
      Some("alice.td"),
      "detail should be the vault-relative path"
    );

    // filterText should include path, basename, and label for fuzzy matching
    let filter = alice_item.filter_text.as_deref().unwrap_or("");
    assert!(
      filter.contains("alice") && filter.contains("Alice Chen"),
      "filterText should contain basename and label: {filter}"
    );
  }

  // Empty value on a schema-typed field suggests fref snippet
  #[test]
  fn schema_typed_empty_value_suggests_fref() {
    let (content, offset) = cursor(
      r#"---
_type: Directory
featured: |
---
"#,
    );
    let (analysis, uri) = setup_with_content(&content);
    let params = make_params(uri, &content, offset);

    let response = completion(&analysis, params);
    let Some(CompletionResponse::Array(items)) = response else {
      panic!("expected completions for empty schema-typed field");
    };

    // Should have specific file suggestions with fref("path") insert text
    let has_alice = items.iter().any(|item| {
      item
        .insert_text
        .as_deref()
        .is_some_and(|t| t.contains("alice"))
    });
    assert!(
      has_alice,
      "should suggest alice.td: {:?}",
      items.iter().map(|i| &i.label).collect::<Vec<_>>()
    );
  }

  // Typing in a schema-typed field value auto-suggests fref completions
  #[test]
  fn schema_typed_field_suggests_fref() {
    let (content, offset) = cursor(
      r#"---
_type: Directory
featured: a|
---
"#,
    );
    let (analysis, uri) = setup_with_content(&content);
    let params = make_params(uri, &content, offset);

    let response = completion(&analysis, params);
    let Some(CompletionResponse::Array(items)) = response else {
      panic!("expected completions for schema-typed field");
    };

    // Should suggest alice with fref() wrapper
    let alice_item = items
      .iter()
      .find(|item| {
        item
          .insert_text
          .as_deref()
          .is_some_and(|t| t.contains("alice"))
      })
      .expect("should suggest alice for Person-typed field");

    assert!(
      alice_item
        .insert_text
        .as_deref()
        .is_some_and(|t| t.starts_with("fref(\"") && t.ends_with("\")")),
      "insert_text should wrap path in fref(): {:?}",
      alice_item.insert_text
    );
  }

  // Nullable schema field (Person?) also auto-suggests fref on empty value
  #[test]
  fn nullable_schema_field_suggests_fref() {
    let (content, offset) = cursor(
      r#"---
_type: TaskRef
title: "Test"
assignee: |
---
"#,
    );
    let (analysis, uri) = setup_with_content(&content);
    let params = make_params(uri, &content, offset);

    let response = completion(&analysis, params);
    let Some(CompletionResponse::Array(items)) = response else {
      panic!("expected completions for nullable schema field");
    };

    let has_fref = items.iter().any(|item| {
      item
        .insert_text
        .as_deref()
        .is_some_and(|t| t.contains("fref("))
    });
    assert!(
      has_fref,
      "nullable Person? field should suggest fref completions: {:?}",
      items.iter().map(|i| &i.label).collect::<Vec<_>>()
    );
  }

  // Non-schema field (string) should NOT auto-suggest fref
  #[test]
  fn string_field_does_not_suggest_fref() {
    let (content, offset) = cursor(
      r#"---
_type: Person
name: a|
---
"#,
    );
    let (analysis, uri) = setup_with_content(&content);
    let params = make_params(uri, &content, offset);

    let response = completion(&analysis, params);
    let Some(CompletionResponse::Array(items)) = response else {
      panic!("expected value completions");
    };

    let has_fref = items.iter().any(|item| {
      item
        .insert_text
        .as_deref()
        .is_some_and(|t| t.contains("fref("))
    });
    assert!(
      !has_fref,
      "string field should not suggest fref: {:?}",
      items.iter().map(|i| &i.label).collect::<Vec<_>>()
    );
  }

  // Enum field suggests literal values
  #[test]
  fn enum_field_suggests_literal_values() {
    let (content, offset) = cursor(
      r#"---
_type: Task
status: t|
priority: "high"
---
"#,
    );
    let (analysis, uri) = setup(&content);
    let params = make_params(uri, &content, offset);

    let response = completion(&analysis, params);
    let Some(CompletionResponse::Array(items)) = response else {
      panic!("expected value completions for enum field");
    };
    let labels: Vec<&str> = items.iter().map(|i| i.label.as_str()).collect();
    assert!(
      labels.contains(&"\"todo\""),
      "should suggest 'todo': {:?}",
      labels
    );
    assert!(
      labels.contains(&"\"in_progress\""),
      "should suggest 'in_progress': {:?}",
      labels
    );
    assert!(
      labels.contains(&"\"done\""),
      "should suggest 'done': {:?}",
      labels
    );
  }

  // Date field suggests ISO 8601 placeholder
  #[test]
  fn date_field_suggests_date_placeholder() {
    let (content, offset) = cursor(
      r#"---
_type: Event
date: d|
---
"#,
    );
    let (analysis, uri) = setup(&content);
    let params = make_params(uri, &content, offset);

    let response = completion(&analysis, params);
    let Some(CompletionResponse::Array(items)) = response else {
      panic!("expected value completions for date field");
    };
    let has_date = items.iter().any(|i| i.label.contains("YYYY-MM-DD"));
    assert!(
      has_date,
      "date field should suggest ISO format: {:?}",
      items.iter().map(|i| &i.label).collect::<Vec<_>>()
    );
  }

  // Cursor on a key inside a nested mapping whose type is inferred from the parent schema field
  #[test]
  fn field_completion_in_nested_mapping_without_type() {
    let (content, offset) = cursor(
      r#"---
_type: PersonWithAddress
name: Alice
address:
  str|:
---
"#,
    );
    let (analysis, uri) = setup(&content);
    let params = make_params(uri, &content, offset);

    let response = completion(&analysis, params);
    let Some(CompletionResponse::Array(items)) = response else {
      panic!("expected field completions for nested address mapping");
    };
    let labels: Vec<&str> = items.iter().map(|item| item.label.as_str()).collect();
    assert!(
      labels.contains(&"street"),
      "should suggest 'street' from nested address type, got: {:?}",
      labels
    );
    assert!(
      labels.contains(&"city"),
      "should suggest 'city' from nested address type, got: {:?}",
      labels
    );
  }

  // --- Expression and dot-access completion tests ---

  #[test]
  fn value_position_suggests_builtin_variables() {
    let (content, offset) = cursor(
      r#"---
_type: Person
name: i|
---
"#,
    );
    let (analysis, uri) = setup(&content);
    let params = make_params(uri, &content, offset);

    let response = completion(&analysis, params);
    let Some(CompletionResponse::Array(items)) = response else {
      panic!("expected value completions");
    };
    let labels: Vec<&str> = items.iter().map(|i| i.label.as_str()).collect();
    assert!(labels.contains(&"icon"), "should suggest builtin 'icon'");
    assert!(labels.contains(&"fref"), "should suggest builtin 'fref'");
    assert!(labels.contains(&"vault"), "should suggest builtin 'vault'");
  }

  #[test]
  fn value_position_suggests_type_names() {
    let (content, offset) = cursor(
      r#"---
_type: Person
name: s|
---
"#,
    );
    let (analysis, uri) = setup(&content);
    let params = make_params(uri, &content, offset);

    let response = completion(&analysis, params);
    let Some(CompletionResponse::Array(items)) = response else {
      panic!("expected value completions");
    };
    let labels: Vec<&str> = items.iter().map(|i| i.label.as_str()).collect();
    assert!(
      labels.contains(&"string"),
      "should suggest builtin type 'string'"
    );
    assert!(
      labels.contains(&"number"),
      "should suggest builtin type 'number'"
    );
    assert!(
      labels.contains(&"boolean"),
      "should suggest builtin type 'boolean'"
    );
  }

  #[test]
  fn value_position_suggests_project_file_names() {
    let (content, offset) = cursor(
      r#"---
_type: Person
name: a|
---
"#,
    );
    let (analysis, uri) = setup_with_content(&content);
    let params = make_params(uri, &content, offset);

    let response = completion(&analysis, params);
    let Some(CompletionResponse::Array(items)) = response else {
      panic!("expected value completions");
    };
    let labels: Vec<&str> = items.iter().map(|i| i.label.as_str()).collect();
    assert!(
      labels.contains(&"alice"),
      "should suggest project file 'alice'"
    );
    assert!(
      labels.contains(&"birthday"),
      "should suggest project file 'birthday'"
    );
  }

  #[test]
  fn value_position_excludes_underscore_prefixed() {
    let (content, offset) = cursor(
      r#"---
_type: Person
name: x|
---
"#,
    );
    let (analysis, uri) = setup(&content);
    let params = make_params(uri, &content, offset);

    let response = completion(&analysis, params);
    let Some(CompletionResponse::Array(items)) = response else {
      panic!("expected value completions");
    };
    let labels: Vec<&str> = items.iter().map(|i| i.label.as_str()).collect();
    assert!(
      !labels.iter().any(|l| l.starts_with('_')),
      "should not suggest underscore-prefixed items: {:?}",
      labels
    );
  }

  #[test]
  fn value_position_includes_keywords_and_variables() {
    let (content, offset) = cursor(
      r#"---
_type: Person
name: ic|
---
"#,
    );
    let (analysis, uri) = setup(&content);
    let params = make_params(uri, &content, offset);

    let response = completion(&analysis, params);
    let Some(CompletionResponse::Array(items)) = response else {
      panic!("expected value completions");
    };
    let labels: Vec<&str> = items.iter().map(|i| i.label.as_str()).collect();
    // Keywords
    assert!(labels.contains(&"true"), "should include keyword 'true'");
    assert!(labels.contains(&"false"), "should include keyword 'false'");
    // Variables
    assert!(labels.contains(&"icon"), "should include variable 'icon'");
  }

  #[test]
  fn dot_access_on_icon_suggests_icon_names() {
    let (content, offset) = cursor(
      r#"---
_icon: icon.|
---
"#,
    );
    let (analysis, uri) = setup(&content);
    let params = make_params(uri, &content, offset);

    let response = completion(&analysis, params);
    let Some(CompletionResponse::Array(items)) = response else {
      panic!("expected dot-access completions for icon");
    };
    let labels: Vec<&str> = items.iter().map(|i| i.label.as_str()).collect();
    assert!(
      labels.contains(&"book"),
      "should suggest 'book' icon: {:?}",
      labels
    );
    assert!(
      labels.contains(&"star"),
      "should suggest 'star' icon: {:?}",
      labels
    );
    // Should NOT include the "icon." prefix (just the member name)
    assert!(
      !labels.iter().any(|l| l.contains('.')),
      "labels should be bare names, not dotted: {:?}",
      labels
    );
  }

  #[test]
  fn dot_access_on_icon_with_partial_input() {
    let (content, offset) = cursor(
      r#"---
_icon: icon.bo|
---
"#,
    );
    let (analysis, uri) = setup(&content);
    let params = make_params(uri, &content, offset);

    let response = completion(&analysis, params);
    let Some(CompletionResponse::Array(items)) = response else {
      panic!("expected dot-access completions");
    };
    let labels: Vec<&str> = items.iter().map(|i| i.label.as_str()).collect();
    assert!(
      labels.contains(&"book"),
      "should suggest 'book' with partial 'bo': {:?}",
      labels
    );
  }

  #[test]
  fn no_dot_access_before_dot_typed() {
    // Typing just "ic" in _icon should NOT trigger icon member completions
    let (content, offset) = cursor(
      r#"---
_icon: ic|
---
"#,
    );
    let (analysis, uri) = setup(&content);
    let params = make_params(uri, &content, offset);

    let response = completion(&analysis, params);
    if let Some(CompletionResponse::Array(items)) = response {
      let labels: Vec<&str> = items.iter().map(|i| i.label.as_str()).collect();
      // Should not have icon member names like "book" without the dot
      assert!(
        !labels.contains(&"book"),
        "should not suggest icon members without dot: {:?}",
        labels
      );
    }
  }

  #[test]
  fn dot_access_on_icon_in_regular_field() {
    // Dot-access should work in any value position, not just _icon
    let (content, offset) = cursor(
      r#"---
_type: Person
name: icon.|
---
"#,
    );
    let (analysis, uri) = setup(&content);
    let params = make_params(uri, &content, offset);

    let response = completion(&analysis, params);
    let Some(CompletionResponse::Array(items)) = response else {
      panic!("expected dot-access completions");
    };
    let labels: Vec<&str> = items.iter().map(|i| i.label.as_str()).collect();
    assert!(
      labels.contains(&"book"),
      "dot-access should work in any field: {:?}",
      labels
    );
  }

  #[test]
  fn closure_param_suggested_in_body() {
    let (content, offset) = cursor(
      r#"---
_type: Person
name: (x) -> x|
---
"#,
    );
    let (analysis, uri) = setup(&content);
    let params = make_params(uri, &content, offset);

    let response = completion(&analysis, params);
    let Some(CompletionResponse::Array(items)) = response else {
      panic!("expected completions inside closure body");
    };
    let labels: Vec<&str> = items.iter().map(|i| i.label.as_str()).collect();
    assert!(
      labels.contains(&"x"),
      "should suggest closure param 'x': {:?}",
      labels
    );
    assert!(
      labels.contains(&"icon"),
      "should also suggest builtins: {:?}",
      labels
    );
  }

  #[test]
  fn multi_param_closure_suggests_all_params() {
    let (content, offset) = cursor(
      r#"---
_type: Person
name: (a, b) -> a|
---
"#,
    );
    let (analysis, uri) = setup(&content);
    let params = make_params(uri, &content, offset);

    let response = completion(&analysis, params);
    let Some(CompletionResponse::Array(items)) = response else {
      panic!("expected completions inside closure body");
    };
    let labels: Vec<&str> = items.iter().map(|i| i.label.as_str()).collect();
    assert!(
      labels.contains(&"a"),
      "should suggest param 'a': {:?}",
      labels
    );
    assert!(
      labels.contains(&"b"),
      "should suggest param 'b': {:?}",
      labels
    );
  }

  #[test]
  fn self_suggested_in_value_position() {
    let (content, offset) = cursor(
      r#"---
_type: Person
name: s|
---
"#,
    );
    let (analysis, uri) = setup(&content);
    let params = make_params(uri, &content, offset);

    let response = completion(&analysis, params);
    let Some(CompletionResponse::Array(items)) = response else {
      panic!("expected value completions");
    };
    let labels: Vec<&str> = items.iter().map(|i| i.label.as_str()).collect();
    assert!(
      labels.contains(&"self"),
      "should suggest 'self': {:?}",
      labels
    );
  }
}
