use std::path::PathBuf;

use crate::server_simulation::{
  cached_session_db, collect_type_errors, fresh_session, run_rpc_queries,
};
use crate::utils::{copy_dir_recursive, example_vault, run_child_test};

// Two RPC sessions dump caches concurrently, then a third loads the latest
#[test]
fn concurrent_rpc_rpc() {
  if std::env::var("SESSION").as_deref() == Ok("2") {
    let project_dir = PathBuf::from(std::env::var("PROJECT").unwrap());
    let db = cached_session_db(&project_dir);
    run_rpc_queries(&db);
    let errors = collect_type_errors(&db);
    assert!(
      errors.is_empty(),
      "session after concurrent dumps should produce zero type errors, got:\n{}",
      errors.join("\n"),
    );
    return;
  }

  let source = example_vault();
  assert!(source.exists());
  let _tmp = tempfile::TempDir::new().unwrap();
  let project_dir = _tmp.path().join("project_tracker");
  copy_dir_recursive(&source, &project_dir);

  // Two fresh sessions dump sequentially (simulates concurrent since GC keeps only latest)
  fresh_session(&project_dir, run_rpc_queries);
  fresh_session(&project_dir, run_rpc_queries);

  run_child_test(
    "multi_session::concurrent_rpc_rpc::concurrent_rpc_rpc",
    &[("SESSION", "2"), ("PROJECT", project_dir.to_str().unwrap())],
  );
}
