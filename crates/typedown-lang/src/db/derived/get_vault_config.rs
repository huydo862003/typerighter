//! Tracked query to get the vault configuration from typedown.yaml

use std::path::{Path, PathBuf};

use typedown_types::stream::Utf8Result;

use typedown_macros::query_derived;

use crate::syntax::diagnostic::Diagnostic;

use crate::db::TypedownDatabase;
use crate::db::types::{Project, VaultConfigResult};
use typedown_incremental::QueryDatabase;

#[query_derived]
pub fn get_vault_config<'db>(db: &'db TypedownDatabase, project: Project) -> VaultConfigResult<'db> {
  let root = project.root_dir(db);
  let mut diagnostics = Vec::new();

  let Some((config_path, contents)) = read_config_file(db, project, &root, &mut diagnostics) else {
    return VaultConfigResult::new(
      db,
      String::new(),
      PathBuf::new(),
      "/".to_string(),
      String::new(),
      String::new(),
      None,
      "public".to_string(),
      diagnostics,
    );
  };

  let Some(doc) = parse_yaml(&config_path, &contents, &mut diagnostics) else {
    return VaultConfigResult::new(
      db,
      String::new(),
      PathBuf::new(),
      "/".to_string(),
      String::new(),
      String::new(),
      None,
      "public".to_string(),
      diagnostics,
    );
  };

  let path_str = config_path.display().to_string();

  check_unknown_fields(&doc, &contents, &path_str, &mut diagnostics);

  let version = extract_version(&doc, &contents, &path_str, &mut diagnostics);
  let root_dir = extract_root_dir(&doc, &contents, &path_str, &root, &mut diagnostics);
  let base_path = extract_base_path(&doc, &contents, &path_str, &mut diagnostics);
  let site_title = doc["site"]["title"].as_str().unwrap_or("").to_string();
  let site_description = doc["site"]["description"]
    .as_str()
    .unwrap_or("")
    .to_string();
  let repo = doc["repo"].as_str().map(str::to_string);
  let public_dir = doc["site"]["public_dir"]
    .as_str()
    .unwrap_or("public")
    .to_string();

  VaultConfigResult::new(
    db,
    version,
    root_dir,
    base_path,
    site_title,
    site_description,
    repo,
    public_dir,
    diagnostics,
  )
}

/// Locate `typedown.yaml` (preferred) or `typedown.yml` in the project files, open it, and
/// return its resolved path and full text contents. Returns `None` and pushes a diagnostic if
/// the file is absent or cannot be opened.
fn read_config_file(
  db: &'db TypedownDatabase,
  project: Project,
  root: &Path,
  diagnostics: &mut Vec<Diagnostic>,
) -> Option<(PathBuf, String)> {
  let files = project.files(db);
  let yaml_path = root.join("typedown.yaml");
  let yml_path = root.join("typedown.yml");

  let (config_path, config_file) = if let Some(file) = files.get(&yaml_path) {
    (yaml_path, *file)
  } else if let Some(file) = files.get(&yml_path) {
    (yml_path, *file)
  } else {
    diagnostics.push(Diagnostic::MissingVaultConfig {
      root_dir: root.display().to_string(),
    });
    return None;
  };

  let mut reader = match config_file.handle(db).open() {
    Ok(reader) => reader,
    Err(err) => {
      diagnostics.push(Diagnostic::VaultConfigReadError {
        path: config_path.display().to_string(),
        message: err.to_string(),
      });
      return None;
    }
  };

  let mut contents = String::new();
  let mut char_offset = 0usize;
  loop {
    match reader.advance() {
      Utf8Result::Char(ch) => {
        contents.push(ch);
        char_offset += 1;
      }
      Utf8Result::Invalid { len, .. } => {
        diagnostics.push(Diagnostic::InvalidUtf8 {
          start_offset: char_offset,
          end_offset: char_offset + len,
        });
        char_offset += len;
      }
      Utf8Result::Eof => break,
    }
  }

  Some((config_path, contents))
}

/// Parse the YAML source text and return the first document
/// Returns `None` and pushes a diagnostic if parsing fails or the document is empty
fn parse_yaml(
  config_path: &Path,
  contents: &str,
  diagnostics: &mut Vec<Diagnostic>,
) -> Option<yaml_rust2::Yaml> {
  let path_str = config_path.display().to_string();

  let mut docs = match yaml_rust2::YamlLoader::load_from_str(contents) {
    Ok(docs) => docs,
    Err(err) => {
      let offset = err.marker().index();
      diagnostics.push(Diagnostic::VaultConfigParseError {
        path: path_str,
        message: err.to_string(),
        start_offset: offset,
        end_offset: offset,
      });
      return None;
    }
  };

  if docs.is_empty() {
    diagnostics.push(Diagnostic::VaultConfigEmpty { path: path_str });
    return None;
  }

  Some(docs.swap_remove(0))
}

/// Walk the top-level mapping and the `vault` sub-mapping, pushing a diagnostic for every key
/// that is not part of the expected schema.
fn check_unknown_fields(
  doc: &yaml_rust2::Yaml,
  contents: &str,
  path_str: &str,
  diagnostics: &mut Vec<Diagnostic>,
) {
  if let Some(hash) = doc.as_hash() {
    for key in hash.keys() {
      if let Some(key_str) = key.as_str()
        && !matches!(key_str, "version" | "vault" | "build" | "site" | "repo")
      {
        let offset = key_char_offset(contents, key_str).unwrap_or(0);
        diagnostics.push(Diagnostic::VaultConfigUnknownField {
          path: path_str.to_string(),
          field: key_str.to_string(),
          start_offset: offset,
          end_offset: offset + key_str.chars().count(),
        });
      }
    }
  }

  if let Some(vault_hash) = doc["vault"].as_hash() {
    for key in vault_hash.keys() {
      if let Some(key_str) = key.as_str()
        && !matches!(key_str, "root_dir")
      {
        let offset = key_char_offset(contents, key_str).unwrap_or(0);
        diagnostics.push(Diagnostic::VaultConfigUnknownField {
          path: path_str.to_string(),
          field: format!("vault.{key_str}"),
          start_offset: offset,
          end_offset: offset + key_str.chars().count(),
        });
      }
    }
  }
}

/// Extract the `version` string, pushing a missing-field diagnostic if absent.
fn extract_version(
  doc: &yaml_rust2::Yaml,
  contents: &str,
  path_str: &str,
  diagnostics: &mut Vec<Diagnostic>,
) -> String {
  doc["version"].as_str().map_or_else(
    || {
      let offset = key_char_offset(contents, "version").unwrap_or(0);
      diagnostics.push(Diagnostic::VaultConfigMissingField {
        path: path_str.to_string(),
        field: "version".to_string(),
        start_offset: offset,
        end_offset: offset,
      });
      String::new()
    },
    |s| s.to_string(),
  )
}

/// Extract `vault.root_dir` as an absolute path, defaulting to `root` if absent.
fn extract_root_dir(
  doc: &yaml_rust2::Yaml,
  _contents: &str,
  _path_str: &str,
  root: &Path,
  _diagnostics: &mut Vec<Diagnostic>,
) -> PathBuf {
  doc["vault"]["root_dir"].as_str().map_or_else(
    || root.to_path_buf(),
    |s| {
      if s == "." || s.is_empty() {
        root.to_path_buf()
      } else {
        root.join(s)
      }
    },
  )
}

/// Extract `build.base_path`, defaulting to "/" if absent.
/// Validates that the value is a valid URL path segment.
fn extract_base_path(
  doc: &yaml_rust2::Yaml,
  contents: &str,
  path_str: &str,
  diagnostics: &mut Vec<Diagnostic>,
) -> String {
  let Some(raw) = doc["build"]["base_path"].as_str() else {
    return "/".to_string();
  };

  let trimmed = raw.trim_end_matches('/');
  let normalized = if trimmed.starts_with('/') {
    trimmed.to_string()
  } else {
    format!("/{trimmed}")
  };

  let is_valid = normalized.is_ascii()
    && !normalized.contains(' ')
    && !normalized.contains('#')
    && !normalized.contains('?');

  if !is_valid {
    let offset = key_char_offset(contents, "base_path").unwrap_or(0);
    diagnostics.push(Diagnostic::VaultConfigInvalidValue {
      path: path_str.to_string(),
      field: "build.base_path".to_string(),
      message: format!("invalid URL path: {raw}"),
      start_offset: offset,
      end_offset: offset + raw.chars().count(),
    });
    return "/".to_string();
  }

  normalized
}

/// Find the char offset of `key:` in the source text, returning `None` if the key is absent
fn key_char_offset(source: &str, key: &str) -> Option<usize> {
  let pattern = format!("{}:", key);
  let byte_offset = source.find(pattern.as_str())?;
  // Convert byte offset to char offset.
  Some(source[..byte_offset].chars().count())
}
