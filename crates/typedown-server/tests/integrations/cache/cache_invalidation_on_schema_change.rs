use std::path::PathBuf;

use super::{run_diagnostics, session1_dump};
use crate::utils::{run_child_test, setup_db_cached};

#[test]
fn cache_invalidation_on_schema_change() {
  if std::env::var("SESSION").as_deref() == Ok("2") {
    let project_dir = PathBuf::from(std::env::var("PROJECT").unwrap());
    let cache_dir = PathBuf::from(std::env::var("CACHE").unwrap());

    let db = setup_db_cached(&cache_dir, &project_dir);
    run_diagnostics(&db);

    let stats = db.storage.ingredient_stats();
    let hashed_recomputes: usize = stats
      .iter()
      .filter(|s| !s.no_hash)
      .map(|s| s.recompute_count)
      .sum();
    assert!(
      hashed_recomputes > 0,
      "hashed queries should recompute after schema change"
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
    "cache::cache_invalidation_on_schema_change::cache_invalidation_on_schema_change",
    &[
      ("SESSION", "2"),
      ("PROJECT", project_dir.to_str().unwrap()),
      ("CACHE", cache_dir.to_str().unwrap()),
    ],
  );
}
