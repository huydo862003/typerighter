use std::path::Path;

/// Normalize a path to use forward slashes, for cross-platform consistency
pub fn normalize_path(path: &Path) -> String {
  path
    .components()
    .map(|component| component.as_os_str().to_string_lossy())
    .collect::<Vec<_>>()
    .join("/")
}
