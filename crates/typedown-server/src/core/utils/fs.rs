use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::{fs, io};

use typedown_lang::db::types::{AssetKind, FileHandle, FileMetadata};
use typedown_lang::db::utils::is_content_file;

pub fn disk_handle(path: &Path) -> Option<FileHandle> {
  let meta = fs::metadata(path).ok()?;
  let mtime = meta.modified().ok()?;
  let ctime = meta.created().unwrap_or(mtime);
  Some(FileHandle::Path(
    path.to_path_buf(),
    FileMetadata { mtime, ctime },
  ))
}

pub fn scan_project_files(root: &Path) -> io::Result<HashSet<PathBuf>> {
  let mut files = HashSet::new();
  let (content_dir, schema_dir) = vault_dirs(root);

  scan_dir(&content_dir, &mut files)?;
  if schema_dir != content_dir {
    scan_dir(&schema_dir, &mut files)?;
  }

  // Include typedown.yaml/yml at root
  for name in ["typedown.yaml", "typedown.yml"] {
    let p = root.join(name);
    if p.exists() {
      files.insert(p);
    }
  }

  Ok(files)
}

// Read content_dir and schema_dir from typedown.yaml without the full DB
fn vault_dirs(root: &Path) -> (PathBuf, PathBuf) {
  let config = root
    .join("typedown.yaml")
    .exists()
    .then(|| root.join("typedown.yaml"))
    .or_else(|| {
      root
        .join("typedown.yml")
        .exists()
        .then(|| root.join("typedown.yml"))
    });

  let text = config
    .and_then(|p| fs::read_to_string(p).ok())
    .unwrap_or_default();

  let content_dir = text
    .lines()
    .find(|l| l.trim().starts_with("content_dir:"))
    .and_then(|l| l.split_once(':').map(|(_, v)| v.trim().to_string()))
    .unwrap_or_else(|| "content".to_string());

  let schema_dir = text
    .lines()
    .find(|l| l.trim().starts_with("schema_dir:"))
    .and_then(|l| l.split_once(':').map(|(_, v)| v.trim().to_string()))
    .unwrap_or_else(|| "schemas".to_string());

  (root.join(content_dir), root.join(schema_dir))
}

fn scan_dir(dir: &Path, files: &mut HashSet<PathBuf>) -> io::Result<()> {
  if !dir.is_dir() {
    return Ok(());
  }
  for entry in fs::read_dir(dir)? {
    let entry = entry?;
    let path = entry.path();
    if path.is_dir() {
      scan_dir(&path, files)?;
    } else if is_content_file(&path) || is_asset_file(&path) {
      files.insert(path);
    }
  }
  Ok(())
}

pub fn is_asset_file(path: &Path) -> bool {
  path
    .extension()
    .and_then(|ext| ext.to_str())
    .and_then(AssetKind::from_extension)
    .is_some()
}

pub fn is_vault_config(path: &Path) -> bool {
  matches!(
    path.file_name().and_then(|name| name.to_str()),
    Some("typedown.yaml") | Some("typedown.yml")
  )
}
