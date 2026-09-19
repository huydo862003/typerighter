// Simulates RPC and LSP server sessions at the db level
// Each session creates a db, runs queries, and optionally dumps cache

use std::path::Path;
use std::sync::atomic::Ordering;

use typedown_incremental::{CacheSession, InputId, SerializableQueryDatabase};
use typedown_lang::db::TypedownDatabase;
use typedown_lang::db::derived::evaluate::evaluate_resource::evaluate_resource;
use typedown_lang::db::derived::hir::lower_node;
use typedown_lang::db::derived::name_resolver::file_symbol::file_symbol;
use typedown_lang::db::derived::parse_file::parse_file;
use typedown_lang::db::derived::typechecker::typecheck::typecheck;
use typedown_lang::db::types::{FileRedNode, Project};
use typedown_lang::integrations::export::{export_resource_html, export_resource_summary};
use typedown_server::core::utils::fs::get_cache_dir;

use super::utils::{setup_db_cached, setup_db_fresh};

// Run the queries that an RPC server would run when serving a full site build
// Asserts that exports produce valid results
pub fn run_rpc_queries(db: &TypedownDatabase) {
  let project = Project::iter(db)
    .into_iter()
    .next()
    .expect("project should exist");
  let mut export_count = 0;
  for (path, file) in &*project.files(db) {
    if path.extension().and_then(|e| e.to_str()) != Some("td") {
      continue;
    }
    let _ = parse_file(db, project, *file);
    if let Some(sym) = file_symbol(db, project, *file).value(db) {
      let _ = evaluate_resource(db, sym);
      if let Some(exported) = export_resource_html(db, project, *file) {
        assert!(
          !exported.header.is_null(),
          "export header should not be null for {}",
          path.display()
        );
        export_count += 1;
      }
      let _ = export_resource_summary(db, project, *file);
    }
  }
  assert!(
    export_count > 0,
    "at least one file should produce an export"
  );
}

// Run the queries that an LSP server would run when opening files
// Asserts that parse and typecheck produce results
pub fn run_lsp_queries(db: &TypedownDatabase) {
  let project = Project::iter(db)
    .into_iter()
    .next()
    .expect("project should exist");
  let mut file_count = 0;
  for (path, file) in &*project.files(db) {
    if path.extension().and_then(|e| e.to_str()) != Some("td") {
      continue;
    }
    let parse_result = parse_file(db, project, *file);
    assert!(
      !parse_result.ast(db).text().is_empty(),
      "parse result should have content for {}",
      path.display()
    );
    let root = parse_result.ast(db).node.clone();
    let hir = lower_node(db, project, FileRedNode::new(*file, root));
    let _ = typecheck(db, hir);
    if let Some(sym) = file_symbol(db, project, *file).value(db) {
      let _ = evaluate_resource(db, sym);
    }
    file_count += 1;
  }
  assert!(file_count > 0, "at least one .td file should exist");
}

// Collect all type errors across all files
pub fn collect_type_errors(db: &TypedownDatabase) -> Vec<String> {
  let project = Project::iter(db)
    .into_iter()
    .next()
    .expect("project should exist");

  let mut errors = Vec::new();
  for (path, file) in &*project.files(db) {
    if path.extension().and_then(|e| e.to_str()) != Some("td") {
      continue;
    }
    let parse_result = parse_file(db, project, *file);
    let root = parse_result.ast(db).node.clone();
    let hir = lower_node(db, project, FileRedNode::new(*file, root));
    let typecheck_result = typecheck(db, hir);
    for diag in typecheck_result.diagnostics(db).iter() {
      errors.push(format!("{}: {}", path.display(), diag.message()));
    }
  }
  errors
}

// Simulate a fresh server session: create db, run queries, dump cache
pub fn fresh_session(project_dir: &Path, queries: fn(&TypedownDatabase)) {
  let db = setup_db_fresh(project_dir);
  queries(&db);
  dump_cache(&db, project_dir);
}

// Simulate a cached server session that asserts zero type errors before dumping
pub fn cached_session_assert_no_errors(project_dir: &Path, queries: fn(&TypedownDatabase)) {
  let cache_dir = get_cache_dir(project_dir);
  let db = setup_db_cached(&cache_dir, project_dir);
  queries(&db);
  let errors = collect_type_errors(&db);
  assert!(
    errors.is_empty(),
    "intermediate session should produce zero type errors, got:\n{}",
    errors.join("\n"),
  );
  dump_cache(&db, project_dir);
}

// Simulate a cached session that returns the db for assertions
pub fn cached_session_db(project_dir: &Path) -> TypedownDatabase {
  let cache_dir = get_cache_dir(project_dir);
  setup_db_cached(&cache_dir, project_dir)
}

fn dump_cache(db: &TypedownDatabase, project_dir: &Path) {
  let cache_dir = get_cache_dir(project_dir);
  let serialized = db.dump();
  let (session, _) = CacheSession::open(&cache_dir).unwrap();
  let revision = db.storage.revision.load(Ordering::Acquire) as u64;
  session.finalize(&serialized, revision).unwrap();
  // Verify the cache was written and is loadable
  let (_, data) = CacheSession::open(&cache_dir).unwrap();
  assert!(data.is_some(), "cache should be loadable after dump");
}
