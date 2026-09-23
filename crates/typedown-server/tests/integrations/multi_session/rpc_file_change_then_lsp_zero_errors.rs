use std::path::PathBuf;

use crate::server_simulation::{
  cached_session_db, collect_type_errors, fresh_session, run_lsp_queries, run_rpc_queries,
};
use crate::utils::{copy_dir_recursive, example_vault, run_child_test};

#[test]
fn rpc_file_change_then_lsp_zero_errors() {
  if std::env::var("SESSION").as_deref() == Ok("2") {
    let project_dir = PathBuf::from(std::env::var("PROJECT").unwrap());
    let db = cached_session_db(&project_dir);
    run_lsp_queries(&db);
    let errors = collect_type_errors(&db);
    assert!(
      errors.is_empty(),
      "RPC then file change then LSP should produce zero type errors, got:\n{}",
      errors.join("\n"),
    );
    return;
  }

  let source = example_vault();
  assert!(source.exists());
  let _tmp = tempfile::TempDir::new().unwrap();
  let project_dir = _tmp.path().join("project_tracker");
  copy_dir_recursive(&source, &project_dir);

  fresh_session(&project_dir, run_rpc_queries);

  // Modify a content file between RPC and LSP sessions
  let target = project_dir.join("vault/people/alice.td");
  let original = std::fs::read_to_string(&target).unwrap();
  std::fs::write(&target, format!("{original}\n<!-- changed -->\n")).unwrap();

  run_child_test(
    "multi_session::rpc_file_change_then_lsp_zero_errors::rpc_file_change_then_lsp_zero_errors",
    &[("SESSION", "2"), ("PROJECT", project_dir.to_str().unwrap())],
  );
}
