// Promote: entries not accessed in the current session must survive into the next dump

use super::fixtures::identity;
use identity::{Database as IdDb, InputId, InternedId, QueryStorage};

// An entry that was never accessed in the current session must still appear in the next db
#[test]
fn unaccessed_entry_survives_passthrough_session() {
  // Session 1: compute identity(7)
  let db1 = IdDb {
    storage: QueryStorage::default(),
  };
  let input = identity::IdInput::new(&db1, 7);
  identity::identity(&db1, input);

  // Session 2: load but never touch identity(7), then dump again
  let db2 = identity::dump_and_reload(&db1, |s| IdDb { storage: s });
  let db3 = identity::dump_and_reload(&db2, |s| IdDb { storage: s });

  // Session 3: identity(7) must still be cached
  let input3 = identity::find_entry(
    identity::IdInput::iter(&db3),
    |i| i.n(&db3) == 7,
    "IdInput(7)",
  );
  identity::take_log();
  identity::identity(&db3, input3);
  let log = identity::take_log();

  assert!(
    log.is_empty(),
    "unaccessed entry must survive passthrough session: {log:?}"
  );
}

// All subqueries in a chain must survive a passthrough session
#[test]
fn all_subqueries_survive_passthrough_session() {
  let db1 = IdDb {
    storage: QueryStorage::default(),
  };
  let config = identity::VersionConfig::new(&db1, 1);
  identity::root_query_versioned_result(&db1, config);

  let db2 = identity::dump_and_reload(&db1, |s| IdDb { storage: s });
  // Access nothing in session 2
  let db3 = identity::dump_and_reload(&db2, |s| IdDb { storage: s });

  let config3 = identity::find_entry(
    identity::VersionConfig::iter(&db3),
    |c| c.version(&db3) == 1,
    "VersionConfig(1)",
  );
  identity::take_log();
  identity::root_query_versioned_result(&db3, config3);
  let log = identity::take_log();

  assert!(
    log.is_empty(),
    "all subqueries must survive passthrough: {log:?}"
  );
}

// Three consecutive sessions with no mutations must never recompute
#[test]
fn three_sessions_no_recomputation() {
  let db1 = IdDb {
    storage: QueryStorage::default(),
  };
  let config = identity::VersionConfig::new(&db1, 2);
  identity::root_query_versioned_result(&db1, config);

  let db2 = identity::dump_and_reload(&db1, |s| IdDb { storage: s });
  let db3 = identity::dump_and_reload(&db2, |s| IdDb { storage: s });
  let db4 = identity::dump_and_reload(&db3, |s| IdDb { storage: s });

  let config4 = identity::find_entry(
    identity::VersionConfig::iter(&db4),
    |c| c.version(&db4) == 2,
    "VersionConfig(2)",
  );
  identity::take_log();
  identity::root_query_versioned_result(&db4, config4);
  let log = identity::take_log();

  assert!(
    log.is_empty(),
    "three-session chain must not recompute: {log:?}"
  );
}

// A mutated value must be cached after the mutation session dumps
#[test]
fn mutated_value_is_cached_in_next_session() {
  let db1 = IdDb {
    storage: QueryStorage::default(),
  };
  let config = identity::VersionConfig::new(&db1, 1);
  identity::root_query_versioned_result(&db1, config);

  // Session 2: mutate and recompute
  let mut db2 = identity::dump_and_reload(&db1, |s| IdDb { storage: s });
  let config2 = identity::find_entry(
    identity::VersionConfig::iter(&db2),
    |c| c.version(&db2) == 1,
    "VersionConfig(1)",
  );
  config2.set_version(&mut db2, 99);
  let result2 = identity::root_query_versioned_result(&db2, config2);
  assert_eq!(result2.value(&db2), 99);

  // Session 3: mutated result must be cached
  let db3 = identity::dump_and_reload(&db2, |s| IdDb { storage: s });
  let config3 = identity::find_entry(
    identity::VersionConfig::iter(&db3),
    |c| c.version(&db3) == 99,
    "VersionConfig(99)",
  );
  identity::take_log();
  let result3 = identity::root_query_versioned_result(&db3, config3);
  let log = identity::take_log();

  assert_eq!(result3.value(&db3), 99);
  assert!(
    log.is_empty(),
    "mutated value must be cached in next session: {log:?}"
  );
}
