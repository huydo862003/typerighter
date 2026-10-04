//! Reference resolution for fref interpolations and display names

use typedown_types::string::split_pascal_case;

use crate::db::TypedownDatabase;
use crate::db::derived::evaluate::evaluate_resource::evaluate_resource;
use crate::db::derived::evaluate::evaluate_type::evaluate_type;
use crate::db::derived::get_vault_config::get_vault_config;
use crate::db::derived::hir::lower_node;
use crate::db::derived::name_resolver::file_symbol::file_symbol;
use crate::db::derived::name_resolver::referee::referee;
use crate::db::utils::strip_content_extension;
use crate::db::types::{
  File, FileRedNode, Project, Symbol, SymbolKind, TdRuntimeObject,
};
use crate::syntax::red::RedNode;
use super::utils;

/// Resolved reference: display name and URL
pub struct ResolvedRef {
  pub name: String,
  pub url: String,
}

/// Resolve a symbol to a display name and URL
pub fn resolve_ref(
  db: &TypedownDatabase,
  project: Project,
  symbol: &Symbol,
) -> Option<ResolvedRef> {
  let name = resolve_display_name(db, project, symbol);

  match symbol.kind(db) {
    SymbolKind::UserDefinedResource(_, target_file)
    | SymbolKind::UserDefinedSchema(_, target_file) => {
      let handle = target_file.handle(db);
      let path = handle.path()?;
      let config = get_vault_config(db, project);
      let root_dir = config.root_dir(db);
      let relative = path.strip_prefix(&root_dir).unwrap_or(path);
      let relative_str = relative.to_string_lossy();
      let without_ext = strip_content_extension(&relative_str);
      let url = utils::prepend_base_path(&config.base_path(db), without_ext);
      Some(ResolvedRef { name, url })
    }
    SymbolKind::Asset(_, _, target_file) => {
      let handle = target_file.handle(db);
      let path = handle.path()?;
      let config = get_vault_config(db, project);
      let root_dir = config.root_dir(db);
      let relative = path.strip_prefix(&root_dir).unwrap_or(path);
      let url = utils::prepend_base_path(&config.base_path(db), &relative.to_string_lossy());
      Some(ResolvedRef { name, url })
    }
    _ => None,
  }
}

/// Resolved fref target with display name, URL, optional icon, and image flag
pub struct FrefTarget {
  pub name: String,
  pub url: String,
  pub icon: Option<String>,
  pub is_image: bool,
}

pub fn resolve_fref_target(
  db: &TypedownDatabase,
  project: Project,
  file: File,
  node: &RedNode,
) -> Option<FrefTarget> {
  let hir = lower_node(db, project, FileRedNode::new(file, node.clone()));
  let referee_result = referee(db, hir);
  let target_symbol = referee_result.value(db)?;
  let resolved = resolve_ref(db, project, &target_symbol)?;

  let is_image = matches!(
    target_symbol.kind(db),
    SymbolKind::Asset(asset_kind, _, _) if asset_kind.is_image()
  );

  let icon = if is_image {
    None
  } else {
    resolve_fref_icon(db, project, &target_symbol)
  };

  Some(FrefTarget {
    name: resolved.name,
    url: resolved.url,
    icon,
    is_image,
  })
}

/// Resolve a fref interpolation to a markdown link string with optional icon
pub(super) fn try_resolve_fref(
  db: &TypedownDatabase,
  project: Project,
  file: File,
  node: &RedNode,
) -> Option<String> {
  let target = resolve_fref_target(db, project, file, node)?;

  if target.is_image {
    return Some(format!("![{}]({})", target.name, target.url));
  }

  let icon_html = target
    .icon
    .map(|name| format!("<LucideIcon name=\"{}\" />", name))
    .unwrap_or_default();

  Some(format!("{}[{}]({})", icon_html, target.name, target.url))
}

// Get the lucide icon name for a fref target
fn resolve_fref_icon(db: &TypedownDatabase, project: Project, symbol: &Symbol) -> Option<String> {
  match symbol.kind(db) {
    SymbolKind::UserDefinedResource(_, target_file) => {
      let target_symbol = file_symbol(db, project, target_file).value(db)?;
      let obj = evaluate_resource(db, target_symbol).value(db)?;
      obj
        .get_builtin_field(db, "_icon")
        .and_then(|o| o.as_td_icon_obj().map(|i| i.lucide_name(db)))
    }
    SymbolKind::UserDefinedSchema(_, _) => {
      let typ = evaluate_type(db, *symbol).typ(db)?;
      typ
        .get_builtin_field(db, "_icon")
        .and_then(|o| o.as_td_icon_obj().map(|i| i.lucide_name(db)))
    }
    _ => None,
  }
}

pub fn resolve_schema_label(db: &TypedownDatabase, project: Project, file: File) -> String {
  // Try _label from the schema type
  if let Some(symbol) = file_symbol(db, project, file).value(db)
    && let Some(typ) = evaluate_type(db, symbol).typ(db)
    && let Some(label_obj) = typ.get_builtin_field(db, "_label")
    && let Some(str_obj) = label_obj.as_td_str_obj()
  {
    return str_obj.value(db);
  }

  // Fall back to PascalCase split of file stem
  let handle = file.handle(db);
  let stem = handle
    .path()
    .and_then(|p| p.file_stem())
    .and_then(|s| s.to_str())
    .unwrap_or("unknown");
  split_pascal_case(stem)
}

/// Get a display name for a symbol: try _label, then file stem
fn resolve_display_name(db: &TypedownDatabase, project: Project, symbol: &Symbol) -> String {
  let kind = symbol.kind(db);

  // Try _label from the evaluated resource or schema type
  match &kind {
    SymbolKind::UserDefinedResource(_, target_file) => {
      if let Some(target_symbol) = file_symbol(db, project, *target_file).value(db)
        && let Some(obj) = evaluate_resource(db, target_symbol).value(db)
        && let Some(label_obj) = obj.get_builtin_field(db, "_label")
        && let Some(str_obj) = label_obj.as_td_str_obj()
      {
        return str_obj.value(db);
      }
    }
    SymbolKind::UserDefinedSchema(_, _) => {
      if let Some(typ) = evaluate_type(db, *symbol).typ(db)
        && let Some(label_obj) = typ.get_builtin_field(db, "_label")
        && let Some(str_obj) = label_obj.as_td_str_obj()
      {
        return str_obj.value(db);
      }
    }
    _ => {}
  }

  // Fallback: file stem, or parent directory name for index files
  match &kind {
    SymbolKind::UserDefinedResource(_, target_file)
    | SymbolKind::UserDefinedSchema(_, target_file) => {
      let handle = target_file.handle(db);
      let path = handle.path();
      let stem = path.and_then(|p| p.file_stem()).and_then(|s| s.to_str());

      match stem {
        Some("index") => path
          .and_then(|p| p.parent())
          .and_then(|p| p.file_name())
          .and_then(|n| n.to_str())
          .unwrap_or("index")
          .to_string(),
        Some(name) => name.to_string(),
        None => "unknown".to_string(),
      }
    }
    _ => symbol.name(db).to_string(),
  }
}
