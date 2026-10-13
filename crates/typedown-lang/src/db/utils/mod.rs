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

// Whether a container name is a built-in callout or built-in component
pub fn is_builtin_container(name: &str) -> bool {
  const BUILTIN_COMPONENT_NAMES: &[&str] =
    &["toc", "directory-index", "glossary-index", "flashcard"];

  is_callout(name) || BUILTIN_COMPONENT_NAMES.contains(&name)
}

// Extract the label (component name) from a ::: container block
// Handles kebab-case names like `td-alert` which are parsed as multiple tokens
pub fn extract_container_name(node: &RedNode) -> Option<String> {
  let mut seen_opening = false;
  let mut name = String::new();
  for child in node.children() {
    if child.kind() == SyntaxKind::MdSymbol && child.text() == ":::" && !seen_opening {
      seen_opening = true;
      continue;
    }
    if !seen_opening {
      continue;
    }
    match child.kind() {
      SyntaxKind::Whitespace => {
        // Skip leading whitespace, stop if name already started (whitespace ends the name)
        if !name.is_empty() {
          break;
        }
      }
      SyntaxKind::Ident => name.push_str(&child.text()),
      SyntaxKind::MdSymbol if child.text() == "-" && !name.is_empty() => name.push('-'),
      SyntaxKind::Newline => break,
      _ => {
        if !name.is_empty() {
          break;
        }
      }
    }
  }
  if name.is_empty() { None } else { Some(name) }
}

// Extract the component name from a [[shorthand]] container
pub fn extract_shorthand_name(node: &RedNode) -> Option<String> {
  let mut name = String::new();
  for child in node.children() {
    match child.kind() {
      SyntaxKind::Ident => name.push_str(&child.text()),
      SyntaxKind::MdSymbol if child.text() == "-" => name.push('-'),
      _ => {}
    }
  }
  if name.is_empty() { None } else { Some(name) }
}

// Extract prop key names from a container's {key=value} block
// The first Ident child of each MdContainerPropItem is the key
pub fn extract_container_prop_names(node: &RedNode) -> Vec<String> {
  let prop_block = node
    .children()
    .find(|child| child.kind() == SyntaxKind::MdContainerPropBlock);

  let Some(prop_block) = prop_block else {
    return Vec::new();
  };

  prop_block
    .children()
    .filter(|item| item.kind() == SyntaxKind::MdContainerPropItem)
    .filter_map(|item| {
      item
        .children()
        .find(|child| child.kind() == SyntaxKind::Ident)
        .map(|ident| ident.text().to_string())
    })
    .collect()
}

// Convert a kebab-case string to PascalCase
// e.g. "video-embed" -> "VideoEmbed", "chart" -> "Chart"
pub fn to_pascal_case(name: &str) -> String {
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
    .and_then(|e| e.to_str())
    .is_some_and(|ext| ext == "td" || ext == "md")
}

/// Whether a path is inside an underscore-prefixed directory (e.g. `_types`, `_partials`)
// These are excluded from content discovery
pub fn is_internal_file(path: &Path) -> bool {
  is_content_file(path)
    && path.parent().is_some_and(|p| {
      p.components()
        .any(|c| c.as_os_str().to_str().is_some_and(|s| s.starts_with('_')))
    })
}

/// Whether a path is located inside a `_types` directory anywhere in the vault
pub fn is_typ_file(path: &Path) -> bool {
  is_content_file(path) && path.components().any(|c| c.as_os_str() == "_types")
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
/// Paths starting with `./` or `../` are resolved relative to `source_file_dir`
/// All other paths are resolved relative to `vault_root`
pub fn resolve_fref_path(path: &str, source_file_dir: &Path, vault_root: &Path) -> PathBuf {
  if path.starts_with("./") || path.starts_with("../") {
    normalize_joined_path(source_file_dir, path)
  } else {
    vault_root.join(path)
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use std::path::Path;

  #[test]
  fn to_pascal_case_converts_hyphenated() {
    assert_eq!(to_pascal_case("video-embed"), "VideoEmbed");
    assert_eq!(to_pascal_case("chart"), "Chart");
    assert_eq!(to_pascal_case("my-custom-chart"), "MyCustomChart");
  }

  #[test]
  fn to_pascal_case_preserves_pascal() {
    assert_eq!(to_pascal_case("Chart"), "Chart");
    assert_eq!(to_pascal_case("VideoEmbed"), "VideoEmbed");
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
}
