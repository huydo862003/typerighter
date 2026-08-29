use std::path::{Path, PathBuf};

use lsp_types::{RenameParams, WorkspaceEdit};
use typedown_lang::db::derived::get_vault_config::get_vault_config;
use typedown_lang::db::derived::hir::lower_node;
use typedown_lang::db::derived::name_resolver::referee::referee;
use typedown_lang::db::derived::name_resolver::resolution_index::references;
use typedown_lang::db::types::SymbolKind;
use typedown_lang::db::utils::is_content_file;
use typedown_lang::syntax::ast::AstNode;

use crate::core::analysis::Analysis;
use crate::core::utils::position::lsp_position_to_text_offset;
use crate::core::utils::uri::uri_to_path;
use crate::lsp::service::rename_symbol::utils::{build_workspace_edit, collect_reference_edits};
use crate::lsp::service::utils::symbol::{CursorSymbol, find_symbol_at_cursor, symbol_file_path};

pub fn rename(analysis: &Analysis, params: RenameParams) -> Option<WorkspaceEdit> {
  let db = &analysis.db;
  let project = analysis.project;
  let new_name = params.new_name.trim();

  // Locate the file and offset of the rename request
  let path = uri_to_path(&params.text_document_position.text_document.uri)?;
  let file = *project.files(db).get(&path)?;
  let rope = analysis.file_rope(&path)?;
  let offset = lsp_position_to_text_offset(&rope, params.text_document_position.position)?;

  // Find the renameable symbol at the cursor (fref or ident)
  let rename_symbol = find_symbol_at_cursor(db, project, file, offset)?;

  // Resolve to the underlying symbol
  let syntax = match &rename_symbol {
    CursorSymbol::Fref { call_node } => call_node.syntax().clone(),
    CursorSymbol::Identifier { ident_node } => ident_node.syntax().clone(),
  };
  let symbol = referee(db, lower_node(db, project, file, syntax)).value(db)?;

  // Builtins cannot be renamed
  if matches!(
    symbol.kind(db),
    SymbolKind::BuiltinMacro(_) | SymbolKind::BuiltinSchema(_)
  ) {
    return None;
  }

  let old_path = symbol_file_path(db, symbol)?;
  let root_dir = get_vault_config(db, project).root_dir(db);

  // Compute new file path and identifier stem based on rename kind
  let (new_path, new_stem) = match &rename_symbol {
    CursorSymbol::Fref { .. } => compute_fref_target(new_name, &root_dir),
    CursorSymbol::Identifier { .. } => compute_ident_target(db, new_name, symbol, &old_path)?,
  };

  // Collect text edits for all references + file rename
  let refs = references(db, project, symbol);
  let edits = collect_reference_edits(analysis, &refs, &new_stem, &new_path, &root_dir)?;

  build_workspace_edit(analysis, edits, vec![(old_path, new_path)])
}

fn compute_fref_target(new_name: &str, root_dir: &Path) -> (PathBuf, String) {
  let new_path = Path::new(new_name);
  // Check extension on filename only, not the full path (e.g. "v2.0/file" has no extension)
  let has_extension = new_path
    .file_name()
    .and_then(|f| Path::new(f).extension())
    .is_some();
  let absolute = if has_extension {
    root_dir.join(new_name)
  } else {
    root_dir.join(format!("{}.td", new_name))
  };
  let stem = new_path
    .file_stem()
    .and_then(|s| s.to_str())
    .unwrap_or(new_name)
    .to_string();
  (absolute, stem)
}

fn compute_ident_target(
  db: &dyn typedown_incremental::QueryDatabase,
  new_name: &str,
  symbol: typedown_lang::db::types::Symbol,
  old_path: &Path,
) -> Option<(PathBuf, String)> {
  let new_path = Path::new(new_name);
  let has_extension = new_path
    .file_name()
    .and_then(|f| Path::new(f).extension())
    .is_some();
  let is_schema = matches!(symbol.kind(db), SymbolKind::UserDefinedSchema(_, _));
  // Schemas must have a content file extension
  if is_schema && has_extension && !is_content_file(new_path) {
    return None;
  }
  let stem = if has_extension {
    new_path
      .file_stem()
      .and_then(|s| s.to_str())
      .unwrap_or(new_name)
  } else {
    new_name
  };
  if is_schema && stem.contains('/') {
    return None;
  }
  let filename = if has_extension {
    new_name.to_string()
  } else {
    format!("{}.td", new_name)
  };
  let absolute = old_path.parent()?.join(filename);
  Some((absolute, stem.to_string()))
}

#[cfg(test)]
mod tests {
  use std::collections::HashMap;
  use std::path::PathBuf;
  use std::sync::{Arc, Condvar, Mutex};

  use lsp_types::{
    DocumentChangeOperation, DocumentChanges, Position, RenameParams, ResourceOp,
    TextDocumentIdentifier, TextDocumentPositionParams,
  };
  use ropey::Rope;
  use typedown_lang::db::types::{File, FileHandle, FileMetadata, Project};
  use typedown_lang::db::{QueryStorage, TypedownDatabase};

  use super::rename;
  use crate::core::analysis::Analysis;
  use crate::core::utils::uri::path_to_uri;

  const VAULT_CONFIG: &str = r#"
version: "1"
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

  fn cursor(content: &str) -> (String, usize) {
    let offset = content
      .find('|')
      .expect("content must have a cursor marker");
    (content.replacen('|', "", 1), offset)
  }

  fn make_params(
    uri: lsp_types::Uri,
    content: &str,
    offset: usize,
    new_name: &str,
  ) -> RenameParams {
    let rope = Rope::from(content);
    let line = rope.char_to_line(offset);
    let character = offset - rope.line_to_char(line);
    RenameParams {
      text_document_position: TextDocumentPositionParams {
        text_document: TextDocumentIdentifier { uri },
        position: Position {
          line: line as u32,
          character: character as u32,
        },
      },
      new_name: new_name.to_string(),
      work_done_progress_params: Default::default(),
    }
  }

  fn setup(content: &str) -> (Analysis, lsp_types::Uri) {
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

  /// Serialize a WorkspaceEdit to a deterministic snapshot string
  fn snapshot(edit: &lsp_types::WorkspaceEdit) -> String {
    let mut lines = vec![];
    if let Some(DocumentChanges::Operations(ops)) = &edit.document_changes {
      for op in ops {
        match op {
          DocumentChangeOperation::Edit(doc_edit) => {
            let uri = doc_edit.text_document.uri.as_str();
            let short = uri.rfind("/vault/").map_or(uri, |i| &uri[i..]);
            for edit in &doc_edit.edits {
              if let lsp_types::OneOf::Left(text_edit) = edit {
                let r = &text_edit.range;
                lines.push(format!(
                  "EDIT {} [{}:{}-{}:{}] -> {:?}",
                  short,
                  r.start.line,
                  r.start.character,
                  r.end.line,
                  r.end.character,
                  text_edit.new_text
                ));
              }
            }
          }
          DocumentChangeOperation::Op(ResourceOp::Rename(rename_file)) => {
            let old = rename_file.old_uri.as_str();
            let new = rename_file.new_uri.as_str();
            let old_short = old.rfind("/vault/").map_or(old, |i| &old[i..]);
            let new_short = new.rfind("/vault/").map_or(new, |i| &new[i..]);
            lines.push(format!("RENAME {} -> {}", old_short, new_short));
          }
          _ => {}
        }
      }
    }
    lines.sort();
    lines.join("\n")
  }

  // Rename an identifier in _type position
  #[test]
  fn rename_ident_in_type_field() {
    let (raw, offset) = cursor(
      r#"---
_type: |Person
name: Alice
---
"#,
    );
    let (analysis, uri) = setup(&raw);
    let edit =
      rename(&analysis, make_params(uri, &raw, offset, "Human")).expect("should produce edits");
    let snap = snapshot(&edit);

    assert!(snap.contains("EDIT"), "should have text edits");
    assert!(snap.contains("RENAME"), "should have file rename");
    assert_eq!(
      snap.matches("EDIT").count(),
      2,
      "should have 2 text edits:\n{}",
      snap
    );
    assert!(
      snap
        .lines()
        .filter(|l| l.starts_with("EDIT"))
        .all(|l| l.contains("\"Human\"")),
      "all text edits should rename to Human:\n{}",
      snap
    );
    assert!(
      snap.contains("Human.td"),
      "file rename should use Human.td:\n{}",
      snap
    );
  }

  // Rename a fref target
  #[test]
  fn rename_fref_snapshot() {
    let (raw, offset) = cursor(
      r#"---
_type: Person
friend: fref("|alice.td")
---
"#,
    );
    let (analysis, uri) = setup(&raw);
    let edit =
      rename(&analysis, make_params(uri, &raw, offset, "bob")).expect("should produce edits");
    let snap = snapshot(&edit);

    assert!(snap.contains("EDIT"), "should have text edits:\n{}", snap);
    assert!(
      snap.contains("RENAME"),
      "should have file rename:\n{}",
      snap
    );
    assert!(
      snap.contains("bob"),
      "edits should reference new name:\n{}",
      snap
    );
  }

  // Edits are ordered: text edits before file renames
  #[test]
  fn rename_edits_before_renames() {
    let (raw, offset) = cursor(
      r#"---
_type: Person
friend: fref("|alice.td")
---
"#,
    );
    let (analysis, uri) = setup(&raw);
    let edit =
      rename(&analysis, make_params(uri, &raw, offset, "bob")).expect("should produce edits");

    if let Some(DocumentChanges::Operations(ops)) = &edit.document_changes {
      let mut seen_rename = false;
      for op in ops {
        match op {
          DocumentChangeOperation::Edit(_) => {
            assert!(!seen_rename, "text edit found after file rename");
          }
          DocumentChangeOperation::Op(ResourceOp::Rename(_)) => {
            seen_rename = true;
          }
          _ => {}
        }
      }
      assert!(seen_rename, "should have a file rename");
    }
  }

  // Simulates rename Person->Human, then Human->Person by rebuilding the analysis with the first rename's edits applied
  #[test]
  fn rename_ident_roundtrip() {
    // First rename: Person -> Human
    let (raw, offset) = cursor(
      r#"---
_type: |Person
name: Alice
---
"#,
    );
    let (analysis, uri) = setup(&raw);
    let edit = rename(&analysis, make_params(uri, &raw, offset, "Human")).expect("first rename");
    let snap = snapshot(&edit);
    assert!(snap.contains("\"Human\""), "first rename:\n{}", snap);

    // Apply the rename: rebuild with Human schema and updated content
    let human_content = raw.replace("Person", "Human");
    let root = PathBuf::from(if cfg!(windows) { "C:\\vault" } else { "/vault" });
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
    let schema_file = File::new(
      &db,
      FileHandle::Content(
        root.join("_types/Human.td"),
        SCHEMA_PERSON.to_string(),
        FileMetadata::default(),
      ),
    );
    let alice_file = File::new(
      &db,
      FileHandle::Content(
        root.join("alice.td"),
        CONTENT_ALICE.replace("Person", "Human"),
        FileMetadata::default(),
      ),
    );
    let test_file = File::new(
      &db,
      FileHandle::Content(
        root.join("file.td"),
        human_content.clone(),
        FileMetadata::default(),
      ),
    );
    let files = HashMap::from([
      (root.join("typedown.yaml"), config_file),
      (root.join("_types/Human.td"), schema_file),
      (root.join("alice.td"), alice_file),
      (root.join("file.td"), test_file),
    ]);
    let project = Project::new(&db, root.clone(), files);
    let analysis2 = Analysis::new(
      db,
      project,
      Arc::new(HashMap::new()),
      Arc::new(HashMap::new()),
      Arc::new((Mutex::new(1), Condvar::new())),
    );
    let uri2 = path_to_uri(&root.join("file.td"), "file");

    // Second rename: Human -> Person (cursor on the first "Human" which is the _type value)
    let (raw2, offset2) = cursor(&human_content.replacen("Human", "|Human", 1));
    let edit2 =
      rename(&analysis2, make_params(uri2, &raw2, offset2, "Person")).expect("second rename");
    let snap2 = snapshot(&edit2);
    assert!(snap2.contains("\"Person\""), "second rename:\n{}", snap2);
    assert!(
      snap2.contains("Person.td"),
      "should rename back to Person.td:\n{}",
      snap2
    );
  }

  // Verify the file rename operation contains correct old/new URIs
  #[test]
  fn rename_produces_correct_file_rename() {
    let (raw, offset) = cursor(
      r#"---
_type: |Person
name: Alice
---
"#,
    );
    let (analysis, uri) = setup(&raw);
    let edit =
      rename(&analysis, make_params(uri, &raw, offset, "Human")).expect("should produce edits");

    let rename_ops: Vec<_> = edit
      .document_changes
      .as_ref()
      .and_then(|dc| match dc {
        DocumentChanges::Operations(ops) => Some(ops),
        _ => None,
      })
      .unwrap()
      .iter()
      .filter_map(|op| match op {
        DocumentChangeOperation::Op(ResourceOp::Rename(r)) => Some(r),
        _ => None,
      })
      .collect();

    assert_eq!(rename_ops.len(), 1, "should have exactly 1 file rename");
    let rename_op = rename_ops[0];
    assert!(
      rename_op.old_uri.as_str().contains("Person.td"),
      "old URI should contain Person.td: {:?}",
      rename_op.old_uri.as_str()
    );
    assert!(
      rename_op.new_uri.as_str().contains("Human.td"),
      "new URI should contain Human.td: {:?}",
      rename_op.new_uri.as_str()
    );
  }

  // Cursor on a non-renameable position returns None
  #[test]
  fn rename_on_value_returns_none() {
    let (raw, offset) = cursor(
      r#"---
_type: Person
name: |Alice
---
"#,
    );
    let (analysis, uri) = setup(&raw);
    assert!(
      rename(&analysis, make_params(uri, &raw, offset, "Bob")).is_none(),
      "renaming a string value should return None"
    );
  }
}
