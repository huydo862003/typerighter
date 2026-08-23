use lsp_types::{PrepareRenameResponse, TextDocumentPositionParams};

use crate::core::analysis::Analysis;
use crate::core::utils::{position::lsp_position_to_text_offset, uri::uri_to_path};
use crate::lsp::service::utils::symbol::find_symbol_at_cursor;

pub fn prepare_rename(
  analysis: &Analysis,
  params: TextDocumentPositionParams,
) -> Option<PrepareRenameResponse> {
  let project = analysis.project;

  // Locate the file of the rename request
  let path = uri_to_path(&params.text_document.uri)?;
  let file = *project.files(&analysis.db).get(&path)?;
  let rope = analysis.file_rope(&path)?;

  // Locate the offset of the rename request
  let editor_pos = params.position;
  let offset = lsp_position_to_text_offset(&rope, editor_pos)?;

  // Find the symbol that is requested a rename + qualifying information
  let rename_symbol = find_symbol_at_cursor(&analysis.db, project, file, offset)?;

  Some(PrepareRenameResponse::Range(rename_symbol.get_range(&rope)))
}

#[cfg(test)]
mod tests {
  use std::collections::HashMap;
  use std::path::PathBuf;
  use std::sync::{Arc, Condvar, Mutex};

  use lsp_types::{
    Position, PrepareRenameResponse, TextDocumentIdentifier, TextDocumentPositionParams,
  };
  use ropey::Rope;
  use typedown_lang::db::types::{File, FileHandle, FileMetadata, Project};
  use typedown_lang::db::{QueryStorage, TypedownDatabase};

  use super::prepare_rename;
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
---
"#;
  const CONTENT_ALICE: &str = r#"---
_type: Person
name: Alice
---
"#;

  fn test_vault_root() -> PathBuf {
    PathBuf::from(if cfg!(windows) { "C:\\vault" } else { "/vault" })
  }

  fn cursor(content: &str) -> (String, usize) {
    let offset = content
      .find('|')
      .expect("content must have a cursor marker");
    (content.replacen('|', "", 1), offset)
  }

  fn make_params(uri: lsp_types::Uri, content: &str, offset: usize) -> TextDocumentPositionParams {
    let rope = Rope::from(content);
    let line = rope.char_to_line(offset);
    let character = offset - rope.line_to_char(line);
    TextDocumentPositionParams {
      text_document: TextDocumentIdentifier { uri },
      position: Position {
        line: line as u32,
        character: character as u32,
      },
    }
  }

  fn setup(content: &str) -> (Analysis, lsp_types::Uri) {
    let root = test_vault_root();
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

  // Ident in _type position returns exact identifier range
  #[test]
  fn prepare_rename_ident() {
    let (raw, offset) = cursor(
      r#"---
_type: |Person
name: Alice
---
"#,
    );
    let (analysis, uri) = setup(&raw);
    let result = prepare_rename(&analysis, make_params(uri, &raw, offset));
    let Some(PrepareRenameResponse::Range(range)) = result else {
      panic!("expected a range response");
    };
    // "Person" starts at line 1, col 7 and ends at col 13
    assert_eq!(
      range.start,
      Position {
        line: 1,
        character: 7
      }
    );
    assert_eq!(
      range.end,
      Position {
        line: 1,
        character: 13
      }
    );
  }

  // Fref returns the string content range (minus quotes)
  #[test]
  fn prepare_rename_fref_returns_content_range() {
    let (raw, offset) = cursor(
      r#"---
_type: Person
name: fref("|content/alice.td")
---
"#,
    );
    let (analysis, uri) = setup(&raw);
    let result = prepare_rename(&analysis, make_params(uri, &raw, offset));
    let Some(PrepareRenameResponse::Range(range)) = result else {
      panic!("expected a range response");
    };
    // Should cover "content/alice.td" (inside quotes), not the whole fref(...) call
    let rope = Rope::from(raw.as_str());
    let start_offset =
      rope.line_to_char(range.start.line as usize) + range.start.character as usize;
    let end_offset = rope.line_to_char(range.end.line as usize) + range.end.character as usize;
    let selected: String = rope.slice(start_offset..end_offset).into();
    assert_eq!(selected, "content/alice.td");
  }

  // Interpolated fref argument is not renameable
  #[test]
  fn prepare_rename_fref_interpolated_returns_none() {
    let (raw, offset) = cursor(
      r#"---
_type: Person
name: fref("|content/${name}.td")
---
"#,
    );
    let (analysis, uri) = setup(&raw);
    let result = prepare_rename(&analysis, make_params(uri, &raw, offset));
    assert!(
      result.is_none(),
      "interpolated fref should not be renameable"
    );
  }

  // Cursor outside any symbol returns None
  #[test]
  fn prepare_rename_outside_symbol_returns_none() {
    let (raw, offset) = cursor(
      r#"|---
_type: Person
---
"#,
    );
    let (analysis, uri) = setup(&raw);
    let result = prepare_rename(&analysis, make_params(uri, &raw, offset));
    assert!(result.is_none());
  }
}
