use std::path::PathBuf;

use super::{run_diagnostics, session1_dump};
use crate::utils::{run_child_test, setup_db_cached};

#[test]
fn no_hash_recomputes_on_file_change() {
  if std::env::var("SESSION").as_deref() == Ok("2") {
    let project_dir = PathBuf::from(std::env::var("PROJECT").unwrap());
    let cache_dir = PathBuf::from(std::env::var("CACHE").unwrap());

    let db = setup_db_cached(&cache_dir, &project_dir);
    run_diagnostics(&db);

    let stats = db.storage.ingredient_stats();
    let no_hash_recomputes: usize = stats
      .iter()
      .filter(|s| s.no_hash)
      .map(|s| s.recompute_count)
      .sum();
    // no_hash queries must recompute since file content changed
    assert!(
      no_hash_recomputes > 0,
      "no_hash queries should recompute after file change"
    );
    return;
  }

  let (_tmp, project_dir, cache_dir, _) = session1_dump();

  let target = project_dir.join("vault/people/alice.td");
  let original = std::fs::read_to_string(&target).unwrap();
  std::fs::write(&target, format!("{original}\n<!-- changed -->\n")).unwrap();

  run_child_test(
    "cache::no_hash_recomputes_on_file_change::no_hash_recomputes_on_file_change",
    &[
      ("SESSION", "2"),
      ("PROJECT", project_dir.to_str().unwrap()),
      ("CACHE", cache_dir.to_str().unwrap()),
    ],
  );
}
