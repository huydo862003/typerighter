use crate::{LRU_CAPACITY, Lru};

use super::fixtures::identity::*;

#[test]
fn touch_returns_no_evictions_under_capacity() {
  let lru = Lru::default();
  for i in 0..100 {
    assert!(lru.touch(i).is_empty());
  }
}

#[test]
fn touch_evicts_oldest_when_over_capacity() {
  let lru = Lru::default();
  for i in 0..LRU_CAPACITY {
    assert!(lru.touch(i).is_empty());
  }
  let evicted = lru.touch(LRU_CAPACITY);
  assert_eq!(evicted, vec![0]);
}

#[test]
fn touch_moves_to_back() {
  let lru = Lru::default();
  for i in 0..LRU_CAPACITY {
    lru.touch(i);
  }
  lru.touch(0);
  let evicted = lru.touch(LRU_CAPACITY);
  assert_eq!(evicted, vec![1]);
}

#[test]
fn duplicate_touch_does_not_grow() {
  let lru = Lru::default();
  for _ in 0..2000 {
    assert!(lru.touch(42).is_empty());
  }
}

#[test]
fn evicted_memo_recomputes_on_access() {
  let db = Database {
    storage: QueryStorage::default(),
  };

  // Fill LRU to capacity with identity(0) through identity(LRU_CAPACITY - 1)
  for i in 0..LRU_CAPACITY {
    let input = IdInput::new(&db, i);
    let result = identity(&db, input);
    assert_eq!(result.value(&db), i);
  }
  take_log(); // clear

  // Accessing a cached entry should not recompute
  let input_0 = IdInput::new(&db, 0);
  let result = identity(&db, input_0);
  assert_eq!(result.value(&db), 0);
  let log = take_log();
  assert!(
    log.is_empty(),
    "expected cache hit, got recomputation: {log:?}"
  );

  // Add one more to evict the oldest (1, since 0 was just touched)
  let overflow = IdInput::new(&db, LRU_CAPACITY);
  identity(&db, overflow);
  take_log(); // clear the computation of the overflow entry

  // Now identity(1) was evicted, accessing it should recompute
  let input_1 = IdInput::new(&db, 1);
  let result = identity(&db, input_1);
  assert_eq!(result.value(&db), 1);
  let log = take_log();
  assert_eq!(log, vec![1], "expected recomputation of identity(1)");
}
