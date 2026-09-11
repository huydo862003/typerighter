use std::collections::HashSet;

use typedown_incremental::InputId;
use typedown_lang::db::derived::get_vault_config::get_vault_config;
use typedown_lang::db::derived::name_resolver::file_symbol::file_symbol;
use typedown_lang::db::derived::name_resolver::resolution_index::references;
use typedown_lang::db::types::Project;
use typedown_types::path::normalize_path;

use super::utils::{example_vault, setup_db_fresh};

// Collect vault-relative paths of files that transitively reference the given file
fn collect_affected(
  db: &typedown_lang::db::TypedownDatabase,
  project: Project,
  changed_path: &std::path::Path,
  root_dir: &std::path::Path,
) -> Vec<String> {
  let changed_file = match project.files(db).get(changed_path) {
    Some(f) => *f,
    None => return vec![],
  };
  let symbol = match file_symbol(db, project, changed_file).value(db) {
    Some(s) => s,
    None => return vec![],
  };

  let mut affected = HashSet::new();
  let mut queue = vec![symbol];

  while let Some(sym) = queue.pop() {
    for reference in references(db, project, sym) {
      let ref_file = reference.hir.node(db).owner_file;
      let ref_path = match ref_file.handle(db).path() {
        Some(p) => p.clone(),
        None => continue,
      };
      if ref_path == changed_path {
        continue;
      }
      if affected.insert(ref_path.clone()) {
        if let Some(ref_sym) = file_symbol(db, project, ref_file).value(db) {
          queue.push(ref_sym);
        }
      }
    }
  }

  affected
    .into_iter()
    .filter_map(|p| p.strip_prefix(root_dir).ok().map(|r| normalize_path(r)))
    .collect()
}

// alice.td is referenced by several tasks, milestones, and projects
#[test]
fn affected_files_for_alice() {
  let source = example_vault();
  let db = setup_db_fresh(&source);
  let project = Project::iter(&db).into_iter().next().unwrap();
  let root_dir = get_vault_config(&db, project).root_dir(&db);
  let alice_path = root_dir.join("people/alice.td");

  let mut affected = collect_affected(&db, project, &alice_path, &root_dir);
  affected.sort();

  assert!(
    affected.iter().any(|f| f.contains("implement-auth")),
    "implement-auth.td references alice: {affected:?}"
  );
  assert!(
    affected.iter().any(|f| f.contains("write-tests")),
    "write-tests.td references alice: {affected:?}"
  );
  assert!(
    affected.iter().any(|f| f.contains("website-redesign")),
    "website-redesign.td references alice: {affected:?}"
  );
}

// A file with no incoming references returns empty
#[test]
fn affected_files_for_unreferenced_file() {
  let source = example_vault();
  let db = setup_db_fresh(&source);
  let project = Project::iter(&db).into_iter().next().unwrap();
  let root_dir = get_vault_config(&db, project).root_dir(&db);
  let index_path = root_dir.join("index.td");

  let affected = collect_affected(&db, project, &index_path, &root_dir);
  assert!(
    affected.is_empty(),
    "index.td should have no referrers: {affected:?}"
  );
}

// Transitive: if A references B and B references C, changing C affects both A and B
#[test]
fn affected_files_transitive() {
  let source = example_vault();
  let db = setup_db_fresh(&source);
  let project = Project::iter(&db).into_iter().next().unwrap();
  let root_dir = get_vault_config(&db, project).root_dir(&db);
  // alice references implement-auth; milestones reference alice
  // so changing implement-auth should transitively affect milestones via alice
  let auth_path = root_dir.join("tasks/implement-auth.td");

  let affected = collect_affected(&db, project, &auth_path, &root_dir);

  assert!(
    affected.iter().any(|f| f.contains("alice")),
    "alice.td references implement-auth: {affected:?}"
  );
  // Transitive: milestones reference alice who references implement-auth
  assert!(
    affected
      .iter()
      .any(|f| f.contains("milestone") || f.contains("website-redesign")),
    "should have transitive referrers via alice: {affected:?}"
  );
}
