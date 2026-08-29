use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::Arc;
use std::time::SystemTime;

use typedown_incremental::{CacheSession, InputId};
use typedown_lang::db::types::{AssetKind, File, FileHandle, FileMetadata, Project};
use typedown_lang::db::{QueryStorage, TypedownDatabase};

pub fn example_vault() -> PathBuf {
  Path::new(env!("CARGO_MANIFEST_DIR"))
    .parent()
    .unwrap()
    .parent()
    .unwrap()
    .join("examples/project_tracker")
}

pub fn setup_db_fresh(project_dir: &Path) -> TypedownDatabase {
  let db = TypedownDatabase {
    storage: QueryStorage::default(),
  };
  register_project_fresh(&db, project_dir);
  db
}

// Reuses cached File/Project IDs to avoid unnecessary invalidation
// This mirrors what AnalysisHost::new does so the incremental cache can match entries and skip recomputation
pub fn setup_db_cached(cache_dir: &Path, project_dir: &Path) -> TypedownDatabase {
  let (_session, data) = CacheSession::open(cache_dir).unwrap();
  let serialized = data.expect("cache should exist");
  let arc = QueryStorage::from_serialized(serialized);
  let mut db = TypedownDatabase {
    storage: Arc::try_unwrap(arc).unwrap_or_else(|arc| (*arc).clone()),
  };
  register_project_cached(&mut db, project_dir);
  db
}

// Spawn a child process to run a specific test in an isolated session
// Child processes get clean statics, which is needed for incremental cache tests
pub fn run_child_test(test_name: &str, envs: &[(&str, &str)]) {
  let mut cmd = Command::new(std::env::current_exe().unwrap());
  for (key, val) in envs {
    cmd.env(key, val);
  }
  let output = cmd
    .arg(test_name)
    .arg("--exact")
    .arg("--nocapture")
    .output()
    .expect("failed to spawn child process");

  if !output.status.success() {
    panic!(
      "Child session failed:\nstdout: {}\nstderr: {}",
      String::from_utf8_lossy(&output.stdout),
      String::from_utf8_lossy(&output.stderr),
    );
  }
}

fn register_project_fresh(db: &TypedownDatabase, project_dir: &Path) {
  let mut files = HashMap::new();
  for path in scan_project_files(project_dir) {
    let meta = std::fs::metadata(&path).ok();
    let mtime = meta
      .as_ref()
      .and_then(|m| m.modified().ok())
      .unwrap_or(SystemTime::UNIX_EPOCH);
    let ctime = meta
      .as_ref()
      .and_then(|m| m.created().ok())
      .unwrap_or(mtime);
    let handle = FileHandle::Path(path.clone(), FileMetadata { mtime, ctime });
    let file = File::new(db, handle);
    files.insert(path, file);
  }
  Project::new(db, project_dir.to_path_buf(), files);
}

fn register_project_cached(db: &mut TypedownDatabase, project_dir: &Path) {
  let cached_files: HashMap<PathBuf, File> = File::iter(db)
    .into_iter()
    .filter_map(|file| {
      let handle = file.handle(db);
      handle.path().cloned().map(|path| (path, file))
    })
    .collect();

  let mut files = HashMap::new();
  for path in scan_project_files(project_dir) {
    let meta = std::fs::metadata(&path).ok();
    let mtime = meta
      .as_ref()
      .and_then(|m| m.modified().ok())
      .unwrap_or(SystemTime::UNIX_EPOCH);
    let ctime = meta
      .as_ref()
      .and_then(|m| m.created().ok())
      .unwrap_or(mtime);
    let handle = FileHandle::Path(path.clone(), FileMetadata { mtime, ctime });
    let file = if let Some(&cached) = cached_files.get(&path) {
      cached.set_handle(db, handle);
      cached
    } else {
      File::new(db, handle)
    };
    files.insert(path, file);
  }

  let cached_project = Project::iter(db)
    .into_iter()
    .find(|proj| proj.root_dir(db) == project_dir);
  if let Some(proj) = cached_project {
    proj.set_files(db, files);
  } else {
    Project::new(db, project_dir.to_path_buf(), files);
  }
}

pub fn scan_project_files(dir: &Path) -> Vec<PathBuf> {
  let root = dir;
  let mut result = Vec::new();
  let mut stack = vec![dir.to_path_buf()];
  while let Some(dir) = stack.pop() {
    let Ok(entries) = std::fs::read_dir(&dir) else {
      continue;
    };
    for entry in entries.flatten() {
      let path = entry.path();
      if path.is_dir() {
        stack.push(path);
      } else {
        let ext = path.extension().and_then(|e| e.to_str());
        let name = path.file_name().and_then(|n| n.to_str());
        let is_asset = ext.and_then(AssetKind::from_extension).is_some();
        if ext == Some("td")
          || is_asset
          || (dir == root && matches!(name, Some("typedown.yaml") | Some("typedown.yml")))
        {
          result.push(path);
        }
      }
    }
  }
  result
}

// Preserves file modification times so incremental cache sees unchanged files
pub fn copy_dir_recursive(src: &Path, dst: &Path) {
  std::fs::create_dir_all(dst).unwrap();
  for entry in std::fs::read_dir(src).unwrap().flatten() {
    let src_path = entry.path();
    let dst_path = dst.join(entry.file_name());
    if src_path.is_dir() {
      copy_dir_recursive(&src_path, &dst_path);
    } else {
      std::fs::copy(&src_path, &dst_path).unwrap();
      if let Ok(meta) = entry.metadata()
        && let Ok(mtime) = meta.modified()
      {
        let _ = filetime::set_file_mtime(&dst_path, filetime::FileTime::from_system_time(mtime));
      }
    }
  }
}
