// Filesystem-level cache tests: CacheSession open/finalize, disk roundtrip,
// multi-session chains, stale dir cleanup, corrupt-file fallback

#![cfg(feature = "session")]

use std::fs;

use crate::serial::format::fs::CacheSession;
use crate::{QueryStorage, SerializableQueryDatabase};

use super::fixtures::identity;
use identity::{Database as IdDb, InputId, InternedId, QueryStorage as IdQueryStorage};

fn make_db() -> IdDb {
  IdDb {
    storage: IdQueryStorage::default(),
  }
}

fn open_session(dir: &std::path::Path) -> (CacheSession, Option<IdDb>) {
  let (session, serialized) = CacheSession::open(dir).expect("open failed");
  let db = serialized.map(|s| {
    let storage = QueryStorage::from_serialized(s);
    IdDb {
      storage: std::sync::Arc::try_unwrap(storage).unwrap_or_else(|arc| (*arc).clone()),
    }
  });
  (session, db)
}

fn finalize(session: CacheSession, db: &IdDb) {
  let serialized = db.dump();
  let revision = serialized.dep_graph.header.revision;
  session
    .finalize(&serialized, revision)
    .expect("finalize failed");
}

// Basic open/finalize roundtrip through real files: cached entry must not recompute
#[test]
fn disk_roundtrip_basic() {
  let dir = tempfile::tempdir().unwrap();

  // Session 1: compute identity(7), finalize
  {
    let (session, _) = open_session(dir.path());
    let db = make_db();
    let input = identity::IdInput::new(&db, 7);
    identity::identity(&db, input);
    finalize(session, &db);
  }

  // Session 2: load from disk, must serve identity(7) from cache
  {
    let (session, db) = open_session(dir.path());
    let db = db.expect("session 2 must load previous session");
    let input = identity::find_entry(
      identity::IdInput::iter(&db),
      |i| i.n(&db) == 7,
      "IdInput(7)",
    );
    identity::take_log();
    identity::identity(&db, input);
    let log = identity::take_log();
    assert!(
      log.is_empty(),
      "identity(7) must be cached after disk roundtrip: {log:?}"
    );
    finalize(session, &db);
  }
}

// Three disk sessions with no mutations must never recompute
#[test]
fn disk_three_sessions_no_recomputation() {
  let dir = tempfile::tempdir().unwrap();

  {
    let (session, _) = open_session(dir.path());
    let db = make_db();
    let config = identity::VersionConfig::new(&db, 1);
    identity::root_query_versioned_result(&db, config);
    finalize(session, &db);
  }

  for session_num in 2..=3 {
    let (session, db) = open_session(dir.path());
    let db = db.unwrap_or_else(|| panic!("session {session_num} must load previous"));
    let config = identity::find_entry(
      identity::VersionConfig::iter(&db),
      |c| c.version(&db) == 1,
      "VersionConfig(1)",
    );
    identity::take_log();
    identity::root_query_versioned_result(&db, config);
    let log = identity::take_log();
    assert!(
      log.is_empty(),
      "session {session_num}: must not recompute when deps unchanged: {log:?}"
    );
    finalize(session, &db);
  }
}

// Mutation in session 2 must recompute and be cached in session 3
#[test]
fn disk_mutation_then_cached_next_session() {
  let dir = tempfile::tempdir().unwrap();

  {
    let (session, _) = open_session(dir.path());
    let db = make_db();
    let config = identity::VersionConfig::new(&db, 1);
    identity::root_query_versioned_result(&db, config);
    finalize(session, &db);
  }

  {
    let (session, db) = open_session(dir.path());
    let mut db = db.expect("session 2 must load");
    let config = identity::find_entry(
      identity::VersionConfig::iter(&db),
      |c| c.version(&db) == 1,
      "VersionConfig(1)",
    );
    config.set_version(&mut db, 42);
    let result = identity::root_query_versioned_result(&db, config);
    assert_eq!(result.value(&db), 42);
    finalize(session, &db);
  }

  {
    let (session, db) = open_session(dir.path());
    let db = db.expect("session 3 must load");
    let config = identity::find_entry(
      identity::VersionConfig::iter(&db),
      |c| c.version(&db) == 42,
      "VersionConfig(42)",
    );
    identity::take_log();
    let result = identity::root_query_versioned_result(&db, config);
    let log = identity::take_log();
    assert_eq!(result.value(&db), 42);
    assert!(
      log.is_empty(),
      "mutated result must be cached in session 3: {log:?}"
    );
    finalize(session, &db);
  }
}

// A stale working directory (no owner) must be cleaned up on the next open
#[test]
fn stale_working_dir_is_removed() {
  let dir = tempfile::tempdir().unwrap();
  fs::create_dir_all(dir.path()).unwrap();

  // Create a fake stale working directory with no lock file
  let stale = dir.path().join("s-0000000000000-deadbeef-working");
  fs::create_dir_all(&stale).unwrap();
  // No lock file means no owner process

  let (session, _) = open_session(dir.path());
  drop(session);

  assert!(!stale.exists(), "stale working dir must be removed on open");
}

// Corrupt dep-graph file must cause open to start a clean session instead of panicking
#[test]
fn corrupt_dep_graph_starts_clean() {
  let dir = tempfile::tempdir().unwrap();

  // Session 1: write a valid cache
  {
    let (session, _) = open_session(dir.path());
    let db = make_db();
    identity::identity(&db, identity::IdInput::new(&db, 1));
    finalize(session, &db);
  }

  // Corrupt the dep-graph file in the finalized session dir
  let session_dir = fs::read_dir(dir.path())
    .unwrap()
    .filter_map(|e| e.ok())
    .find(|e| {
      e.file_name()
        .to_str()
        .is_some_and(|n| n.starts_with("s-") && !n.ends_with("-working"))
    })
    .expect("finalized session dir")
    .path();
  fs::write(session_dir.join("dep-graph.bin"), b"garbage data").unwrap();

  // Session 2 must open cleanly without panicking, returning no prior data
  let (session, prior) = open_session(dir.path());
  assert!(
    prior.is_none(),
    "corrupt cache must be ignored, got prior data"
  );
  drop(session);
}

// Old finalized session directories must be removed when a newer one exists
// GC keeps only the most recent and deletes the rest
#[test]
fn old_finalized_sessions_are_removed() {
  let dir = tempfile::tempdir().unwrap();

  // Run three sessions so three finalized dirs accumulate (open adds a tiny sleep to ensure unique millisecond timestamps)
  // We just finalize without sleeping and check the count instead
  let mut last_finalized_dir: Option<std::path::PathBuf> = None;
  for i in 0u8..3 {
    let (session, _) = open_session(dir.path());
    let db = make_db();
    identity::identity(&db, identity::IdInput::new(&db, i as usize));
    finalize(session, &db);

    // Record the name of the just-finalized directory
    last_finalized_dir = fs::read_dir(dir.path())
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
      .max_by_key(|p| p.file_name().unwrap().to_os_string());
  }

  // Opening a 4th session triggers GC, which must remove all but the most recent
  let (session, _) = open_session(dir.path());
  drop(session);

  let finalized_dirs: Vec<_> = fs::read_dir(dir.path())
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

  assert_eq!(
    finalized_dirs.len(),
    1,
    "GC must keep only the most recent finalized session, found: {finalized_dirs:?}"
  );
  if let Some(expected) = last_finalized_dir {
    assert_eq!(
      finalized_dirs[0], expected,
      "GC must keep the most recent session dir"
    );
  }
}

// A non-directory file named like a session dir must not be mistaken for a session
// Must not cause GC or loading to crash
#[test]
fn non_directory_session_lookalike_is_ignored() {
  let dir = tempfile::tempdir().unwrap();

  // Place a regular file with a session-like name in the cache dir
  fs::write(dir.path().join("s-0000000000000-deadbeef-99"), b"not a dir").unwrap();

  // Open must not panic and must not load the file as a prior session
  let (session, prior) = open_session(dir.path());
  assert!(
    prior.is_none(),
    "a regular file with a session name must not be loaded as a session"
  );
  drop(session);
}

// Gitignore written on first open must not be overwritten if it already has custom content
#[test]
fn gitignore_not_overwritten_when_exists() {
  let parent = tempfile::tempdir().unwrap();
  let cache_dir = parent.path().join("cache");
  fs::create_dir_all(&cache_dir).unwrap();

  let gitignore = parent.path().join(".gitignore");
  fs::write(&gitignore, "custom content\n").unwrap();

  let (session, _) = CacheSession::open(&cache_dir).expect("open failed");
  drop(session);

  let content = fs::read_to_string(&gitignore).unwrap();
  assert_eq!(
    content, "custom content\n",
    "existing .gitignore must not be overwritten by CacheSession::open"
  );
}

// Values inside interned blobs must survive a disk roundtrip exactly
// Catches parser offset bugs in read_interned_blobs / write_interned_blobs
#[test]
fn interned_blob_values_survive_disk_roundtrip() {
  let dir = tempfile::tempdir().unwrap();

  // Session 1: intern several distinct values
  {
    let (session, _) = open_session(dir.path());
    let db = make_db();
    for n in [0usize, 1, 100, 999] {
      identity::IdInput::new(&db, n);
    }
    finalize(session, &db);
  }

  // Session 2: all interned values must be present and have correct content
  {
    let (session, db) = open_session(dir.path());
    let db = db.expect("session 2 must load previous session");
    for n in [0usize, 1, 100, 999] {
      let found = identity::IdInput::iter(&db)
        .into_iter()
        .any(|i| i.n(&db) == n);
      assert!(found, "IdInput(n={n}) must survive disk roundtrip");
    }
    finalize(session, &db);
  }
}

// Corrupt interned-blobs file must cause open to start a clean session
#[test]
fn corrupt_interned_blobs_starts_clean() {
  let dir = tempfile::tempdir().unwrap();

  {
    let (session, _) = open_session(dir.path());
    let db = make_db();
    identity::identity(&db, identity::IdInput::new(&db, 42));
    finalize(session, &db);
  }

  let session_dir = fs::read_dir(dir.path())
    .unwrap()
    .filter_map(|e| e.ok())
    .find(|e| {
      e.file_name()
        .to_str()
        .is_some_and(|n| n.starts_with("s-") && !n.ends_with("-working"))
    })
    .expect("finalized session dir")
    .path();
  fs::write(session_dir.join("interned-blobs.bin"), b"corrupt").unwrap();

  let (session, prior) = open_session(dir.path());
  assert!(
    prior.is_none(),
    "corrupt interned-blobs must be ignored, got prior data"
  );
  drop(session);
}

// Two sessions can open concurrently: reader gets shared lock while writer holds working dir
#[test]
fn concurrent_read_sessions() {
  let dir = tempfile::tempdir().unwrap();

  // Write a valid session first
  {
    let (session, _) = open_session(dir.path());
    let db = make_db();
    identity::identity(&db, identity::IdInput::new(&db, 5));
    finalize(session, &db);
  }

  // Open two sessions simultaneously without finalizing either
  let (session_a, db_a) = open_session(dir.path());
  let (session_b, db_b) = open_session(dir.path());

  // Both must see the previous session's data
  assert!(db_a.is_some(), "session A must load previous session");
  assert!(db_b.is_some(), "session B must load previous session");

  drop(session_a);
  drop(session_b);
}
