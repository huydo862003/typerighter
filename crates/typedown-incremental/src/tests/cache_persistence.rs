// Cache persistence: values survive a dump+reload roundtrip and are served from cache

use super::fixtures::fibonacci::*;
use super::fixtures::identity;
use identity::{Database as IdDb, InternedId, QueryStorage};

// Interned inputs serialized in db1 must be queryable in db2
#[test]
fn interned_entries_preserved_across_roundtrip() {
  let db1 = IdDb {
    storage: QueryStorage::default(),
  };
  let input = identity::IdInput::new(&db1, 7);
  identity::identity(&db1, input);

  let db2 = identity::dump_and_reload(&db1, |s| IdDb { storage: s });
  let input2 = identity::find_entry(
    identity::IdInput::iter(&db2),
    |i| i.n(&db2) == 7,
    "IdInput(7)",
  );
  assert_eq!(input2.n(&db2), 7);
}

// Derived results cached in db1 must not trigger recomputation in db2
#[test]
fn no_recomputation_after_roundtrip() {
  let db1 = IdDb {
    storage: QueryStorage::default(),
  };
  let input = identity::IdInput::new(&db1, 3);
  identity::identity(&db1, input);

  let db2 = identity::dump_and_reload(&db1, |s| IdDb { storage: s });
  let input2 = identity::find_entry(
    identity::IdInput::iter(&db2),
    |i| i.n(&db2) == 3,
    "IdInput(3)",
  );

  identity::take_log();
  identity::identity(&db2, input2);
  let log = identity::take_log();

  assert!(
    log.is_empty(),
    "must not recompute cached result after roundtrip: {log:?}"
  );
}

// Field values on derived structs must match after roundtrip
#[test]
fn derived_field_values_preserved_across_roundtrip() {
  let db1 = IdDb {
    storage: QueryStorage::default(),
  };
  let input = identity::IdInput::new(&db1, 42);
  let result1 = identity::identity(&db1, input);
  assert_eq!(result1.value(&db1), 42);

  let db2 = identity::dump_and_reload(&db1, |s| IdDb { storage: s });
  let input2 = identity::find_entry(
    identity::IdInput::iter(&db2),
    |i| i.n(&db2) == 42,
    "IdInput(42)",
  );
  let result2 = identity::identity(&db2, input2);

  assert_eq!(result2.value(&db2), 42);
  assert_eq!(result2.n(&db2), 42);
}

// All subqueries in a recursive chain must be cached after roundtrip
#[test]
fn all_subqueries_cached_after_roundtrip() {
  let db1 = Database {
    storage: QueryStorage::default(),
  };
  let input = FibInput::new(&db1, 5);
  fibonacci(&db1, input);

  let db2 = dump_and_reload(&db1, |s| Database { storage: s });

  let input2 = find_entry(FibInput::iter(&db2), |i| i.n(&db2) == 5, "FibInput(5)");
  take_log();
  fibonacci(&db2, input2);
  let log = take_log();

  assert!(
    log.is_empty(),
    "all fib subqueries must be cached after roundtrip: {log:?}"
  );
}

// Two back-to-back roundtrips must both preserve the cache
#[test]
fn double_roundtrip_preserves_cache() {
  let db1 = IdDb {
    storage: QueryStorage::default(),
  };
  let input = identity::IdInput::new(&db1, 9);
  identity::identity(&db1, input);

  let db2 = identity::dump_and_reload(&db1, |s| IdDb { storage: s });
  let db3 = identity::dump_and_reload(&db2, |s| IdDb { storage: s });

  let input3 = identity::find_entry(
    identity::IdInput::iter(&db3),
    |i| i.n(&db3) == 9,
    "IdInput(9)",
  );
  identity::take_log();
  identity::identity(&db3, input3);
  let log = identity::take_log();

  assert!(log.is_empty(), "cache must survive two roundtrips: {log:?}");
}

// An empty database must round-trip without panicking
#[test]
fn empty_database_roundtrip() {
  let db1 = IdDb {
    storage: QueryStorage::default(),
  };
  let db2 = identity::dump_and_reload(&db1, |s| IdDb { storage: s });
  // Verify the db is usable after loading from an empty dump
  let input = identity::IdInput::new(&db2, 1);
  let result = identity::identity(&db2, input);
  assert_eq!(result.value(&db2), 1);
}

// Base case fibonacci values must be cached after roundtrip
#[test]
fn fibonacci_base_cases_cached_after_roundtrip() {
  let db1 = Database {
    storage: QueryStorage::default(),
  };
  let i0 = FibInput::new(&db1, 0);
  let i1 = FibInput::new(&db1, 1);
  fibonacci(&db1, i0);
  fibonacci(&db1, i1);

  let db2 = dump_and_reload(&db1, |s| Database { storage: s });
  let i0b = find_entry(FibInput::iter(&db2), |i| i.n(&db2) == 0, "FibInput(0)");
  let i1b = find_entry(FibInput::iter(&db2), |i| i.n(&db2) == 1, "FibInput(1)");

  take_log();
  fibonacci(&db2, i0b);
  fibonacci(&db2, i1b);
  let log = take_log();

  assert!(
    log.is_empty(),
    "base cases must be cached after roundtrip: {log:?}"
  );
}

// Derived query returning an interned type with lifetime must round-trip correctly
#[test]
fn interned_ret_type_roundtrip() {
  let db1 = IdDb {
    storage: QueryStorage::default(),
  };
  let input = identity::IdInput::new(&db1, 4);
  let pair = identity::make_pair(&db1, input);
  assert_eq!(pair.a(&db1).value(&db1), 4);

  let db2 = identity::dump_and_reload(&db1, |s| IdDb { storage: s });
  let input2 = identity::find_entry(
    identity::IdInput::iter(&db2),
    |i| i.n(&db2) == 4,
    "IdInput(4)",
  );

  identity::take_log();
  let pair2 = identity::make_pair(&db2, input2);
  let log = identity::take_log();

  assert!(
    log.is_empty(),
    "make_pair must be cached after roundtrip: {log:?}"
  );
  assert_eq!(pair2.a(&db2).value(&db2), 4);
}

// Derived query returning an interned type without lifetime must round-trip correctly
#[test]
fn interned_no_lifetime_ret_type_roundtrip() {
  let db1 = IdDb {
    storage: QueryStorage::default(),
  };
  let input = identity::IdInput::new(&db1, 6);
  let doubled = identity::double_input(&db1, input);
  assert_eq!(doubled.n(&db1), 12);

  let db2 = identity::dump_and_reload(&db1, |s| IdDb { storage: s });
  let input2 = identity::find_entry(
    identity::IdInput::iter(&db2),
    |i| i.n(&db2) == 6,
    "IdInput(6)",
  );

  identity::take_log();
  let doubled2 = identity::double_input(&db2, input2);
  let log = identity::take_log();

  assert!(
    log.is_empty(),
    "double_input must be cached after roundtrip: {log:?}"
  );
  assert_eq!(doubled2.n(&db2), 12);
}
