use std::path::PathBuf;

use super::{run_diagnostics, session1_dump};
use crate::utils::{run_child_test, setup_db_cached};

#[test]
fn cache_invalidation_on_new_file() {
  if std::env::var("SESSION").as_deref() == Ok("2") {
    let project_dir = PathBuf::from(std::env::var("PROJECT").unwrap());
    let cache_dir = PathBuf::from(std::env::var("CACHE").unwrap());

    let db = setup_db_cached(&cache_dir, &project_dir);
    run_diagnostics(&db);

    let total = db.storage.total_recompute_count();
    assert!(total > 0, "should recompute after new file added");
    return;
  }

  let (_tmp, project_dir, cache_dir, _) = session1_dump();

  let new_content = r#"---
_type: Person
name: "Dave"
email: "dave@example.com"
role: "developer"
---
"#;
  std::fs::write(project_dir.join("vault/people/dave.td"), new_content).unwrap();

  run_child_test(
    "cache::cache_invalidation_on_new_file::cache_invalidation_on_new_file",
    &[
      ("SESSION", "2"),
      ("PROJECT", project_dir.to_str().unwrap()),
      ("CACHE", cache_dir.to_str().unwrap()),
    ],
  );
}
