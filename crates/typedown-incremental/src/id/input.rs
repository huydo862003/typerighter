//! Input declaration engine for the incremental database

/// A fast id for an input state
/// Input id is bound to a database's lifetime
pub trait InputId: super::Id + From<u32> + Into<u32> {
  /// Marker used by macros to verify a type implements InputId at compile time.
  #[cfg(debug_assertions)]
  #[doc(hidden)]
  const __TYPEDOWN_INPUT_ID: () = ();

  fn iter<DB: crate::QueryDatabase + ?Sized>(db: &DB) -> Vec<Self>
  where
    Self: Sized;
}

#[cfg(test)]
mod tests {
  use std::path::PathBuf;

  use typedown_macros::{query_db, query_input};

  use std::sync::atomic::Ordering;

  use crate::QueryStorage;

  #[query_db]
  struct Database {
    storage: QueryStorage,
  }

  #[query_input]
  struct ProgramFile {
    path: PathBuf,
    source: String,
  }

  #[test]
  fn input_holds_correct_value_after_init() {
    let db = Database {
      storage: QueryStorage::default(),
    };

    let rev_before = db.storage.revision.load(Ordering::Acquire);

    let path = PathBuf::from("/home/huydna/");
    let source = String::from("Hello, testing from input_holds_correct_value_after_init");

    let program_input = ProgramFile::new(&db, path.clone(), source.clone());

    assert_eq!(program_input.path(&db), path);
    assert_eq!(program_input.source(&db), source);

    // Creating an input should not bump the revision
    let rev_after = db.storage.revision.load(Ordering::Acquire);
    assert_eq!(rev_before, rev_after);
  }

  #[test]
  fn input_holds_correct_value_after_reassign() {
    let mut db = Database {
      storage: QueryStorage::default(),
    };

    let path = PathBuf::from("/home/huydna/");
    let source = String::from("Hello, testing from input_holds_correct_value_after_init");

    let program_input = { ProgramFile::new(&db, path.clone(), source.clone()) };

    let rev_before = db.storage.revision.load(Ordering::Acquire);

    let new_path = PathBuf::from("/home/corgi/");
    let new_source = String::from("Hello, testing from input_holds_correct_value_after_reassign");

    program_input.set_path(&mut db, new_path.clone());
    program_input.set_source(&mut db, new_source.clone());

    assert_eq!(program_input.path(&db), new_path);
    assert_eq!(program_input.source(&db), new_source);

    // Setting to different values should bump the revision
    let rev_after = db.storage.revision.load(Ordering::Acquire);
    assert!(
      rev_after > rev_before,
      "revision should bump after setting new values"
    );
  }

  #[test]
  fn input_setter_same_value_does_not_bump_revision() {
    let mut db = Database {
      storage: QueryStorage::default(),
    };

    let path = PathBuf::from("/home/huydna/");
    let source = String::from("Hello");

    let program_input = ProgramFile::new(&db, path.clone(), source.clone());

    let rev_before = db.storage.revision.load(Ordering::Acquire);

    // Set to the same values
    program_input.set_path(&mut db, path.clone());
    program_input.set_source(&mut db, source.clone());

    // Revision should not change since values are the same
    let rev_after = db.storage.revision.load(Ordering::Acquire);
    assert_eq!(
      rev_before, rev_after,
      "revision should not bump when setting same values"
    );
  }
}
