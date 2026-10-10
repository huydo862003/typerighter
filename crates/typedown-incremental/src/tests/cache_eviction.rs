// LRU eviction: in-memory cap, tombstone serialization, recomputation after eviction

use std::panic::catch_unwind;

use crate::{LRU_CAPACITY, SerializableQueryDatabase};

use super::fixtures::identity::*;

#[test]
fn evicted_memo_recomputes_after_revision_bump() {
  let db = Database {
    storage: QueryStorage::default(),
  };

  for i in 0..=LRU_CAPACITY {
    let input = IdInput::new(&db, i);
    let result = identity(&db, input);
    assert_eq!(result.value(&db), i);
  }
  take_log();

  db.storage.reset_for_new_revision();

  let input_0 = IdInput::new(&db, 0);
  let result = identity(&db, input_0);
  assert_eq!(result.value(&db), 0);
  let log = take_log();
  assert_eq!(log, vec![0], "expected recomputation of identity(0)");
}

#[test]
fn serialize_after_lru_eviction() {
  let db = Database {
    storage: QueryStorage::default(),
  };

  for i in 0..=LRU_CAPACITY {
    let input = IdInput::new(&db, i);
    identity(&db, input);
  }
  db.storage.reset_for_new_revision();

  let db2 = dump_and_reload(&db, |storage| Database { storage });

  let input = find_entry(IdInput::iter(&db2), |i| i.n(&db2) == LRU_CAPACITY, "last");
  take_log();
  let result = identity(&db2, input);
  assert_eq!(result.value(&db2), LRU_CAPACITY);
}

#[test]
fn evicted_entry_recomputes_after_roundtrip() {
  let db = Database {
    storage: QueryStorage::default(),
  };

  for i in 0..=LRU_CAPACITY {
    let input = IdInput::new(&db, i);
    identity(&db, input);
  }
  db.storage.reset_for_new_revision();

  let db2 = dump_and_reload(&db, |storage| Database { storage });

  let input_0 = find_entry(IdInput::iter(&db2), |i| i.n(&db2) == 0, "IdInput(0)");
  take_log();
  let result = identity(&db2, input_0);
  assert_eq!(result.value(&db2), 0);
  let log = take_log();
  assert_eq!(
    log,
    vec![0],
    "expected recomputation of evicted identity(0)"
  );
}

#[test]
#[cfg(debug_assertions)]
fn tombstone_entry_panics_on_access() {
  let db = Database {
    storage: QueryStorage::default(),
  };

  let input = IdInput::new(&db, 42);
  let result = identity(&db, input);
  assert_eq!(result.value(&db), 42);

  let tombstone: IdResult = IdResult::from(crate::TOMBSTONE_ENTRY_ID);
  let caught = catch_unwind(std::panic::AssertUnwindSafe(|| {
    tombstone.value(&db);
  }));
  assert!(caught.is_err(), "expected panic on tombstone access");
}

// In-memory LRU eviction keeps entries with the highest verified_at
// Entries from revision 1 have lower verified_at and are evicted first
// Evicted entries become DepNode::Evicted in the serialized dep graph
// On-demand deserialization cannot restore an Evicted slot: the parent query must recompute
#[test]
fn lru_evicts_oldest_verified_entries_first() {
  let db = Database {
    storage: QueryStorage::default(),
  };

  // Revision 1: compute 50 entries (verified_at = 1)
  for i in 0..50usize {
    identity(&db, IdInput::new(&db, i));
  }

  // Advance to revision 2
  // Only 50 entries in memory, none evicted yet
  db.storage.reset_for_new_revision();

  // Revision 2: compute LRU_CAPACITY more entries (verified_at = 2)
  // Total in memory = LRU_CAPACITY + 50, exceeds cap
  for i in 50..50 + LRU_CAPACITY {
    identity(&db, IdInput::new(&db, i));
  }

  // Reset evicts the 50 oldest (verified_at = 1, entries 0..49)
  // LRU_CAPACITY entries remain
  db.storage.reset_for_new_revision();

  // Dump must have exactly LRU_CAPACITY derived queries (50 were evicted before dump)
  let stats = db.dump().stats();
  assert_eq!(
    stats.derived_queries, LRU_CAPACITY,
    "dump must contain exactly LRU_CAPACITY derived queries after eviction"
  );

  let db2 = dump_and_reload(&db, |storage| Database { storage });

  // Entry 0 (verified_at = 1) was evicted before dump, slot is Evicted, must recompute
  let input_0 = find_entry(IdInput::iter(&db2), |i| i.n(&db2) == 0, "IdInput(0)");
  take_log();
  identity(&db2, input_0);
  let log = take_log();
  assert_eq!(
    log,
    vec![0],
    "entry evicted by LRU must recompute after roundtrip"
  );

  // Entry 50 + LRU_CAPACITY - 1 (verified_at = 2) survived eviction, must be cached
  let last_n = 50 + LRU_CAPACITY - 1;
  let input_last = find_entry(
    IdInput::iter(&db2),
    |i| i.n(&db2) == last_n,
    "last identity entry",
  );
  take_log();
  identity(&db2, input_last);
  let log = take_log();
  assert!(
    log.is_empty(),
    "entry kept by LRU (higher verified_at) must be cached after roundtrip"
  );
}

// When a dep is Evicted in the serialized graph, has_missing propagates the eviction to its parent
// Both the dep and its parent must recompute after roundtrip
#[test]
fn evicted_dep_causes_parent_to_recompute() {
  let db = Database {
    storage: QueryStorage::default(),
  };

  // Revision 1: compute downstream(version = 0), which transitively computes root(version = 0)
  let config0 = VersionConfig::new(&db, 0);
  downstream_query_of_versioned(&db, config0);

  // Revision 2: fill root_query_versioned_result with LRU_CAPACITY entries (version = 1..=LRU_CAPACITY)
  // So that version = 0 (verified_at = 1) becomes the oldest entry
  db.storage.reset_for_new_revision();
  for i in 1..=LRU_CAPACITY {
    let config = VersionConfig::new(&db, i);
    root_query_versioned_result(&db, config);
  }

  // Revision 3: eviction runs on root
  // Version = 0 (verified_at = 1) is oldest and gets dropped
  db.storage.reset_for_new_revision();

  // root has exactly LRU_CAPACITY entries (version = 1..=LRU_CAPACITY)
  // Version = 0 was evicted
  // Downstream has 0 entries (its dep was evicted so it also got dropped by has_missing)
  let stats = db.dump().stats();
  assert_eq!(
    stats.derived_queries, LRU_CAPACITY,
    "dump must have LRU_CAPACITY derived queries: root(v=0) and downstream(v=0) both evicted"
  );

  let db2 = dump_and_reload(&db, |storage| Database { storage });

  // root(version = 0) slot = Evicted
  // Downstream(version = 0) also = Evicted (has_missing in finalize)
  let config0_2 = find_entry(
    VersionConfig::iter(&db2),
    |c| c.version(&db2) == 0,
    "VersionConfig(0)",
  );
  take_log();
  downstream_query_of_versioned(&db2, config0_2);
  let log = take_log();

  // root logs version (0), downstream logs version + 1000 (1000)
  assert!(
    log.contains(&0),
    "root_query must recompute because its dep was evicted: log={log:?}"
  );
  assert!(
    log.iter().any(|&v| v >= 1000),
    "downstream_query must recompute because its dep (root) was evicted: log={log:?}"
  );
}
