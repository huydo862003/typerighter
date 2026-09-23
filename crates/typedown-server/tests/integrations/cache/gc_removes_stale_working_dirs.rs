use tempfile::TempDir;
use typedown_incremental::CacheSession;

#[test]
fn gc_removes_stale_working_dirs() {
  let tmp = TempDir::new().unwrap();
  let cache_dir = tmp.path().join("cache");
  std::fs::create_dir_all(&cache_dir).unwrap();

  let stale = cache_dir.join("s-0000000000000-deadbeef-working");
  std::fs::create_dir_all(&stale).unwrap();
  std::fs::write(stale.join("dummy.bin"), b"stale data").unwrap();

  let (_session, _) = CacheSession::open(&cache_dir).unwrap();

  assert!(
    !stale.exists(),
    "stale working directory should be removed by GC"
  );
}
