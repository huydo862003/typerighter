use std::path::PathBuf;
use std::sync::atomic::Ordering;

use tempfile::TempDir;
use typedown_incremental::{CacheSession, InputId, SerializableQueryDatabase};
use typedown_lang::db::TypedownDatabase;
use typedown_lang::db::derived::evaluate::evaluate_resource::evaluate_resource;
use typedown_lang::db::derived::name_resolver::file_symbol::file_symbol;
use typedown_lang::db::derived::parse_file::parse_file;
use typedown_lang::db::types::Project;

use super::utils::{
  copy_dir_recursive, example_vault, run_child_test, setup_db_cached, setup_db_fresh,
};

// Helper: Set up a temp copy of the example vault, run queries, dump cache, return paths
fn session1_dump() -> (TempDir, PathBuf, PathBuf, usize) {
  let source = example_vault();
  assert!(source.exists(), "examples/project_tracker must exist");

  let tmp = TempDir::new().unwrap();
  let project_dir = tmp.path().join("project_tracker");
  copy_dir_recursive(&source, &project_dir);

  let cache_dir = project_dir.join(".typedown/.local/cache");

  let db = setup_db_fresh(&project_dir);
  run_diagnostics(&db);

  let fresh_count = db.storage.total_recompute_count();
  let serialized = db.dump();
  let (session, _) = CacheSession::open(&cache_dir).unwrap();
  let revision = db.storage.revision.load(Ordering::Acquire) as u64;
  session.finalize(&serialized, revision).unwrap();

  (tmp, project_dir, cache_dir, fresh_count)
}

#[test]
fn cache_roundtrip_with_project() {
  if std::env::var("CACHE_ROUNDTRIP_SESSION").as_deref() == Ok("2") {
    let project_dir = PathBuf::from(std::env::var("CACHE_ROUNDTRIP_PROJECT").unwrap());
    let cache_dir = PathBuf::from(std::env::var("CACHE_ROUNDTRIP_CACHE").unwrap());

    let db = setup_db_cached(&cache_dir, &project_dir);
    run_diagnostics(&db);
    return;
  }

  let (_tmp, project_dir, cache_dir, _) = session1_dump();

  run_child_test(
    "cache_roundtrip::cache_roundtrip_with_project",
    &[
      ("CACHE_ROUNDTRIP_SESSION", "2"),
      ("CACHE_ROUNDTRIP_PROJECT", project_dir.to_str().unwrap()),
      ("CACHE_ROUNDTRIP_CACHE", cache_dir.to_str().unwrap()),
    ],
  );
}

// Cached session should recompute far fewer queries than a fresh session
#[test]
fn cache_hit_no_recomputation_on_unchanged() {
  if std::env::var("CACHE_HIT_SESSION").as_deref() == Ok("2") {
    let project_dir = PathBuf::from(std::env::var("CACHE_HIT_PROJECT").unwrap());
    let cache_dir = PathBuf::from(std::env::var("CACHE_HIT_CACHE").unwrap());

    let db = setup_db_cached(&cache_dir, &project_dir);
    run_diagnostics(&db);

    let cached_count = db.storage.total_recompute_count();
    let fresh_count: usize = std::env::var("CACHE_HIT_FRESH_COUNT")
      .unwrap()
      .parse()
      .unwrap();
    assert!(
      cached_count < fresh_count / 2,
      "cached session should recompute far fewer queries than fresh: cached={cached_count}, fresh={fresh_count}"
    );
    return;
  }

  let (_tmp, project_dir, cache_dir, fresh_count) = session1_dump();

  run_child_test(
    "cache_roundtrip::cache_hit_no_recomputation_on_unchanged",
    &[
      ("CACHE_HIT_SESSION", "2"),
      ("CACHE_HIT_PROJECT", project_dir.to_str().unwrap()),
      ("CACHE_HIT_CACHE", cache_dir.to_str().unwrap()),
      ("CACHE_HIT_FRESH_COUNT", &fresh_count.to_string()),
    ],
  );
}

// Modifying a content file between sessions triggers recomputation
#[test]
fn cache_miss_on_file_change() {
  if std::env::var("CACHE_MISS_CHANGE_SESSION").as_deref() == Ok("2") {
    let project_dir = PathBuf::from(std::env::var("CACHE_MISS_CHANGE_PROJECT").unwrap());
    let cache_dir = PathBuf::from(std::env::var("CACHE_MISS_CHANGE_CACHE").unwrap());

    let db = setup_db_cached(&cache_dir, &project_dir);
    let before = db.storage.total_recompute_count();
    run_diagnostics(&db);
    let after = db.storage.total_recompute_count();

    assert!(
      after > before,
      "expected recomputations after file change, but count stayed at {before}"
    );
    return;
  }

  let (_tmp, project_dir, cache_dir, _) = session1_dump();

  let target = project_dir.join("vault/content/people/alice.td");
  let original = std::fs::read_to_string(&target).unwrap();
  std::fs::write(&target, original.replace("Alice", "Alicia")).unwrap();

  run_child_test(
    "cache_roundtrip::cache_miss_on_file_change",
    &[
      ("CACHE_MISS_CHANGE_SESSION", "2"),
      ("CACHE_MISS_CHANGE_PROJECT", project_dir.to_str().unwrap()),
      ("CACHE_MISS_CHANGE_CACHE", cache_dir.to_str().unwrap()),
    ],
  );
}

// Adding a new file between sessions triggers computation
#[test]
fn cache_miss_on_new_file() {
  if std::env::var("CACHE_MISS_NEW_SESSION").as_deref() == Ok("2") {
    let project_dir = PathBuf::from(std::env::var("CACHE_MISS_NEW_PROJECT").unwrap());
    let cache_dir = PathBuf::from(std::env::var("CACHE_MISS_NEW_CACHE").unwrap());

    let db = setup_db_cached(&cache_dir, &project_dir);
    let before = db.storage.total_recompute_count();
    run_diagnostics(&db);
    let after = db.storage.total_recompute_count();

    assert!(
      after > before,
      "expected recomputations after adding a new file, but count stayed at {before}"
    );
    return;
  }

  let (_tmp, project_dir, cache_dir, _) = session1_dump();

  std::fs::write(
    project_dir.join("vault/content/people/dave.td"),
    "---\n_type: Person\nname: \"Dave\"\nrole: \"developer\"\n---\n",
  )
  .unwrap();

  run_child_test(
    "cache_roundtrip::cache_miss_on_new_file",
    &[
      ("CACHE_MISS_NEW_SESSION", "2"),
      ("CACHE_MISS_NEW_PROJECT", project_dir.to_str().unwrap()),
      ("CACHE_MISS_NEW_CACHE", cache_dir.to_str().unwrap()),
    ],
  );
}

// Deleting a file between sessions does not crash
#[test]
fn cache_miss_on_file_deleted() {
  if std::env::var("CACHE_MISS_DEL_SESSION").as_deref() == Ok("2") {
    let project_dir = PathBuf::from(std::env::var("CACHE_MISS_DEL_PROJECT").unwrap());
    let cache_dir = PathBuf::from(std::env::var("CACHE_MISS_DEL_CACHE").unwrap());

    let db = setup_db_cached(&cache_dir, &project_dir);
    run_diagnostics(&db);
    return;
  }

  let (_tmp, project_dir, cache_dir, _) = session1_dump();

  std::fs::remove_file(project_dir.join("vault/content/people/carol.td")).unwrap();

  run_child_test(
    "cache_roundtrip::cache_miss_on_file_deleted",
    &[
      ("CACHE_MISS_DEL_SESSION", "2"),
      ("CACHE_MISS_DEL_PROJECT", project_dir.to_str().unwrap()),
      ("CACHE_MISS_DEL_CACHE", cache_dir.to_str().unwrap()),
    ],
  );
}

// Modifying a schema invalidates content files that reference it
#[test]
fn cache_miss_on_schema_change() {
  if std::env::var("CACHE_MISS_SCHEMA_SESSION").as_deref() == Ok("2") {
    let project_dir = PathBuf::from(std::env::var("CACHE_MISS_SCHEMA_PROJECT").unwrap());
    let cache_dir = PathBuf::from(std::env::var("CACHE_MISS_SCHEMA_CACHE").unwrap());

    let db = setup_db_cached(&cache_dir, &project_dir);
    let before = db.storage.total_recompute_count();
    run_diagnostics(&db);
    let after = db.storage.total_recompute_count();

    assert!(
      after > before,
      "expected recomputations after schema change, but count stayed at {before}"
    );
    return;
  }

  let (_tmp, project_dir, cache_dir, _) = session1_dump();

  let schema_path = project_dir.join("vault/schemas/person/Person.td");
  let original = std::fs::read_to_string(&schema_path).unwrap();
  std::fs::write(
    &schema_path,
    original.replace("  email:", "  phone:\n    type: string\n  email:"),
  )
  .unwrap();

  run_child_test(
    "cache_roundtrip::cache_miss_on_schema_change",
    &[
      ("CACHE_MISS_SCHEMA_SESSION", "2"),
      ("CACHE_MISS_SCHEMA_PROJECT", project_dir.to_str().unwrap()),
      ("CACHE_MISS_SCHEMA_CACHE", cache_dir.to_str().unwrap()),
    ],
  );
}

#[test]
fn corrupted_cache_falls_back() {
  let (_tmp, _project_dir, cache_dir, _) = session1_dump();

  // Corrupt all finalized session dep-graphs
  let finalized_dirs: Vec<_> = std::fs::read_dir(&cache_dir)
    .unwrap()
    .filter_map(|e| e.ok())
    .map(|e| e.path())
    .filter(|p| {
      p.is_dir()
        && p
          .file_name()
          .and_then(|n| n.to_str())
          .is_some_and(|n| n.starts_with("s-") && !n.ends_with("-working"))
    })
    .collect();
  assert!(!finalized_dirs.is_empty(), "should have finalized sessions");

  for dir in &finalized_dirs {
    std::fs::write(dir.join("dep-graph.bin"), b"corrupted").unwrap();
  }

  let (_, data) = CacheSession::open(&cache_dir).unwrap();
  assert!(
    data.is_none(),
    "corrupted cache should not produce valid data"
  );
}

#[test]
fn gc_removes_stale_working_dirs() {
  let tmp = TempDir::new().unwrap();
  let cache_dir = tmp.path().join("cache");
  std::fs::create_dir_all(&cache_dir).unwrap();

  // Stale working directory with no lock file holder
  let stale = cache_dir.join("s-0000000000000-deadbeef-working");
  std::fs::create_dir_all(&stale).unwrap();
  std::fs::write(stale.join("dummy.bin"), b"stale data").unwrap();

  // Opening triggers GC
  let (_session, _) = CacheSession::open(&cache_dir).unwrap();

  assert!(
    !stale.exists(),
    "stale working directory should be removed by GC"
  );
}

#[test]
fn gc_keeps_only_latest_finalized() {
  let tmp = TempDir::new().unwrap();
  let cache_dir = tmp.path().join("cache");
  std::fs::create_dir_all(&cache_dir).unwrap();

  let old = cache_dir.join("s-0000000000001-1");
  let new = cache_dir.join("s-0000000000002-2");
  std::fs::create_dir_all(&old).unwrap();
  std::fs::create_dir_all(&new).unwrap();
  std::fs::write(old.join("lock"), b"").unwrap();
  std::fs::write(new.join("lock"), b"").unwrap();

  let (_session, _) = CacheSession::open(&cache_dir).unwrap();

  assert!(
    !old.exists(),
    "older finalized session should be removed by GC"
  );
}

#[test]
fn mid_session_crash_recovery() {
  let tmp = TempDir::new().unwrap();
  let cache_dir = tmp.path().join("cache");

  let source = example_vault();
  if !source.exists() {
    eprintln!("skipping: examples/project_tracker not found");
    return;
  }
  let project_dir = tmp.path().join("project_tracker");
  copy_dir_recursive(&source, &project_dir);

  let db = setup_db_fresh(&project_dir);
  run_diagnostics(&db);
  let serialized = db.dump();
  let (session, _) = CacheSession::open(&cache_dir).unwrap();
  let revision = db.storage.revision.load(Ordering::Acquire) as u64;
  session.finalize(&serialized, revision).unwrap();

  // Simulate crashed session with orphaned working dir
  let crashed = cache_dir.join("s-9999999999999-crashed01-working");
  std::fs::create_dir_all(&crashed).unwrap();
  std::fs::write(crashed.join("lock"), b"").unwrap();
  std::fs::write(crashed.join("dep-graph.bin"), b"partial").unwrap();

  let (_session, data) = CacheSession::open(&cache_dir).unwrap();

  assert!(
    !crashed.exists(),
    "crashed working directory should be cleaned up"
  );
  assert!(
    data.is_some(),
    "finalized cache should still be loadable after crash cleanup"
  );
}

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
