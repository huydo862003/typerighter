use std::cell::RefCell;

use typedown_macros::{query_db, query_derived, query_input, query_interned};

pub use super::super::utils::{dump_and_reload, find_entry};
pub use crate::{InputId, InternedId, QueryDatabase, QueryStorage};

thread_local! {
  static LOG: RefCell<Vec<usize>> = const { RefCell::new(Vec::new()) };
}

pub fn take_log() -> Vec<usize> {
  LOG.with(|log| std::mem::take(&mut *log.borrow_mut()))
}

#[query_db]
pub struct Database {
  pub storage: QueryStorage,
}

#[query_interned]
pub struct IdInput {
  n: usize,
}

#[query_derived]
pub struct IdResult<'db> {
  #[id]
  n: usize,
  value: usize,
}

#[query_input]
pub struct RangeConfig {
  count: usize,
}

#[query_derived]
pub fn identity<'db>(db: &'db Database, input: IdInput) -> IdResult<'db> {
  let number = input.n(db);
  LOG.with(|log| log.borrow_mut().push(number));
  IdResult::new(db, number, number)
}

// Interned type with a lifetime, for testing interned return values
#[query_interned]
pub struct Pair<'db> {
  a: IdResult<'db>,
  b: IdResult<'db>,
}

// Returns an interned type without lifetime from a derived query
#[query_derived]
pub fn double_input<'db>(db: &'db Database, input: IdInput) -> IdInput {
  let number = input.n(db);
  LOG.with(|log| log.borrow_mut().push(number));
  IdInput::new(db, number * 2)
}

// Returns an interned type with lifetime from a derived query
#[query_derived]
pub fn make_pair<'db>(db: &'db Database, input: IdInput) -> Pair<'db> {
  let number = input.n(db);
  LOG.with(|log| log.borrow_mut().push(number));
  let left_id = IdResult::new(db, number, number);
  let right_id = IdResult::new(db, number + 1, number + 1);
  Pair::new(db, left_id, right_id)
}

// Creates `count` derived structs, used to test identity map cleanup
#[query_derived]
pub fn make_range<'db>(db: &'db Database, config: RangeConfig) -> IdResult<'db> {
  let count = config.count(db);
  for index in 0..count {
    IdResult::new(db, index, index);
  }
  IdResult::new(db, count, count)
}

// Derived struct with no #[id] fields, identity is only disambiguator-based
#[query_derived]
pub struct Opaque<'db> {
  name: String,
  tag: usize,
}

// Creates multiple Opaque structs, count controls how many
// All share the same identity_hash (no #[id] fields), distinguished only by disambiguator
#[query_derived]
pub fn make_opaques<'db>(db: &'db Database, config: VersionConfig) -> Opaque<'db> {
  let value = config.version(db);
  LOG.with(|log| log.borrow_mut().push(value));
  for index in 0..3 {
    Opaque::new(db, format!("item_{}", index), value);
  }
  Opaque::new(db, "last".to_string(), value)
}

// Returns IdResult with fixed identity (n = 0) but value derived from version
// Changing version forces re-execution while the struct identity stays the same
#[query_input]
pub struct VersionConfig {
  version: usize,
}

#[query_derived]
pub fn root_query_versioned_result<'db>(db: &'db Database, config: VersionConfig) -> IdResult<'db> {
  let value = config.version(db);
  LOG.with(|log| log.borrow_mut().push(value));
  IdResult::new(db, 0, value)
}

// Derived struct whose field is another derived struct
// Used to test DerivedField fingerprints when the field value is itself a derived struct
#[query_derived]
pub struct Wrapper<'db> {
  inner: IdResult<'db>,
}

// Creates a Wrapper holding the identity result for the given input
#[query_derived]
pub fn wrap<'db>(db: &'db Database, input: IdInput) -> Wrapper<'db> {
  let result = identity(db, input);
  LOG.with(|log| log.borrow_mut().push(input.n(db) + 2000));
  Wrapper::new(db, result)
}

// Reads wrapper.inner, creating a DerivedField<IdResult> dep
// Cross-session fingerprint check must use stored fingerprint, not stable_hash on nested IdResult
// IdResult sub-fields (n, value) may not yet be loaded from the previous session
#[query_derived]
pub fn outer_query<'db>(db: &'db Database, input: IdInput) -> IdResult<'db> {
  let wrapper = wrap(db, input);
  let inner = wrapper.inner(db);
  LOG.with(|log| log.borrow_mut().push(input.n(db) + 3000));
  inner
}

// Query keyed by an interned value whose fields contain derived structs
// Tests that the key fingerprint stable_hash(pair, db) works correctly when IdResult fields are loaded
#[query_derived]
pub fn use_pair<'db>(db: &'db Database, pair: Pair<'db>) -> IdResult<'db> {
  let this_pair = pair.a(db);
  LOG.with(|log| log.borrow_mut().push(this_pair.n(db) + 4000));
  this_pair
}

// Like identity but also reads all VersionConfig entries as side deps
// Changing version forces re-execution without changing the key
#[query_derived]
pub fn versioned_identity<'db>(db: &'db Database, input: IdInput) -> IdResult<'db> {
  let number = input.n(db);
  // Record deps on all VersionConfig entries without including them in the key
  for version_config in VersionConfig::iter(db) {
    let _ = version_config.version(db);
  }
  LOG.with(|log| log.borrow_mut().push(number + 9000));
  IdResult::new(db, number, number)
}

#[query_derived]
pub fn versioned_identity_downstream<'db>(db: &'db Database, input: IdInput) -> IdResult<'db> {
  let version = versioned_identity(db, input);
  LOG.with(|log| log.borrow_mut().push(version.n(db) + 10000));
  version
}

// Downstream query that depends on versioned_result, used to test backdating: if versioned_result is re-executed but returns the same value (same changed_at), downstream must not re-execute
#[query_derived]
pub fn downstream_query_of_versioned<'db>(
  db: &'db Database,
  config: VersionConfig,
) -> IdResult<'db> {
  let upstream = root_query_versioned_result(db, config);
  let value = upstream.value(db);
  LOG.with(|log| log.borrow_mut().push(value + 1000));
  IdResult::new(db, 1, value)
}
