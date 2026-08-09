use std::cell::RefCell;

use typedown_macros::{query_db, query_derived, query_interned};

pub use super::super::utils::{dump_and_reload, find_entry};
pub use crate::{InternedId, QueryDatabase, QueryStorage};

thread_local! {
  static FIB_LOG: RefCell<Vec<String>> = const { RefCell::new(Vec::new()) };
}

pub fn log(msg: String) {
  FIB_LOG.with(|log| log.borrow_mut().push(msg));
}

pub fn take_log() -> Vec<String> {
  FIB_LOG.with(|log| std::mem::take(&mut *log.borrow_mut()))
}

#[query_db]
pub struct Database {
  pub storage: QueryStorage,
}

#[query_interned]
pub struct FibInput {
  n: usize,
}

#[query_derived]
pub struct FibResult {
  #[id]
  n: usize,
  value: usize,
}

#[query_derived]
pub fn fibonacci(db: &Database, input: FibInput) -> FibResult {
  let n = input.n(db);
  log(format!("computing fib({})", n));
  if n <= 1 {
    return FibResult::new(db, n, n);
  }

  let input_a = FibInput::new(db, n - 1);
  let input_b = FibInput::new(db, n - 2);

  let a = fibonacci(db, input_a);
  let b = fibonacci(db, input_b);

  FibResult::new(db, n, a.value(db) + b.value(db))
}
