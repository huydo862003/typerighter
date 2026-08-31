use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

use typedown_incremental::{QueryDatabase, query_derived};
use typedown_types::either::Either;

use crate::db::types::*;
use crate::db::utils::is_content_file;
use crate::db::{QueryStorage, TypedownDatabase};

// Factory functions for creating derived structs in test query context

#[query_derived]
pub fn make_str_obj<'db>(db: &'db TypedownDatabase, value: String) -> TdStrObj<'db> {
  TdStrObj::new(db, value)
}

#[query_derived]
pub fn make_num_obj<'db>(db: &'db TypedownDatabase, value: u64) -> TdNumObj<'db> {
  TdNumObj::new(db, f64::from_bits(value))
}

#[query_derived]
pub fn make_bool_obj<'db>(db: &'db TypedownDatabase, value: bool) -> TdBoolObj<'db> {
  TdBoolObj::new(db, value)
}

#[query_derived]
pub fn make_math_obj<'db>(db: &'db TypedownDatabase, value: String) -> TdMathObj<'db> {
  TdMathObj::new(db, value)
}

#[query_derived]
pub fn make_datetime_obj<'db>(db: &'db TypedownDatabase, value: String) -> TdDateTimeObj<'db> {
  TdDateTimeObj::new(db, value)
}

#[query_derived]
pub fn make_date_obj<'db>(db: &'db TypedownDatabase, value: String) -> TdDateObj<'db> {
  TdDateObj::new(db, value)
}

#[query_derived]
pub fn make_time_obj<'db>(db: &'db TypedownDatabase, value: String) -> TdTimeObj<'db> {
  TdTimeObj::new(db, value)
}

#[query_derived]
pub fn make_blob_obj<'db>(
  db: &'db TypedownDatabase,
  asset_kind: AssetKind,
  file: File,
) -> TdBlobObj<'db> {
  TdBlobObj::new(db, asset_kind, file)
}

// Factory functions for types with HashMap fields
// Accept Vec of tuples (hashable) and convert to HashMap inside

#[query_derived]
pub fn make_list_obj<'db>(
  db: &'db TypedownDatabase,
  items: Vec<Either<HirValue<'db>, TdObjectEnum<'db>>>,
) -> TdListObj<'db> {
  TdListObj::new(db, items)
}

#[query_derived]
pub fn make_dict_obj<'db>(
  db: &'db TypedownDatabase,
  entries: Vec<(String, Either<HirValue<'db>, TdObjectEnum<'db>>)>,
) -> TdDictObj<'db> {
  TdDictObj::new(db, entries.into_iter().collect())
}

#[query_derived]
pub fn make_product_obj<'db>(
  db: &'db TypedownDatabase,
  product_type: TdTypeEnum<'db>,
  file_symbol: Option<Symbol<'db>>,
  fields: Vec<(String, Either<HirValue<'db>, TdObjectEnum<'db>>)>,
) -> TdProductObj<'db> {
  TdProductObj::new(db, product_type, file_symbol, fields.into_iter().collect())
}

#[query_derived]
pub fn make_product_type<'db>(
  db: &'db TypedownDatabase,
  name: Option<String>,
  fields: Vec<(String, LazyType<'db>)>,
) -> TdProductType<'db> {
  TdProductType::new(db, name, fields.into_iter().collect())
}

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
