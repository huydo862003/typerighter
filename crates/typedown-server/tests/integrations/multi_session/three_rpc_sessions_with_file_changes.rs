use std::path::PathBuf;

use crate::server_simulation::{
  cached_session_assert_no_errors, cached_session_db, collect_type_errors, fresh_session,
  run_rpc_queries,
};
use crate::utils::{copy_dir_recursive, example_vault, run_child_test};

#[test]
fn three_rpc_sessions_with_file_changes() {
  if std::env::var("SESSION").as_deref() == Ok("3") {
    let project_dir = PathBuf::from(std::env::var("PROJECT").unwrap());
    let db = cached_session_db(&project_dir);
    run_rpc_queries(&db);
    let errors = collect_type_errors(&db);
    assert!(
      errors.is_empty(),
      "third RPC session should produce zero type errors, got:\n{}",
      errors.join("\n"),
    );
    return;
  }

  if std::env::var("SESSION").as_deref() == Ok("2") {
    let project_dir = PathBuf::from(std::env::var("PROJECT").unwrap());
    cached_session_assert_no_errors(&project_dir, run_rpc_queries);

    // Modify bob between session 2 and 3
    let target = project_dir.join("vault/people/bob.td");
    let original = std::fs::read_to_string(&target).unwrap();
    std::fs::write(&target, format!("{original}\n<!-- changed again -->\n")).unwrap();

    run_child_test(
      "multi_session::three_rpc_sessions_with_file_changes::three_rpc_sessions_with_file_changes",
      &[("SESSION", "3"), ("PROJECT", project_dir.to_str().unwrap())],
    );
    return;
  }

  let source = example_vault();
  assert!(source.exists());
  let _tmp = tempfile::TempDir::new().unwrap();
  let project_dir = _tmp.path().join("project_tracker");
  copy_dir_recursive(&source, &project_dir);

  fresh_session(&project_dir, run_rpc_queries);

  // Modify alice between session 1 and 2
  let target = project_dir.join("vault/people/alice.td");
  let original = std::fs::read_to_string(&target).unwrap();
  std::fs::write(&target, format!("{original}\n<!-- changed -->\n")).unwrap();

  run_child_test(
    "multi_session::three_rpc_sessions_with_file_changes::three_rpc_sessions_with_file_changes",
    &[("SESSION", "2"), ("PROJECT", project_dir.to_str().unwrap())],
  );
}
