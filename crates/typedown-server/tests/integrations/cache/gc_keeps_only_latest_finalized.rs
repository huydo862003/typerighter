use tempfile::TempDir;
use typedown_incremental::CacheSession;

#[test]
fn gc_keeps_only_latest_finalized() {
  let tmp = TempDir::new().unwrap();
  let cache_dir = tmp.path().join("cache");
  std::fs::create_dir_all(&cache_dir).unwrap();

  let old = cache_dir.join("s-0000000000001-1");
  let new = cache_dir.join("s-0000000000002-2");
  std::fs::create_dir_all(&old).unwrap();
  std::fs::create_dir_all(&new).unwrap();
  std::fs::write(old.join("lock"), b"").unwrap();
  std::fs::write(new.join("lock"), b"").unwrap();

  let (_session, _) = CacheSession::open(&cache_dir).unwrap();

  assert!(
    !old.exists(),
    "older finalized session should be removed by GC"
  );
}
