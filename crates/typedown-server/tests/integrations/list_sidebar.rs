use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::OnceLock;

use typedown_incremental::InputId;
use typedown_lang::db::derived::get_vault_config::get_vault_config;
use typedown_lang::db::types::{File, FileHandle, FileMetadata, Project};
use typedown_lang::db::utils::{is_content_file, is_internal_file};
use typedown_lang::db::{QueryStorage, TypedownDatabase};
use typedown_lang::integrations::export::{export_resource_nav, is_hidden_from_sidebar};
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
    .filter(|(_, file)| !is_hidden_from_sidebar(db, project, **file))
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

  let meta = || FileMetadata::default();

  let config_file = File::new(
    &db,
    FileHandle::Content(
      config_path.clone(),
      r#"version: "1.0.0"
vault:
  root_dir: "vault"
"#
      .to_string(),
      meta(),
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
      meta(),
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
      meta(),
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
    !is_hidden_from_sidebar(&db, project, visible),
    "visible.td should not be hidden"
  );

  // Hidden file should be detected as hidden
  let hidden = *project_files.get(&hidden_path).unwrap();
  assert!(
    is_hidden_from_sidebar(&db, project, hidden),
    "hidden.td with _ssg.show_in_sidebar: false should be hidden"
  );

  // Sidebar listing should contain visible but not hidden
  let sidebar_paths: Vec<String> = project_files
    .iter()
    .filter(|(path, _)| {
      path.starts_with(&root_dir) && is_content_file(path) && !is_internal_file(path)
    })
    .filter(|(_, file)| !is_hidden_from_sidebar(&db, project, **file))
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
