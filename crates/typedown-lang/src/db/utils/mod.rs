//! Shared utilities for derived queries

pub mod static_type;

use std::path::{Component, Path, PathBuf};

use crate::db::TypedownDatabase;
use crate::db::derived::parse_file::parse_file;
use crate::db::types::{File, Project};
use crate::syntax::ast::{AstNode, SourceFile};
use crate::syntax::red::RedNode;
use crate::syntax::syntax_kind::SyntaxKind;

// Whether a container name is a built-in callout type
pub fn is_callout(name: &str) -> bool {
  matches!(
    name,
    "tip" | "info" | "warning" | "danger" | "details" | "note" | "important" | "caution"
  )
}

// Convert a kebab-case string to PascalCase
// e.g. "video-embed" -> "VideoEmbed", "chart" -> "Chart"
pub fn kebab_to_pascal(name: &str) -> String {
  name
    .split('-')
    .map(|segment| {
      let mut chars = segment.chars();
      match chars.next() {
        Some(first) => {
          let upper: String = first.to_uppercase().collect();
          upper + chars.as_str()
        }
        None => String::new(),
      }
    })
    .collect()
}

// Convert a snake_case string to kebab-case
// e.g. "layout_top" -> "layout-top"
pub fn snake_to_kebab(name: &str) -> String {
  name.replace('_', "-")
}

pub fn is_external_url(url: &str) -> bool {
  url.starts_with("http://")
    || url.starts_with("https://")
    || url.starts_with("//")
    || url.starts_with("mailto:")
}

/// Whether a path has a content file extension (.td or .md)
pub fn is_content_file(path: &Path) -> bool {
  path
    .extension()
    .and_then(|extension| extension.to_str())
    .is_some_and(|ext| ext == "td" || ext == "md")
}

/// Whether a path is inside an underscore-prefixed directory (e.g. `_types`, `_partials`)
// These are excluded from content discovery
pub fn is_internal_file(path: &Path) -> bool {
  is_content_file(path)
    && path.parent().is_some_and(|path| {
      path.components().any(|component| {
        component
          .as_os_str()
          .to_str()
          .is_some_and(|string| string.starts_with('_'))
      })
    })
}

/// Whether a path is located inside a `_types` directory anywhere in the vault
pub fn is_typ_file(path: &Path) -> bool {
  is_content_file(path)
    && path
      .components()
      .any(|component| component.as_os_str() == "_types")
}

/// Strip a content file extension (.td or .md) from a string
pub fn strip_content_extension(s: &str) -> &str {
  s.strip_suffix(".td")
    .or_else(|| s.strip_suffix(".md"))
    .unwrap_or(s)
}

/// Check if a file has no frontmatter (schemaless)
pub fn is_schemaless_file(db: &TypedownDatabase, project: Project, file: File) -> bool {
  let result = parse_file(db, project, file);
  let root = result.ast(db).node.clone();
  let source_file = match SourceFile::cast(root) {
    Some(source_file) => source_file,
    None => return false,
  };
  !source_file.has_nonempty_frontmatter()
}

/// Find the value of the _type field in a mapping or dict node
pub fn get_mapping_schema_name(mapping: &RedNode) -> Option<String> {
  for entry in mapping.children() {
    // Block mapping entry
    if entry.kind() == SyntaxKind::YamlMappingEntry {
      let mut children = entry.children();
      let key = children.find(|child| child.kind() == SyntaxKind::YamlMappingEntryKey)?;
      if key.text().trim() != "_type" {
        continue;
      }
      let value = children.find(|child| child.kind() == SyntaxKind::YamlMappingEntryValue)?;
      return Some(value.text().trim().to_string());
    }
    // Flow dict entry
    if entry.kind() == SyntaxKind::DictEntry {
      let mut children = entry.children();
      let key = children.find(|child| child.kind() == SyntaxKind::DictEntryKey)?;
      if key.text().trim() != "_type" {
        continue;
      }
      let value = children.find(|child| child.kind() == SyntaxKind::DictEntryValue)?;
      return Some(value.text().trim().to_string());
    }
  }
  None
}

/// Join `base` with `relative` and collapse `.` and `..` components
pub fn normalize_joined_path(base: &Path, relative: &str) -> PathBuf {
  let joined = base.join(relative);
  let mut result = PathBuf::new();
  for component in joined.components() {
    match component {
      Component::CurDir => {}
      Component::ParentDir => {
        result.pop();
      }
      other => result.push(other),
    }
  }
  result
}

/// Resolve a fref path string to an absolute file system path
/// - Paths starting with `/` are resolved relative to `vault_root`
/// - Paths starting with `./` or `../` are resolved relative to `source_file_dir`
pub fn resolve_fref_path(path: &str, source_file_dir: &Path, vault_root: &Path) -> PathBuf {
  if path.starts_with("./") || path.starts_with("../") {
    normalize_joined_path(source_file_dir, path)
  } else {
    // Handles absolute paths starting with `/` (strip leading slash before joining)
    vault_root.join(path.trim_start_matches('/'))
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use std::path::Path;

  #[test]
  fn kebab_to_pascal_converts_hyphenated() {
    assert_eq!(kebab_to_pascal("video-embed"), "VideoEmbed");
    assert_eq!(kebab_to_pascal("chart"), "Chart");
    assert_eq!(kebab_to_pascal("my-custom-chart"), "MyCustomChart");
  }

  #[test]
  fn kebab_to_pascal_preserves_pascal() {
    assert_eq!(kebab_to_pascal("Chart"), "Chart");
    assert_eq!(kebab_to_pascal("VideoEmbed"), "VideoEmbed");
  }

  #[test]
  fn snake_to_kebab_converts() {
    assert_eq!(snake_to_kebab("layout_top"), "layout-top");
    assert_eq!(snake_to_kebab("doc_content_after"), "doc-content-after");
    assert_eq!(snake_to_kebab("sidebar"), "sidebar");
  }

  #[test]
  fn is_content_file_accepts_td() {
    assert!(is_content_file(Path::new("file.td")));
    assert!(is_content_file(Path::new("path/to/file.td")));
  }

  #[test]
  fn is_content_file_accepts_md() {
    assert!(is_content_file(Path::new("file.md")));
    assert!(is_content_file(Path::new("path/to/file.md")));
  }

  #[test]
  fn is_content_file_rejects_other() {
    assert!(!is_content_file(Path::new("file.txt")));
    assert!(!is_content_file(Path::new("file.yaml")));
    assert!(!is_content_file(Path::new("file.png")));
    assert!(!is_content_file(Path::new("file")));
  }

  #[test]
  fn strip_content_extension_strips_td() {
    assert_eq!(strip_content_extension("file.td"), "file");
    assert_eq!(strip_content_extension("path/to/file.td"), "path/to/file");
  }

  #[test]
  fn strip_content_extension_strips_md() {
    assert_eq!(strip_content_extension("file.md"), "file");
    assert_eq!(strip_content_extension("path/to/file.md"), "path/to/file");
  }

  #[test]
  fn strip_content_extension_preserves_other() {
    assert_eq!(strip_content_extension("file.txt"), "file.txt");
    assert_eq!(strip_content_extension("file"), "file");
  }

  // resolve_fref_path: absolute `/` paths are relative to vault_root, not filesystem root
  #[test]
  fn resolve_fref_absolute_path_nested_vault() {
    // vault lives at /project/vault, file is in a subdirectory
    let vault_root = Path::new("/project/vault");
    let source_file_dir = Path::new("/project/vault/articles/webrtc");
    let result = resolve_fref_path("/articles/intro.td", source_file_dir, vault_root);
    assert_eq!(result, PathBuf::from("/project/vault/articles/intro.td"));
  }

  #[test]
  fn resolve_fref_absolute_path_vault_at_project_root() {
    // vault_root is `.` - the project root itself
    let vault_root = Path::new("/project");
    let source_file_dir = Path::new("/project/articles");
    let result = resolve_fref_path("/articles/intro.td", source_file_dir, vault_root);
    assert_eq!(result, PathBuf::from("/project/articles/intro.td"));
  }

  #[test]
  fn resolve_fref_relative_dot_path_resolves_from_source_dir() {
    let vault_root = Path::new("/project/vault");
    let source_file_dir = Path::new("/project/vault/articles/webrtc");
    let result = resolve_fref_path("./peer.td", source_file_dir, vault_root);
    assert_eq!(
      result,
      PathBuf::from("/project/vault/articles/webrtc/peer.td")
    );
  }

  #[test]
  fn resolve_fref_relative_parent_path_resolves_from_source_dir() {
    let vault_root = Path::new("/project/vault");
    let source_file_dir = Path::new("/project/vault/articles/webrtc");
    let result = resolve_fref_path("../intro.td", source_file_dir, vault_root);
    assert_eq!(result, PathBuf::from("/project/vault/articles/intro.td"));
  }
}
