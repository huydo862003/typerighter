use std::path::PathBuf;

use crate::server_simulation::{
  cached_session_assert_no_errors, cached_session_db, collect_type_errors, fresh_session,
  run_rpc_queries,
};
use crate::utils::{copy_dir_recursive, example_vault, run_child_test};

#[test]
fn three_rpc_sessions_unchanged() {
  match std::env::var("SESSION").as_deref() {
    Ok("4") => {
      let project_dir = PathBuf::from(std::env::var("PROJECT").unwrap());
      let db = cached_session_db(&project_dir);
      run_rpc_queries(&db);
      let errors = collect_type_errors(&db);
      assert!(
        errors.is_empty(),
        "third RPC session should produce zero type errors, got:\n{}",
        errors.join("\n"),
      );
    }
    Ok("3") => {
      let project_dir = PathBuf::from(std::env::var("PROJECT").unwrap());
      cached_session_assert_no_errors(&project_dir, run_rpc_queries);
    }
    Ok("2") => {
      let project_dir = PathBuf::from(std::env::var("PROJECT").unwrap());
      cached_session_assert_no_errors(&project_dir, run_rpc_queries);
    }
    _ => {
      let source = example_vault();
      assert!(source.exists());
      let _tmp = tempfile::TempDir::new().unwrap();
      let project_dir = _tmp.path().join("project_tracker");
      copy_dir_recursive(&source, &project_dir);

      fresh_session(&project_dir, run_rpc_queries);

      // Run all child sessions sequentially from parent (no cascading spawns)
      let test_name = "multi_session::three_rpc_sessions_unchanged::three_rpc_sessions_unchanged";
      let project_str = project_dir.to_str().unwrap();
      run_child_test(test_name, &[("SESSION", "2"), ("PROJECT", project_str)]);
      run_child_test(test_name, &[("SESSION", "3"), ("PROJECT", project_str)]);
      run_child_test(test_name, &[("SESSION", "4"), ("PROJECT", project_str)]);
    }
  }
}
