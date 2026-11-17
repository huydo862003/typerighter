use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

use typedown_incremental::InputId;
use typedown_lang::db::derived::get_vault_config::get_vault_config;
use typedown_lang::db::types::{File, FileHandle, FileMetadata, Project};
use typedown_lang::db::utils::{is_content_file, is_internal_file};
use typedown_lang::db::{QueryStorage, TypedownDatabase};
use typedown_lang::integrations::export::{export_resource_nav, export_ssg_flags};
use typedown_server::rpc::server::{
  collect_hidden_directories_in_sidebar, is_under_hidden_directory_in_sidebar,
};
use typedown_types::path::normalize_path;

use super::utils::example_vault;

static SHARED_DB: OnceLock<TypedownDatabase> = OnceLock::new();

fn shared_db() -> &'static TypedownDatabase {
  SHARED_DB.get_or_init(|| {
    let source = example_vault();
    super::utils::setup_db_fresh(&source)
  })
}

fn list_sidebar_paths(db: &TypedownDatabase) -> Vec<String> {
  let project = Project::iter(db)
    .into_iter()
    .next()
    .expect("project should exist");
  let config = get_vault_config(db, project);
  let root_dir = config.root_dir(db);
  let files = project.files(db);

  let mut paths: Vec<String> = files
    .iter()
    .filter(|(path, _)| {
      path.starts_with(&root_dir) && is_content_file(path) && !is_internal_file(path)
    })
    .filter(|(_, file)| export_ssg_flags(db, project, **file).show_in_sidebar)
    .filter_map(|(path, file)| {
      export_resource_nav(db, project, *file)?;
      Some(normalize_path(path.strip_prefix(&root_dir).unwrap_or(path)))
    })
    .collect();

  paths.sort();
  paths
}

#[test]
fn list_sidebar_excludes_hidden_files() {
  // Build a minimal in-memory vault with one visible and one hidden file
  let db = TypedownDatabase {
    storage: QueryStorage::default(),
  };

  let vault = PathBuf::from("/test_vault");
  let config_path = vault.join("typedown.yaml");
  let visible_path = vault.join("vault/visible.td");
  let hidden_path = vault.join("vault/hidden.td");

  let config_file = File::new(
    &db,
    FileHandle::Content(
      config_path.clone(),
      r#"version: "1.0.0"
vault:
  root_dir: "vault"
"#
      .to_string(),
      FileMetadata::default(),
    ),
  );

  let visible_file = File::new(
    &db,
    FileHandle::Content(
      visible_path.clone(),
      r#"---
_label: "Visible Page"
---

This page is visible.
"#
      .to_string(),
      FileMetadata::default(),
    ),
  );

  let hidden_file = File::new(
    &db,
    FileHandle::Content(
      hidden_path.clone(),
      r#"---
_label: "Hidden Page"
_ssg:
  show_in_sidebar: false
---

This page is hidden.
"#
      .to_string(),
      FileMetadata::default(),
    ),
  );

  let files: HashMap<PathBuf, File> = [
    (config_path.clone(), config_file),
    (visible_path.clone(), visible_file),
    (hidden_path.clone(), hidden_file),
  ]
  .into_iter()
  .collect();

  let project = Project::new(&db, vault.clone(), files, None);
  let config = get_vault_config(&db, project);
  let root_dir = config.root_dir(&db);
  let project_files = project.files(&db);

  // Visible file should not be hidden
  let visible = *project_files.get(&visible_path).unwrap();
  assert!(
    export_ssg_flags(&db, project, visible).show_in_sidebar,
    "visible.td should not be hidden"
  );

  // Hidden file should be detected as hidden
  let hidden = *project_files.get(&hidden_path).unwrap();
  assert!(
    !export_ssg_flags(&db, project, hidden).show_in_sidebar,
    "hidden.td with _ssg.show_in_sidebar: false should be hidden"
  );

  // Sidebar listing should contain visible but not hidden
  let sidebar_paths: Vec<String> = project_files
    .iter()
    .filter(|(path, _)| {
      path.starts_with(&root_dir) && is_content_file(path) && !is_internal_file(path)
    })
    .filter(|(_, file)| export_ssg_flags(&db, project, **file).show_in_sidebar)
    .filter_map(|(path, file)| {
      export_resource_nav(&db, project, *file)?;
      Some(normalize_path(path.strip_prefix(&root_dir).unwrap_or(path)))
    })
    .collect();

  assert!(
    sidebar_paths.iter().any(|path| path.contains("visible")),
    "visible.td should appear in sidebar: {sidebar_paths:?}"
  );
  assert!(
    !sidebar_paths.iter().any(|path| path.contains("hidden")),
    "hidden.td should not appear in sidebar: {sidebar_paths:?}"
  );
}

#[test]
fn list_sidebar_includes_pages_without_ssg() {
  let db = shared_db();
  let paths = list_sidebar_paths(db);
  assert!(!paths.is_empty(), "sidebar should have entries");
}

// Build an in-memory vault and return vault-relative sidebar paths using the same two-pass filtering logic as list_sidebar in server.rs
fn get_sidebar_paths_for_vault(
  db: &TypedownDatabase,
  vault: &Path,
  files: HashMap<PathBuf, File>,
) -> Vec<String> {
  let project = Project::new(db, vault.into(), files, None);
  let config = get_vault_config(db, project);
  let root_dir = config.root_dir(db);
  let project_files = project.files(db);

  let entries: Vec<_> = project_files
    .iter()
    .filter(|(path, _)| {
      path.starts_with(&root_dir) && is_content_file(path) && !is_internal_file(path)
    })
    .map(|(path, file)| {
      (
        normalize_path(path.strip_prefix(&root_dir).unwrap_or(path)),
        *file,
      )
    })
    .collect();

  let hidden_directories = collect_hidden_directories_in_sidebar(db, project, &entries);

  let mut paths: Vec<String> = entries
    .into_iter()
    .filter(|(relative, file)| {
      export_ssg_flags(db, project, *file).show_in_sidebar
        && !is_under_hidden_directory_in_sidebar(relative, &hidden_directories)
    })
    .filter_map(|(relative, file)| {
      export_resource_nav(db, project, file)?;
      Some(relative)
    })
    .collect();

  paths.sort();
  paths
}

fn make_vault(
  db: &TypedownDatabase,
  vault: &Path,
  td_files: &[(&str, &str)],
) -> HashMap<PathBuf, File> {
  let config_path = vault.join("typedown.yaml");
  let mut files: HashMap<PathBuf, File> = HashMap::new();

  files.insert(
    config_path.clone(),
    File::new(
      db,
      FileHandle::Content(
        config_path,
        r#"version: "1.0.0"
vault:
  root_dir: "vault"
"#
        .to_string(),
        FileMetadata::default(),
      ),
    ),
  );

  for (rel, body) in td_files {
    let path = vault.join("vault").join(rel);
    files.insert(
      path.clone(),
      File::new(
        db,
        FileHandle::Content(path, body.to_string(), FileMetadata::default()),
      ),
    );
  }

  files
}

#[test]
fn hidden_index_hides_entire_subtree() {
  // people/index.td is hidden, so people/alice.td and people/bob.td should not appear
  let db = TypedownDatabase {
    storage: QueryStorage::default(),
  };
  let vault = PathBuf::from("/vault_hidden_index");
  let files = make_vault(
    &db,
    &vault,
    &[
      (
        "root.td",
        r#"---
_label: "Root"
---
"#,
      ),
      (
        "people/index.td",
        r#"---
_label: "People"
_ssg:
  show_in_sidebar: false
---
"#,
      ),
      (
        "people/alice.td",
        r#"---
_label: "Alice"
---
"#,
      ),
      (
        "people/bob.td",
        r#"---
_label: "Bob"
---
"#,
      ),
    ],
  );
  let paths = get_sidebar_paths_for_vault(&db, &vault, files);

  assert!(
    paths.iter().any(|path| path.contains("root")),
    "root.td should be visible: {paths:?}"
  );
  assert!(
    !paths.iter().any(|path| path.contains("people")),
    "all people/* should be hidden when index is hidden: {paths:?}"
  );
}

#[test]
fn hidden_index_hides_nested_subdirs() {
  // people/index.td hidden, people/authors/hemingway.td also hidden transitively
  let db = TypedownDatabase {
    storage: QueryStorage::default(),
  };
  let vault = PathBuf::from("/vault_nested_hidden");
  let files = make_vault(
    &db,
    &vault,
    &[
      (
        "root.td",
        r#"---
_label: "Root"
---
"#,
      ),
      (
        "people/index.td",
        r#"---
_ssg:
  show_in_sidebar: false
---
"#,
      ),
      (
        "people/alice.td",
        r#"---
_label: "Alice"
---
"#,
      ),
      (
        "people/authors/hemingway.td",
        r#"---
_label: "Hemingway"
---
"#,
      ),
      (
        "notes/note.td",
        r#"---
_label: "Note"
---
"#,
      ),
    ],
  );
  let paths = get_sidebar_paths_for_vault(&db, &vault, files);

  assert!(
    !paths.iter().any(|path| path.starts_with("people")),
    "all people/* should be hidden transitively: {paths:?}"
  );
  assert!(
    paths.iter().any(|path| path.contains("notes")),
    "notes should still be visible: {paths:?}"
  );
}

#[test]
fn nested_dir_hidden_while_parent_visible() {
  // people/index.td visible, people/internal/index.td hidden
  // people/alice.td visible; people/internal/secret.td hidden
  let db = TypedownDatabase {
    storage: QueryStorage::default(),
  };
  let vault = PathBuf::from("/vault_nested_partial");
  let files = make_vault(
    &db,
    &vault,
    &[
      (
        "people/index.td",
        r#"---
_label: "People"
---
"#,
      ),
      (
        "people/alice.td",
        r#"---
_label: "Alice"
---
"#,
      ),
      (
        "people/internal/index.td",
        r#"---
_ssg:
  show_in_sidebar: false
---
"#,
      ),
      (
        "people/internal/secret.td",
        r#"---
_label: "Secret"
---
"#,
      ),
    ],
  );
  let paths = get_sidebar_paths_for_vault(&db, &vault, files);

  assert!(
    paths.iter().any(|path| path.contains("alice")),
    "alice should be visible: {paths:?}"
  );
  assert!(
    paths.iter().any(|path| path == "people/index.td"),
    "people/index.td should be visible: {paths:?}"
  );
  assert!(
    !paths.iter().any(|path| path.contains("internal")),
    "people/internal/* should be hidden: {paths:?}"
  );
}

#[test]
fn no_ssg_field_does_not_hide() {
  // No _ssg field - all files should be visible
  let db = TypedownDatabase {
    storage: QueryStorage::default(),
  };
  let vault = PathBuf::from("/vault_no_ssg");
  let files = make_vault(
    &db,
    &vault,
    &[
      (
        "index.td",
        r#"---
_label: "Home"
---
"#,
      ),
      (
        "people/index.td",
        r#"---
_label: "People"
---
"#,
      ),
      (
        "people/alice.td",
        r#"---
_label: "Alice"
---
"#,
      ),
    ],
  );
  let paths = get_sidebar_paths_for_vault(&db, &vault, files);

  assert_eq!(paths.len(), 3, "all files should appear: {paths:?}");
}

#[test]
fn ssg_show_in_sidebar_true_does_not_hide() {
  // Explicit show_in_sidebar: true - all files visible
  let db = TypedownDatabase {
    storage: QueryStorage::default(),
  };
  let vault = PathBuf::from("/vault_ssg_true");
  let files = make_vault(
    &db,
    &vault,
    &[
      (
        "people/index.td",
        r#"---
_ssg:
  show_in_sidebar: true
---
"#,
      ),
      (
        "people/alice.td",
        r#"---
_label: "Alice"
---
"#,
      ),
    ],
  );
  let paths = get_sidebar_paths_for_vault(&db, &vault, files);

  assert!(
    paths.iter().any(|path| path.contains("alice")),
    "alice should be visible: {paths:?}"
  );
  assert!(
    paths.iter().any(|path| path.contains("index")),
    "index should be visible: {paths:?}"
  );
}
