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

// Cache roundtrip should not panic
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

// On unchanged vault: hashed queries must not recompute, no_hash queries may recompute
#[test]
fn cache_hit_no_recomputation_on_unchanged() {
  if std::env::var("CACHE_HIT_SESSION").as_deref() == Ok("2") {
    let project_dir = PathBuf::from(std::env::var("CACHE_HIT_PROJECT").unwrap());
    let cache_dir = PathBuf::from(std::env::var("CACHE_HIT_CACHE").unwrap());

    let db = setup_db_cached(&cache_dir, &project_dir);
    run_diagnostics(&db);

    let stats = db.storage.ingredient_stats();
    let hashed_recomputes: usize = stats
      .iter()
      .filter(|s| !s.no_hash)
      .map(|s| s.recompute_count)
      .sum();
    assert_eq!(
      hashed_recomputes, 0,
      "hashed queries should have zero recomputations on unchanged vault"
    );
    return;
  }

  let (_tmp, project_dir, cache_dir, _) = session1_dump();

  run_child_test(
    "cache_roundtrip::cache_hit_no_recomputation_on_unchanged",
    &[
      ("CACHE_HIT_SESSION", "2"),
      ("CACHE_HIT_PROJECT", project_dir.to_str().unwrap()),
      ("CACHE_HIT_CACHE", cache_dir.to_str().unwrap()),
    ],
  );
}

// Modifying a content file must trigger recomputation
#[test]
fn cache_miss_on_file_change() {
  if std::env::var("CACHE_MISS_CHANGE_SESSION").as_deref() == Ok("2") {
    let project_dir = PathBuf::from(std::env::var("CACHE_MISS_CHANGE_PROJECT").unwrap());
    let cache_dir = PathBuf::from(std::env::var("CACHE_MISS_CHANGE_CACHE").unwrap());

    let db = setup_db_cached(&cache_dir, &project_dir);
    run_diagnostics(&db);

    let stats = db.storage.ingredient_stats();
    let hashed_recomputes: usize = stats
      .iter()
      .filter(|s| !s.no_hash)
      .map(|s| s.recompute_count)
      .sum();
    assert!(
      hashed_recomputes > 0,
      "hashed queries should recompute after file content change"
    );
    return;
  }

  let (_tmp, project_dir, cache_dir, _) = session1_dump();

  let target = project_dir.join("vault/people/alice.td");
  let original = std::fs::read_to_string(&target).unwrap();
  std::fs::write(&target, format!("{original}\n<!-- appended -->\n")).unwrap();

  run_child_test(
    "cache_roundtrip::cache_miss_on_file_change",
    &[
      ("CACHE_MISS_CHANGE_SESSION", "2"),
      ("CACHE_MISS_CHANGE_PROJECT", project_dir.to_str().unwrap()),
      ("CACHE_MISS_CHANGE_CACHE", cache_dir.to_str().unwrap()),
    ],
  );
}

// Adding a new content file must trigger recomputation
#[test]
fn cache_miss_on_new_file() {
  if std::env::var("CACHE_MISS_NEW_SESSION").as_deref() == Ok("2") {
    let project_dir = PathBuf::from(std::env::var("CACHE_MISS_NEW_PROJECT").unwrap());
    let cache_dir = PathBuf::from(std::env::var("CACHE_MISS_NEW_CACHE").unwrap());

    let db = setup_db_cached(&cache_dir, &project_dir);
    run_diagnostics(&db);

    let total = db.storage.total_recompute_count();
    assert!(total > 0, "should recompute after new file added");
    return;
  }

  let (_tmp, project_dir, cache_dir, _) = session1_dump();

  let new_content = r#"---
_type: Person
name: "Dave"
email: "dave@example.com"
role: "developer"
---
"#;
  std::fs::write(project_dir.join("vault/people/dave.td"), new_content).unwrap();

  run_child_test(
    "cache_roundtrip::cache_miss_on_new_file",
    &[
      ("CACHE_MISS_NEW_SESSION", "2"),
      ("CACHE_MISS_NEW_PROJECT", project_dir.to_str().unwrap()),
      ("CACHE_MISS_NEW_CACHE", cache_dir.to_str().unwrap()),
    ],
  );
}

// Deleting a content file must trigger recomputation
#[test]
fn cache_miss_on_file_deleted() {
  if std::env::var("CACHE_MISS_DEL_SESSION").as_deref() == Ok("2") {
    let project_dir = PathBuf::from(std::env::var("CACHE_MISS_DEL_PROJECT").unwrap());
    let cache_dir = PathBuf::from(std::env::var("CACHE_MISS_DEL_CACHE").unwrap());

    let db = setup_db_cached(&cache_dir, &project_dir);
    run_diagnostics(&db);

    let total = db.storage.total_recompute_count();
    assert!(total > 0, "should recompute after file deletion");
    return;
  }

  let (_tmp, project_dir, cache_dir, _) = session1_dump();

  std::fs::remove_file(project_dir.join("vault/people/carol.td")).unwrap();

  run_child_test(
    "cache_roundtrip::cache_miss_on_file_deleted",
    &[
      ("CACHE_MISS_DEL_SESSION", "2"),
      ("CACHE_MISS_DEL_PROJECT", project_dir.to_str().unwrap()),
      ("CACHE_MISS_DEL_CACHE", cache_dir.to_str().unwrap()),
    ],
  );
}

// Modifying a schema must invalidate content files that reference it
#[test]
fn cache_miss_on_schema_change() {
  if std::env::var("CACHE_MISS_SCHEMA_SESSION").as_deref() == Ok("2") {
    let project_dir = PathBuf::from(std::env::var("CACHE_MISS_SCHEMA_PROJECT").unwrap());
    let cache_dir = PathBuf::from(std::env::var("CACHE_MISS_SCHEMA_CACHE").unwrap());

    let db = setup_db_cached(&cache_dir, &project_dir);
    run_diagnostics(&db);

    let stats = db.storage.ingredient_stats();
    let hashed_recomputes: usize = stats
      .iter()
      .filter(|s| !s.no_hash)
      .map(|s| s.recompute_count)
      .sum();
    assert!(
      hashed_recomputes > 0,
      "hashed queries should recompute after schema change"
    );
    return;
  }

  let (_tmp, project_dir, cache_dir, _) = session1_dump();

  let schema_path = project_dir.join("vault/_types/people/Person.td");
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

  let stale = cache_dir.join("s-0000000000000-deadbeef-working");
  std::fs::create_dir_all(&stale).unwrap();
  std::fs::write(stale.join("dummy.bin"), b"stale data").unwrap();

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

// no_hash queries always recompute, hashed queries should get cache hits
#[test]
fn no_hash_queries_recompute_hashed_queries_cached() {
  if std::env::var("NO_HASH_SESSION").as_deref() == Ok("2") {
    let project_dir = PathBuf::from(std::env::var("NO_HASH_PROJECT").unwrap());
    let cache_dir = PathBuf::from(std::env::var("NO_HASH_CACHE").unwrap());

    let db = setup_db_cached(&cache_dir, &project_dir);
    run_diagnostics(&db);

    let stats = db.storage.ingredient_stats();
    let no_hash_recomputes: usize = stats
      .iter()
      .filter(|s| s.no_hash)
      .map(|s| s.recompute_count)
      .sum();
    let hashed_recomputes: usize = stats
      .iter()
      .filter(|s| !s.no_hash)
      .map(|s| s.recompute_count)
      .sum();

    assert!(no_hash_recomputes > 0, "no_hash queries should re-execute");
    assert_eq!(
      hashed_recomputes, 0,
      "hashed queries should have zero recomputations on unchanged vault"
    );
    return;
  }

  let (_tmp, project_dir, cache_dir, _) = session1_dump();

  run_child_test(
    "cache_roundtrip::no_hash_queries_recompute_hashed_queries_cached",
    &[
      ("NO_HASH_SESSION", "2"),
      ("NO_HASH_PROJECT", project_dir.to_str().unwrap()),
      ("NO_HASH_CACHE", cache_dir.to_str().unwrap()),
    ],
  );
}

// Fingerprint::SKIPPED from no_hash queries must not corrupt the dep graph
#[test]
fn no_hash_does_not_corrupt_cache() {
  if std::env::var("ZERO_FP_SESSION").as_deref() == Ok("2") {
    let project_dir = PathBuf::from(std::env::var("ZERO_FP_PROJECT").unwrap());
    let cache_dir = PathBuf::from(std::env::var("ZERO_FP_CACHE").unwrap());

    let db = setup_db_cached(&cache_dir, &project_dir);

    let project = Project::iter(&db)
      .into_iter()
      .next()
      .expect("project should exist");
    for (path, file) in &*project.files(&db) {
      if path.extension().and_then(|ext| ext.to_str()) != Some("td") {
        continue;
      }
      let result = parse_file(&db, project, *file);
      assert!(
        !result.ast(&db).text().is_empty(),
        "parse result should have content for {}",
        path.display()
      );
      if let Some(sym) = file_symbol(&db, project, *file).value(&db) {
        let _eval = evaluate_resource(&db, sym);
      }
    }
    return;
  }

  let (_tmp, project_dir, cache_dir, _) = session1_dump();

  run_child_test(
    "cache_roundtrip::no_hash_does_not_corrupt_cache",
    &[
      ("ZERO_FP_SESSION", "2"),
      ("ZERO_FP_PROJECT", project_dir.to_str().unwrap()),
      ("ZERO_FP_CACHE", cache_dir.to_str().unwrap()),
    ],
  );
}

// no_hash queries should produce correct updated results after file change
#[test]
fn no_hash_recomputes_with_changed_file() {
  if std::env::var("NO_HASH_CHANGE_SESSION").as_deref() == Ok("2") {
    let project_dir = PathBuf::from(std::env::var("NO_HASH_CHANGE_PROJECT").unwrap());
    let cache_dir = PathBuf::from(std::env::var("NO_HASH_CHANGE_CACHE").unwrap());

    let db = setup_db_cached(&cache_dir, &project_dir);
    run_diagnostics(&db);

    let stats = db.storage.ingredient_stats();
    let no_hash_recomputes: usize = stats
      .iter()
      .filter(|s| s.no_hash)
      .map(|s| s.recompute_count)
      .sum();
    // no_hash queries must recompute since file content changed
    assert!(
      no_hash_recomputes > 0,
      "no_hash queries should recompute after file change"
    );
    return;
  }

  let (_tmp, project_dir, cache_dir, _) = session1_dump();

  let target = project_dir.join("vault/people/alice.td");
  let original = std::fs::read_to_string(&target).unwrap();
  std::fs::write(&target, format!("{original}\n<!-- changed -->\n")).unwrap();

  run_child_test(
    "cache_roundtrip::no_hash_recomputes_with_changed_file",
    &[
      ("NO_HASH_CHANGE_SESSION", "2"),
      ("NO_HASH_CHANGE_PROJECT", project_dir.to_str().unwrap()),
      ("NO_HASH_CHANGE_CACHE", cache_dir.to_str().unwrap()),
    ],
  );
}

fn run_diagnostics(db: &TypedownDatabase) {
  let project = Project::iter(db)
    .into_iter()
    .next()
    .expect("project should exist");
  for (path, file) in &*project.files(db) {
    if path.extension().and_then(|e| e.to_str()) != Some("td") {
      continue;
    }
    let result = parse_file(db, project, *file);
    let _ = result.diagnostics(db);
    if let Some(sym) = file_symbol(db, project, *file).value(db) {
      let eval = evaluate_resource(db, sym);
      let _ = eval.diagnostics(db);
    }
  }
}
