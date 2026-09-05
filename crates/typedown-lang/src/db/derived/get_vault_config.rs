//! Tracked query to get the vault configuration from typedown.yaml

use std::path::Path;

use typedown_macros::query_derived;

use crate::db::TypedownDatabase;
use crate::db::derived::evaluate::evaluate_config::evaluate_config;
use crate::db::types::derived::object_system::TdRuntimeObject;
use crate::db::types::{Project, TdObjectEnum, VaultConfigResult};
use typedown_incremental::QueryDatabase;

#[query_derived]
pub fn get_vault_config<'db>(
  db: &'db TypedownDatabase,
  project: Project,
) -> VaultConfigResult<'db> {
  let root = project.root_dir(db).clone();
  let result = evaluate_config(db, project);
  let diagnostics = result.diagnostics(db).to_vec();

  let Some(obj) = result.value(db) else {
    return empty_config(db, &root, diagnostics);
  };

  let version = get_str_field(db, &obj, "version").unwrap_or_default();
  let repo = get_str_field(db, &obj, "repo");

  // Extract vault.root_dir
  let root_dir = get_nested_str_field(db, &obj, "vault", "root_dir")
    .map(|s| {
      if s.is_empty() || s == "." {
        root.clone()
      } else {
        root.join(s)
      }
    })
    .unwrap_or_else(|| root.clone());

  // Extract site fields
  let site = get_field(db, &obj, "site");
  let site_title = site_str(db, &site, "title").unwrap_or_default();
  let site_description = site_str(db, &site, "description").unwrap_or_default();
  let base_path = site_str(db, &site, "base_path")
    .map(|s| normalize_base_path(&s))
    .unwrap_or_else(|| "/".to_string());
  let author = site_str(db, &site, "author");
  let license = site_str(db, &site, "license");
  let public_dir = site_str(db, &site, "public_dir").unwrap_or_else(|| "public".to_string());
  let nav = extract_nav(db, &site);
  if nav.len() > 4 {
    use std::sync::Once;
    static WARN: Once = Once::new();
    WARN.call_once(|| {
      eprintln!(
        "[typedown] warning: site.nav has {} items (max recommended: 4)",
        nav.len()
      );
    });
  }

  VaultConfigResult::new(
    db,
    version,
    root_dir,
    base_path,
    site_title,
    site_description,
    repo,
    author,
    license,
    public_dir,
    nav,
    diagnostics,
  )
}

fn empty_config<'db>(
  db: &'db TypedownDatabase,
  root: &Path,
  diagnostics: Vec<crate::syntax::diagnostic::Diagnostic>,
) -> VaultConfigResult<'db> {
  VaultConfigResult::new(
    db,
    String::new(),
    root.to_path_buf(),
    "/".to_string(),
    String::new(),
    String::new(),
    None,
    None,
    None,
    "public".to_string(),
    Vec::new(),
    diagnostics,
  )
}

fn get_field<'db>(
  db: &'db TypedownDatabase,
  obj: &TdObjectEnum<'db>,
  key: &str,
) -> Option<TdObjectEnum<'db>> {
  let field = obj.get_owned_field(db, key)?;

  if field.is_td_null_obj() {
    None
  } else {
    Some(field)
  }
}

fn get_str_field<'db>(
  db: &'db TypedownDatabase,
  obj: &TdObjectEnum<'db>,
  key: &str,
) -> Option<String> {
  get_field(db, obj, key).and_then(|o| o.as_td_str_obj().map(|s| s.value(db)))
}

fn get_nested_str_field<'db>(
  db: &'db TypedownDatabase,
  obj: &TdObjectEnum<'db>,
  parent_key: &str,
  child_key: &str,
) -> Option<String> {
  let parent = get_field(db, obj, parent_key)?;
  get_str_field(db, &parent, child_key)
}

fn site_str<'db>(
  db: &'db TypedownDatabase,
  site: &Option<TdObjectEnum<'db>>,
  key: &str,
) -> Option<String> {
  site.as_ref().and_then(|s| get_str_field(db, s, key))
}

fn extract_nav<'db>(
  db: &'db TypedownDatabase,
  site: &Option<TdObjectEnum<'db>>,
) -> Vec<(String, String, Option<String>)> {
  let Some(site) = site else {
    return Vec::new();
  };
  let Some(list) = get_field(db, site, "nav") else {
    return Vec::new();
  };
  let Some(list_obj) = list.as_td_list_obj() else {
    return Vec::new();
  };

  let mut items = Vec::new();
  let Some(len) = list_obj.len(db) else {
    return items;
  };
  for i in 0..len {
    let Some(item) = list_obj.get(db, i) else {
      continue;
    };
    let title = get_str_field(db, &item, "title").unwrap_or_default();
    let link = get_str_field(db, &item, "link").unwrap_or_default();
    let icon =
      get_field(db, &item, "icon").and_then(|o| o.as_td_icon_obj().map(|i| i.lucide_name(db)));
    if !title.is_empty() && !link.is_empty() {
      items.push((title, link, icon));
    }
  }
  items
}

// Normalize and validate base_path
fn normalize_base_path(raw: &str) -> String {
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

  if is_valid {
    normalized
  } else {
    "/".to_string()
  }
}

#[cfg(test)]
mod tests {
  use crate::db::fixtures::load_vault_fixture;

  use super::*;

  #[test]
  fn extracts_nav_items() {
    let (db, project, _) = load_vault_fixture("evaluate/nav_vault", "page.td");
    let config = get_vault_config(&db, project);
    let nav = config.nav_items(&db);
    // Items with empty text or missing text are skipped
    assert_eq!(nav.len(), 2);
    // Item with icon
    assert_eq!(nav[0].0, "Guide");
    assert_eq!(nav[0].1, "/guide");
    assert_eq!(nav[0].2, Some("book-open".to_string()));
    // Item without icon
    assert_eq!(nav[1].0, "GitHub");
    assert_eq!(nav[1].1, "https://github.com/example");
    assert_eq!(nav[1].2, None);
  }

  #[test]
  fn empty_nav_when_not_configured() {
    let (db, project, _) = load_vault_fixture("evaluate/my_vault", "valid_person.td");
    let config = get_vault_config(&db, project);
    assert!(config.nav_items(&db).is_empty());
  }
}
