use typedown_incremental::StableCompare;
use typedown_lang::db::utils::is_content_file;

use lsp_types::{
  CompletionItem, CompletionItemKind, CompletionParams, CompletionResponse, InsertTextFormat,
};
use typedown_lang::db::TypedownDatabase;
use typedown_lang::db::derived::evaluate::evaluate_type::evaluate_type;
use typedown_lang::db::derived::hir::lower_node;
use typedown_lang::db::derived::name_resolver::file_symbol::file_symbol;
use typedown_lang::db::derived::name_resolver::members::members;
use typedown_lang::db::derived::parse_file::parse_file;
use typedown_lang::db::derived::typechecker::expected_node_type::expected_node_type;
use typedown_lang::db::derived::typechecker::get_symbol_type::get_symbol_type;
use typedown_lang::db::typecheck::utils::{is_nullable, is_subtype_of};
use typedown_lang::db::types::{
  File, LazyType, LiteralValue, Project, Scope, SymbolKind, TdProductType, TdTypeEnum,
};
use typedown_lang::db::utils::schema_name_in_mapping;
use typedown_lang::syntax::ast::{AstNode, Expr};
use typedown_lang::syntax::red::RedNode;
use typedown_lang::syntax::syntax_kind::SyntaxKind;

use crate::core::analysis::Analysis;
use crate::core::utils::ast::{find_ancestor, is_in_mapping_value_position, node_at_offset};
use crate::core::utils::position::lsp_position_to_text_offset;
use crate::core::utils::uri::uri_to_path;

pub fn completion(analysis: &Analysis, params: CompletionParams) -> Option<CompletionResponse> {
  let db = &analysis.db;
  let project = analysis.project;

  let path = uri_to_path(&params.text_document_position.text_document.uri)?;
  let rope = analysis.file_rope(&path)?;
  let offset = lsp_position_to_text_offset(&rope, params.text_document_position.position)?;

  let file = *project.files(db).get(&path)?;
  let root = parse_file(db, project, file).ast(db);
  // Use offset-1 so the cursor position (between characters) resolves to the token just typed.
  let lookup = offset.saturating_sub(1);
  let node = node_at_offset(root, lookup)?;

  // Cursor in a _type value: suggest schema names.
  if is_type_value_position(&node) {
    return Some(CompletionResponse::Array(schema_completions(db, project)));
  }

  // Cursor inside a fref() string argument. Suggest .td files matching the field's declared type.
  if is_fref_arg_position(&node) {
    return Some(CompletionResponse::Array(fref_completions(
      db, project, file, &node,
    )));
  }

  // Cursor in a field value: suggest value completions (booleans, null for optional fields).
  if let Some(items) = value_completions(db, project, file, &node) {
    return Some(CompletionResponse::Array(items));
  }

  // Cursor in a mapping key or blank line: suggest field names from the declared schema
  if let Some((product, mapping)) = enclosing_mapping_product(db, project, file, &node) {
    let existing = existing_keys(&mapping);
    return Some(CompletionResponse::Array(field_completions_from_type(
      db, &product, &existing,
    )));
  }

  None
}

/// Returns true if `node` is inside the value position of a `_type` mapping entry.
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

/// Returns true if `node` is inside the string argument of a `fref()` call.
fn is_fref_arg_position(node: &RedNode) -> bool {
  // Walk up to find an enclosing StrLit, then a CallExpr above it.
  let str_lit = find_ancestor(node, SyntaxKind::StrLit);
  let call = match str_lit {
    Some(ref lit) => find_ancestor(lit, SyntaxKind::CallExpr),
    None => find_ancestor(node, SyntaxKind::CallExpr),
  };
  let Some(call) = call else {
    return false;
  };
  // Check callee text is "fref".
  call
    .children()
    .next()
    .is_some_and(|callee| callee.text().trim() == "fref")
}

/// Suggest .td file paths whose type is compatible with the declared field type.
/// Falls back to all .td files if no declared type can be resolved.
fn fref_completions(
  db: &TypedownDatabase,
  project: Project,
  file: File,
  node: &RedNode,
) -> Vec<CompletionItem> {
  // Resolve the expected type for the field containing this fref() call.
  let expected_type = declared_field(db, project, file, node);

  let root = project.root_dir(db);
  project
    .files(db)
    .iter()
    .filter(|(path, _)| is_content_file(path))
    .filter(|(_, target_file)| {
      // If we have an expected type, only include files whose type is compatible.
      let Some(ref expected_typ) = expected_type else {
        return true;
      };
      let sym = match file_symbol(db, project, **target_file).value(db) {
        Some(sym) => sym,
        None => return false,
      };
      let file_type = match get_symbol_type(db, sym).typ(db) {
        Some(typ) => typ,
        None => return false,
      };
      is_subtype_of(db, &file_type, expected_typ)
    })
    .filter_map(|(path, _)| path.strip_prefix(&root).ok().map(|rel| rel.to_path_buf()))
    .map(|rel| CompletionItem {
      label: rel.to_string_lossy().into_owned(),
      kind: Some(CompletionItemKind::FILE),
      ..Default::default()
    })
    .collect()
}

// Resolve the product type and mapping node the cursor belongs to
// Works on keys, blank lines, and trailing whitespace inside a mapping
fn enclosing_mapping_product(
  db: &TypedownDatabase,
  project: Project,
  file: File,
  node: &RedNode,
) -> Option<(TdProductType, RedNode)> {
  if is_in_mapping_value_position(node) {
    return None;
  }
  let mapping = find_ancestor(node, SyntaxKind::YamlMapping)?;

  // Explicit _type in this mapping.
  if let Some(schema_name) = schema_name_in_mapping(&mapping) {
    let scope = Scope::project_scope(db, project);
    let symbol = *members(db, scope).members(db).get(&schema_name)?;
    let typ = evaluate_type(db, symbol).typ(db)?;
    return Some((typ.as_td_product_type().cloned()?, mapping));
  }

  // No explicit _type. Try resolving via the parent field's declared type.
  let mapping_expr = Expr::cast(mapping.clone())?;
  let hir = lower_node(db, project, file, mapping_expr.syntax().clone());
  let typ = expected_node_type(db, hir).typ(db)?;
  // Both product types and structural types have fields we can complete
  if let Some(product) = typ.as_td_product_type() {
    return Some((*product, mapping));
  }
  if let Some(structural) = typ.as_td_structural_type() {
    // Create a temporary product type from the structural fields for completion
    let product = TdProductType::new(
      db,
      None,
      typedown_lang::db::derived::get_builtin_types::get_type_type(db).into(),
      structural.fields(db),
      std::collections::HashMap::new(),
    );
    return Some((product, mapping));
  }
  None
}

/// If the cursor is in a field value, return value completions
/// Also suggests `null` for optional fields
fn value_completions(
  db: &TypedownDatabase,
  project: Project,
  file: File,
  node: &RedNode,
) -> Option<Vec<CompletionItem>> {
  // Must be directly inside a value position (not a key that happens to be nested in a value)
  if !is_in_mapping_value_position(node) {
    return None;
  }

  // Suggest true + false
  let mut items = vec![keyword_item("true"), keyword_item("false")];

  // Suggest null only for optional fields
  if let Some(typ) = declared_field(db, project, file, node)
    && is_nullable(db, &typ)
  {
    items.push(keyword_item("null"));
  }

  Some(items)
}

/// Resolve the LazyType for the field whose value the cursor is currently in
fn declared_field(
  db: &TypedownDatabase,
  project: Project,
  file: File,
  node: &RedNode,
) -> Option<TdTypeEnum> {
  // Find the value expression node inside the enclosing YamlMappingEntryValue.
  let entry_value = find_ancestor(node, SyntaxKind::YamlMappingEntryValue)?;
  let value_expr = entry_value.children().find_map(Expr::cast)?;
  let hir = lower_node(db, project, file, value_expr.syntax().clone());
  expected_node_type(db, hir).typ(db)
}

/// Build a keyword completion item (true, false, null).
fn keyword_item(label: &str) -> CompletionItem {
  CompletionItem {
    label: label.to_string(),
    kind: Some(CompletionItemKind::KEYWORD),
    ..Default::default()
  }
}

/// Suggest all user-defined schema names visible in the project scope.
fn schema_completions(db: &TypedownDatabase, project: Project) -> Vec<CompletionItem> {
  let scope = Scope::project_scope(db, project);
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
  let product = typ.as_ref().and_then(|t| t.as_td_product_type());

  let Some(product) = product else {
    return name.to_string();
  };

  let fields = product.fields(db);
  let mut snippet = name.to_string();
  for (tab_stop, (field_name, lazy)) in fields.iter().enumerate() {
    let placeholder = lazy_placeholder(db, lazy, 0);
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
        .filter(|m| !m.resolve(db).is_some_and(|t| t.as_td_null_type().is_some()))
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
    TdTypeEnum::TdProductType(product) => {
      if let Some(schema) = product.name(db) {
        // Named schema: relation ref
        format!("fref(\\\"{schema}\\\")")
      } else {
        // Inline product: nested YAML mapping
        let fields = product.fields(db);
        let pad = "  ".repeat(indent + 1);
        let mut nested = String::new();

        for (field_name, lazy) in &fields {
          let placeholder = lazy_placeholder(db, lazy, indent + 1);

          nested.push_str(&format!("\\n{pad}{field_name}: {placeholder}"));
        }
        nested
      }
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

// Suggest field names from a resolved product type, excluding already-present keys
fn field_completions_from_type(
  db: &TypedownDatabase,
  product: &TdProductType,
  existing: &[String],
) -> Vec<CompletionItem> {
  product
    .fields(db)
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
  use std::collections::HashMap;
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
  content_dir: content
  schema_dir: schemas
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
    type: string
    optional: true
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
    type: Person
    optional: true
---
"#;

  // Schema with a nested inline object field (no named type reference).
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

    let content_root = root.join("content");
    let schema_root = root.join("schemas");

    let test_path = content_root.join("file.td");
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
        schema_root.join("Person.td"),
        SCHEMA_PERSON.to_string(),
        FileMetadata::default(),
      ),
    );
    let event_file = File::new(
      &db,
      FileHandle::Content(
        schema_root.join("Event.td"),
        SCHEMA_EVENT.to_string(),
        FileMetadata::default(),
      ),
    );
    let task_file = File::new(
      &db,
      FileHandle::Content(
        schema_root.join("Task.td"),
        SCHEMA_TASK.to_string(),
        FileMetadata::default(),
      ),
    );
    let person_with_address_file = File::new(
      &db,
      FileHandle::Content(
        schema_root.join("PersonWithAddress.td"),
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

    let files = HashMap::from([
      (root.join("typedown.yaml"), config_file),
      (root.join("schemas/Person.td"), person_file),
      (root.join("schemas/Event.td"), event_file),
      (root.join("schemas/Task.td"), task_file),
      (
        root.join("schemas/PersonWithAddress.td"),
        person_with_address_file,
      ),
      (test_path, test_file),
    ]);

    let project = Project::new(&db, root, files);
    let analysis = Analysis::new(
      db,
      project,
      Arc::new(HashMap::new()),
      Arc::new(HashMap::new()),
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
    // Cursor in the middle of a partially typed schema name.
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
    // Cursor after typing a partial key, _type already set.
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
    // _type appears after the cursor position in the mapping.
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
    // No _type in mapping: no field completions expected.
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
  fn no_completion_in_markdown_body() {
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
    assert!(is_empty, "should not suggest anything in the markdown body");
  }

  #[test]
  fn boolean_keywords_suggested_in_any_value_position() {
    // true/false are keywords usable in any value position, not limited to boolean-typed fields.
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
    // Cursor in the value of an optional field: suggest null.
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

  // A schema with a field typed as another schema (Person).
  const SCHEMA_DIRECTORY: &str = r#"---
_type: schema
properties:
  featured:
    type: Person
---
"#;

  const CONTENT_ALICE: &str = r#"---
_type: Person
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

    let content_root = root.join("content");
    let schema_root = root.join("schemas");

    let test_path = content_root.join("file.td");
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
        schema_root.join("Person.td"),
        SCHEMA_PERSON.to_string(),
        FileMetadata::default(),
      ),
    );
    let event_file = File::new(
      &db,
      FileHandle::Content(
        schema_root.join("Event.td"),
        SCHEMA_EVENT.to_string(),
        FileMetadata::default(),
      ),
    );
    let directory_file = File::new(
      &db,
      FileHandle::Content(
        schema_root.join("Directory.td"),
        SCHEMA_DIRECTORY.to_string(),
        FileMetadata::default(),
      ),
    );
    let alice_file = File::new(
      &db,
      FileHandle::Content(
        content_root.join("alice.td"),
        CONTENT_ALICE.to_string(),
        FileMetadata::default(),
      ),
    );
    let birthday_file = File::new(
      &db,
      FileHandle::Content(
        content_root.join("birthday.td"),
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

    let files = HashMap::from([
      (root.join("typedown.yaml"), config_file),
      (root.join("schemas/Person.td"), person_file),
      (root.join("schemas/Event.td"), event_file),
      (root.join("schemas/Directory.td"), directory_file),
      (root.join("content/alice.td"), alice_file),
      (root.join("content/birthday.td"), birthday_file),
      (test_path, editing_file),
    ]);

    let project = Project::new(&db, root, files);
    let analysis = Analysis::new(
      db,
      project,
      Arc::new(HashMap::new()),
      Arc::new(HashMap::new()),
      Arc::new((Mutex::new(1), Condvar::new())),
    );

    (analysis, uri)
  }

  #[test]
  fn fref_completion_filters_by_declared_field_type() {
    // The 'featured' field on Directory expects type Person.
    // Only content/alice.td (_type: Person) should be suggested, not content/birthday.td (_type: Event).
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
    let labels: Vec<String> = items.iter().map(|item| item.label.clone()).collect();
    assert!(
      labels.iter().any(|label| label.contains("alice")),
      "should suggest alice.td (Person type), got: {:?}",
      labels
    );
    assert!(
      !labels.iter().any(|label| label.contains("birthday")),
      "should not suggest birthday.td (Event type), got: {:?}",
      labels
    );
  }

  // Cursor on a key inside a nested mapping whose type is inferred from the parent schema field.
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
}
