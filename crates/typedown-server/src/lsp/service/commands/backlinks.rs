use lsp_types::Location;
use serde::{Deserialize, Serialize};
use typedown_lang::db::derived::name_resolver::file_symbol::file_symbol;
use typedown_lang::db::derived::name_resolver::resolution_index::{ReferenceKind, references};
use typedown_lang::db::types::SymbolKind;
use typedown_lang::syntax::ast::AstNode;

use crate::core::analysis::Analysis;
use crate::core::utils::position::node_trimmed_range;
use crate::core::utils::uri::path_to_uri;
use crate::lsp::service::utils::symbol::get_string_content_node;

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BacklinksArgs {
  pub uri: String,
}

/// Find all files that reference the given file via fref, _type, or markdown links
pub fn execute(analysis: &Analysis, args: BacklinksArgs) -> Vec<Location> {
  let db = &analysis.db;
  let project = analysis.project;

  let target_uri: lsp_types::Uri = match args.uri.parse() {
    Ok(uri) => uri,
    Err(_) => return vec![],
  };

  let target_path = match crate::core::utils::uri::uri_to_path(&target_uri) {
    Some(path) => path,
    None => return vec![],
  };

  let target_file = match project.files(db).get(&target_path) {
    Some(file) => *file,
    None => return vec![],
  };

  let symbol = match file_symbol(db, project, target_file).value(db) {
    Some(symbol) => symbol,
    None => return vec![],
  };

  // Skip builtin symbols
  if matches!(
    symbol.kind(db),
    SymbolKind::BuiltinSchema(_) | SymbolKind::BuiltinMacro(_)
  ) {
    return vec![];
  }

  let refs = references(db, project, symbol);
  let mut locations: Vec<Location> = refs
    .into_iter()
    .filter_map(|reference| {
      let reference_path = reference.node.file.handle(db).path()?.clone();
      // Skip self-references
      if reference_path == target_path {
        return None;
      }
      let reference_rope = analysis.file_rope(&reference_path)?;

      let range = match reference.kind {
        ReferenceKind::Fref => {
          // Point to the fref path string content
          let call = typedown_lang::syntax::ast::CallExpr::cast(reference.node.node.clone())?;
          let first_arg = call.args().into_iter().next()?;
          let content = get_string_content_node(first_arg.syntax())?;
          node_trimmed_range(&reference_rope, &content)
        }
        _ => node_trimmed_range(&reference_rope, &reference.node.node),
      };

      let scheme = analysis
        .scheme_map
        .get(&reference_path)
        .map(String::as_str)
        .unwrap_or("file");
      Some(Location {
        uri: path_to_uri(&reference_path, scheme),
        range,
      })
    })
    .collect();

  // Deduplicate by file (keep first occurrence per file)
  locations.sort_by(|first, second| first.uri.as_str().cmp(second.uri.as_str()));
  locations.dedup_by(|second, first| first.uri == second.uri);

  locations
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::core::analysis::Analysis;
  use crate::core::utils::uri::path_to_uri;
  use std::collections::HashMap;
  use std::path::PathBuf;
  use std::sync::{Arc, Condvar, Mutex};
  use typedown_lang::db::types::{File, FileHandle, FileMetadata, Project};
  use typedown_lang::db::{QueryStorage, TypedownDatabase};

  const VAULT_CONFIG: &str = r#"version: "1"
vault:
  root_dir: "."
"#;
  const SCHEMA_NOTE: &str = r#"---
_type: schema
properties:
  title:
    type: string
---
"#;
  const SCHEMA_FLASHCARD: &str = r#"---
_type: schema
properties:
  source:
    type: Note
---
"#;

  fn root() -> PathBuf {
    PathBuf::from(if cfg!(windows) { "C:\\vault" } else { "/vault" })
  }

  fn setup(extra_files: Vec<(PathBuf, &str)>) -> Analysis {
    let root = root();
    let db = TypedownDatabase {
      storage: QueryStorage::default(),
    };
    let mut files = HashMap::from([
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
        root.join("_types/Note.td"),
        File::new(
          &db,
          FileHandle::Content(
            root.join("_types/Note.td"),
            SCHEMA_NOTE.to_string(),
            FileMetadata::default(),
          ),
        ),
      ),
      (
        root.join("_types/Flashcard.td"),
        File::new(
          &db,
          FileHandle::Content(
            root.join("_types/Flashcard.td"),
            SCHEMA_FLASHCARD.to_string(),
            FileMetadata::default(),
          ),
        ),
      ),
    ]);
    for (path, content) in extra_files {
      files.insert(
        path.clone(),
        File::new(
          &db,
          FileHandle::Content(path, content.to_string(), FileMetadata::default()),
        ),
      );
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

  fn backlinks(analysis: &Analysis, path: &PathBuf) -> Vec<Location> {
    let uri = path_to_uri(path, "file");
    execute(
      analysis,
      BacklinksArgs {
        uri: uri.to_string(),
      },
    )
  }

  fn uri_filenames(locations: &[Location]) -> Vec<String> {
    locations
      .iter()
      .map(|location| {
        location
          .uri
          .as_str()
          .rsplit('/')
          .next()
          .unwrap_or("")
          .to_string()
      })
      .collect()
  }

  #[test]
  fn fref_backlink_found() {
    let root = root();
    let note_path = root.join("notes/intro.td");
    let flashcard_path = root.join("flashcards/q1.td");
    let analysis = setup(vec![
      (
        note_path.clone(),
        r#"---
_type: Note
title: "Intro"
---

Introduction to the topic.
"#,
      ),
      (
        flashcard_path,
        r#"---
_type: Flashcard
source: fref("/notes/intro.td")
---
"#,
      ),
    ]);

    let locations = backlinks(&analysis, &note_path);
    let filenames = uri_filenames(&locations);

    assert_eq!(
      filenames,
      vec!["q1.td"],
      "flashcard should backlink to note"
    );
  }

  #[test]
  fn multiple_backlinks_sorted() {
    let root = root();
    let note_path = root.join("notes/intro.td");
    let analysis = setup(vec![
      (
        note_path.clone(),
        r#"---
_type: Note
title: "Intro"
---
"#,
      ),
      (
        root.join("flashcards/a.td"),
        r#"---
_type: Flashcard
source: fref("/notes/intro.td")
---
"#,
      ),
      (
        root.join("flashcards/b.td"),
        r#"---
_type: Flashcard
source: fref("/notes/intro.td")
---
"#,
      ),
    ]);

    let locations = backlinks(&analysis, &note_path);
    let filenames = uri_filenames(&locations);

    assert_eq!(
      filenames,
      vec!["a.td", "b.td"],
      "should find both flashcards, sorted"
    );
  }

  #[test]
  fn self_references_excluded() {
    let root = root();
    let note_path = root.join("notes/intro.td");
    let analysis = setup(vec![(
      note_path.clone(),
      r#"---
_type: Note
title: "Intro"
---
"#,
    )]);

    let locations = backlinks(&analysis, &note_path);

    assert!(locations.is_empty(), "self-references should be excluded");
  }

  #[test]
  fn no_backlinks_returns_empty() {
    let root = root();
    let note_path = root.join("notes/lonely.td");
    let analysis = setup(vec![(
      note_path.clone(),
      r#"---
_type: Note
title: "Lonely"
---
"#,
    )]);

    let locations = backlinks(&analysis, &note_path);

    assert!(
      locations.is_empty(),
      "file with no backlinks should return empty"
    );
  }

  #[test]
  fn deduplicated_by_file() {
    let root = root();
    let note_path = root.join("notes/intro.td");
    let analysis = setup(vec![
      (
        note_path.clone(),
        r#"---
_type: Note
title: "Intro"
---
"#,
      ),
      (
        root.join("flashcards/q1.td"),
        r#"---
_type: Flashcard
source: fref("/notes/intro.td")
---

See also: [intro](../notes/intro.td)
"#,
      ),
    ]);

    let locations = backlinks(&analysis, &note_path);
    let filenames = uri_filenames(&locations);

    assert_eq!(
      filenames,
      vec!["q1.td"],
      "multiple references from the same file should be deduplicated"
    );
  }

  #[test]
  fn nonexistent_file_returns_empty() {
    let root = root();
    let analysis = setup(vec![]);
    let fake_path = root.join("does-not-exist.td");

    let locations = backlinks(&analysis, &fake_path);

    assert!(locations.is_empty());
  }
}
