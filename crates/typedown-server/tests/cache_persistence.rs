use std::sync::Arc;

use tempfile::TempDir;
use typedown_incremental::{CacheSession, SerializableQueryDatabase};
use typedown_lang::db::{QueryStorage, TypedownDatabase};
use typedown_server::core::utils::fs::get_cache_dir;

/// Verify that the cache session creates files on disk and can be loaded back.
#[test]
fn cache_roundtrip_creates_and_loads_files() {
  let dir = TempDir::new().unwrap();
  let cache_dir = get_cache_dir(dir.path());

  // Session 1: create a database, dump, finalize
  {
    let db = TypedownDatabase {
      storage: QueryStorage::default(),
    };
    let (session, prev) = CacheSession::open(&cache_dir).unwrap();
    assert!(prev.is_none(), "no previous session should exist");

    let serialized = db.dump();
    let revision = 1;
    session.finalize(&serialized, revision).unwrap();
  }

  // Verify files exist in a finalized session directory
  let entries: Vec<_> = std::fs::read_dir(&cache_dir)
    .unwrap()
    .filter_map(|e| e.ok())
    .collect();
  assert_eq!(
    entries.len(),
    1,
    "should have exactly one finalized session"
  );

  let session_dir = entries[0].path();
  let name = session_dir.file_name().unwrap().to_str().unwrap();
  assert!(name.starts_with("s-"), "should start with s-");
  assert!(!name.ends_with("-working"), "should not end with -working");
  assert!(session_dir.join("dep-graph.bin").exists());
  assert!(session_dir.join("query-cache.bin").exists());
  assert!(session_dir.join("interned-blobs.bin").exists());
  assert!(session_dir.join("lock").exists());

  // Session 2: load from the finalized session
  {
    let (session, prev) = CacheSession::open(&cache_dir).unwrap();
    assert!(prev.is_some(), "should load previous session");

    let data = prev.unwrap();
    let storage = QueryStorage::from_serialized(data);
    let db = TypedownDatabase {
      storage: Arc::try_unwrap(storage).unwrap_or_else(|arc| (*arc).clone()),
    };

    // Finalize session 2
    session.finalize(&db.dump(), 2).unwrap();
  }

  // Session 3: open triggers GC, should keep only the latest finalized session
  {
    let (session, prev) = CacheSession::open(&cache_dir).unwrap();
    assert!(prev.is_some(), "should load session 2");

    let finalized: Vec<_> = std::fs::read_dir(&cache_dir)
      .unwrap()
      .filter_map(|e| e.ok())
      .filter(|e| {
        e.path().is_dir()
          && e
            .file_name()
            .to_str()
            .is_some_and(|n| n.starts_with("s-") && !n.ends_with("-working"))
      })
      .collect();
    assert_eq!(
      finalized.len(),
      1,
      "GC should keep only the latest finalized session"
    );
    let name = finalized[0].file_name();
    assert!(
      name.to_str().unwrap().ends_with("-2"),
      "latest session should have revision 2"
    );

    // Clean up: don't finalize, let working dir be GC'd next time
    drop(session);
  }
}
