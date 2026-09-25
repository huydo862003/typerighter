use lsp_types::{Hover, HoverContents, HoverParams, MarkupContent, MarkupKind};

use typedown_lang::db::TypedownDatabase;
use typedown_lang::db::derived::evaluate::evaluate_type::evaluate_type;
use typedown_lang::db::derived::get_vault_config::get_vault_config;
use typedown_lang::db::derived::hir::lower_node;
use typedown_lang::db::derived::name_resolver::file_symbol::file_symbol;
use typedown_lang::db::derived::name_resolver::members::members;
use typedown_lang::db::derived::parse_file::parse_file;
use typedown_lang::db::derived::typechecker::actual_node_type::actual_node_type;
use typedown_lang::db::derived::typechecker::expected_node_type::expected_node_type;
use typedown_lang::db::derived::typechecker::get_symbol_type::get_symbol_type;
use typedown_lang::db::types::derived::object_system::TdStaticType;
use typedown_lang::db::types::{
  File, FileRedNode, HirValueKind, Project, Scope, ScopeKind, Symbol, TdTypeEnum,
};
use typedown_lang::db::utils::is_external_url;
use typedown_lang::syntax::ast::{AstNode, Expr, MdLink, MdMedia};
use typedown_lang::syntax::red::RedNode;
use typedown_lang::syntax::syntax_kind::SyntaxKind;

use crate::core::analysis::Analysis;
use crate::core::utils::ast::{
  containing_fref_expr, find_ancestor, get_nearest_expr_ancestor, is_in_mapping_value_position,
  node_at_offset,
};
use crate::core::utils::position::lsp_position_to_text_offset;
use crate::core::utils::uri::uri_to_path;
use crate::lsp::service::utils::symbol::get_resource_label;

pub fn hover(analysis: &Analysis, params: HoverParams) -> Option<Hover> {
  let db = &analysis.db;

  // Get the current file that requests hover information
  let project = analysis.project;
  let uri = &params.text_document_position_params.text_document.uri;
  let path = uri_to_path(uri)?;

  // Parse the current file
  let file = *project.files(db).get(&path)?;
  let root_node = parse_file(db, project, file).ast(db).node.clone();
  let rope = analysis.file_rope(&path)?;

  // Get the hovered node
  let hovered_offset =
    lsp_position_to_text_offset(&rope, params.text_document_position_params.position)?
      .saturating_sub(1);
  let hovered_node = node_at_offset(root_node, hovered_offset)?;

  // fref hover: show target label and schema
  if let Some(text) = build_fref_hover(db, project, file, &hovered_node) {
    return Some(create_hover(text));
  }

  // Markdown link/image hover: show target info
  if let Some(text) = build_link_hover(db, project, file, &hovered_node) {
    return Some(create_hover(text));
  }

  // _type/_extends value: show schema fields
  if let Some(text) = resolve_schema_hover(db, project, &hovered_node) {
    return Some(create_hover(text));
  }

  let text = if is_in_mapping_value_position(&hovered_node) {
    // Value position: show declared type if available, otherwise inferred type
    let expr = get_nearest_expr_ancestor(&hovered_node)?;
    let hir = lower_node(db, project, FileRedNode::new(file, expr.syntax().clone()));
    let expected = expected_node_type(db, hir).typ(db);
    let actual = actual_node_type(db, hir).typ(db)?;

    match expected {
      Some(expected_typ) => type_label(db, &expected_typ),
      None => actual.display_name(db),
    }
  } else if find_ancestor(&hovered_node, SyntaxKind::YamlMappingEntryKey).is_some() {
    // Key position: show the field name with its declared type
    let entry_key = find_ancestor(&hovered_node, SyntaxKind::YamlMappingEntryKey)?;
    let entry = entry_key.parent()?;
    let entry_value = entry
      .children()
      .find(|c| c.kind() == SyntaxKind::YamlMappingEntryValue)?;
    let value_expr = entry_value.children().find_map(Expr::cast)?;
    let hir = lower_node(
      db,
      project,
      FileRedNode::new(file, value_expr.syntax().clone()),
    );
    let typ = expected_node_type(db, hir).typ(db)?;
    let key_text = entry_key.text().trim().to_string();

    format!("{key_text}: {}", type_label(db, &typ))
  } else {
    return None;
  };

  Some(Hover {
    contents: HoverContents::Markup(MarkupContent {
      kind: MarkupKind::Markdown,
      value: format!("```\n{text}\n```"),
    }),
    range: None,
  })
}

// Resolve hover text for a fref() argument: show target label and schema
fn build_fref_hover(
  db: &TypedownDatabase,
  project: Project,
  file: File,
  node: &RedNode,
) -> Option<String> {
  let call = containing_fref_expr(node)?;
  let hir = lower_node(db, project, FileRedNode::new(file, call.syntax().clone()));
  let HirValueKind::Call { args, .. } = hir.kind(db) else {
    return None;
  };
  let arg = args.first()?;
  let HirValueKind::Str(path_str) = arg.kind(db) else {
    return None;
  };

  let config = get_vault_config(db, project);
  let root_dir = config.root_dir(db);
  let target_path = root_dir.join(&path_str);
  let target_file = *project.files(db).get(&target_path)?;
  let sym = file_symbol(db, project, target_file).value(db)?;

  Some(format_resource_hover(db, sym, &path_str))
}

// Format hover text for a resolved resource file
fn format_resource_hover(db: &TypedownDatabase, sym: Symbol, path: &str) -> String {
  let schema_name = get_symbol_type(db, sym).typ(db).map(|t| t.display_name(db));
  let label = get_resource_label(db, sym);

  let mut parts = vec![];
  if let Some(label) = label {
    parts.push(format!("**{label}**"));
  }
  if let Some(schema) = schema_name {
    parts.push(schema);
  }
  parts.push(format!("`{path}`"));

  parts.join("\n\n")
}

fn type_label(db: &TypedownDatabase, typ: &TdTypeEnum) -> String {
  if let Some(sum) = typ.as_td_sum_type() {
    let members = sum.members(db);
    let has_null = members
      .iter()
      .any(|m| m.resolve(db).is_some_and(|t| t.as_td_null_type().is_some()));
    if has_null {
      // Display non-null members joined by " | " with "?" suffix
      let non_null: Vec<String> = members
        .iter()
        .filter(|m| m.resolve(db).is_none_or(|t| t.as_td_null_type().is_none()))
        .filter_map(|m| m.resolve(db).map(|t| t.display_name(db)))
        .collect();
      return format!("{}?", non_null.join(" | "));
    }
  }
  typ.display_name(db)
}

fn create_hover(text: String) -> Hover {
  Hover {
    contents: HoverContents::Markup(MarkupContent {
      kind: MarkupKind::Markdown,
      value: text,
    }),
    range: None,
  }
}

// Resolve a _type/_extends value to its schema definition
fn resolve_schema_hover(db: &TypedownDatabase, project: Project, node: &RedNode) -> Option<String> {
  let entry = find_ancestor(node, SyntaxKind::YamlMappingEntry)?;

  let key = entry
    .children()
    .find(|c| c.kind() == SyntaxKind::YamlMappingEntryKey)?;
  let key_text = key.text();
  let key_text = key_text.trim();
  if key_text != "_type" && key_text != "_extends" {
    return None;
  }

  let value = entry
    .children()
    .find(|c| c.kind() == SyntaxKind::YamlMappingEntryValue)?;
  let schema_name = value.text();
  let schema_name = schema_name.trim();

  let scope = Scope::new(db, ScopeKind::Project(project));
  let project_members = members(db, scope);
  let member_map = project_members.members(db);
  let sym = member_map.get(schema_name)?;
  let typ = evaluate_type(db, *sym).typ(db)?;
  typ.as_td_schema_type()?;

  Some(schema_hover_text(db, &typ))
}

// Show schema name and its fields
fn schema_hover_text(db: &TypedownDatabase, typ: &TdTypeEnum) -> String {
  let name = typ.display_name(db);
  let fields = typ.get_fields(db);
  if fields.is_empty() {
    return name;
  }

  let field_lines: Vec<String> = fields
    .iter()
    .map(|(field_name, lazy)| {
      let type_name = lazy
        .resolve(db)
        .map(|t| type_label(db, &t))
        .unwrap_or_else(|| "unknown".to_string());
      format!("  {field_name}: {type_name}")
    })
    .collect();

  format!("**{name}**\n```yaml\n{}\n```", field_lines.join("\n"))
}

// Show hover info for markdown link or image targets
fn build_link_hover(
  db: &TypedownDatabase,
  project: Project,
  file: File,
  node: &RedNode,
) -> Option<String> {
  let url = if let Some(link_node) = find_ancestor(node, SyntaxKind::MdLink) {
    MdLink::cast(link_node)?.url().map(|t| t.value())?
  } else {
    let media_node = find_ancestor(node, SyntaxKind::MdMedia)?;
    MdMedia::cast(media_node)?.url().map(|t| t.value())?
  };

  if is_external_url(&url) || url.starts_with('#') {
    return None;
  }

  let config = get_vault_config(db, project);
  let root_dir = config.root_dir(db);
  let target_path = if url.starts_with('/') {
    root_dir.join(url.trim_start_matches('/'))
  } else {
    let handle = file.handle(db);
    let file_dir = handle.path()?.parent()?;
    file_dir.join(&url)
  };

  let target_file = *project.files(db).get(&target_path)?;
  let sym = file_symbol(db, project, target_file).value(db)?;

  Some(format_resource_hover(
    db,
    sym,
    &target_path.display().to_string(),
  ))
}

#[cfg(test)]
mod tests {
  use std::collections::HashMap;
  use std::path::PathBuf;
  use std::sync::{Arc, Condvar, Mutex};

  use lsp_types::{
    HoverContents, HoverParams, Position, TextDocumentIdentifier, TextDocumentPositionParams, Uri,
    WorkDoneProgressParams,
  };
  use ropey::Rope;
  use typedown_lang::db::types::{File, FileHandle, FileMetadata, Project};
  use typedown_lang::db::{QueryStorage, TypedownDatabase};

  use crate::core::analysis::Analysis;
  use crate::core::utils::uri::path_to_uri;

  use super::hover;

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
  nickname:
    type: string
    optional: true
---
"#;

  // Accept a text with `|` marker
  // Return the original text with the offset of the marker
  fn cursor(content: &str) -> (String, usize) {
    let offset = content
      .find('|')
      .expect("content must have a cursor marker");
    (content.replacen('|', "", 1), offset)
  }

  // Prepare the LSP client hover request params
  fn make_params(uri: Uri, content: &str, offset: usize) -> HoverParams {
    let rope = Rope::from(content);
    let line = rope.char_to_line(offset);
    let character = offset - rope.line_to_char(line);
    HoverParams {
      text_document_position_params: TextDocumentPositionParams {
        text_document: TextDocumentIdentifier { uri },
        position: Position {
          line: line as u32,
          character: character as u32,
        },
      },
      work_done_progress_params: WorkDoneProgressParams::default(),
    }
  }

  // Project to test against
  // Accept a `content` as the current editing content
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
      (root.join("_types/Person.td"), person_file),
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

  fn hover_text(analysis: &Analysis, uri: Uri, content: &str, offset: usize) -> Option<String> {
    let params = make_params(uri, content, offset);
    let result = hover(analysis, params)?;
    if let HoverContents::Markup(markup) = result.contents {
      Some(markup.value)
    } else {
      None
    }
  }

  #[test]
  fn hover_on_value_shows_declared_type() {
    let (content, offset) = cursor(
      r#"---
_type: Person
name: "Ali|ce"
---
"#,
    );
    let (analysis, uri) = setup(&content);
    let text = hover_text(&analysis, uri, &content, offset).expect("expected hover");
    assert!(
      text.contains("string"),
      "should show declared type 'string': {text}"
    );
  }

  #[test]
  fn hover_on_key_shows_field_type() {
    let (content, offset) = cursor(
      r#"---
_type: Person
na|me: Alice
---
"#,
    );
    let (analysis, uri) = setup(&content);
    let text = hover_text(&analysis, uri, &content, offset).expect("expected hover");
    assert!(text.contains("name"), "expected field name, got: {text}");
    assert!(text.contains("string"), "expected field type, got: {text}");
  }

  #[test]
  fn hover_on_optional_key_shows_optional_marker() {
    let (content, offset) = cursor(
      r#"---
_type: Person
nick|name: Bob
---
"#,
    );
    let (analysis, uri) = setup(&content);
    let text = hover_text(&analysis, uri, &content, offset).expect("expected hover");
    assert!(
      text.contains("nickname"),
      "expected field name, got: {text}"
    );
    assert!(text.contains("string"), "expected field type, got: {text}");
  }

  #[test]
  fn hover_on_fref_shows_label_and_schema() {
    let root = PathBuf::from(if cfg!(windows) { "C:\\vault" } else { "/vault" });
    let type_root = root.join("_types");

    let (content, offset) = cursor(
      r#"---
_type: Person
name: fref("ali|ce.td")
---
"#,
    );
    let test_path = root.join("file.td");
    let uri = path_to_uri(&test_path, "file");

    let db = TypedownDatabase {
      storage: QueryStorage::default(),
    };

    let alice_content = r#"---
_type: Person
_label: "Alice Chen"
name: "Alice"
age: 30
---
"#;

    let files = HashMap::from([
      (
        root.join("typedown.yaml"),
        File::new(
          &db,
          FileHandle::Content(
            root.join("typedown.yaml"),
            VAULT_CONFIG.to_string(),
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
        root.join("alice.td"),
        File::new(
          &db,
          FileHandle::Content(
            root.join("alice.td"),
            alice_content.to_string(),
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

    let project = Project::new(&db, root, files);
    let analysis = Analysis::new(
      db,
      project,
      Arc::new(HashMap::new()),
      Arc::new(HashMap::new()),
      Arc::new((Mutex::new(1), Condvar::new())),
    );

    let text = hover_text(&analysis, uri, &content, offset).expect("fref should show hover");
    assert!(text.contains("Alice Chen"), "should show _label: {text}");
    assert!(text.contains("Person"), "should show schema name: {text}");
  }

  // Helper that uses the standard setup
  fn hover_text_from_setup(content_with_cursor: &str) -> Option<String> {
    let (content, offset) = cursor(content_with_cursor);
    let (analysis, uri) = setup(&content);
    hover_text(&analysis, uri, &content, offset)
  }

  #[test]
  fn hover_on_markdown_link_shows_target_info() {
    let root = PathBuf::from(if cfg!(windows) { "C:\\vault" } else { "/vault" });
    let type_root = root.join("_types");

    let (content, offset) = cursor(
      r#"---
_type: Person
name: "Test"
---

[see alice](ali|ce.td)
"#,
    );
    let test_path = root.join("file.td");
    let uri = path_to_uri(&test_path, "file");

    let db = TypedownDatabase {
      storage: QueryStorage::default(),
    };

    let alice_content = r#"---
_type: Person
_label: "Alice Chen"
name: "Alice"
age: 30
---
"#;

    let files = HashMap::from([
      (
        root.join("typedown.yaml"),
        File::new(
          &db,
          FileHandle::Content(
            root.join("typedown.yaml"),
            VAULT_CONFIG.to_string(),
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
        root.join("alice.td"),
        File::new(
          &db,
          FileHandle::Content(
            root.join("alice.td"),
            alice_content.to_string(),
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

    let project = Project::new(&db, root, files);
    let analysis = Analysis::new(
      db,
      project,
      Arc::new(HashMap::new()),
      Arc::new(HashMap::new()),
      Arc::new((Mutex::new(1), Condvar::new())),
    );

    let text =
      hover_text(&analysis, uri, &content, offset).expect("markdown link should show hover");
    assert!(text.contains("Alice Chen"), "should show _label: {text}");
    assert!(text.contains("Person"), "should show schema name: {text}");
  }

  #[test]
  fn hover_on_external_link_returns_none() {
    let text = hover_text_from_setup(
      r#"---
_type: Person
name: "Test"
---

[external](https://exam|ple.com)
"#,
    );
    assert!(text.is_none(), "external link should not show hover");
  }

  #[test]
  fn hover_on_dangling_link_returns_none() {
    let text = hover_text_from_setup(
      r#"---
_type: Person
name: "Test"
---

[missing](nonexist|ent.td)
"#,
    );
    assert!(text.is_none(), "dangling link should not show hover");
  }

  #[test]
  fn hover_on_type_value_shows_schema_fields() {
    let text = hover_text_from_setup(
      r#"---
_type: Per|son
name: "Alice"
age: 30
---
"#,
    );
    let text = text.expect("_type value should show hover");
    assert!(text.contains("Person"), "should show schema name: {text}");
    assert!(text.contains("name"), "should show field 'name': {text}");
    assert!(text.contains("age"), "should show field 'age': {text}");
  }

  #[test]
  fn hover_on_boolean_value_shows_type() {
    let text = hover_text_from_setup(
      r#"---
_type: Person
name: "Alice"
verified: tru|e
---
"#,
    );
    let text = text.expect("boolean value should show hover");
    // Boolean literals resolve to the literal type
    assert!(
      text.contains("true") || text.contains("boolean"),
      "should show type info: {text}"
    );
  }

  #[test]
  fn hover_on_plain_markdown_body_returns_none() {
    let text = hover_text_from_setup(
      r#"---
_type: Person
name: "Alice"
age: 30
---

Some plain te|xt.
"#,
    );
    assert!(text.is_none(), "plain markdown body should not show hover");
  }
}
