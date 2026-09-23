use std::path::PathBuf;
use std::sync::atomic::Ordering;
use typedown_incremental::InputId;

use tempfile::TempDir;
use typedown_incremental::{CacheSession, SerializableQueryDatabase};
use typedown_lang::db::derived::evaluate::evaluate_resource::evaluate_resource;
use typedown_lang::db::derived::name_resolver::file_symbol::file_symbol;
use typedown_lang::db::derived::parse_file::parse_file;
use typedown_lang::db::types::Project;
use typedown_lang::integrations::export::{export_resource_html, export_resource_summary};
use typedown_server::core::utils::fs::get_cache_dir;

use crate::utils::{
  copy_dir_recursive, example_vault, run_child_test, setup_db_cached, setup_db_fresh,
};

#[test]
fn export_pipeline_roundtrip() {
  if std::env::var("SESSION").as_deref() == Ok("2") {
    let project_dir = PathBuf::from(std::env::var("PROJECT").unwrap());
    let cache_dir = PathBuf::from(std::env::var("CACHE").unwrap());

    let db = setup_db_cached(&cache_dir, &project_dir);
    let project = Project::iter(&db)
      .into_iter()
      .next()
      .expect("project should exist");
    for (path, file) in &*project.files(&db) {
      if path.extension().and_then(|e| e.to_str()) != Some("td") {
        continue;
      }
      if let Some(exported) = export_resource_html(&db, project, *file) {
        assert!(
          !exported.header.is_null(),
          "header should not be null for {}",
          path.display()
        );
      }
    }
    return;
  }

  let source = example_vault();
  assert!(source.exists(), "examples/project_tracker must exist");
  let tmp = TempDir::new().unwrap();
  let project_dir = tmp.path().join("project_tracker");
  copy_dir_recursive(&source, &project_dir);
  let cache_dir = get_cache_dir(&project_dir);

  // Session 1: run full export pipeline and dump cache
  let db = setup_db_fresh(&project_dir);
  let project = Project::iter(&db).into_iter().next().unwrap();
  for (path, file) in &*project.files(&db) {
    if path.extension().and_then(|e| e.to_str()) != Some("td") {
      continue;
    }
    let _ = parse_file(&db, project, *file);
    if let Some(sym) = file_symbol(&db, project, *file).value(&db) {
      let _ = evaluate_resource(&db, sym);
      let _ = export_resource_html(&db, project, *file);
      let _ = export_resource_summary(&db, project, *file);
    }
  }
  let serialized = db.dump();
  let (session, _) = CacheSession::open(&cache_dir).unwrap();
  let revision = db.storage.revision.load(Ordering::Acquire) as u64;
  session.finalize(&serialized, revision).unwrap();

  run_child_test(
    "cache::export_pipeline_roundtrip::export_pipeline_roundtrip",
    &[
      ("SESSION", "2"),
      ("PROJECT", project_dir.to_str().unwrap()),
      ("CACHE", cache_dir.to_str().unwrap()),
    ],
  );
}
