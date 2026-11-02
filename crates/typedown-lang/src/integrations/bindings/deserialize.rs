//! Deserialize JSON values back into Typedown objects (round-trip support)

use std::collections::BTreeMap;
use typedown_types::either::Either;

use crate::db::TypedownDatabase;
use crate::db::derived::evaluate::evaluate_resource::evaluate_resource;
use crate::db::derived::evaluate::evaluate_typ::evaluate_typ;
use crate::db::derived::get_vault_config::get_vault_config;
use crate::db::derived::name_resolver::file_symbol::file_symbol;
use crate::db::types::derived::obj_system::{
  TdBoolObj, TdDateObj, TdDateTimeObj, TdDictObj, TdIconObj, TdListObj, TdMathObj, TdNullObj,
  TdNumberObj, TdStringObj, TdTimeObj,
};
use crate::db::types::{File, Project, TdObjEnum, TdTypEnum};
use crate::db::utils::{is_typ_file, prepend_base_path, strip_content_extension};
use crate::integrations::json_schema::parse_json_schema;

/// Deserialize a JSON value into a Typedown object
/// Resolves `$kind: "ref"` links to their evaluated objects via project file lookup
pub fn deserialize_value<'db>(
  db: &'db TypedownDatabase,
  project: Project,
  value: serde_json::Value,
) -> Option<TdObjEnum<'db>> {
  match value {
    serde_json::Value::Null => Some(TdNullObj::get(db).into()),
    serde_json::Value::Bool(bool) => Some(TdBoolObj::new(db, bool).into()),
    serde_json::Value::Number(number) => Some(TdNumberObj::new(db, number.as_f64()?).into()),
    serde_json::Value::String(string) => Some(TdStringObj::new(db, string).into()),
    serde_json::Value::Array(array) => {
      let items: Vec<_> = array
        .into_iter()
        .map(|item| deserialize_value(db, project, item).map(Either::Right))
        .collect::<Option<_>>()?;
      Some(TdListObj::new(db, items).into())
    }
    serde_json::Value::Object(map) => {
      // Dispatch on $kind
      match map.get("$kind").and_then(|value| value.as_str()) {
        Some("ref") => {
          let url = map
            .get("$ref")
            .and_then(|reference| reference.get("url"))
            .and_then(|url| url.as_str())?;
          let file = file_from_url(db, project, url)?;
          let symbol = file_symbol(db, project, file).value(db)?;
          return evaluate_resource(db, symbol).value(db);
        }
        Some("type") => {
          return deserialize_typ(db, project, &map).map(TdObjEnum::TdTypObj);
        }
        _ => {}
      }

      // _type tag -> dispatch to typed handler
      if let Some(kind) = map
        .get("_type")
        .and_then(|value| value.as_str())
        .map(str::to_string)
      {
        return deserialize_tagged(db, project, &kind, map);
      }

      // Plain object -> TdDictObj
      let entries: BTreeMap<String, _> = map
        .into_iter()
        .map(|(key, value)| {
          deserialize_value(db, project, value).map(|obj| (key, Either::Right(obj)))
        })
        .collect::<Option<Vec<_>>>()?
        .into_iter()
        .collect();
      Some(TdDictObj::new(db, entries).into())
    }
  }
}

/// Resolve a URL back to a project file by matching against each file's computed URL.
fn file_from_url(db: &TypedownDatabase, project: Project, url: &str) -> Option<File> {
  let config = get_vault_config(db, project);
  let base_path = config.base_path(db);
  let root_dir = config.root_dir(db);

  for (path, file) in &*project.files(db) {
    let handle = file.handle(db);
    let file_path = handle.path()?;
    let relative = file_path.strip_prefix(&root_dir).unwrap_or(file_path);
    let relative_str = relative.to_string_lossy();
    let without_ext = strip_content_extension(&relative_str);
    let file_url = prepend_base_path(&base_path, without_ext);
    if file_url == url {
      return Some(*file);
    }
    // Also match asset URLs (with extension)
    let asset_url = prepend_base_path(&base_path, &relative_str);
    if asset_url == url {
      return Some(*file);
    }
    let _ = path;
  }
  None
}

/// Reconstruct a `TdTypEnum` from a `$kind: "type"` JSON object
fn deserialize_typ<'db>(
  db: &'db TypedownDatabase,
  project: Project,
  map: &serde_json::Map<String, serde_json::Value>,
) -> Option<TdTypEnum<'db>> {
  let typ_str = map.get("type").and_then(|v| v.as_str());

  if let Some("schema") = typ_str {
    let name = map.get("name").and_then(|v| v.as_str())?;
    return schema_typ_by_name(db, project, name);
  }

  // Delegate to parse_json_schema which now handles math/date/datetime/time/object/array/etc
  parse_json_schema(db, &serde_json::Value::Object(map.clone()))
}

/// Find a schema type by name by evaluating all schema files in the project
fn schema_typ_by_name<'db>(
  db: &'db TypedownDatabase,
  project: Project,
  name: &str,
) -> Option<TdTypEnum<'db>> {
  let config = get_vault_config(db, project);
  let root_dir = config.root_dir(db);

  for (path, file) in &*project.files(db) {
    if !path.starts_with(&root_dir) || !is_typ_file(path) {
      continue;
    }
    let stem = path.file_stem().and_then(|s| s.to_str()).unwrap_or("");
    if stem != name {
      continue;
    }
    let symbol = file_symbol(db, project, *file).value(db)?;
    return evaluate_typ(db, symbol).typ(db);
  }
  None
}

/// Deserialize a _type-tagged JSON object
fn deserialize_tagged<'db>(
  db: &'db TypedownDatabase,
  project: Project,
  kind: &str,
  map: serde_json::Map<String, serde_json::Value>,
) -> Option<TdObjEnum<'db>> {
  match kind {
    "nan" => Some(TdNumberObj::new(db, f64::NAN).into()),
    "inf" => Some(TdNumberObj::new(db, f64::INFINITY).into()),
    "-inf" => Some(TdNumberObj::new(db, f64::NEG_INFINITY).into()),

    "math" => Some(TdMathObj::new(db, map.get("value")?.as_str()?.to_string()).into()),
    "date" => Some(TdDateObj::new(db, map.get("value")?.as_str()?.to_string()).into()),
    "datetime" => Some(TdDateTimeObj::new(db, map.get("value")?.as_str()?.to_string()).into()),
    "time" => Some(TdTimeObj::new(db, map.get("value")?.as_str()?.to_string()).into()),

    "icon" => Some(
      TdIconObj::new(
        db,
        map.get("name")?.as_str()?.to_string(),
        map.get("lucide_name")?.as_str()?.to_string(),
      )
      .into(),
    ),

    // Schema and product: deserialize fields as a dict
    "schema" | "product" => {
      let fields_map = map.get("fields")?.as_object()?;
      let entries: BTreeMap<String, _> = fields_map
        .iter()
        .map(|(key, value)| {
          deserialize_value(db, project, value.clone()).map(|obj| (key.clone(), Either::Right(obj)))
        })
        .collect::<Option<Vec<_>>>()?
        .into_iter()
        .collect();
      Some(TdDictObj::new(db, entries).into())
    }

    // Blob: not round-trippable, return as dict
    "blob" => {
      let mut entries = BTreeMap::new();
      for key in ["format", "handle"] {
        if let Some(value) = map.get(key)
          && let Some(obj) = deserialize_value(db, project, value.clone())
        {
          entries.insert(key.to_string(), Either::Right(obj));
        }
      }
      Some(TdDictObj::new(db, entries).into())
    }

    _ => {
      let entries: BTreeMap<String, _> = map
        .into_iter()
        .map(|(key, value)| {
          deserialize_value(db, project, value).map(|obj| (key, Either::Right(obj)))
        })
        .collect::<Option<Vec<_>>>()?
        .into_iter()
        .collect();
      Some(TdDictObj::new(db, entries).into())
    }
  }
}

/// Deserialize a JSON string value that carries math notation.
pub fn deserialize_math<'db>(db: &'db TypedownDatabase, latex: String) -> TdObjEnum<'db> {
  TdMathObj::new(db, latex).into()
}
