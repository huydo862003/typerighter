use std::sync::atomic::Ordering;

use tempfile::TempDir;
use typedown_incremental::{CacheSession, SerializableQueryDatabase};

use super::run_diagnostics;
use crate::utils::{copy_dir_recursive, example_vault, setup_db_fresh};

#[test]
fn crash_recovery() {
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
