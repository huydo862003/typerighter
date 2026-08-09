use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

use crate::db::types::{AssetKind, File, FileHandle, FileMetadata, Project};
use crate::db::utils::is_content_file;
use crate::db::{QueryStorage, TypedownDatabase};

pub struct Fixture {
  pub path: PathBuf,
  pub contents: String,
}

/// Load all files in a fixture subdirectory as a map of filename to Fixture
pub fn load_fixtures(subdir: &str) -> HashMap<String, Fixture> {
  // TIL: CARGO_MANIFEST_DIR is set to the folder containing the Cargo.toml
  let fixtures_dir = Path::new(env!("CARGO_MANIFEST_DIR"))
    .join("tests/fixtures")
    .join(subdir);

  let mut result = HashMap::new();

  for entry in std::fs::read_dir(&fixtures_dir).unwrap_or_else(|_| {
    panic!(
      "failed to read fixtures directory: {}",
      fixtures_dir.display()
    )
  }) {
    let entry = entry.expect("failed to read directory entry");
    let path = entry.path();
    if path.is_file() {
      let filename = path.file_name().unwrap().to_string_lossy().to_string();
      let contents = std::fs::read_to_string(&path)
        .unwrap_or_else(|_| panic!("failed to read fixture: {}", path.display()));
      result.insert(filename, Fixture { path, contents });
    }
  }

  result
}

/// Create a database with a vault project loaded from a fixture directory.
pub fn load_vault_fixture(
  vault_subdir: &str,
  file_path: &str,
) -> (TypedownDatabase, Project, File) {
  let vault = Path::new(env!("CARGO_MANIFEST_DIR"))
    .join("tests/fixtures")
    .join(vault_subdir);
  let db = TypedownDatabase {
    storage: QueryStorage::default(),
  };

  let target_path = vault.join(file_path);

  // Collect all .td and config files in the vault
  let mut files = collect_vault_files(&vault, &db);

  // Ensure the target file is registered
  let target_file = *files.entry(target_path.clone()).or_insert_with(|| {
    File::new(
      &db,
      FileHandle::Path(target_path.clone(), path_metadata(&target_path)),
    )
  });

  let project = Project::new(&db, vault, files);
  (db, project, target_file)
}

/// Collect all vault files (`.td` and `typedown.yaml`/`typedown.yml`) recursively.
fn collect_vault_files(dir: &Path, db: &TypedownDatabase) -> HashMap<PathBuf, File> {
  fn is_vault_file(path: &Path) -> bool {
    let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
    let is_asset = path
      .extension()
      .and_then(|ext| ext.to_str())
      .and_then(AssetKind::from_extension)
      .is_some();
    is_content_file(path) || is_asset || name == "typedown.yaml" || name == "typedown.yml"
  }

  fn walk(dir: &Path, db: &TypedownDatabase, files: &mut HashMap<PathBuf, File>) {
    if let Ok(entries) = std::fs::read_dir(dir) {
      for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
          walk(&path, db, files);
        } else if is_vault_file(&path) {
          let file = File::new(db, FileHandle::Path(path.clone(), path_metadata(&path)));
          files.insert(path, file);
        }
      }
    }
  }

  let mut files = HashMap::new();
  walk(dir, db, &mut files);
  files
}

fn path_metadata(path: &Path) -> FileMetadata {
  let meta = fs::metadata(path).ok();
  let mtime = meta
    .as_ref()
    .and_then(|m| m.modified().ok())
    .unwrap_or(SystemTime::UNIX_EPOCH);
  let ctime = meta
    .as_ref()
    .and_then(|m| m.created().ok())
    .unwrap_or(mtime);
  FileMetadata { mtime, ctime }
}
