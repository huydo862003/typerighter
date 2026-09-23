use std::path::PathBuf;

use super::session1_dump;
use crate::server_simulation::collect_type_errors;
use crate::utils::{run_child_test, setup_db_cached};

#[test]
fn zero_spurious_errors_after_file_change() {
  if std::env::var("SESSION").as_deref() == Ok("2") {
    let project_dir = PathBuf::from(std::env::var("PROJECT").unwrap());
    let cache_dir = PathBuf::from(std::env::var("CACHE").unwrap());

    let db = setup_db_cached(&cache_dir, &project_dir);
    let errors = collect_type_errors(&db);
    assert!(
      errors.is_empty(),
      "should have zero type errors after file change, got:\n{}",
      errors.join("\n"),
    );
    return;
  }

  let (_tmp, project_dir, cache_dir, _) = session1_dump();

  let target = project_dir.join("vault/people/alice.td");
  let original = std::fs::read_to_string(&target).unwrap();
  std::fs::write(&target, format!("{original}\n<!-- changed -->\n")).unwrap();

  run_child_test(
    "cache::zero_spurious_errors_after_file_change::zero_spurious_errors_after_file_change",
    &[
      ("SESSION", "2"),
      ("PROJECT", project_dir.to_str().unwrap()),
      ("CACHE", cache_dir.to_str().unwrap()),
    ],
  );
}
