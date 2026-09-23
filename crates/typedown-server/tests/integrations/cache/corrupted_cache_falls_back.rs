use typedown_incremental::CacheSession;

use super::session1_dump;

#[test]
fn corrupted_cache_falls_back() {
  let (_tmp, _project_dir, cache_dir, _) = session1_dump();

  let finalized_dirs: Vec<_> = std::fs::read_dir(&cache_dir)
    .unwrap()
    .filter_map(|e| e.ok())
    .map(|e| e.path())
    .filter(|p| {
      p.is_dir()
        && p
          .file_name()
          .and_then(|n| n.to_str())
          .is_some_and(|n| n.starts_with("s-") && !n.ends_with("-working"))
    })
    .collect();
  assert!(!finalized_dirs.is_empty(), "should have finalized sessions");

  for dir in &finalized_dirs {
    std::fs::write(dir.join("dep-graph.bin"), b"corrupted").unwrap();
  }

  let (_, data) = CacheSession::open(&cache_dir).unwrap();
  assert!(
    data.is_none(),
    "corrupted cache should not produce valid data"
  );
}
