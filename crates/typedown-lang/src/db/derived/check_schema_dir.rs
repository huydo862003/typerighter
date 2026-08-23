//! Check that schema files have unique names across the schema directory

use std::collections::HashMap;
use std::path::PathBuf;

use typedown_macros::query_derived;
use typedown_types::path::normalize_path;

use crate::db::TypedownDatabase;
use crate::db::derived::get_vault_config::get_vault_config;
use crate::db::types::Project;
use crate::db::utils::is_content_file;
use crate::syntax::diagnostic::Diagnostic;
use typedown_incremental::QueryDatabase;

#[query_derived]
pub struct SchemaCheckResult {
  diagnostics: Vec<Diagnostic>,
}

/// Check all files in schema_dir for duplicate schema names
#[query_derived]
pub fn check_schema_dir(db: &TypedownDatabase, project: Project) -> SchemaCheckResult {
  let config = get_vault_config(db, project);
  let schema_dir = config.schema_dir(db);
  let proj_files = project.files(db);
  let mut diagnostics = vec![];
  let mut seen_schemas: HashMap<String, PathBuf> = HashMap::new();

  let mut schema_paths: Vec<_> = proj_files
    .keys()
    .filter(|path| path.starts_with(&schema_dir) && is_content_file(path))
    .collect();
  schema_paths.sort();

  for path in schema_paths {
    let name = path
      .file_stem()
      .and_then(|s| s.to_str())
      .unwrap_or_default()
      .to_string();

    let relative = path.strip_prefix(&schema_dir).unwrap_or(path);
    let norm_path = normalize_path(relative);

    if let Some(existing_path) = seen_schemas.get(&name) {
      let existing_relative = existing_path
        .strip_prefix(&schema_dir)
        .unwrap_or(existing_path);
      diagnostics.push(Diagnostic::DuplicateSchemaName {
        name: name.clone(),
        path: norm_path,
        duplicate_of: normalize_path(existing_relative),
      });
    } else {
      seen_schemas.insert(name, path.clone());
    }
  }

  SchemaCheckResult::new(db, diagnostics)
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::db::derived::evaluate::evaluate_resource::evaluate_resource;
  use crate::db::derived::name_resolver::file_symbol::file_symbol;
  use crate::db::fixtures::load_vault_fixture;
  use crate::db::types::TdStaticType;

  #[test]
  fn check_schema_dir_allows_nested_schemas_with_unique_names() {
    let (db, project, _) = load_vault_fixture("evaluate/my_vault", "schemas/Person.td");
    let result = check_schema_dir(&db, project);
    assert!(result.diagnostics(&db).is_empty());
  }

  #[test]
  fn evaluate_resource_with_nested_schema_resolves_correctly() {
    let (db, project, file) =
      load_vault_fixture("evaluate/my_vault", "content/valid_nested_schema.td");
    let symbol = file_symbol(&db, project, file).value(&db).unwrap();
    let res = evaluate_resource(&db, symbol);
    assert!(res.diagnostics(&db).is_empty());

    let val = res.value(&db).expect("should evaluate to a value");
    let prod = val.as_td_product_obj().expect("should be a product object");
    assert_eq!(prod.schema(&db).display_name(&db), "SpecialPerson");
  }

  #[test]
  fn check_schema_dir_detects_duplicate_schema_names() {
    let (db, project, _) = load_vault_fixture("evaluate/duplicate_schema_vault", "schemas/Item.td");
    let result = check_schema_dir(&db, project);
    let diags = result.diagnostics(&db);
    assert_eq!(diags.len(), 1);
    assert!(matches!(
      &diags[0],
      Diagnostic::DuplicateSchemaName { name, path, duplicate_of }
        if name == "Item" && path == "nested/Item.td" && duplicate_of == "Item.td"
    ));
  }
}
