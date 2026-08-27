use std::path::PathBuf;

use lsp_types::{GotoDefinitionParams, GotoDefinitionResponse, Location, Range};

use typedown_lang::db::TypedownDatabase;
use typedown_lang::db::derived::get_vault_config::get_vault_config;
use typedown_lang::db::derived::hir::lower_node;
use typedown_lang::db::derived::name_resolver::members::members;
use typedown_lang::db::derived::name_resolver::referee::referee;
use typedown_lang::db::derived::parse_file::parse_file;
use typedown_lang::db::types::{
  File, FileHandle, HirValueKind, Project, Scope, Symbol, SymbolKind,
};
use typedown_lang::db::utils::schema_name_in_mapping;
use typedown_lang::syntax::ast::AstNode;
use typedown_lang::syntax::red::RedNode;
use typedown_lang::syntax::syntax_kind::SyntaxKind;

use crate::core::analysis::Analysis;
use crate::core::utils::ast::{
  containing_fref_expr, find_ancestor, nearest_expr_ancestor, node_at_offset,
};
use crate::core::utils::position::{lsp_position_to_text_offset, text_offset_to_lsp_position};
use crate::core::utils::uri::{path_to_uri, uri_to_path};

pub fn definition(
  analysis: &Analysis,
  params: GotoDefinitionParams,
) -> Option<GotoDefinitionResponse> {
  let db = &analysis.db;
  let project = analysis.project;

  let uri = &params.text_document_position_params.text_document.uri;
  let path = uri_to_path(uri)?;
  let rope = analysis.file_rope(&path)?;
  let offset = lsp_position_to_text_offset(&rope, params.text_document_position_params.position)?;

  let file = *project.files(db).get(&path)?;
  let root = parse_file(db, project, file).ast(db);
  let lookup = offset.saturating_sub(1);
  let node = node_at_offset(root, lookup)?;

  // Field key: jump to the property definition in the schema file
  if let Some(response) = field_key_definition(analysis, db, project, &node) {
    return Some(response);
  }

  // fref("path") string argument: jump to the referenced file
  if let Some(target_path) = fref_target(db, project, &node) {
    let scheme = analysis
      .scheme_map
      .get(&target_path)
      .map(String::as_str)
      .unwrap_or("file");
    let target_uri = path_to_uri(&target_path, scheme);
    let location = Location {
      uri: target_uri,
      range: Range::default(),
    };
    return Some(GotoDefinitionResponse::Scalar(location));
  }

  // Identifier or type reference: resolve via referee.
  let expr_node = nearest_expr_ancestor(&node)?;
  let hir = lower_node(db, project, file, expr_node);
  let symbol = referee(db, hir).value(db)?;

  let target_file = match symbol.kind(db) {
    SymbolKind::UserDefinedSchema(_, target_file)
    | SymbolKind::UserDefinedResource(_, target_file) => target_file,
    _ => return None,
  };

  let target_path = match target_file.handle(db) {
    FileHandle::Path(path, _) => path,
    FileHandle::Content(_, _, _) => project
      .files(db)
      .iter()
      .find(|(_, f)| **f == target_file)
      .map(|(p, _)| p.clone())?,
  };

  let scheme = analysis
    .scheme_map
    .get(&target_path)
    .map(String::as_str)
    .unwrap_or("file");
  let target_uri = path_to_uri(&target_path, scheme);
  let location = Location {
    uri: target_uri,
    range: Range::default(),
  };
  Some(GotoDefinitionResponse::Scalar(location))
}

// Jump from a field key to its property definition in the schema file
fn field_key_definition(
  analysis: &Analysis,
  db: &TypedownDatabase,
  project: Project,
  node: &RedNode,
) -> Option<GotoDefinitionResponse> {
  // Must be on a mapping entry key
  let entry_key = find_ancestor(node, SyntaxKind::YamlMappingEntryKey)?;
  let key_text = entry_key.text().trim().to_string();

  // Skip reserved keys
  if key_text.starts_with('_') {
    return None;
  }

  // Find the enclosing mapping and its _type
  let entry = entry_key.parent()?;
  let mapping = find_ancestor(&entry, SyntaxKind::YamlMapping)?;
  let schema_name = schema_name_in_mapping(&mapping)?;

  // Resolve the schema and walk the _extends chain to find the file that defines the field
  let scope = Scope::project_scope(db, project);
  let sym = *members(db, scope).members(db).get(&schema_name)?;
  let (schema_file, field_offset) =
    find_field_in_schema_chain(db, project, sym, &key_text, &scope)?;

  let schema_path = match schema_file.handle(db) {
    FileHandle::Path(path, _) => path,
    FileHandle::Content(path, _, _) => path,
  };

  let scheme = analysis
    .scheme_map
    .get(&schema_path)
    .map(String::as_str)
    .unwrap_or("file");
  let target_uri = path_to_uri(&schema_path, scheme);

  let schema_rope = analysis.file_rope(&schema_path)?;
  let position = text_offset_to_lsp_position(&schema_rope, field_offset);

  Some(GotoDefinitionResponse::Scalar(Location {
    uri: target_uri,
    range: Range {
      start: position,
      end: position,
    },
  }))
}

// Walk the schema's _extends chain to find which file defines the given field
fn find_field_in_schema_chain(
  db: &TypedownDatabase,
  project: Project,
  sym: Symbol,
  field_name: &str,
  scope: &Scope,
) -> Option<(File, usize)> {
  let SymbolKind::UserDefinedSchema(_, schema_file) = sym.kind(db) else {
    return None;
  };

  // Check if this schema file directly defines the field
  let root = parse_file(db, project, schema_file).ast(db);
  if let Some(offset) = find_property_key_offset(&root, field_name) {
    return Some((schema_file, offset));
  }

  // Check parent schema via _extends
  let extends_entry = find_entry_by_key(&root, "_extends")?;
  let extends_name = entry_value_text(&extends_entry)?;

  let parent_sym = *members(db, *scope).members(db).get(&extends_name)?;
  find_field_in_schema_chain(db, project, parent_sym, field_name, scope)
}

// Find a mapping entry by key name, recursing through wrapper nodes
fn find_entry_by_key(node: &RedNode, key_name: &str) -> Option<RedNode> {
  for child in node.children() {
    if child.kind() == SyntaxKind::YamlMappingEntry
      && child
        .children()
        .any(|c| c.kind() == SyntaxKind::YamlMappingEntryKey && c.text().trim() == key_name)
    {
      return Some(child);
    }
    if let Some(found) = find_entry_by_key(&child, key_name) {
      return Some(found);
    }
  }
  None
}

fn entry_value_text(entry: &RedNode) -> Option<String> {
  entry
    .children()
    .find(|c| c.kind() == SyntaxKind::YamlMappingEntryValue)
    .map(|v| v.text().trim().to_string())
}

fn entry_key_offset(entry: &RedNode) -> Option<usize> {
  entry
    .children()
    .find(|c| c.kind() == SyntaxKind::YamlMappingEntryKey)
    .map(|k| k.offset())
}

// Find the offset of a property key inside a schema's properties block
fn find_property_key_offset(root: &RedNode, field_name: &str) -> Option<usize> {
  let props_entry = find_entry_by_key(root, "properties")?;
  let props_value = props_entry
    .children()
    .find(|c| c.kind() == SyntaxKind::YamlMappingEntryValue)?;
  let field_entry = find_entry_by_key(&props_value, field_name)?;
  entry_key_offset(&field_entry)
}

// Resolve the target path from a fref() string argument
fn fref_target(db: &TypedownDatabase, project: Project, node: &RedNode) -> Option<PathBuf> {
  let call_expr = containing_fref_expr(node)?;

  // Any file works as context since fref args don't depend on the enclosing file
  let context_file = *project.files(db).values().next()?;
  let hir = lower_node(db, project, context_file, call_expr.syntax().clone());
  if let HirValueKind::Call { args, .. } = hir.kind(db)
    && let Some(arg) = args.first()
    && let HirValueKind::Str(path_str) = arg.kind(db)
  {
    let config = get_vault_config(db, project);
    let root_dir = config.root_dir(db);
    return Some(root_dir.join(path_str));
  }
  None
}

#[cfg(test)]
mod tests {
  use std::collections::HashMap;
  use std::path::PathBuf;
  use std::sync::{Arc, Condvar, Mutex};

  use lsp_types::{
    GotoDefinitionParams, GotoDefinitionResponse, PartialResultParams, Position,
    TextDocumentIdentifier, TextDocumentPositionParams, Uri, WorkDoneProgressParams,
  };
  use ropey::Rope;
  use typedown_lang::db::types::{File, FileHandle, FileMetadata, Project};
  use typedown_lang::db::{QueryStorage, TypedownDatabase};

  use crate::core::analysis::Analysis;
  use crate::core::utils::uri::path_to_uri;

  use super::definition;

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
---
"#;
  const CONTENT_ALICE: &str = r#"---
_type: Person
name: Alice
age: 30
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

  // Prepare the LSP client definition request params
  fn make_params(uri: Uri, content: &str, offset: usize) -> GotoDefinitionParams {
    let rope = Rope::from(content);
    let line = rope.char_to_line(offset);
    let character = offset - rope.line_to_char(line);
    GotoDefinitionParams {
      text_document_position_params: TextDocumentPositionParams {
        text_document: TextDocumentIdentifier { uri },
        position: Position {
          line: line as u32,
          character: character as u32,
        },
      },
      work_done_progress_params: WorkDoneProgressParams::default(),
      partial_result_params: PartialResultParams::default(),
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
    let alice_file = File::new(
      &db,
      FileHandle::Content(
        root.join("alice.td"),
        CONTENT_ALICE.to_string(),
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
      (root.join("_types/Person.td"), person_file),
      (root.join("alice.td"), alice_file),
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
  fn definition_on_type_value_jumps_to_schema_file() {
    let (content, offset) = cursor(
      r#"---
_type: Per|son
name: Alice
---
"#,
    );
    let (analysis, uri) = setup(&content);
    let params = make_params(uri, &content, offset);

    let response = definition(&analysis, params);
    let Some(GotoDefinitionResponse::Scalar(location)) = response else {
      panic!("expected a definition location");
    };
    assert!(
      location.uri.as_str().contains("Person"),
      "should point to Person.td, got: {:?}",
      location.uri
    );
  }

  #[test]
  fn definition_on_fref_arg_jumps_to_target_file() {
    let (content, offset) = cursor(
      r#"---
_type: Person
name: fref("ali|ce.td")
---
"#,
    );
    let (analysis, uri) = setup(&content);
    let params = make_params(uri, &content, offset);

    let response = definition(&analysis, params);
    let Some(GotoDefinitionResponse::Scalar(location)) = response else {
      panic!("expected a definition location");
    };
    assert!(
      location.uri.as_str().contains("alice"),
      "should point to alice.td, got: {:?}",
      location.uri
    );
  }

  // fref resolves from vault root, not project root.
  // When root_dir is "vault", fref("alice.td") should resolve to /project/vault/alice.td.
  #[test]
  fn definition_on_fref_uses_vault_root() {
    let project_root = PathBuf::from(if cfg!(windows) {
      "C:\\project"
    } else {
      "/project"
    });
    let vault_root = project_root.join("vault");
    let type_root = vault_root.join("_types");

    let (content, offset) = cursor(
      r#"---
_type: Person
name: fref("ali|ce.td")
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

    let files = HashMap::from([
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
      Arc::new(HashMap::new()),
      Arc::new(HashMap::new()),
      Arc::new((Mutex::new(1), Condvar::new())),
    );

    let params = make_params(uri, &content, offset);
    let response = definition(&analysis, params);
    let Some(GotoDefinitionResponse::Scalar(location)) = response else {
      panic!("expected a definition location");
    };
    let uri_str = location.uri.as_str();
    assert!(
      uri_str.contains("alice"),
      "should point to alice.td, got: {uri_str}",
    );
    assert!(
      uri_str.contains("vault"),
      "should resolve under vault root, got: {uri_str}",
    );
  }

  // Field key jumps to the schema's property definition
  #[test]
  fn definition_on_field_key_jumps_to_schema_property() {
    let (content, offset) = cursor(
      r#"---
_type: Person
na|me: "Alice"
---
"#,
    );
    let (analysis, uri) = setup(&content);
    let params = make_params(uri, &content, offset);

    let response = definition(&analysis, params);
    let Some(GotoDefinitionResponse::Scalar(location)) = response else {
      panic!("expected a definition location for field key");
    };
    assert!(
      location.uri.as_str().contains("Person"),
      "should jump to Person.td, got: {:?}",
      location.uri
    );
    // Should point to a non-zero position (the name field inside properties)
    assert!(
      location.range.start.line > 0 || location.range.start.character > 0,
      "should point to the field definition, not the file start"
    );
  }

  // Inherited field jumps to the parent schema file, not the child
  #[test]
  fn definition_on_inherited_field_jumps_to_parent_schema() {
    let root = PathBuf::from(if cfg!(windows) { "C:\\vault" } else { "/vault" });
    let type_root = root.join("_types");

    let contractor_schema = r#"---
_type: schema
_extends: Person
properties:
  agency:
    type: string
---
"#;

    let (content, offset) = cursor(
      r#"---
_type: Contractor
na|me: "Alice"
agency: "Acme"
---
"#,
    );

    let test_path = root.join("file.td");
    let uri = path_to_uri(&test_path, "file");

    let db = TypedownDatabase {
      storage: QueryStorage::default(),
    };

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
        type_root.join("Contractor.td"),
        File::new(
          &db,
          FileHandle::Content(
            type_root.join("Contractor.td"),
            contractor_schema.to_string(),
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

    let params = make_params(uri, &content, offset);
    let response = definition(&analysis, params);
    let Some(GotoDefinitionResponse::Scalar(location)) = response else {
      panic!("expected definition for inherited field");
    };
    // name is defined in Person.td, not Contractor.td
    assert!(
      location.uri.as_str().contains("Person"),
      "inherited field should jump to parent schema Person.td, got: {:?}",
      location.uri
    );
  }

  // Reserved keys (_type, _label) should not trigger field key definition
  #[test]
  fn definition_on_reserved_key_returns_none() {
    let (content, offset) = cursor(
      r#"---
_ty|pe: Person
name: "Alice"
---
"#,
    );
    let (analysis, uri) = setup(&content);
    let params = make_params(uri, &content, offset);

    let response = definition(&analysis, params);
    // _type should resolve to the schema via the referee path, not the field key path
    // but it should NOT jump to a property definition
    if let Some(GotoDefinitionResponse::Scalar(location)) = &response {
      assert!(
        location.uri.as_str().contains("Person"),
        "_type should resolve to the schema"
      );
    }
  }

  #[test]
  fn definition_on_plain_value_returns_none() {
    let (content, offset) = cursor(
      r#"---
_type: Person
name: Ali|ce
---
"#,
    );
    let (analysis, uri) = setup(&content);
    let params = make_params(uri, &content, offset);

    let response = definition(&analysis, params);
    assert!(
      response.is_none(),
      "plain string value should not have a definition"
    );
  }
}
