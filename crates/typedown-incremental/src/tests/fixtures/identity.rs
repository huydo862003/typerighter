use std::cell::RefCell;

use typedown_macros::{query_db, query_derived, query_interned};

pub use crate::{QueryDatabase, QueryStorage};

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

#[query_derived]
pub fn identity<'db>(db: &'db Database, input: IdInput) -> IdResult<'db> {
  let n = input.n(db);
  LOG.with(|log| log.borrow_mut().push(n));
  IdResult::new(db, n, n)
}
