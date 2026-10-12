// Cache persistence: values survive a dump + reload roundtrip and are served from cache

use super::fixtures::fibonacci::*;
use super::fixtures::identity;
use crate::InputId;
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
fn interned_ret_typ_roundtrip() {
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

// outer_query depends on a DerivedField<IdResult> via wrapper.inner(db)
// The cross-session fingerprint check must use the stored fingerprint
// IdResult sub-fields may not be deserialized yet when the check runs
#[test]
fn derived_field_containing_derived_struct_cached_after_roundtrip() {
  let db1 = IdDb {
    storage: QueryStorage::default(),
  };
  let input = identity::IdInput::new(&db1, 5);
  let result1 = identity::outer_query(&db1, input);
  assert_eq!(result1.value(&db1), 5);

  let db2 = identity::dump_and_reload(&db1, |s| IdDb { storage: s });
  let input2 = identity::find_entry(
    identity::IdInput::iter(&db2),
    |i| i.n(&db2) == 5,
    "IdInput(5)",
  );

  identity::take_log();
  let result2 = identity::outer_query(&db2, input2);
  let log = identity::take_log();

  assert_eq!(result2.value(&db2), 5);
  assert!(
    log.is_empty(),
    "outer_query must not recompute after roundtrip: {log:?}"
  );
}

// outer_query must survive two back-to-back roundtrips
// Each roundtrip re-exercises the DerivedField<IdResult> fingerprint check
#[test]
fn derived_field_containing_derived_struct_double_roundtrip() {
  let db1 = IdDb {
    storage: QueryStorage::default(),
  };
  let input = identity::IdInput::new(&db1, 8);
  identity::outer_query(&db1, input);

  let db2 = identity::dump_and_reload(&db1, |s| IdDb { storage: s });
  let db3 = identity::dump_and_reload(&db2, |s| IdDb { storage: s });

  let input3 = identity::find_entry(
    identity::IdInput::iter(&db3),
    |i| i.n(&db3) == 8,
    "IdInput(8)",
  );

  identity::take_log();
  identity::outer_query(&db3, input3);
  let log = identity::take_log();

  assert!(
    log.is_empty(),
    "outer_query must not recompute after two roundtrips: {log:?}"
  );
}

// Query keyed by an interned value whose fields are derived structs must be cached after roundtrip
// The key fingerprint stable_hash(pair, db) traverses pair.a and pair.b (IdResult values)
// IdResult sub-fields must be in memory for the hash to be correct
// Known limitation: identity must be called first to load those sub-fields
#[test]
fn interned_key_with_derived_struct_fields_cached_after_roundtrip() {
  let db1 = IdDb {
    storage: QueryStorage::default(),
  };
  let input = identity::IdInput::new(&db1, 6);
  let pair = identity::make_pair(&db1, input);
  let result1 = identity::use_pair(&db1, pair);
  assert_eq!(result1.value(&db1), 6);

  let db2 = identity::dump_and_reload(&db1, |s| IdDb { storage: s });
  let input2 = identity::find_entry(
    identity::IdInput::iter(&db2),
    |i| i.n(&db2) == 6,
    "IdInput(6)",
  );

  // Loads IdResult(6) sub-fields so stable_hash(pair, db2) produces a correct key fingerprint
  // Without this, unloaded DerivedField deps hash as None, producing a wrong key
  identity::identity(&db2, input2);
  let pair2 = identity::make_pair(&db2, input2);

  identity::take_log();
  let result2 = identity::use_pair(&db2, pair2);
  let log = identity::take_log();

  assert_eq!(result2.value(&db2), 6);
  assert!(
    log.is_empty(),
    "use_pair must not recompute after roundtrip: {log:?}"
  );
}

// After a stale dep forces re-execution, the re-executed query and the cached downstream
// must return the same IdResult entry ID.
#[test]
fn reused_entry_ids_after_cross_session_stale_dep_recompute() {
  let db1 = IdDb {
    storage: QueryStorage::default(),
  };
  let _vc = identity::VersionConfig::new(&db1, 1);
  let input = identity::IdInput::new(&db1, 5);
  identity::versioned_identity(&db1, input);
  identity::versioned_identity_downstream(&db1, input);

  let mut db2 = identity::dump_and_reload(&db1, |s| IdDb { storage: s });

  let vc2 = identity::find_entry(
    identity::VersionConfig::iter(&db2),
    |v| v.version(&db2) == 1,
    "VersionConfig(1)",
  );
  vc2.set_version(&mut db2, 2);

  let input2 = identity::find_entry(
    identity::IdInput::iter(&db2),
    |i| i.n(&db2) == 5,
    "IdInput(5)",
  );

  // versioned_identity re-executes (dep mismatch), cached_downstream stays cached via backdating
  let recomputed = identity::versioned_identity(&db2, input2);
  let cached = identity::versioned_identity_downstream(&db2, input2);

  let recomputed_id: u32 = recomputed.into();
  let cached_id: u32 = cached.into();

  assert_eq!(
    recomputed_id, cached_id,
    "re-executed query must reuse the same entry ID as cached downstream: recomputed={recomputed_id}, cached={cached_id}"
  );
}

// When nothing changes between sessions, both a query and its cached downstream must return
// the exact same entry ID. No recomputation, no ID drift.
// Mirrors the integration test: zero_spurious_errors (no mutations between sessions).
#[test]
fn no_id_drift_after_unchanged_roundtrip() {
  let db1 = IdDb {
    storage: QueryStorage::default(),
  };
  let _vc = identity::VersionConfig::new(&db1, 1);
  let input = identity::IdInput::new(&db1, 5);
  let r1 = identity::versioned_identity(&db1, input);
  let r2 = identity::versioned_identity_downstream(&db1, input);
  let id_before: u32 = r1.into();
  let downstream_before: u32 = r2.into();
  assert_eq!(id_before, downstream_before);

  let db2 = identity::dump_and_reload(&db1, |s| IdDb { storage: s });
  let input2 = identity::find_entry(
    identity::IdInput::iter(&db2),
    |i| i.n(&db2) == 5,
    "IdInput(5)",
  );

  identity::take_log();
  let r3 = identity::versioned_identity(&db2, input2);
  let r4 = identity::versioned_identity_downstream(&db2, input2);
  let log = identity::take_log();

  assert!(log.is_empty(), "no recomputation expected: {log:?}");
  let id_after: u32 = r3.into();
  let downstream_after: u32 = r4.into();
  assert_eq!(
    id_after, downstream_after,
    "IDs must match after unchanged roundtrip"
  );
  assert_eq!(r3.n(&db2), 5);
  assert_eq!(r3.value(&db2), 5);
}

// After re-execution due to a stale side dep, the cached downstream result must return
// a struct whose fields are accessible with correct values.
// If the bug exists: re-execution creates a new entry_id, the cached downstream holds the old
// entry_id whose field data is stale or cleaned up -> field access panics or returns wrong value.
// Mirrors the integration test: zero_spurious_errors_after_file_change.
#[test]
fn cached_downstream_fields_accessible_after_stale_dep_recompute() {
  let db1 = IdDb {
    storage: QueryStorage::default(),
  };
  let _vc = identity::VersionConfig::new(&db1, 1);
  let input = identity::IdInput::new(&db1, 5);
  identity::versioned_identity(&db1, input);
  identity::versioned_identity_downstream(&db1, input);

  let mut db2 = identity::dump_and_reload(&db1, |s| IdDb { storage: s });
  let vc2 = identity::find_entry(
    identity::VersionConfig::iter(&db2),
    |v| v.version(&db2) == 1,
    "VersionConfig(1)",
  );
  vc2.set_version(&mut db2, 2);
  let input2 = identity::find_entry(
    identity::IdInput::iter(&db2),
    |i| i.n(&db2) == 5,
    "IdInput(5)",
  );

  // versioned_identity re-executes (dep mismatch on version)
  // versioned_identity_downstream stays cached via backdating (same IdResult value fingerprint)
  let cached = identity::versioned_identity_downstream(&db2, input2);

  // Field access must work correctly, not panic or return stale data
  assert_eq!(cached.n(&db2), 5, "n must be 5 after stale dep recompute");
  assert_eq!(
    cached.value(&db2),
    5,
    "value must be 5 after stale dep recompute"
  );
}

// Derived query returning an interned type without lifetime must round-trip correctly
#[test]
fn interned_no_lifetime_ret_typ_roundtrip() {
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
