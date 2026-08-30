use std::path::{Component, Path, PathBuf};

use lsp_types::Uri;

pub fn path_to_uri(path: &Path, scheme: &str) -> Uri {
  let mut uri_path = String::new();
  for component in path.components() {
    match component {
      Component::Prefix(prefix) => {
        // Windows drive prefix: C: -> /C:
        uri_path.push('/');
        uri_path.push_str(&prefix.as_os_str().to_string_lossy());
      }
      Component::RootDir if uri_path.is_empty() => {
        uri_path.push('/');
      }
      Component::Normal(seg) => {
        if !uri_path.ends_with('/') {
          uri_path.push('/');
        }
        uri_path.push_str(&seg.to_string_lossy());
      }
      _ => {}
    }
  }
  format!("{scheme}://{uri_path}")
    .parse()
    .unwrap_or_else(|_| "file:///".parse().unwrap())
}

pub fn uri_to_path(uri: &Uri) -> Option<PathBuf> {
  let path = uri.path().as_str();
  if path.is_empty() {
    log::warn!("URI has empty path: {}", uri.as_str());
    return None;
  }
  // On Windows, strip the leading / from paths like /C:/vault/...
  #[cfg(windows)]
  let path = path.strip_prefix('/').unwrap_or(path);
  Some(PathBuf::from(path))
}

pub fn uri_scheme(uri: &Uri) -> &str {
  let s = uri.as_str();
  s.find(':').map(|i| &s[..i]).unwrap_or("file")
}
