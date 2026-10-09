// Backdating: upstream re-executes but unchanged value must not propagate to downstream
// Also covers the opposite: changed upstream must reach downstream

use super::fixtures::identity;
use identity::{Database as IdDb, QueryStorage};

// Compute upstream and downstream in the same session, bump an unrelated input,
// then verify downstream stays cached without re-executing
#[test]
fn downstream_cached_when_upstream_unchanged_after_revision_bump() {
  let mut db = IdDb {
    storage: QueryStorage::default(),
  };
  let config = identity::VersionConfig::new(&db, 1);
  identity::root_query_versioned_result(&db, config);
  identity::downstream_query_of_versioned(&db, config);

  // Bump revision via an unrelated input
  let other = identity::RangeConfig::new(&db, 10);
  other.set_count(&mut db, 11);

  identity::take_log();
  identity::downstream_query_of_versioned(&db, config);
  let log = identity::take_log();

  assert!(
    log.is_empty(),
    "downstream must stay cached when upstream value unchanged: {log:?}"
  );
}

// Upstream value changes, so downstream must re-execute and log with the new value
#[test]
fn changed_upstream_propagates_to_downstream() {
  let mut db = IdDb {
    storage: QueryStorage::default(),
  };
  let config = identity::VersionConfig::new(&db, 1);
  identity::root_query_versioned_result(&db, config);
  identity::downstream_query_of_versioned(&db, config);

  config.set_version(&mut db, 2);

  identity::take_log();
  let result = identity::downstream_query_of_versioned(&db, config);
  let log = identity::take_log();

  // downstream logs v + 1000 so log entry 1002 means it re-executed with value 2
  assert_eq!(result.value(&db), 2);
  assert!(
    log.contains(&1002),
    "downstream must re-execute when upstream value changes: {log:?}"
  );
}

// Multiple unrelated revision bumps must not accumulate into a recomputation
#[test]
fn downstream_stays_cached_across_multiple_revision_bumps() {
  let mut db = IdDb {
    storage: QueryStorage::default(),
  };
  let config = identity::VersionConfig::new(&db, 5);
  identity::root_query_versioned_result(&db, config);
  identity::downstream_query_of_versioned(&db, config);

  let other = identity::RangeConfig::new(&db, 1);
  for count in 2..=5 {
    other.set_count(&mut db, count);
  }

  identity::take_log();
  identity::downstream_query_of_versioned(&db, config);
  let log = identity::take_log();

  assert!(
    log.is_empty(),
    "downstream must stay cached after repeated unrelated bumps: {log:?}"
  );
}

// Calling downstream twice in the same revision after an upstream mutation
// must execute downstream exactly once
#[test]
fn downstream_executes_once_per_revision_after_upstream_change() {
  let mut db = IdDb {
    storage: QueryStorage::default(),
  };
  let config = identity::VersionConfig::new(&db, 1);
  identity::root_query_versioned_result(&db, config);
  identity::downstream_query_of_versioned(&db, config);

  config.set_version(&mut db, 3);

  identity::take_log();
  identity::downstream_query_of_versioned(&db, config);
  identity::downstream_query_of_versioned(&db, config);
  let log = identity::take_log();

  // downstream logs v + 1000 on execution so exactly one entry of 1003
  assert_eq!(
    log.iter().filter(|&&x| x == 1003).count(),
    1,
    "downstream must execute exactly once per revision: {log:?}"
  );
}
