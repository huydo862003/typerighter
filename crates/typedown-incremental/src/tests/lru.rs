use std::panic::catch_unwind;

use crate::{LRU_CAPACITY, Lru};

use super::fixtures::identity::*;

#[test]
fn drain_evicts_oldest_when_over_capacity() {
  let lru = Lru::default();
  for i in 0..=LRU_CAPACITY {
    lru.touch(i);
  }
  let evicted = lru.drain_evicted();
  assert_eq!(evicted, vec![0]);
}

#[test]
fn drain_respects_touch_order() {
  let lru = Lru::default();
  for i in 0..LRU_CAPACITY {
    lru.touch(i);
  }
  // Touch 0 again to move it to back
  lru.touch(0);
  // Add one more to overflow
  lru.touch(LRU_CAPACITY);
  let evicted = lru.drain_evicted();
  assert_eq!(evicted, vec![1]);
}

#[test]
fn drain_returns_empty_under_capacity() {
  let lru = Lru::default();
  for i in 0..100 {
    lru.touch(i);
  }
  assert!(lru.drain_evicted().is_empty());
}

#[test]
fn duplicate_touch_does_not_grow() {
  let lru = Lru::default();
  for _ in 0..2000 {
    lru.touch(42);
  }
  assert!(lru.drain_evicted().is_empty());
}

#[test]
fn evicted_memo_recomputes_after_revision_bump() {
  let db = Database {
    storage: QueryStorage::default(),
  };

  // Fill LRU to capacity
  for i in 0..LRU_CAPACITY {
    let input = IdInput::new(&db, i);
    let result = identity(&db, input);
    assert_eq!(result.value(&db), i);
  }
  take_log();

  // Accessing a cached entry should not recompute
  let input_0 = IdInput::new(&db, 0);
  let result = identity(&db, input_0);
  assert_eq!(result.value(&db), 0);
  let log = take_log();
  assert!(
    log.is_empty(),
    "expected cache hit, got recomputation: {log:?}"
  );

  // Overflow the LRU (eviction deferred until revision bump)
  let overflow = IdInput::new(&db, LRU_CAPACITY);
  identity(&db, overflow);
  take_log();

  // Trigger revision bump which processes evictions
  db.storage.reset_for_new_revision();

  // identity(1) was evicted (oldest after 0 was touched), accessing it should recompute
  let input_1 = IdInput::new(&db, 1);
  let result = identity(&db, input_1);
  assert_eq!(result.value(&db), 1);
  let log = take_log();
  assert_eq!(log, vec![1], "expected recomputation of identity(1)");
}

#[test]
fn serialize_after_identity_map_cleanup() {
  let mut db = Database {
    storage: QueryStorage::default(),
  };

  // Create a range config that produces 5 structs
  let config = RangeConfig::new(&db, 5);
  let result = make_range(&db, config);
  assert_eq!(result.value(&db), 5);

  // Change count to 2, re-execute -> identity map sweep removes 3 structs + their field data
  config.set_count(&mut db, 2);
  let result = make_range(&db, config);
  assert_eq!(result.value(&db), 2);

  // Serialization should handle deleted field data gracefully
  let db2 = dump_and_reload(&db, |storage| Database { storage });

  // The surviving result should work after reload
  let config2 = find_entry(RangeConfig::iter(&db2), |c| c.count(&db2) == 2, "config");
  take_log();
  let result2 = make_range(&db2, config2);
  assert_eq!(result2.value(&db2), 2);
}

#[test]
fn serialize_after_lru_eviction() {
  let db = Database {
    storage: QueryStorage::default(),
  };

  // Fill LRU and overflow to trigger eviction
  for i in 0..=LRU_CAPACITY {
    let input = IdInput::new(&db, i);
    identity(&db, input);
  }
  db.storage.reset_for_new_revision();

  // Serialization should handle evicted memos gracefully
  let db2 = dump_and_reload(&db, |storage| Database { storage });

  // Non-evicted entries should still work after reload
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

  // Fill LRU and overflow
  for i in 0..=LRU_CAPACITY {
    let input = IdInput::new(&db, i);
    identity(&db, input);
  }
  db.storage.reset_for_new_revision();

  // Roundtrip
  let db2 = dump_and_reload(&db, |storage| Database { storage });

  // Evicted entry (0) should recompute after reload
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

  // Create a valid entry then forge a tombstone struct
  let input = IdInput::new(&db, 42);
  let result = identity(&db, input);
  assert_eq!(result.value(&db), 42);

  // Create a struct with TOMBSTONE_ENTRY_ID
  let tombstone: IdResult = IdResult::from(crate::TOMBSTONE_ENTRY_ID);
  let caught = catch_unwind(std::panic::AssertUnwindSafe(|| {
    tombstone.value(&db);
  }));
  assert!(caught.is_err(), "expected panic on tombstone access");
}
