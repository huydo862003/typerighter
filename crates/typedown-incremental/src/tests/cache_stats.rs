use super::fixtures::fibonacci::*;
use super::fixtures::identity;
use crate::{LRU_CAPACITY, SerializableQueryDatabase};
use identity::{Database as IdDb, InternedId, QueryStorage};

// Cache stats are deterministic across runs
#[test]
fn fib_cache_stats_are_stable() {
  let make_stats = || {
    let db = Database {
      storage: QueryStorage::default(),
    };
    for n in 0..=5 {
      fibonacci(&db, FibInput::new(&db, n));
    }
    db.dump().stats()
  };

  let stats1 = make_stats();
  let stats2 = make_stats();
  assert_eq!(stats1, stats2, "stats should be identical across runs");
}

// Sanity check on fib(0..=5) cache size
#[test]
fn fib_cache_stats_are_reasonable() {
  let db = Database {
    storage: QueryStorage::default(),
  };
  for n in 0..=5 {
    fibonacci(&db, FibInput::new(&db, n));
  }
  let stats = db.dump().stats();

  // fib(0)..fib(5) = 6 derived queries
  assert!(
    stats.derived_queries >= 6,
    "derived_queries={}",
    stats.derived_queries
  );
  // FibInput is interned, 6 entries
  assert!(stats.interned >= 6, "interned={}", stats.interned);
}

// Recompute count must be zero after roundtrip: cache reuse regression guard
#[test]
#[cfg(debug_assertions)]
fn no_recomputation_after_roundtrip_recompute_count() {
  let db1 = Database {
    storage: QueryStorage::default(),
  };
  for n in 0..=5 {
    fibonacci(&db1, FibInput::new(&db1, n));
  }

  let db2 = dump_and_reload(&db1, |s| Database { storage: s });
  for n in 0..=5 {
    let input = find_entry(FibInput::iter(&db2), |i| i.n(&db2) == n, "FibInput");
    fibonacci(&db2, input);
  }

  assert_eq!(
    db2.storage.total_recompute_count(),
    0,
    "no query must recompute after roundtrip"
  );
}

// Dump stats must not grow across a passthrough session (no new queries run)
#[test]
fn dump_stats_stable_across_passthrough_session() {
  let db1 = Database {
    storage: QueryStorage::default(),
  };
  for n in 0..=5 {
    fibonacci(&db1, FibInput::new(&db1, n));
  }
  let stats1 = db1.dump().stats();

  // Session 2: passthrough, no new queries
  let db2 = dump_and_reload(&db1, |s| Database { storage: s });
  let stats2 = db2.dump().stats();

  assert_eq!(
    stats1, stats2,
    "dump stats must not grow across a passthrough session"
  );
}

// Promoted entries from a previous session must not push dump size past LRU_CAPACITY
// The cap in promote_cached prevents unbounded growth across many passthrough sessions
#[test]
fn dump_promote_cached_capped_at_lru_capacity() {
  // Session 1: compute LRU_CAPACITY + extra entries
  let db1 = IdDb {
    storage: QueryStorage::default(),
  };
  for i in 0..=(LRU_CAPACITY + 50) {
    identity::identity(&db1, identity::IdInput::new(&db1, i));
  }

  // Session 2: load but access nothing, then dump
  let db2 = identity::dump_and_reload(&db1, |s| IdDb { storage: s });
  let stats2 = db2.dump().stats();

  assert!(
    stats2.derived_queries <= LRU_CAPACITY,
    "carry-forward must not exceed LRU_CAPACITY: got {} derived queries",
    stats2.derived_queries
  );
}
