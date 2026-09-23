use std::path::PathBuf;

use super::session1_dump;
use crate::server_simulation::collect_type_errors;
use crate::utils::{run_child_test, setup_db_cached};

#[test]
fn zero_spurious_errors() {
  if std::env::var("SESSION").as_deref() == Ok("2") {
    let project_dir = PathBuf::from(std::env::var("PROJECT").unwrap());
    let cache_dir = PathBuf::from(std::env::var("CACHE").unwrap());

    let db = setup_db_cached(&cache_dir, &project_dir);
    let errors = collect_type_errors(&db);
    assert!(
      errors.is_empty(),
      "cache roundtrip should produce zero type errors, got:\n{}",
      errors.join("\n"),
    );
    return;
  }

  let (_tmp, project_dir, cache_dir, _) = session1_dump();

  run_child_test(
    "cache::zero_spurious_errors::zero_spurious_errors",
    &[
      ("SESSION", "2"),
      ("PROJECT", project_dir.to_str().unwrap()),
      ("CACHE", cache_dir.to_str().unwrap()),
    ],
  );
}
