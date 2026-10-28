// Struct entry ID stability across roundtrips and drain

use std::sync::Arc;

use super::fixtures::identity;
use crate::{Id, InputId, InternedId, SerializableQueryDatabase, tests::fixtures::identity::{IdInput, RangeConfig, VersionConfig}};
use identity::{Database as IdDatabase, QueryStorage};

#[test]
fn derived_struct_identity_preserved_across_roundtrip() {
  let db1 = IdDatabase {
    storage: QueryStorage::default(),
  };
  let config = VersionConfig::new(&db1, 1);
  let result1 = identity::root_query_versioned_result(&db1, config);
  assert_eq!(result1.value(&db1), 1);

  let mut db2 = identity::dump_and_reload(&db1, |storage| IdDatabase { storage });
  let config2 = identity::find_entry(
    VersionConfig::iter(&db2),
    |config| config.version(&db2) == 1,
    "VersionConfig(version=1)",
  );

  let cached = identity::root_query_versioned_result(&db2, config2);
  let id_before_reexecution = cached.as_id().entry_id();

  config2.set_version(&mut db2, 2);
  identity::take_log();
  let result2 = identity::root_query_versioned_result(&db2, config2);
  let log = identity::take_log();
  assert!(!log.is_empty(), "versioned_result should have re-executed");

  let id_after_reexecution = result2.as_id().entry_id();
  assert_eq!(
    id_before_reexecution, id_after_reexecution,
    "re-executed struct should reuse the deserialized entry ID"
  );
  assert_eq!(result2.n(&db2), 0);
  assert_eq!(result2.value(&db2), 2);
}

#[test]
fn multiple_derived_structs_preserve_identity_across_roundtrip() {
  let db1 = IdDatabase {
    storage: QueryStorage::default(),
  };
  let config = RangeConfig::new(&db1, 3);
  identity::make_range(&db1, config);

  let mut db2 = identity::dump_and_reload(&db1, |storage| IdDatabase { storage });
  let config2 = identity::find_entry(
    RangeConfig::iter(&db2),
    |config| config.count(&db2) == 3,
    "RangeConfig(count=3)",
  );

  identity::make_range(&db2, config2);

  config2.set_count(&mut db2, 5);
  let result = identity::make_range(&db2, config2);
  assert_eq!(result.value(&db2), 5);

  let db3 = identity::dump_and_reload(&db2, |storage| IdDatabase { storage });
  let config3 = identity::find_entry(
    RangeConfig::iter(&db3),
    |config| config.count(&db3) == 5,
    "RangeConfig(count=5)",
  );
  let result3 = identity::make_range(&db3, config3);
  assert_eq!(result3.value(&db3), 5);
}

#[test]
fn no_id_fields_preserve_identity_across_roundtrip() {
  let db1 = IdDatabase {
    storage: QueryStorage::default(),
  };
  let config = identity::VersionConfig::new(&db1, 1);
  let result1 = identity::make_opaques(&db1, config);
  assert_eq!(result1.tag(&db1), 1);
  assert_eq!(result1.name(&db1), "last");

  let mut db2 = identity::dump_and_reload(&db1, |storage| IdDatabase { storage });
  let config2 = identity::find_entry(
    VersionConfig::iter(&db2),
    |config| config.version(&db2) == 1,
    "VersionConfig(version=1)",
  );

  let cached = identity::make_opaques(&db2, config2);
  let id_before = cached.as_id().entry_id();

  config2.set_version(&mut db2, 2);
  identity::take_log();
  let result2 = identity::make_opaques(&db2, config2);
  let log = identity::take_log();
  assert!(!log.is_empty(), "make_opaques should have re-executed");

  let id_after = result2.as_id().entry_id();
  assert_eq!(
    id_before, id_after,
    "no-id-field struct should preserve entry ID by disambiguator across roundtrip"
  );
  assert_eq!(result2.name(&db2), "last");
  assert_eq!(result2.tag(&db2), 2);
}

// Drain: query creates fewer structs, stale ones must be cleaned up
#[test]
fn drain_removes_stale_structs_after_roundtrip() {
  let db1 = IdDatabase {
    storage: QueryStorage::default(),
  };
  let config = identity::RangeConfig::new(&db1, 5);
  let result1 = identity::make_range(&db1, config);
  assert_eq!(result1.value(&db1), 5);

  let mut db2 = identity::dump_and_reload(&db1, |storage| IdDatabase { storage });
  let config2 = identity::find_entry(
    RangeConfig::iter(&db2),
    |config| config.count(&db2) == 5,
    "RangeConfig(count=5)",
  );

  identity::make_range(&db2, config2);

  config2.set_count(&mut db2, 2);
  let result2 = identity::make_range(&db2, config2);
  assert_eq!(result2.value(&db2), 2);

  let db3 = identity::dump_and_reload(&db2, |storage| IdDatabase { storage });
  let config3 = identity::find_entry(
    RangeConfig::iter(&db3),
    |config| config.count(&db3) == 2,
    "RangeConfig(count=2)",
  );
  let result3 = identity::make_range(&db3, config3);
  assert_eq!(result3.value(&db3), 2);
}

// After drain, serialization must not include removed structs' field data
#[test]
fn drain_shrinks_serialized_field_count() {
  let mut db = IdDatabase {
    storage: QueryStorage::default(),
  };
  let config = identity::RangeConfig::new(&db, 5);
  identity::make_range(&db, config);
  let before = db.dump().stats();

  config.set_count(&mut db, 2);
  identity::make_range(&db, config);
  let after = db.dump().stats();

  assert!(
    after.derived_fields < before.derived_fields,
    "field count must shrink after drain: before={}, after={}",
    before.derived_fields,
    after.derived_fields,
  );
}

// Regression: concurrent deserialize_derived_identity for shared struct entries must not panic
//
// Two sets of queries share IdResult entries in their derived_identities
// After roundtrip, concurrent loading of both sets races on deserialize_derived_identity for the same entries
#[test]
fn concurrent_deserialize_of_shared_derived_identity_does_not_panic() {
  let db1 = IdDatabase {
    storage: QueryStorage::default(),
  };

  // make_range and identity both create IdResult::new(db, i, i) for i in 0..=count
  // They produce the same struct entries (same identity_hash), so both memos list them in derived_identities. After roundtrip, loading these concurrently races on deserialize_derived_identity for each shared IdResult entry
  let count = 50usize;
  let config = RangeConfig::new(&db1, count);
  identity::make_range(&db1, config);
  for i in 0..=count {
    let input = IdInput::new(&db1, i);
    identity::identity(&db1, input);
  }

  let db2 = Arc::new(identity::dump_and_reload(&db1, |storage| IdDatabase {
    storage,
  }));

  std::thread::scope(|thread_scope| {
    let db = Arc::as_ref(&db2);

    // One thread loads make_range (deserialize_derived_identity for IdResult[0..=count])
    thread_scope.spawn(|| {
      let config = identity::find_entry(
        identity::RangeConfig::iter(db),
        |config| config.count(db) == count,
        "RangeConfig",
      );
      let result = identity::make_range(db, config);
      assert_eq!(result.n(db), count);
      assert_eq!(result.value(db), count);
    });

    // Many threads each load identity(input_i), racing on deserialize_derived_identity for IdResult[i] which is also in make_range's derived_identities
    for index in 0..=count {
      thread_scope.spawn(move || {
        let input = identity::find_entry(
          identity::IdInput::iter(db),
          |input| input.n(db) == index,
          format!("IdInput(n={index})"),
        );
        let result = identity::identity(db, input);
        // These reads panic if sibling fields were not fully written before entry_id was returned
        assert_eq!(result.n(db), index);
        assert_eq!(result.value(db), index);
      });
    }
  });
}

// Identity map cleanup must not break serialization
#[test]
fn drain_then_roundtrip_is_clean() {
  let mut db = IdDatabase {
    storage: QueryStorage::default(),
  };
  let config = identity::RangeConfig::new(&db, 5);
  identity::make_range(&db, config);

  config.set_count(&mut db, 2);
  let result = identity::make_range(&db, config);
  assert_eq!(result.value(&db), 2);

  let db2 = identity::dump_and_reload(&db, |storage| IdDatabase { storage });
  let config2 = identity::find_entry(
    identity::RangeConfig::iter(&db2),
    |config| config.count(&db2) == 2,
    "count=2",
  );
  identity::take_log();
  let result2 = identity::make_range(&db2, config2);
  let log = identity::take_log();

  assert_eq!(result2.value(&db2), 2);
  assert!(
    log.is_empty(),
    "surviving entry must be cached after drain+roundtrip: {log:?}"
  );
}
