// Inlay hints for fref() calls, showing the target's _label after the call
use lsp_types::{InlayHint, InlayHintKind, InlayHintLabel, InlayHintParams};

use typedown_lang::db::TypedownDatabase;
use typedown_lang::db::derived::get_vault_config::get_vault_config;
use typedown_lang::db::derived::name_resolver::file_symbol::file_symbol;
use typedown_lang::db::derived::parse_file::parse_file;
use typedown_lang::db::types::Project;
use typedown_lang::syntax::ast::StrLit;
use typedown_lang::syntax::ast::{AstNode, CallExpr};
use typedown_lang::syntax::red::RedNode;
use typedown_lang::syntax::syntax_kind::SyntaxKind;

use crate::core::analysis::Analysis;
use crate::core::utils::position::text_offset_to_lsp_position;
use crate::core::utils::uri::uri_to_path;
use crate::lsp::service::utils::symbol::get_resource_label;

pub fn resolve_inlay_hints(analysis: &Analysis, params: InlayHintParams) -> Option<Vec<InlayHint>> {
  let db = &analysis.db;
  let project = analysis.project;

  let path = uri_to_path(&params.text_document.uri)?;
  let file = *project.files(db).get(&path)?;
  let rope = analysis.file_rope(&path)?;
  let root = parse_file(db, project, file).ast(db).node.clone();

  let mut hints = Vec::new();
  collect_fref_hints(db, project, &root, &rope, &mut hints);
  Some(hints)
}

fn collect_fref_hints(
  db: &TypedownDatabase,
  project: Project,
  node: &RedNode,
  rope: &ropey::Rope,
  hints: &mut Vec<InlayHint>,
) {
  if node.kind() == SyntaxKind::CallExpr
    && let Some(hint) = build_fref_hint(db, project, node, rope)
  {
    hints.push(hint);
    return;
  }

  for child in node.children() {
    collect_fref_hints(db, project, &child, rope, hints);
  }
}

fn build_fref_hint(
  db: &TypedownDatabase,
  project: Project,
  call_node: &RedNode,
  rope: &ropey::Rope,
) -> Option<InlayHint> {
  let callee = call_node.children().next()?;
  if callee.text().trim() != "fref" {
    return None;
  }

  let call = CallExpr::cast(call_node.clone())?;
  let path_string = call
    .args()
    .into_iter()
    .next()
    .and_then(|arg| StrLit::cast(arg.syntax().clone()).and_then(|string| string.plain_value()))?;

  let config = get_vault_config(db, project);
  let root_dir = config.root_dir(db);
  let target_path = root_dir.join(&path_string);
  let target_file = *project.files(db).get(&target_path)?;
  let symbol = file_symbol(db, project, target_file).value(db)?;
  let label = get_resource_label(db, symbol)?;

  let end_offset = call_node.end_offset();
  let position = text_offset_to_lsp_position(rope, end_offset);

  Some(InlayHint {
    position,
    label: InlayHintLabel::String(label),
    kind: Some(InlayHintKind::PARAMETER),
    text_edits: None,
    tooltip: None,
    padding_left: Some(true),
    padding_right: None,
    data: None,
  })
}

#[cfg(test)]
mod tests {
  use super::resolve_inlay_hints;
  use crate::core::analysis::Analysis;
  use crate::core::utils::uri::path_to_uri;
  use lsp_types::{InlayHintParams, Range, TextDocumentIdentifier, WorkDoneProgressParams};
  use std::collections::HashMap;
  use std::path::PathBuf;
  use std::sync::{Arc, Condvar, Mutex};
  use typedown_lang::db::types::{File, FileHandle, FileMetadata, Project};
  use typedown_lang::db::{QueryStorage, TypedownDatabase};

  const VAULT_CONFIG: &str = r#"version: "1"
vault:
  root_dir: "."
"#;

  const SCHEMA_PERSON: &str = r#"---
_type: schema
properties:
  name:
    type: string
  friend:
    type: Person?
---
"#;

  const CONTENT_ALICE: &str = r#"---
_type: Person
_label: "Alice Chen"
name: "Alice"
---
"#;

  #[test]
  fn fref_in_frontmatter_shows_label_hint() {
    let root = PathBuf::from(if cfg!(windows) { "C:\\vault" } else { "/vault" });
    let type_root = root.join("_types");

    let content = r#"---
_type: Person
name: "Bob"
friend: fref("alice.td")
---
"#;

    let test_path = root.join("bob.td");
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
        root.join("alice.td"),
        File::new(
          &db,
          FileHandle::Content(
            root.join("alice.td"),
            CONTENT_ALICE.to_string(),
            FileMetadata::default(),
          ),
        ),
      ),
      (
        test_path.clone(),
        File::new(
          &db,
          FileHandle::Content(test_path, content.to_string(), FileMetadata::default()),
        ),
      ),
    ]);

    let project = Project::new(&db, root, files, None);
    let analysis = Analysis::new(
      db,
      project,
      Arc::new(HashMap::new()),
      Arc::new(HashMap::new()),
      Arc::new((Mutex::new(1), Condvar::new())),
    );

    let params = InlayHintParams {
      text_document: TextDocumentIdentifier { uri },
      range: Range::default(),
      work_done_progress_params: WorkDoneProgressParams::default(),
    };

    let hints = resolve_inlay_hints(&analysis, params).expect("should return hints");
    assert!(
      hints
        .iter()
        .any(|hint| matches!(&hint.label, lsp_types::InlayHintLabel::String(string) if string == "Alice Chen")),
      "should show Alice Chen hint: {:?}",
      hints.iter().map(|hint| &hint.label).collect::<Vec<_>>()
    );
  }
}
