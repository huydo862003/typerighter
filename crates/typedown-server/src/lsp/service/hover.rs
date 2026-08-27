use lsp_types::{Hover, HoverContents, HoverParams, MarkupContent, MarkupKind};

use typedown_lang::db::TypedownDatabase;
use typedown_lang::db::derived::get_vault_config::get_vault_config;
use typedown_lang::db::derived::hir::lower_node;
use typedown_lang::db::derived::name_resolver::file_symbol::file_symbol;
use typedown_lang::db::derived::parse_file::parse_file;
use typedown_lang::db::derived::typechecker::actual_node_type::actual_node_type;
use typedown_lang::db::derived::typechecker::expected_node_type::expected_node_type;
use typedown_lang::db::derived::typechecker::get_symbol_type::get_symbol_type;
use typedown_lang::db::types::derived::object_system::TdStaticType;
use typedown_lang::db::types::{File, HirValueKind, Project, TdTypeEnum};
use typedown_lang::syntax::ast::{AstNode, Expr};
use typedown_lang::syntax::red::RedNode;
use typedown_lang::syntax::syntax_kind::SyntaxKind;

use crate::core::analysis::Analysis;
use crate::core::utils::ast::{
  containing_fref_expr, find_ancestor, is_in_mapping_value_position, nearest_expr_ancestor,
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
  let root_node = parse_file(db, project, file).ast(db);
  let rope = analysis.file_rope(&path)?;

  // Get the hovered node
  let hovered_offset =
    lsp_position_to_text_offset(&rope, params.text_document_position_params.position)?
      .saturating_sub(1);
  let hovered_node = node_at_offset(root_node, hovered_offset)?;

  // fref hover: show target label and schema
  if let Some(fref_text) = fref_hover_text(db, project, file, &hovered_node) {
    return Some(Hover {
      contents: HoverContents::Markup(MarkupContent {
        kind: MarkupKind::Markdown,
        value: fref_text,
      }),
      range: None,
    });
  }

  let text = if is_in_mapping_value_position(&hovered_node) {
    // Value position: show the resolved type of the expression.
    let expr_node = nearest_expr_ancestor(&hovered_node)?;
    let hir = lower_node(db, project, file, expr_node);

    let typ = actual_node_type(db, hir).typ(db)?;
    typ.display_name(db)
  } else if find_ancestor(&hovered_node, SyntaxKind::YamlMappingEntryKey).is_some() {
    // Key position: show the field name with its declared type.
    let entry_key = find_ancestor(&hovered_node, SyntaxKind::YamlMappingEntryKey)?;
    let entry = entry_key.parent()?;
    let entry_value = entry
      .children()
      .find(|c| c.kind() == SyntaxKind::YamlMappingEntryValue)?;
    let value_expr = entry_value.children().find_map(Expr::cast)?;
    let hir = lower_node(db, project, file, value_expr.syntax().clone());
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
fn fref_hover_text(
  db: &TypedownDatabase,
  project: Project,
  file: File,
  node: &RedNode,
) -> Option<String> {
  let call = containing_fref_expr(node)?;
  let hir = lower_node(db, project, file, call.syntax().clone());
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

  let schema_name = get_symbol_type(db, sym).typ(db).map(|t| t.display_name(db));
  let label = get_resource_label(db, sym);

  let mut parts = vec![];
  if let Some(label) = label {
    parts.push(format!("**{label}**"));
  }
  if let Some(schema) = schema_name {
    parts.push(schema);
  }
  parts.push(format!("`{path_str}`"));

  Some(parts.join("\n\n"))
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
        .filter(|m| !m.resolve(db).is_some_and(|t| t.as_td_null_type().is_some()))
        .filter_map(|m| m.resolve(db).map(|t| t.display_name(db)))
        .collect();
      return format!("{}?", non_null.join(" | "));
    }
  }
  typ.display_name(db)
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
  fn hover_on_value_shows_resolved_type() {
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
      text.contains("\"Alice\""),
      "expected literal type, got: {text}"
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
}
