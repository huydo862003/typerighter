use std::fmt;
use std::sync::Arc;

use crate::{QueryStorage, SerializableQueryDatabase};

pub fn dump_and_reload<DB: SerializableQueryDatabase>(
  db: &DB,
  make_db: impl FnOnce(QueryStorage) -> DB,
) -> DB {
  let serialized = db.dump();
  let storage = QueryStorage::from_serialized(serialized);
  make_db(Arc::try_unwrap(storage).unwrap_or_else(|arc| (*arc).clone()))
}

pub fn find_entry<T>(
  iter: impl IntoIterator<Item = T>,
  predicate: impl Fn(&T) -> bool,
  label: impl fmt::Display,
) -> T {
  iter
    .into_iter()
    .find(predicate)
    .unwrap_or_else(|| panic!("{} not found", label))
}
