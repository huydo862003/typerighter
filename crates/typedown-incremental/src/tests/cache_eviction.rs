// LRU eviction: in-memory cap, tombstone serialization, recomputation after eviction

use std::panic::catch_unwind;

use crate::LRU_CAPACITY;

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
