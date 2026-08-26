use std::path::PathBuf;

use lsp_types::{GotoDefinitionParams, GotoDefinitionResponse, Location, Range};

use typedown_lang::db::TypedownDatabase;
use typedown_lang::db::derived::get_vault_config::get_vault_config;
use typedown_lang::db::derived::hir::lower_node;
use typedown_lang::db::derived::name_resolver::referee::referee;
use typedown_lang::db::derived::parse_file::parse_file;
use typedown_lang::db::types::{FileHandle, HirValueKind, Project, SymbolKind};
use typedown_lang::syntax::ast::AstNode;
use typedown_lang::syntax::red::RedNode;

use crate::core::analysis::Analysis;
use crate::core::utils::ast::{containing_fref_expr, nearest_expr_ancestor, node_at_offset};
use crate::core::utils::position::lsp_position_to_text_offset;
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

  // fref("path") string argument: jump to the referenced file.
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

/// If the cursor is inside a fref() string argument, return the resolved target path.
fn fref_target(db: &TypedownDatabase, project: Project, node: &RedNode) -> Option<PathBuf> {
  let call_expr = containing_fref_expr(node)?;

  let dummy_file = *project.files(db).values().next()?;
  let hir = lower_node(db, project, dummy_file, call_expr.syntax().clone());
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
