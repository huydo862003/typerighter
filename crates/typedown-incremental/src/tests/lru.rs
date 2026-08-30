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
