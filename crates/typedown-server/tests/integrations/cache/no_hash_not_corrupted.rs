use std::path::PathBuf;
use typedown_incremental::InputId;

use typedown_lang::db::derived::evaluate::evaluate_resource::evaluate_resource;
use typedown_lang::db::derived::name_resolver::file_symbol::file_symbol;
use typedown_lang::db::derived::parse_file::parse_file;
use typedown_lang::db::types::Project;

use super::session1_dump;
use crate::utils::{run_child_test, setup_db_cached};

#[test]
fn no_hash_not_corrupted() {
  if std::env::var("SESSION").as_deref() == Ok("2") {
    let project_dir = PathBuf::from(std::env::var("PROJECT").unwrap());
    let cache_dir = PathBuf::from(std::env::var("CACHE").unwrap());

    let db = setup_db_cached(&cache_dir, &project_dir);

    let project = Project::iter(&db)
      .into_iter()
      .next()
      .expect("project should exist");
    for (path, file) in &*project.files(&db) {
      if path.extension().and_then(|ext| ext.to_str()) != Some("td") {
        continue;
      }
      let result = parse_file(&db, project, *file);
      assert!(
        !result.ast(&db).text().is_empty(),
        "parse result should have content for {}",
        path.display()
      );
      if let Some(sym) = file_symbol(&db, project, *file).value(&db) {
        let _eval = evaluate_resource(&db, sym);
      }
    }
    return;
  }

  let (_tmp, project_dir, cache_dir, _) = session1_dump();

  run_child_test(
    "cache::no_hash_not_corrupted::no_hash_not_corrupted",
    &[
      ("SESSION", "2"),
      ("PROJECT", project_dir.to_str().unwrap()),
      ("CACHE", cache_dir.to_str().unwrap()),
    ],
  );
}
