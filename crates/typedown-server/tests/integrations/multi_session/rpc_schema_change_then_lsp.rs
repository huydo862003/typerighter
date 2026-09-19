use std::path::PathBuf;

use crate::server_simulation::{
  cached_session_db, collect_type_errors, fresh_session, run_lsp_queries, run_rpc_queries,
};
use crate::utils::{copy_dir_recursive, example_vault, run_child_test};

// Schema change between RPC and LSP should produce expected errors, not spurious ones
#[test]
fn rpc_schema_change_then_lsp() {
  if std::env::var("SESSION").as_deref() == Ok("2") {
    let project_dir = PathBuf::from(std::env::var("PROJECT").unwrap());
    let db = cached_session_db(&project_dir);
    run_lsp_queries(&db);
    let errors = collect_type_errors(&db);
    // Adding phone field to Person means all Person files should report missing phone
    assert!(
      !errors.is_empty(),
      "schema change should produce errors for missing required field"
    );
    assert!(
      errors.iter().all(|e| e.contains("phone")),
      "all errors should be about missing phone field, got:\n{}",
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

  // Add a required field to Person schema between RPC and LSP
  let schema_path = project_dir.join("vault/_types/people/Person.td");
  let original = std::fs::read_to_string(&schema_path).unwrap();
  std::fs::write(
    &schema_path,
    original.replace("  email:", "  phone:\n    type: string\n  email:"),
  )
  .unwrap();

  run_child_test(
    "multi_session::rpc_schema_change_then_lsp::rpc_schema_change_then_lsp",
    &[("SESSION", "2"), ("PROJECT", project_dir.to_str().unwrap())],
  );
}
