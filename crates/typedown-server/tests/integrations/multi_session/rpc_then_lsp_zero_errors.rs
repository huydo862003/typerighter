use std::path::PathBuf;

use crate::server_simulation::{
  cached_session_db, collect_type_errors, fresh_session, run_lsp_queries, run_rpc_queries,
};
use crate::utils::{copy_dir_recursive, example_vault, run_child_test};

fn setup_project() -> (tempfile::TempDir, PathBuf) {
  let source = example_vault();
  assert!(source.exists());
  let tmp = tempfile::TempDir::new().unwrap();
  let project_dir = tmp.path().join("project_tracker");
  copy_dir_recursive(&source, &project_dir);
  (tmp, project_dir)
}

#[test]
fn rpc_then_lsp_zero_errors() {
  if std::env::var("SESSION").as_deref() == Ok("2") {
    let project_dir = PathBuf::from(std::env::var("PROJECT").unwrap());
    let db = cached_session_db(&project_dir);
    run_lsp_queries(&db);
    let errors = collect_type_errors(&db);
    assert!(
      errors.is_empty(),
      "RPC then LSP should produce zero type errors, got:\n{}",
      errors.join("\n"),
    );
    return;
  }

  let (_tmp, project_dir) = setup_project();
  fresh_session(&project_dir, run_rpc_queries);

  run_child_test(
    "multi_session::rpc_then_lsp_zero_errors::rpc_then_lsp_zero_errors",
    &[("SESSION", "2"), ("PROJECT", project_dir.to_str().unwrap())],
  );
}
