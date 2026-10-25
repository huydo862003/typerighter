mod cache_invalidation_on_file_change;
mod cache_invalidation_on_file_deleted;
mod cache_invalidation_on_new_file;
mod cache_invalidation_on_schema_change;
mod corrupted_cache_falls_back;
mod crash_recovery;
mod double_roundtrip_zero_errors;
mod expected_errors_after_schema_change;
mod export_pipeline_roundtrip;
mod gc_keeps_only_latest_finalized;
mod gc_removes_stale_working_dirs;
mod no_hash_not_corrupted;
mod no_hash_recomputes_on_file_change;
mod no_recomputation_on_unchanged_vault;
mod zero_spurious_errors;
mod zero_spurious_errors_after_file_change;

use std::path::PathBuf;
use std::sync::OnceLock;
use std::sync::atomic::Ordering;

use tempfile::TempDir;
use typedown_incremental::{CacheSession, InputId, SerializableQueryDatabase};
use typedown_lang::db::TypedownDatabase;
use typedown_lang::db::derived::evaluate::evaluate_resource::evaluate_resource;
use typedown_lang::db::derived::name_resolver::file_symbol::file_symbol;
use typedown_lang::db::derived::parse_file::parse_file;
use typedown_lang::db::types::Project;
use typedown_server::core::utils::fs::get_cache_dir;

use super::utils::{copy_dir_recursive, copy_dir_recursive_all, example_vault, setup_db_fresh};

// Shared session1 result: vault copy + DB build + diagnostics + cache dump done ONCE.
// Individual tests clone the files (cheap) instead of rebuilding the DB (expensive).
struct SharedSession1 {
  _tmp: TempDir,
  project_dir: PathBuf,
  fresh_count: usize,
}

static SHARED_SESSION1: OnceLock<SharedSession1> = OnceLock::new();

fn session1_dump() -> (TempDir, PathBuf, PathBuf, usize) {
  let shared = SHARED_SESSION1.get_or_init(|| {
    let source = example_vault();
    assert!(source.exists(), "examples/project_tracker must exist");

    let tmp = TempDir::new().unwrap();
    let project_dir = tmp.path().join("project_tracker");
    copy_dir_recursive(&source, &project_dir);

    let cache_dir = get_cache_dir(&project_dir);

    let db = setup_db_fresh(&project_dir);
    run_diagnostics(&db);

    let fresh_count = db.storage.total_recompute_count();
    let serialized = db.dump();
    let (session, _) = CacheSession::open(&cache_dir).unwrap();
    let revision = db.storage.revision.load(Ordering::Acquire) as u64;
    session.finalize(&serialized, revision).unwrap();

    SharedSession1 {
      _tmp: tmp,
      project_dir,
      fresh_count,
    }
  });

  // Clone to a fresh TempDir so each test can mutate independently
  let tmp = TempDir::new().unwrap();
  let project_dir = tmp.path().join("project_tracker");
  copy_dir_recursive_all(&shared.project_dir, &project_dir);
  let cache_dir = get_cache_dir(&project_dir);
  (tmp, project_dir, cache_dir, shared.fresh_count)
}

fn run_diagnostics(db: &TypedownDatabase) {
  let project = Project::iter(db)
    .into_iter()
    .next()
    .expect("project should exist");
  for (path, file) in &*project.files(db) {
    if path.extension().and_then(|extension| extension.to_str()) != Some("td") {
      continue;
    }
    let result = parse_file(db, project, *file);
    let _ = result.diagnostics(db);
    if let Some(symbol) = file_symbol(db, project, *file).value(db) {
      let evaluated_result = evaluate_resource(db, symbol);
      let _ = evaluated_result.diagnostics(db);
    }
  }
}
