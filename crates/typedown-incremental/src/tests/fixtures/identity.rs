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
  let n = input.n(db);
  LOG.with(|log| log.borrow_mut().push(n));
  IdResult::new(db, n, n)
}

// Interned type with a lifetime, for testing interned return values
#[query_interned]
pub struct Pair<'db> {
  a: IdResult<'db>,
  b: IdResult<'db>,
}

// Returns an interned type from a derived query
#[query_derived]
pub fn make_pair<'db>(db: &'db Database, input: IdInput) -> Pair<'db> {
  let n = input.n(db);
  LOG.with(|log| log.borrow_mut().push(n));
  let a = IdResult::new(db, n, n);
  let b = IdResult::new(db, n + 1, n + 1);
  Pair::new(db, a, b)
}

// Creates `count` derived structs, used to test identity map cleanup
#[query_derived]
pub fn make_range<'db>(db: &'db Database, config: RangeConfig) -> IdResult<'db> {
  let count = config.count(db);
  for i in 0..count {
    IdResult::new(db, i, i);
  }
  IdResult::new(db, count, count)
}
