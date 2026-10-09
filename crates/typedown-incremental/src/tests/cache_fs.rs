// Filesystem-level cache tests: CacheSession open/finalize, disk roundtrip,
// multi-session chains, stale dir cleanup, and corrupt-file fallback.

#![cfg(feature = "session")]

use std::fs;

use crate::persist::fs::CacheSession;
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
