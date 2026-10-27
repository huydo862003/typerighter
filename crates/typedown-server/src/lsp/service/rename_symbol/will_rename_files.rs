use lsp_types::{RenameFilesParams, WorkspaceEdit};
use typedown_lang::db::derived::get_vault_config::get_vault_config;
use typedown_lang::db::derived::name_resolver::file_symbol::file_symbol;
use typedown_lang::db::derived::name_resolver::resolution_index::references;
use typedown_lang::db::types::SymbolKind;
use typedown_lang::db::utils::is_typ_file;

use crate::core::analysis::Analysis;
use crate::core::utils::uri::uri_to_path;
use crate::lsp::service::rename_symbol::utils::{build_workspace_edit, collect_reference_edits};

use std::collections::BTreeMap;
use std::path::PathBuf;

use lsp_types::TextEdit;

/// Handle workspace/willRenameFiles: update references when files are renamed via explorer
pub fn will_rename_files(analysis: &Analysis, params: RenameFilesParams) -> Option<WorkspaceEdit> {
  let db = &analysis.db;
  let project = analysis.project;
  let root_dir = get_vault_config(db, project).root_dir(db);
  let mut all_edits: BTreeMap<PathBuf, Vec<TextEdit>> = BTreeMap::new();

  for file_rename in &params.files {
    let Some(old_uri) = file_rename.old_uri.parse::<lsp_types::Uri>().ok() else {
      continue;
    };
    let Some(new_uri) = file_rename.new_uri.parse::<lsp_types::Uri>().ok() else {
      continue;
    };
    let Some(old_path) = uri_to_path(&old_uri) else {
      continue;
    };
    let Some(new_path) = uri_to_path(&new_uri) else {
      continue;
    };

    let Some(file) = project.files(db).get(&old_path).copied() else {
      continue;
    };
    let Some(symbol) = file_symbol(db, project, file).value(db) else {
      continue;
    };

    // Schema rename: ensure new path stays inside a _types directory
    if matches!(symbol.kind(db), SymbolKind::UserDefinedSchema(_, _)) && !is_typ_file(&new_path) {
      continue;
    }

    let new_stem = new_path
      .file_stem()
      .and_then(|stem| stem.to_str())
      .unwrap_or_default();

    let references = references(db, project, symbol);
    let edits = collect_reference_edits(analysis, &references, new_stem, &new_path, &root_dir);
    for (path, file_edits) in edits {
      all_edits.entry(path).or_default().extend(file_edits);
    }
  }

  build_workspace_edit(analysis, all_edits, vec![])
}

#[cfg(test)]
mod tests {
  use super::will_rename_files;
  use crate::core::analysis::Analysis;
  use crate::core::utils::uri::path_to_uri;
  use lsp_types::{DocumentChangeOperation, DocumentChanges, FileRename, RenameFilesParams};
  use std::collections::HashMap;
  use std::path::PathBuf;
  use std::sync::{Arc, Condvar, Mutex};
  use typedown_lang::db::types::{File, FileHandle, FileMetadata, Project};
  use typedown_lang::db::{QueryStorage, TypedownDatabase};

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
  const CONTENT_WITH_FREF: &str = r#"---
_type: Person
friend: fref("alice.td")
---
"#;
  const CONTENT_WITH_ASSET_FREF: &str = r#"---
_type: Person
name: Alice
avatar: fref("_assets/icon.svg")
---
"#;
  const CONTENT_WITH_INLINE_ASSET_FREF: &str = r#"---
_type: Person
name: Alice
avatar: fref("icon.svg")
---
"#;

  fn setup(editing_content: &str) -> Analysis {
    setup_with_extra_files(editing_content, vec![])
  }

  fn setup_with_extra_files(editing_content: &str, extra_files: Vec<(PathBuf, &str)>) -> Analysis {
    let root = PathBuf::from(if cfg!(windows) { "C:\\vault" } else { "/vault" });
    let typ_root = root.join("_types");

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
        typ_root.join("Person.td"),
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
        root.join("file.td"),
        editing_content.to_string(),
        FileMetadata::default(),
      ),
    );

    let mut files = HashMap::from([
      (root.join("typedown.yaml"), config_file),
      (root.join("_types/Person.td"), person_file),
      (root.join("alice.td"), alice_file),
      (root.join("file.td"), editing_file),
    ]);

    for (path, content) in extra_files {
      let file = File::new(
        &db,
        FileHandle::Content(path.clone(), content.to_string(), FileMetadata::default()),
      );
      files.insert(path, file);
    }

    let project = Project::new(&db, root, files, None);
    Analysis::new(
      db,
      project,
      Arc::new(HashMap::new()),
      Arc::new(HashMap::new()),
      Arc::new((Mutex::new(1), Condvar::new())),
    )
  }

  fn snapshot(edit: &lsp_types::WorkspaceEdit) -> String {
    let mut lines = vec![];
    if let Some(DocumentChanges::Operations(ops)) = &edit.document_changes {
      for op in ops {
        if let DocumentChangeOperation::Edit(doc_edit) = op {
          let uri = doc_edit.text_document.uri.as_str();
          let short = uri.rfind("/vault/").map_or(uri, |index| &uri[index..]);
          for edit in &doc_edit.edits {
            if let lsp_types::OneOf::Left(text_edit) = edit {
              let range = &text_edit.range;
              lines.push(format!(
                "EDIT {} [{}:{}-{}:{}] -> {:?}",
                short,
                range.start.line,
                range.start.character,
                range.end.line,
                range.end.character,
                text_edit.new_text
              ));
            }
          }
        }
      }
    }
    lines.sort();
    lines.join(
      r"
",
    )
  }

  fn root() -> PathBuf {
    PathBuf::from(if cfg!(windows) { "C:\\vault" } else { "/vault" })
  }

  fn make_params(old_relative: &str, new_relative: &str) -> RenameFilesParams {
    let root = root();
    RenameFilesParams {
      files: vec![FileRename {
        old_uri: path_to_uri(&root.join(old_relative), "file").to_string(),
        new_uri: path_to_uri(&root.join(new_relative), "file").to_string(),
      }],
    }
  }

  // Renaming a schema file updates _type references
  #[test]
  fn will_rename_schema_updates_type_references() {
    let analysis = setup(CONTENT_ALICE);
    let params = make_params("_types/Person.td", "_types/Human.td");
    let edit = will_rename_files(&analysis, params).expect("should produce edits");
    let snapshot = snapshot(&edit);

    assert!(
      snapshot.contains(r#""Human""#),
      r"should rename idents to Human:
{}",
      snapshot
    );
    assert_eq!(
      snapshot.matches("EDIT").count(),
      2,
      r"should edit 2 files:
{}",
      snapshot
    );
  }

  // Renaming a content file updates fref references
  #[test]
  fn will_rename_content_updates_fref() {
    let analysis = setup(CONTENT_WITH_FREF);
    let params = make_params("alice.td", "bob.td");
    let edit = will_rename_files(&analysis, params).expect("should produce edits");
    let snap = snapshot(&edit);

    assert!(
      snap.contains("bob"),
      r"should update fref to bob:
{}",
      snap
    );
  }

  // Renaming a content file to a nested directory updates fref references with full path
  #[test]
  fn will_rename_content_to_nested_dir_updates_fref() {
    let analysis = setup(CONTENT_WITH_FREF);
    let params = make_params("alice.td", "people/alice.td");
    let edit = will_rename_files(&analysis, params).expect("should produce edits");
    let snap = snapshot(&edit);

    assert!(
      snap.contains("people/alice.td"),
      r"should update fref to people/alice.td:
{}",
      snap
    );
  }

  // Renaming schema to outside _types directory produces no edits
  #[test]
  fn will_rename_schema_outside_types_dir_skips_edits() {
    let analysis = setup(CONTENT_ALICE);
    let params = make_params("_types/Person.td", "Person.td");
    let result = will_rename_files(&analysis, params);

    assert!(
      result.is_none(),
      "should produce no edits for schema renamed outside _types directory"
    );
  }

  // Renaming schema to nested dir updates type references if name changes
  #[test]
  fn will_rename_schema_to_nested_dir_updates_type_references() {
    let analysis = setup(CONTENT_ALICE);
    let params = make_params("_types/Person.td", "_types/nested/Human.td");
    let edit = will_rename_files(&analysis, params).expect("should produce edits");
    let snapshot = snapshot(&edit);

    assert!(
      snapshot.contains(r#""Human""#),
      r"should rename idents to Human in nested dir:
{}",
      snapshot
    );
  }

  // Renaming an asset in _assets/ updates fref references pointing to it
  #[test]
  fn will_rename_asset_in_assets_dir_updates_fref() {
    let root = root();
    let asset_path = root.join("_assets/icon.svg");
    let analysis =
      setup_with_extra_files(CONTENT_WITH_ASSET_FREF, vec![(asset_path, "<svg></svg>")]);
    let params = make_params("_assets/icon.svg", "_assets/logo.svg");
    let edit = will_rename_files(&analysis, params).expect("should produce edits");
    let snapshot = snapshot(&edit);

    assert!(
      snapshot.contains("logo"),
      r"should update fref to logo:
{}",
      snapshot
    );
  }

  // Renaming an asset outside _assets/ also updates fref references
  #[test]
  fn will_rename_asset_outside_assets_dir_updates_fref() {
    let root = root();
    let asset_path = root.join("icon.svg");
    let analysis = setup_with_extra_files(
      CONTENT_WITH_INLINE_ASSET_FREF,
      vec![(asset_path, "<svg></svg>")],
    );
    let params = make_params("icon.svg", "logo.svg");
    let edit = will_rename_files(&analysis, params).expect("should produce edits");
    let snapshot = snapshot(&edit);

    assert!(
      snapshot.contains("logo"),
      r"should update fref to logo:
{}",
      snapshot
    );
  }

  // Renaming a file not in the project returns None
  #[test]
  fn will_rename_unknown_file_returns_none() {
    let analysis = setup(CONTENT_ALICE);
    let params = make_params("unknown.td", "other.td");
    let result = will_rename_files(&analysis, params);

    assert!(result.is_none(), "unknown file should return None");
  }
}
