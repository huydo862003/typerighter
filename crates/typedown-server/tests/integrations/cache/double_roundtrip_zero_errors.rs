use std::path::PathBuf;
use std::sync::atomic::Ordering;

use typedown_incremental::{CacheSession, SerializableQueryDatabase};

use super::{run_diagnostics, session1_dump};
use crate::server_simulation::collect_type_errors;
use crate::utils::{run_child_test, setup_db_cached};

#[test]
fn double_roundtrip_zero_errors() {
  match std::env::var("SESSION").as_deref() {
    Ok("2") => {
      // Session 2: load cache, run diagnostics, dump again
      let project_dir = PathBuf::from(std::env::var("PROJECT").unwrap());
      let cache_dir = PathBuf::from(std::env::var("CACHE").unwrap());

      let db = setup_db_cached(&cache_dir, &project_dir);
      run_diagnostics(&db);

      let errors = collect_type_errors(&db);
      assert!(
        errors.is_empty(),
        "session 2 should have zero type errors, got:\n{}",
        errors.join("\n"),
      );

      // Dump session 2 cache
      let serialized = db.dump();
      let (session, _) = CacheSession::open(&cache_dir).unwrap();
      let revision = db.storage.revision.load(Ordering::Acquire) as u64;
      session.finalize(&serialized, revision).unwrap();
    }
    Ok("3") => {
      // Session 3: load double-roundtripped cache, verify zero errors
      let project_dir = PathBuf::from(std::env::var("PROJECT").unwrap());
      let cache_dir = PathBuf::from(std::env::var("CACHE").unwrap());

      let db = setup_db_cached(&cache_dir, &project_dir);
      let errors = collect_type_errors(&db);
      assert!(
        errors.is_empty(),
        "session 3 should have zero type errors after double roundtrip, got:\n{}",
        errors.join("\n"),
      );
    }
    _ => {
      // Session 1: fresh build and dump
      let (_tmp, project_dir, cache_dir, _) = session1_dump();

      // Run session 2 in child process
      run_child_test(
        "cache::double_roundtrip_zero_errors::double_roundtrip_zero_errors",
        &[
          ("SESSION", "2"),
          ("PROJECT", project_dir.to_str().unwrap()),
          ("CACHE", cache_dir.to_str().unwrap()),
        ],
      );

      // Run session 3 in child process
      run_child_test(
        "cache::double_roundtrip_zero_errors::double_roundtrip_zero_errors",
        &[
          ("SESSION", "3"),
          ("PROJECT", project_dir.to_str().unwrap()),
          ("CACHE", cache_dir.to_str().unwrap()),
        ],
      );
    }
  }
}
