use std::path::PathBuf;

use crate::server_simulation::{
  cached_session_db, collect_type_errors, fresh_session, run_lsp_queries, run_rpc_queries,
};
use crate::utils::{copy_dir_recursive, example_vault, run_child_test};

// RPC and LSP both dump caches, then a new session loads the latest
#[test]
fn concurrent_rpc_lsp() {
  if std::env::var("SESSION").as_deref() == Ok("2") {
    let project_dir = PathBuf::from(std::env::var("PROJECT").unwrap());
    let db = cached_session_db(&project_dir);
    run_lsp_queries(&db);
    let errors = collect_type_errors(&db);
    assert!(
      errors.is_empty(),
      "session after RPC+LSP dumps should produce zero type errors, got:\n{}",
      errors.join("\n"),
    );
    return;
  }

  let source = example_vault();
  assert!(source.exists());
  let _tmp = tempfile::TempDir::new().unwrap();
  let project_dir = _tmp.path().join("project_tracker");
  copy_dir_recursive(&source, &project_dir);

  // RPC dumps, then LSP dumps (simulates concurrent, GC keeps only latest)
  fresh_session(&project_dir, run_rpc_queries);
  fresh_session(&project_dir, run_lsp_queries);

  run_child_test(
    "multi_session::concurrent_rpc_lsp::concurrent_rpc_lsp",
    &[("SESSION", "2"), ("PROJECT", project_dir.to_str().unwrap())],
  );
}
