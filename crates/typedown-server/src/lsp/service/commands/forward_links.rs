use std::collections::HashMap;
use std::path::PathBuf;

use lsp_types::{Location, Range};
use serde::{Deserialize, Serialize};
use typedown_lang::db::derived::name_resolver::file_symbol::file_symbol;
use typedown_lang::db::derived::name_resolver::resolution_index::resolution_index;
use typedown_lang::db::types::SymbolKind;

use crate::core::analysis::Analysis;
use crate::core::utils::uri::{path_to_uri, uri_to_path};

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ForwardLinksArgs {
  pub uri: String,
}

/// Find all files that the given file references via fref, _type, or markdown links
pub fn execute(analysis: &Analysis, args: ForwardLinksArgs) -> Vec<Location> {
  let db = &analysis.db;
  let project = analysis.project;

  let source_uri: lsp_types::Uri = match args.uri.parse() {
    Ok(uri) => uri,
    Err(_) => return vec![],
  };

  let source_path = match uri_to_path(&source_uri) {
    Some(path) => path,
    None => return vec![],
  };

  let source_file = match project.files(db).get(&source_path) {
    Some(file) => *file,
    None => return vec![],
  };

  let def_id_to_path: HashMap<String, PathBuf> = project
    .files(db)
    .iter()
    .filter_map(|(path, file)| {
      let symbol = file_symbol(db, project, *file).value(db)?;
      if matches!(
        symbol.kind(db),
        SymbolKind::BuiltinSchema(_) | SymbolKind::BuiltinMacro(_)
      ) {
        return None;
      }
      Some((symbol.definition_id(db).to_string(), path.clone()))
    })
    .collect();

  let index = resolution_index(db, project, source_file);
  let mut locations: Vec<Location> = index
    .definition_ids(db)
    .into_iter()
    .filter_map(|def_id| {
      let target_path = def_id_to_path.get(&def_id)?;
      if *target_path == source_path {
        return None;
      }
      let scheme = analysis
        .scheme_map
        .get(target_path)
        .map(String::as_str)
        .unwrap_or("file");
      Some(Location {
        uri: path_to_uri(target_path, scheme),
        range: Range::default(),
      })
    })
    .collect();

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
  use std::path::{Path, PathBuf};
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

  const SCHEMA_CARD: &str = r#"---
_type: schema
properties:
  source:
    type: Note?
---
"#;

  fn setup_vault(extra_files: Vec<(&str, &str)>) -> (Analysis, PathBuf) {
    let root = PathBuf::from(if cfg!(windows) { "C:\\vault" } else { "/vault" });
    let db = TypedownDatabase {
      storage: QueryStorage::default(),
    };

    let mut files = HashMap::new();
    let config_path = root.join("typedown.yaml");
    files.insert(
      config_path.clone(),
      File::new(
        &db,
        FileHandle::Content(
          config_path,
          VAULT_CONFIG.to_string(),
          FileMetadata::default(),
        ),
      ),
    );
    let note_schema_path = root.join("_types/Note.td");
    files.insert(
      note_schema_path.clone(),
      File::new(
        &db,
        FileHandle::Content(
          note_schema_path,
          SCHEMA_NOTE.to_string(),
          FileMetadata::default(),
        ),
      ),
    );
    let card_schema_path = root.join("_types/Card.td");
    files.insert(
      card_schema_path.clone(),
      File::new(
        &db,
        FileHandle::Content(
          card_schema_path,
          SCHEMA_CARD.to_string(),
          FileMetadata::default(),
        ),
      ),
    );

    for (rel, content) in extra_files {
      let path = root.join(rel);
      files.insert(
        path.clone(),
        File::new(
          &db,
          FileHandle::Content(path, content.to_string(), FileMetadata::default()),
        ),
      );
    }

    let project = Project::new(&db, root.clone(), files, None);
    let analysis = Analysis::new(
      db,
      project,
      Arc::new(HashMap::new()),
      Arc::new(HashMap::new()),
      Arc::new((Mutex::new(1), Condvar::new())),
    );
    (analysis, root)
  }

  fn forward_links(analysis: &Analysis, path: &Path) -> Vec<Location> {
    let uri = path_to_uri(path, "file");
    execute(
      analysis,
      ForwardLinksArgs {
        uri: uri.as_str().to_string(),
      },
    )
  }

  #[test]
  fn fref_forward_link_found() {
    let (analysis, root) = setup_vault(vec![
      ("note.td", "---\n_type: Note\ntitle: \"Rust\"\n---\n"),
      (
        "card.td",
        "---\n_type: Card\nsource: fref(\"/note.td\")\n---\n",
      ),
    ]);

    let card_path = root.join("card.td");
    let note_uri = path_to_uri(&root.join("note.td"), "file");
    let locations = forward_links(&analysis, &card_path);
    assert!(
      locations.iter().any(|loc| loc.uri == note_uri),
      "card should forward-link to note.td, got: {:?}",
      locations
    );
  }

  #[test]
  fn multiple_forward_links_sorted() {
    let (analysis, root) = setup_vault(vec![
      ("a.td", "---\n_type: Note\ntitle: \"A\"\n---\n"),
      ("b.td", "---\n_type: Note\ntitle: \"B\"\n---\n"),
      (
        "index.td",
        "---\n_type: Note\ntitle: \"Index\"\n---\n\nSee ${fref(\"/b.td\")} and ${fref(\"/a.td\")}.\n",
      ),
    ]);

    let index_path = root.join("index.td");
    let locations = forward_links(&analysis, &index_path);
    let uris: Vec<&str> = locations.iter().map(|loc| loc.uri.as_str()).collect();
    assert_eq!(uris.len(), 3, "expected Note.td, a.td, b.td: {uris:?}");
    let mut sorted = uris.clone();
    sorted.sort();
    assert_eq!(uris, sorted, "forward links should be sorted by URI");
  }

  #[test]
  fn self_references_excluded() {
    let (analysis, root) = setup_vault(vec![(
      "note.td",
      "---\n_type: Note\ntitle: \"Self\"\n---\n\n${fref(\"note.td\")}\n",
    )]);

    let note_path = root.join("note.td");
    let note_uri = path_to_uri(&note_path, "file");
    let locations = forward_links(&analysis, &note_path);
    assert!(
      !locations.iter().any(|loc| loc.uri == note_uri),
      "self-reference should be excluded"
    );
  }

  #[test]
  fn no_forward_links_returns_empty() {
    let (analysis, root) = setup_vault(vec![("plain.td", "Just plain markdown.\n")]);

    let plain_path = root.join("plain.td");
    let locations = forward_links(&analysis, &plain_path);
    assert!(
      locations.is_empty(),
      "file with no forward links should return empty"
    );
  }

  #[test]
  fn deduplicated_by_file() {
    let (analysis, root) = setup_vault(vec![
      ("note.td", "---\n_type: Note\ntitle: \"Note\"\n---\n"),
      (
        "card.td",
        "---\n_type: Card\nsource: fref(\"/note.td\")\n---\n\nSee ${fref(\"/note.td\")}\n",
      ),
    ]);

    let card_path = root.join("card.td");
    let note_uri = path_to_uri(&root.join("note.td"), "file");
    let locations = forward_links(&analysis, &card_path);
    let note_count = locations.iter().filter(|loc| loc.uri == note_uri).count();
    assert_eq!(note_count, 1, "multiple refs to same file should dedup");
  }

  #[test]
  fn nonexistent_file_returns_empty() {
    let (analysis, root) = setup_vault(vec![]);
    let fake_path = root.join("does_not_exist.td");
    let locations = forward_links(&analysis, &fake_path);
    assert!(locations.is_empty());
  }
}
