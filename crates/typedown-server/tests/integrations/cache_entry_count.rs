use tempfile::TempDir;
use typedown_incremental::{InputId, SerializableQueryDatabase};
use typedown_lang::db::TypedownDatabase;
use typedown_lang::db::derived::evaluate::evaluate_resource::evaluate_resource;
use typedown_lang::db::derived::name_resolver::file_symbol::file_symbol;
use typedown_lang::db::derived::parse_file::parse_file;
use typedown_lang::db::types::Project;

use super::utils::{copy_dir_recursive, example_vault, setup_db_fresh};

fn run_diagnostics(db: &TypedownDatabase) {
  let project = Project::iter(db)
    .into_iter()
    .next()
    .expect("project should exist");
  for (path, file) in project.files(db) {
    if path.extension().and_then(|e| e.to_str()) != Some("td") {
      continue;
    }
    let result = parse_file(db, project, file);
    let _ = result.diagnostics(db);
    if let Some(sym) = file_symbol(db, project, file).value(db) {
      let eval = evaluate_resource(db, sym);
      let _ = eval.diagnostics(db);
    }
  }
}

fn dump_stats(project_dir: &std::path::Path) -> typedown_incremental::CacheStats {
  let db = setup_db_fresh(project_dir);
  run_diagnostics(&db);
  db.dump().stats()
}

// Catch abnormal cache growth
#[test]
fn cache_entry_counts_are_reasonable() {
  let source = example_vault();
  let tmp = TempDir::new().unwrap();
  let project_dir = tmp.path().join("project_tracker");
  copy_dir_recursive(&source, &project_dir);

  let stats = dump_stats(&project_dir);

  // Baseline: ~616 queries, ~5109 fields, ~15 inputs, ~91 interned, ~1445 blobs
  assert!(
    stats.input_fields > 0 && stats.input_fields < 50,
    "input_fields={}",
    stats.input_fields
  );
  assert!(
    stats.derived_queries > 0 && stats.derived_queries < 2000,
    "derived_queries={}",
    stats.derived_queries
  );
  assert!(
    stats.derived_fields > 0 && stats.derived_fields < 6000,
    "derived_fields={}",
    stats.derived_fields
  );
  assert!(
    stats.interned > 0 && stats.interned < 300,
    "interned={}",
    stats.interned
  );
  assert!(
    stats.intern_blobs > 0 && stats.intern_blobs < 5000,
    "intern_blobs={}",
    stats.intern_blobs
  );
}

// Cache counts must be deterministic across runs
#[test]
fn cache_entry_counts_are_stable() {
  let source = example_vault();

  let stats1 = {
    let tmp = TempDir::new().unwrap();
    let dir = tmp.path().join("project_tracker");
    copy_dir_recursive(&source, &dir);
    dump_stats(&dir)
  };

  let stats2 = {
    let tmp = TempDir::new().unwrap();
    let dir = tmp.path().join("project_tracker");
    copy_dir_recursive(&source, &dir);
    dump_stats(&dir)
  };

  assert_eq!(
    stats1, stats2,
    "cache stats should be identical across runs"
  );
}
