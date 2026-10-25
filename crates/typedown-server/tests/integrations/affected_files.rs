use std::path::Path;
use std::sync::OnceLock;

use typedown_incremental::InputId;
use typedown_lang::db::derived::name_resolver::resolution_index::find_transitive_referrers;
use typedown_lang::db::types::Project;
use typedown_lang::db::{TypedownDatabase, derived::get_vault_config::get_vault_config};
use typedown_types::path::normalize_path;

use super::utils::{example_vault, setup_db_fresh};

// All affected_files tests are read-only, so share a single DB setup
static SHARED_DB: OnceLock<TypedownDatabase> = OnceLock::new();

fn shared_db() -> &'static TypedownDatabase {
  SHARED_DB.get_or_init(|| {
    let source = example_vault();
    setup_db_fresh(&source)
  })
}

fn get_affected_paths(
  db: &TypedownDatabase,
  project: Project,
  changed_path: &Path,
  root_dir: &Path,
) -> Vec<String> {
  let changed_file = match project.files(db).get(changed_path) {
    Some(file) => *file,
    None => return vec![],
  };
  find_transitive_referrers(db, project, changed_file)
    .into_iter()
    .filter_map(|file| {
      let path = file.handle(db).path()?.clone();
      path.strip_prefix(root_dir).ok().map(normalize_path)
    })
    .collect()
}

// alice.td is referenced by several tasks, milestones, and projects
#[test]
fn affected_files_for_alice() {
  let db = shared_db();
  let project = Project::iter(db).into_iter().next().unwrap();
  let root_dir = get_vault_config(db, project).root_dir(db);
  let alice_path = root_dir.join("people/alice.td");

  let mut affected = get_affected_paths(db, project, &alice_path, &root_dir);
  affected.sort();

  assert!(
    affected.iter().any(|file| file.contains("implement-auth")),
    "implement-auth.td references alice: {affected:?}"
  );
  assert!(
    affected.iter().any(|file| file.contains("write-tests")),
    "write-tests.td references alice: {affected:?}"
  );
  assert!(
    affected
      .iter()
      .any(|file| file.contains("website-redesign")),
    "website-redesign.td references alice: {affected:?}"
  );
}

// A file with no incoming references returns empty
#[test]
fn affected_files_for_unreferenced_file() {
  let db = shared_db();
  let project = Project::iter(db).into_iter().next().unwrap();
  let root_dir = get_vault_config(db, project).root_dir(db);
  let index_path = root_dir.join("index.td");

  let affected = get_affected_paths(db, project, &index_path, &root_dir);
  assert!(
    affected.is_empty(),
    "index.td should have no referrers: {affected:?}"
  );
}

// Changing a schema type file propagates to content files using it or any subtype
// - Project.td is the supertype
// - MobileProject.td extends it
// - mobile-app.td uses MobileProject
#[test]
fn affected_files_for_schema_change() {
  let db = shared_db();
  let project = Project::iter(db).into_iter().next().unwrap();
  let root_dir = get_vault_config(db, project).root_dir(db);
  let project_schema_path = root_dir.join("_types/tracker/Project.td");

  let mut affected = get_affected_paths(db, project, &project_schema_path, &root_dir);
  affected.sort();

  assert!(
    affected
      .iter()
      .any(|file| file.contains("website-redesign")),
    "website-redesign.td uses Project directly: {affected:?}"
  );
  assert!(
    affected.iter().any(|file| file.contains("MobileProject")),
    "MobileProject.td extends Project: {affected:?}"
  );
  assert!(
    affected.iter().any(|file| file.contains("mobile-app")),
    "mobile-app.td uses MobileProject which extends Project: {affected:?}"
  );
}

// Transitive: if A references B and B references C, changing C affects both A and B
#[test]
fn affected_files_transitive() {
  let db = shared_db();
  let project = Project::iter(db).into_iter().next().unwrap();
  let root_dir = get_vault_config(db, project).root_dir(db);
  // alice references implement-auth; milestones reference alice
  // so changing implement-auth should transitively affect milestones via alice
  let auth_path = root_dir.join("tasks/implement-auth.td");

  let affected = get_affected_paths(db, project, &auth_path, &root_dir);

  assert!(
    affected.iter().any(|file| file.contains("alice")),
    "alice.td references implement-auth: {affected:?}"
  );
  // Transitive: milestones reference alice who references implement-auth
  assert!(
    affected
      .iter()
      .any(|file| file.contains("milestone") || file.contains("website-redesign")),
    "should have transitive referrers via alice: {affected:?}"
  );
}
