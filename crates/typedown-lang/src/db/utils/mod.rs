//! Shared utilities for derived queries

pub mod static_type;

use crate::syntax::ast::{AstNode, SourceFile};
use crate::syntax::diagnostic::Diagnostic;
use crate::syntax::red::RedNode;
use crate::syntax::syntax_kind::SyntaxKind;

use std::path::Path;

use crate::db::TypedownDatabase;
use crate::db::derived::hir::lower_node;
use crate::db::derived::parse_file::parse_file;
use crate::db::types::{File, HirValue, Project};

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
pub fn is_type_file(path: &Path) -> bool {
  is_content_file(path) && path.components().any(|c| c.as_os_str() == "_types")
}

/// Strip a content file extension (.td or .md) from a string
pub fn strip_content_extension(s: &str) -> &str {
  s.strip_suffix(".td")
    .or_else(|| s.strip_suffix(".md"))
    .unwrap_or(s)
}

pub fn lower_file(
  db: &TypedownDatabase,
  project: Project,
  file: File,
) -> (Option<HirValue<'_>>, Vec<Diagnostic>) {
  let parse_result = parse_file(db, project, file);
  let diagnostics = parse_result.diagnostics(db).to_vec();
  let root = parse_result.ast(db);
  if SourceFile::cast(root.clone()).is_none() {
    return (None, diagnostics);
  }
  let hir = lower_node(db, project, file, root);
  (Some(hir), diagnostics)
}

/// Check if a file has no frontmatter (schemaless)
pub fn is_schemaless_file(db: &TypedownDatabase, project: Project, file: File) -> bool {
  let result = parse_file(db, project, file);
  let root = result.ast(db);
  let source_file = match SourceFile::cast(root) {
    Some(source_file) => source_file,
    None => return false,
  };
  !source_file.has_nonempty_frontmatter()
}

/// Find the value of the _type field in a mapping or dict node
pub fn schema_name_in_mapping(mapping: &RedNode) -> Option<String> {
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

#[cfg(test)]
mod tests {
  use super::*;
  use std::path::Path;

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
