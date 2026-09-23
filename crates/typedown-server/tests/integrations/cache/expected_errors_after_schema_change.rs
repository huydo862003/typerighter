use std::path::PathBuf;

use super::session1_dump;
use crate::server_simulation::collect_type_errors;
use crate::utils::{run_child_test, setup_db_cached};

#[test]
fn expected_errors_after_schema_change() {
  if std::env::var("SESSION").as_deref() == Ok("2") {
    let project_dir = PathBuf::from(std::env::var("PROJECT").unwrap());
    let cache_dir = PathBuf::from(std::env::var("CACHE").unwrap());

    let db = setup_db_cached(&cache_dir, &project_dir);
    let errors = collect_type_errors(&db);
    assert!(
      errors.iter().all(|e| e.contains("phone")),
      "all errors should be about missing phone field, got:\n{}",
      errors.join("\n"),
    );
    assert!(
      !errors.is_empty(),
      "schema change should produce errors for missing required field"
    );
    return;
  }

  let (_tmp, project_dir, cache_dir, _) = session1_dump();

  let schema_path = project_dir.join("vault/_types/people/Person.td");
  let original = std::fs::read_to_string(&schema_path).unwrap();
  std::fs::write(
    &schema_path,
    original.replace("  email:", "  phone:\n    type: string\n  email:"),
  )
  .unwrap();

  run_child_test(
    "cache::expected_errors_after_schema_change::expected_errors_after_schema_change",
    &[
      ("SESSION", "2"),
      ("PROJECT", project_dir.to_str().unwrap()),
      ("CACHE", cache_dir.to_str().unwrap()),
    ],
  );
}
