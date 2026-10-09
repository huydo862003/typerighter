use super::fixtures::fibonacci::*;
use crate::SerializableQueryDatabase;

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
