use tempfile::TempDir;
use typedown_server::core::multiproject::Multiproject;

use super::utils::{copy_dir_recursive, example_vault};

// Create a temp directory with two independent vault projects
fn setup_two_projects() -> TempDir {
  let tmp = TempDir::new().unwrap();

  // Project A
  let project_a = tmp.path().join("project_a");
  copy_dir_recursive(&example_vault(), &project_a);

  // Project B: a minimal vault
  let project_b = tmp.path().join("project_b");
  std::fs::create_dir_all(project_b.join("content")).unwrap();
  std::fs::create_dir_all(project_b.join("schemas")).unwrap();
  std::fs::write(
    project_b.join("typedown.yaml"),
    r#"version: "1.0.0"
vault:
  root_dir: content
"#,
  )
  .unwrap();
  std::fs::write(
    project_b.join("schemas/Note.td"),
    r#"---
_type: schema
properties:
  title:
    type: string
---
"#,
  )
  .unwrap();
  std::fs::write(
    project_b.join("content/hello.td"),
    r#"---
_type: Note
title: "Hello"
---
"#,
  )
  .unwrap();

  tmp
}

// load_nearest_project finds the vault root by walking up from a nested path
#[test]
fn load_nearest_project_from_nested_dir() {
  let tmp = TempDir::new().unwrap();
  copy_dir_recursive(&example_vault(), &tmp.path().join("vault"));

  let multiproject = Multiproject::default();
  let nested = tmp.path().join("vault/tasks");

  let entry = multiproject
    .load_nearest_project(&nested)
    .expect("should find project root from nested dir");

  assert_eq!(entry.root_dir, tmp.path().join("vault"));
}

// Calling load_nearest_project twice for the same vault returns the same entry
#[test]
fn load_nearest_project_returns_cached_entry() {
  let tmp = TempDir::new().unwrap();
  copy_dir_recursive(&example_vault(), &tmp.path().join("vault"));

  let multiproject = Multiproject::default();
  let root = tmp.path().join("vault");

  let first = multiproject
    .load_nearest_project(&root)
    .expect("first load should succeed");
  let second = multiproject
    .load_nearest_project(&root)
    .expect("second load should succeed");

  // Same Arc means same project entry, not a duplicate load
  assert!(std::sync::Arc::ptr_eq(&first, &second));
}

// Two different vaults get separate project entries
#[test]
fn separate_projects_for_different_vaults() {
  let tmp = setup_two_projects();
  let multiproject = Multiproject::default();

  let entry_a = multiproject
    .load_nearest_project(&tmp.path().join("project_a"))
    .expect("project A should load");
  let entry_b = multiproject
    .load_nearest_project(&tmp.path().join("project_b"))
    .expect("project B should load");

  assert_ne!(entry_a.root_dir, entry_b.root_dir);
  assert!(!std::sync::Arc::ptr_eq(&entry_a, &entry_b));
}

// Nested paths within different vaults route to the correct project
#[test]
fn nested_paths_route_to_correct_project() {
  let tmp = setup_two_projects();
  let multiproject = Multiproject::default();

  let entry_from_a = multiproject
    .load_nearest_project(&tmp.path().join("project_a/content/tasks"))
    .expect("should find project A");
  let entry_from_b = multiproject
    .load_nearest_project(&tmp.path().join("project_b/content"))
    .expect("should find project B");

  assert_eq!(entry_from_a.root_dir, tmp.path().join("project_a"));
  assert_eq!(entry_from_b.root_dir, tmp.path().join("project_b"));
}

// load_nearest_project errors when no typedown.yaml exists in any ancestor
#[test]
fn load_nearest_project_errors_without_config() {
  let tmp = TempDir::new().unwrap();
  std::fs::create_dir_all(tmp.path().join("no_vault/deep/nested")).unwrap();

  let multiproject = Multiproject::default();
  let result = multiproject.load_nearest_project(&tmp.path().join("no_vault/deep/nested"));

  assert!(result.is_err());
  let err_msg = result.unwrap_err().to_string();
  assert!(
    err_msg.contains("No typedown.yaml found"),
    "unexpected error: {err_msg}"
  );
}

// The projects() iterator lists all loaded projects
#[test]
fn projects_iterator_lists_loaded() {
  let tmp = setup_two_projects();
  let multiproject = Multiproject::default();

  multiproject
    .load_nearest_project(&tmp.path().join("project_a"))
    .unwrap();
  multiproject
    .load_nearest_project(&tmp.path().join("project_b"))
    .unwrap();

  let roots: Vec<_> = multiproject
    .projects()
    .map(|e| e.root_dir.clone())
    .collect();
  assert_eq!(roots.len(), 2);
  assert!(roots.contains(&tmp.path().join("project_a")));
  assert!(roots.contains(&tmp.path().join("project_b")));
}

// save() consumes the multiproject and does not panic
#[test]
fn save_does_not_panic() {
  let tmp = TempDir::new().unwrap();
  copy_dir_recursive(&example_vault(), &tmp.path().join("vault"));

  let multiproject = Multiproject::default();
  multiproject
    .load_nearest_project(&tmp.path().join("vault"))
    .unwrap();

  // Should not panic
  multiproject.save();
}
