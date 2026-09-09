use super::fixtures::fibonacci::*;
use super::fixtures::identity;
use identity::Database as IdDatabase;

// Interned entries can:
// 1. Survive a dump
// 2. Load again with the same value
#[test]
fn interned_entries_preserved() {
  let db1 = Database {
    storage: QueryStorage::default(),
  };
  for n in 0..=5 {
    FibInput::new(&db1, n);
  }

  let db2 = dump_and_reload(&db1, |storage| Database { storage });

  let inputs: Vec<FibInput> = FibInput::iter(&db2);
  assert_eq!(inputs.len(), 6);
  for n in 0..=5usize {
    assert!(inputs.iter().any(|i| i.n(&db2) == n));
  }
}

// Derived query results are loaded from cache with no recomputation
#[test]
fn no_recomputation() {
  let db1 = Database {
    storage: QueryStorage::default(),
  };
  let input = FibInput::new(&db1, 10);
  let result = fibonacci(&db1, input);
  assert_eq!(result.value(&db1), 55);

  let db2 = dump_and_reload(&db1, |storage| Database { storage });
  let input10 = find_entry(FibInput::iter(&db2), |i| i.n(&db2) == 10, "FibInput(n=10)");

  take_log();
  let result2 = fibonacci(&db2, input10);
  let log = take_log();

  assert_eq!(result2.value(&db2), 55);
  assert!(log.is_empty(), "no recomputation expected, got: {:?}", log);
}

// Derived struct field values (both #[id] and non-id) are preserved
#[test]
fn derived_field_values() {
  let db1 = Database {
    storage: QueryStorage::default(),
  };
  let input = FibInput::new(&db1, 7);
  let result = fibonacci(&db1, input);
  assert_eq!(result.n(&db1), 7);
  assert_eq!(result.value(&db1), 13);

  let db2 = dump_and_reload(&db1, |storage| Database { storage });
  let input7 = find_entry(FibInput::iter(&db2), |i| i.n(&db2) == 7, "FibInput(n=7)");

  take_log();
  let result2 = fibonacci(&db2, input7);
  let log = take_log();

  assert_eq!(result2.n(&db2), 7);
  assert_eq!(result2.value(&db2), 13);
  assert!(log.is_empty(), "no recomputation expected, got: {:?}", log);
}

// All subqueries (fib(0)..fib(n)) are cached, not just the top-level one
#[test]
fn all_subqueries_cached() {
  let db1 = Database {
    storage: QueryStorage::default(),
  };
  fibonacci(&db1, FibInput::new(&db1, 5));

  let db2 = dump_and_reload(&db1, |storage| Database { storage });

  take_log();
  for n in 0..=5usize {
    fibonacci(
      &db2,
      find_entry(
        FibInput::iter(&db2),
        |i| i.n(&db2) == n,
        format!("FibInput(n={})", n),
      ),
    );
  }
  let log = take_log();
  assert!(
    log.is_empty(),
    "all subqueries should be cached, got: {:?}",
    log
  );
}

// Cache survives two consecutive roundtrips
// dump -> load -> dump -> load
#[test]
fn double_roundtrip() {
  let db1 = Database {
    storage: QueryStorage::default(),
  };
  fibonacci(&db1, FibInput::new(&db1, 6));

  let db2 = dump_and_reload(&db1, |storage| Database { storage });
  fibonacci(
    &db2,
    find_entry(FibInput::iter(&db2), |i| i.n(&db2) == 6, "FibInput(n=6)"),
  );

  let db3 = dump_and_reload(&db2, |storage| Database { storage });

  take_log();
  let result = fibonacci(
    &db3,
    find_entry(FibInput::iter(&db3), |i| i.n(&db3) == 6, "FibInput(n=6)"),
  );
  let log = take_log();

  assert_eq!(result.value(&db3), 8);
  assert!(
    log.is_empty(),
    "no recomputation after double roundtrip, got: {:?}",
    log
  );
}

// Empty database roundtrips without error
#[test]
fn empty_database() {
  let db1 = Database {
    storage: QueryStorage::default(),
  };
  let db2 = dump_and_reload(&db1, |storage| Database { storage });
  let inputs: Vec<FibInput> = FibInput::iter(&db2);
  assert!(inputs.is_empty());
}

// Base case queries (fib(0), fib(1)) roundtrip correctly
#[test]
fn base_cases() {
  let db1 = Database {
    storage: QueryStorage::default(),
  };
  fibonacci(&db1, FibInput::new(&db1, 0));
  fibonacci(&db1, FibInput::new(&db1, 1));

  let db2 = dump_and_reload(&db1, |storage| Database { storage });

  take_log();
  let r0 = fibonacci(
    &db2,
    find_entry(FibInput::iter(&db2), |i| i.n(&db2) == 0, "FibInput(n=0)"),
  );
  let r1 = fibonacci(
    &db2,
    find_entry(FibInput::iter(&db2), |i| i.n(&db2) == 1, "FibInput(n=1)"),
  );
  let log = take_log();

  assert_eq!(r0.value(&db2), 0);
  assert_eq!(r1.value(&db2), 1);
  assert!(
    log.is_empty(),
    "base cases should be cached, got: {:?}",
    log
  );
}

// Derived query returning an interned type roundtrips correctly
#[test]
fn interned_return_type_roundtrip() {
  let db1 = IdDatabase {
    storage: QueryStorage::default(),
  };
  let input = identity::IdInput::new(&db1, 5);
  let pair = identity::make_pair(&db1, input);
  assert_eq!(pair.a(&db1).value(&db1), 5);
  assert_eq!(pair.b(&db1).value(&db1), 6);

  let db2 = identity::dump_and_reload(&db1, |storage| IdDatabase { storage });
  let input5 = identity::find_entry(
    identity::IdInput::iter(&db2),
    |i| i.n(&db2) == 5,
    "IdInput(n=5)",
  );

  identity::take_log();
  let pair2 = identity::make_pair(&db2, input5);
  let log = identity::take_log();

  assert_eq!(pair2.a(&db2).value(&db2), 5);
  assert_eq!(pair2.b(&db2).value(&db2), 6);
  assert!(
    log.is_empty(),
    "interned return should be cached, got: {:?}",
    log
  );
}

// Derived query returning an interned type without lifetime roundtrips correctly
#[test]
fn interned_no_lifetime_return_type_roundtrip() {
  let db1 = IdDatabase {
    storage: QueryStorage::default(),
  };
  let input = identity::IdInput::new(&db1, 3);
  let result = identity::double_input(&db1, input);
  assert_eq!(result.n(&db1), 6);

  let db2 = identity::dump_and_reload(&db1, |storage| IdDatabase { storage });
  let input3 = identity::find_entry(
    identity::IdInput::iter(&db2),
    |i| i.n(&db2) == 3,
    "IdInput(n=3)",
  );

  identity::take_log();
  let result2 = identity::double_input(&db2, input3);
  let log = identity::take_log();

  assert_eq!(result2.n(&db2), 6);
  assert!(
    log.is_empty(),
    "interned no-lifetime return should be cached, got: {:?}",
    log
  );
}
