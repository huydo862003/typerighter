// Green/red check: dep changed forces recomputation; unchanged dep stays cached

use super::fixtures::identity;
use identity::{Database as IdDb, InputId, InternedId, QueryStorage};

// Unrelated revision bump must not recompute queries with unchanged deps
#[test]
fn green_check_skips_recomputation_when_deps_unchanged() {
  let mut db = IdDb {
    storage: QueryStorage::default(),
  };
  let config = identity::VersionConfig::new(&db, 1);
  identity::root_query_versioned_result(&db, config);

  // Bump revision via an unrelated input
  let other = identity::RangeConfig::new(&db, 10);
  other.set_count(&mut db, 11);

  identity::take_log();
  let result = identity::root_query_versioned_result(&db, config);
  let log = identity::take_log();

  assert_eq!(result.value(&db), 1);
  assert!(log.is_empty(), "must not recompute when deps unchanged: {log:?}");
}

// Changed dep must trigger recomputation
#[test]
fn red_check_recomputes_when_dep_changed() {
  let mut db = IdDb {
    storage: QueryStorage::default(),
  };
  let config = identity::VersionConfig::new(&db, 5);
  identity::root_query_versioned_result(&db, config);

  config.set_version(&mut db, 6);

  identity::take_log();
  let result = identity::root_query_versioned_result(&db, config);
  let log = identity::take_log();

  assert_eq!(result.value(&db), 6);
  assert!(!log.is_empty(), "must recompute when dep changed: log was empty");
}

// Query not depending on the mutated input must stay cached
#[test]
fn independent_query_stays_cached_after_unrelated_mutation() {
  let db1 = IdDb {
    storage: QueryStorage::default(),
  };
  let input5 = identity::IdInput::new(&db1, 5);
  identity::identity(&db1, input5);
  let config = identity::VersionConfig::new(&db1, 1);
  identity::root_query_versioned_result(&db1, config);

  let mut db2 = identity::dump_and_reload(&db1, |s| IdDb { storage: s });
  let input5_2 =
    identity::find_entry(identity::IdInput::iter(&db2), |i| i.n(&db2) == 5, "IdInput(5)");
  // Load from cache so it's accessible for the assertion below
  identity::identity(&db2, input5_2);

  let config2 = identity::find_entry(
    identity::VersionConfig::iter(&db2),
    |c| c.version(&db2) == 1,
    "VersionConfig(1)",
  );
  config2.set_version(&mut db2, 99);

  identity::take_log();
  identity::identity(&db2, input5_2);
  assert!(
    identity::take_log().is_empty(),
    "identity(5) must be unaffected by unrelated mutation"
  );
}

// Mutated input after a roundtrip must recompute the dependent query
#[test]
fn mutation_after_roundtrip_causes_recomputation() {
  let db1 = IdDb {
    storage: QueryStorage::default(),
  };
  let config = identity::VersionConfig::new(&db1, 1);
  identity::root_query_versioned_result(&db1, config);

  let mut db2 = identity::dump_and_reload(&db1, |s| IdDb { storage: s });
  let config2 = identity::find_entry(
    identity::VersionConfig::iter(&db2),
    |c| c.version(&db2) == 1,
    "VersionConfig(1)",
  );

  // Confirm cached before mutation
  identity::take_log();
  identity::root_query_versioned_result(&db2, config2);
  assert!(identity::take_log().is_empty(), "must be cached before mutation");

  config2.set_version(&mut db2, 2);

  identity::take_log();
  let result = identity::root_query_versioned_result(&db2, config2);
  let log = identity::take_log();

  assert_eq!(result.value(&db2), 2);
  assert!(!log.is_empty(), "must recompute after mutation: log was empty");
}
