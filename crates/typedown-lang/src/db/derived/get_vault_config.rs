//! Tracked query to get the vault configuration from typedown.yaml

use std::collections::BTreeMap;
use std::path::Path;

use typedown_macros::query_derived;

use crate::db::TypedownDatabase;
use crate::db::derived::evaluate::evaluate_config::evaluate_config;
use crate::db::types::derived::obj_system::TdRuntimeObj;
use crate::db::types::{
  CustomComponentEntry, CustomLayoutConfig, Project, TdObjEnum, VaultConfigResult,
};
use crate::syntax::diagnostic::Diagnostic;
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

  let version = get_string_field(db, &obj, "version").unwrap_or_default();

  // repo: "url" (string shorthand) or repo: { url: "...", root: "..." } (object form)
  let (repo_url, repo_root) = match get_string_field(db, &obj, "repo") {
    Some(url) => (Some(url), None),
    None => match get_field(db, &obj, "repo") {
      Some(repo_obj) => (
        get_string_field(db, &repo_obj, "url"),
        get_string_field(db, &repo_obj, "root"),
      ),
      None => (None, None),
    },
  };

  // Extract vault.root_dir
  let root_dir = get_nested_string_field(db, &obj, "vault", "root_dir")
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
  let site_title = get_site_string(db, &site, "title").unwrap_or_default();
  let site_description = get_site_string(db, &site, "description").unwrap_or_default();
  let origin = get_site_string(db, &site, "origin").map(|s| normalize_origin(&s));
  let lang = get_site_string(db, &site, "lang").unwrap_or_else(|| "en".to_string());
  let base_path = get_site_string(db, &site, "base_path")
    .map(|s| normalize_base_path(&s))
    .unwrap_or_else(|| "/".to_string());
  let author = get_site_string(db, &site, "author");
  let license = get_site_string(db, &site, "license");
  let public_dir = get_site_string(db, &site, "public_dir").unwrap_or_else(|| "public".to_string());
  // Extract theme fields
  let theme = get_field(db, &obj, "theme");
  let component_entries = extract_theme_components(db, &theme);
  let layout_config = extract_theme_layout(db, &theme);

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
    origin,
    lang,
    site_title,
    site_description,
    repo_url,
    repo_root,
    author,
    license,
    public_dir,
    nav,
    component_entries,
    Some(layout_config),
    diagnostics,
  )
}

fn empty_config<'db>(
  db: &'db TypedownDatabase,
  root: &Path,
  diagnostics: Vec<Diagnostic>,
) -> VaultConfigResult<'db> {
  VaultConfigResult::new(
    db,
    String::new(),
    root.to_path_buf(),
    "/".to_string(),
    None,
    "en".to_string(),
    String::new(),
    String::new(),
    None,
    None,
    None,
    None,
    "public".to_string(),
    Vec::new(),
    Vec::new(),
    None,
    diagnostics,
  )
}

fn get_field<'db>(
  db: &'db TypedownDatabase,
  obj: &TdObjEnum<'db>,
  key: &str,
) -> Option<TdObjEnum<'db>> {
  let field = obj.get_owned_field(db, key)?;

  if field.is_td_null_obj() {
    None
  } else {
    Some(field)
  }
}

fn get_string_field<'db>(
  db: &'db TypedownDatabase,
  obj: &TdObjEnum<'db>,
  key: &str,
) -> Option<String> {
  get_field(db, obj, key).and_then(|o| o.as_td_string_obj().map(|string| string.value(db)))
}

fn get_nested_string_field<'db>(
  db: &'db TypedownDatabase,
  obj: &TdObjEnum<'db>,
  parent_key: &str,
  child_key: &str,
) -> Option<String> {
  let parent = get_field(db, obj, parent_key)?;
  get_string_field(db, &parent, child_key)
}

fn get_site_string<'db>(
  db: &'db TypedownDatabase,
  site: &Option<TdObjEnum<'db>>,
  key: &str,
) -> Option<String> {
  site.as_ref().and_then(|s| get_string_field(db, s, key))
}

fn extract_nav<'db>(
  db: &'db TypedownDatabase,
  site: &Option<TdObjEnum<'db>>,
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
    let title = get_string_field(db, &item, "title").unwrap_or_default();
    let link = get_string_field(db, &item, "link").unwrap_or_default();
    let icon = get_field(db, &item, "icon")
      .and_then(|object| object.as_td_icon_obj().map(|icon| icon.lucide_name(db)));
    if !title.is_empty() && !link.is_empty() {
      items.push((title, link, icon));
    }
  }
  items
}

// Extract theme.components entries from the evaluated config object
fn extract_theme_components<'db>(
  db: &'db TypedownDatabase,
  theme: &Option<TdObjEnum<'db>>,
) -> Vec<CustomComponentEntry> {
  let Some(theme) = theme else {
    return Vec::new();
  };
  let Some(components) = get_field(db, theme, "components") else {
    return Vec::new();
  };
  let Some(list) = components.as_td_list_obj() else {
    return Vec::new();
  };

  let mut entries = Vec::new();
  let Some(len) = list.len(db) else {
    return entries;
  };
  for i in 0..len {
    let Some(item) = list.get(db, i) else {
      continue;
    };

    // String item: folder path to scan for .vue files
    if let Some(path) = item.as_td_string_obj().map(|s| s.value(db)) {
      entries.push(CustomComponentEntry::Dir { path });
      continue;
    }

    // Mapping item: single {name: source} entry
    if let Some(product) = item.as_td_product_obj() {
      for key in product.fields(db).keys() {
        if let Some(source) = get_string_field(db, &item, key) {
          entries.push(CustomComponentEntry::Explicit {
            name: key.clone(),
            source,
          });
        }
      }
    }
  }
  entries
}

// Extract theme.layout: string path or { slots: { name: component } }
fn extract_theme_layout<'db>(
  db: &'db TypedownDatabase,
  theme: &Option<TdObjEnum<'db>>,
) -> CustomLayoutConfig {
  let Some(theme) = theme else {
    return CustomLayoutConfig::default();
  };
  let Some(layout) = get_field(db, theme, "layout") else {
    return CustomLayoutConfig::default();
  };

  // String form: theme.layout = "./path/to/Layout.vue"
  if let Some(path) = layout.as_td_string_obj().map(|s| s.value(db)) {
    return CustomLayoutConfig {
      component: Some(path),
      ..CustomLayoutConfig::default()
    };
  }

  // Object form: theme.layout = { component, slots, not_found }
  let component = get_string_field(db, &layout, "component");
  let not_found = get_string_field(db, &layout, "not_found");

  let mut slots = BTreeMap::new();
  if let Some(slots_obj) = get_field(db, &layout, "slots") {
    // Try as dict first (dynamic keys), then as product (typed fields)
    let keys: Vec<String> = if let Some(dict) = slots_obj.as_td_dict_obj() {
      dict.entries(db).keys().cloned().collect()
    } else if let Some(product) = slots_obj.as_td_product_obj() {
      product.fields(db).keys().cloned().collect()
    } else {
      Vec::new()
    };

    for key in keys {
      if let Some(comp) = get_string_field(db, &slots_obj, &key) {
        slots.insert(key, comp);
      }
    }
  }

  CustomLayoutConfig {
    component,
    slots,
    not_found,
  }
}

// Normalize origin to include scheme, default to https
fn normalize_origin(raw: &str) -> String {
  let trimmed = raw.trim_end_matches('/');
  if trimmed.starts_with("http://") || trimmed.starts_with("https://") {
    trimmed.to_string()
  } else {
    format!("https://{trimmed}")
  }
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

  #[test]
  fn repo_string_shorthand() {
    let (db, project, _) = load_vault_fixture("evaluate/repo_string_vault", "page.td");
    let config = get_vault_config(&db, project);
    assert_eq!(
      config.repo_url(&db).as_deref(),
      Some("https://github.com/example/repo")
    );
    assert_eq!(config.repo_root(&db).as_ref(), None);
  }

  #[test]
  fn layout_string_form() {
    let (db, project, _) = load_vault_fixture("evaluate/layout_string_vault", "page.td");
    let config = get_vault_config(&db, project);
    assert_eq!(
      config.layout(&db).unwrap().component.as_deref(),
      Some("./theme/MyLayout.vue")
    );
    assert!(config.layout(&db).unwrap().slots.is_empty());
  }

  #[test]
  fn layout_slots_form() {
    let (db, project, _) = load_vault_fixture("evaluate/layout_slots_vault", "page.td");
    let config = get_vault_config(&db, project);
    let binding = config.layout(&db);
    let layout = binding.as_ref().expect("layout should be Some");
    assert!(layout.component.is_none());
    let slots = &layout.slots;
    assert_eq!(slots.len(), 2);
    assert_eq!(
      slots.get("layout_top"),
      Some(&"AnnouncementBar".to_string())
    );
    assert_eq!(slots.get("doc_after"), Some(&"CommentSection".to_string()));
  }

  #[test]
  fn layout_not_found() {
    let (db, project, _) = load_vault_fixture("evaluate/layout_not_found_vault", "page.td");
    let config = get_vault_config(&db, project);
    let binding = config.layout(&db);
    let layout = binding.as_ref().expect("layout should be Some");
    assert_eq!(layout.component.as_deref(), Some("./theme/MyLayout.vue"));
    assert_eq!(layout.not_found.as_deref(), Some("./theme/NotFound.vue"));
    assert!(layout.slots.is_empty());
  }

  #[test]
  fn repo_object_form() {
    let (db, project, _) = load_vault_fixture("evaluate/repo_vault", "page.td");
    let config = get_vault_config(&db, project);
    assert_eq!(
      config.repo_url(&db).as_deref(),
      Some("https://github.com/example/repo")
    );
    assert_eq!(config.repo_root(&db).as_deref(), Some("docs"));
  }
}
