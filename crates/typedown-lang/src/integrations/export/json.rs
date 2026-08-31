//! Serialize Typedown objects to plain JSON for RPC/document serving

use std::collections::HashSet;

use typedown_incremental::Id;

use super::{evaluate_lazy_field, file_symbol, resolve_ref};
use crate::db::TypedownDatabase;
use crate::db::types::derived::object_system::TdStaticType;
use crate::db::types::{FileHandle, LazyType, Project, TdObjectEnum, TdTypeEnum};

/// Serialize a FileHandle to a JSON object
pub fn handle_to_json(handle: &FileHandle) -> serde_json::Value {
  let meta = handle.metadata();
  let metadata = serde_json::json!({
    "mtime": meta.mtime_epoch_secs(),
    "ctime": meta.ctime_epoch_secs(),
  });
  match handle {
    FileHandle::Path(path, _) => {
      serde_json::json!({
        "type": "path",
        "path": path.to_string_lossy(),
        "metadata": metadata,
      })
    }
    FileHandle::Content(path, content, _) => {
      serde_json::json!({
        "type": "content",
        "path": path.to_string_lossy(),
        "content": content,
        "metadata": metadata,
      })
    }
  }
}

/// Returned when a cycle is detected during serialization
#[derive(Debug)]
pub struct CircularRef;

/// Serialize a Typedown object to a plain JSON value
pub fn to_json(
  db: &TypedownDatabase,
  project: Project,
  obj: &TdObjectEnum,
) -> Result<serde_json::Value, CircularRef> {
  serialize(db, project, obj, &mut HashSet::new(), false)
}

fn serialize(
  db: &TypedownDatabase,
  project: Project,
  obj: &TdObjectEnum,
  visiting: &mut HashSet<(usize, usize)>,
  should_serialize_as_fref: bool, /* false for the top-level object */
) -> Result<serde_json::Value, CircularRef> {
  match obj {
    TdObjectEnum::TdStrObj(str_obj) => Ok(serde_json::Value::String(str_obj.value(db))),

    TdObjectEnum::TdNumObj(num_obj) => {
      // NaN and Infinity are not valid JSON, fall back to null
      let value = num_obj.value(db);
      match serde_json::Number::from_f64(value) {
        Some(num) => Ok(serde_json::Value::Number(num)),
        None => Ok(serde_json::Value::Null),
      }
    }

    TdObjectEnum::TdBoolObj(bool_obj) => Ok(serde_json::Value::Bool(bool_obj.value(db))),

    TdObjectEnum::TdMathObj(math_obj) => Ok(serde_json::Value::String(math_obj.value(db))),

    TdObjectEnum::TdDateTimeObj(dt) => Ok(serde_json::Value::String(dt.value(db))),
    TdObjectEnum::TdDateObj(dt) => Ok(serde_json::Value::String(dt.value(db))),
    TdObjectEnum::TdTimeObj(dt) => Ok(serde_json::Value::String(dt.value(db))),

    TdObjectEnum::TdListObj(list) => {
      let mut items = Vec::with_capacity(list.len(db));
      for idx in 0..list.len(db) {
        match list.get(db, idx) {
          Some(item) => items.push(serialize(db, project, &item, visiting, true)?),
          None => items.push(serde_json::Value::Null),
        }
      }
      Ok(serde_json::Value::Array(items))
    }

    TdObjectEnum::TdDictObj(dict) => {
      let mut map = serde_json::Map::new();
      for (key, entry) in dict.entries(db) {
        if let Some(item) = evaluate_lazy_field(db, entry) {
          map.insert(key, serialize(db, project, &item, visiting, true)?);
        }
      }
      Ok(serde_json::Value::Object(map))
    }

    TdObjectEnum::TdSchemaObj(schema_obj) => {
      // Resolve references to other files as project relative paths
      if should_serialize_as_fref
        && let Some(symbol) = schema_obj.file_symbol(db)
        && let Some(resolved) = resolve_ref(db, project, &symbol)
      {
        return Ok(serde_json::json!({
          "$ref": { "url": resolved.url, "name": resolved.name }
        }));
      }

      let id = schema_obj.as_id();
      if !visiting.insert(id) {
        return Err(CircularRef);
      }
      let mut map = serde_json::Map::new();
      for (key, entry) in schema_obj.fields(db) {
        if let Some(item) = evaluate_lazy_field(db, entry) {
          map.insert(key, serialize(db, project, &item, visiting, true)?);
        }
      }
      visiting.remove(&id);
      Ok(serde_json::Value::Object(map))
    }

    TdObjectEnum::TdProductObj(product) => {
      // Resolve references to other files as project relative paths
      if should_serialize_as_fref
        && let Some(symbol) = product.file_symbol(db)
        && let Some(resolved) = resolve_ref(db, project, &symbol)
      {
        return Ok(serde_json::json!({
          "$ref": { "url": resolved.url, "name": resolved.name }
        }));
      }

      let id = product.as_id();
      if !visiting.insert(id) {
        return Err(CircularRef);
      }
      let mut map = serde_json::Map::new();
      for (key, entry) in product.fields(db) {
        if let Some(item) = evaluate_lazy_field(db, entry) {
          map.insert(key, serialize(db, project, &item, visiting, true)?);
        }
      }
      visiting.remove(&id);
      Ok(serde_json::Value::Object(map))
    }

    TdObjectEnum::TdBlobObj(blob) => {
      let format = blob.asset_kind(db).as_format_str();
      let file = blob.file(db);
      if should_serialize_as_fref
        && let Some(symbol) = file_symbol(db, project, file).value(db)
        && let Some(resolved) = resolve_ref(db, project, &symbol)
      {
        return Ok(serde_json::json!({
          "$ref": { "url": resolved.url, "name": resolved.name, "format": format }
        }));
      }
      let handle = handle_to_json(&file.handle(db));
      Ok(serde_json::json!({ "format": format, "handle": handle }))
    }

    // Schema types serialize as a map of field name to field type descriptor
    TdObjectEnum::TdTypeObj(TdTypeEnum::TdSchemaType(schema)) => {
      let id = schema.as_id();
      if !visiting.insert(id) {
        return Err(CircularRef);
      }
      let mut map = serde_json::Map::new();
      for (name, prop_desc) in schema.fields(db) {
        map.insert(
          name,
          serialize_lazy_type(db, project, &prop_desc.field_type, visiting)?,
        );
      }
      visiting.remove(&id);
      Ok(serde_json::Value::Object(map))
    }

    // Other type objects and functions are not meaningful as document values
    _ => Ok(serde_json::Value::Null),
  }
}

// Recurses into nested schema types, everything else becomes a string
fn serialize_lazy_type(
  db: &TypedownDatabase,
  project: Project,
  lazy: &LazyType,
  visiting: &mut HashSet<(usize, usize)>,
) -> Result<serde_json::Value, CircularRef> {
  if let Some(TdTypeEnum::TdSchemaType(schema)) = lazy.resolve(db) {
    serialize(
      db,
      project,
      &TdObjectEnum::TdTypeObj(TdTypeEnum::TdSchemaType(schema)),
      visiting,
      true,
    )
  } else {
    Ok(serde_json::Value::String(
      lazy
        .resolve(db)
        .map_or("?".to_string(), |t| t.display_name(db)),
    ))
  }
}

#[cfg(test)]
mod tests {
  use std::path::PathBuf;

  use typedown_types::either::Either;

  use std::collections::HashMap;

  use super::*;
  use crate::db::derived::evaluate::evaluate_resource::evaluate_resource;
  use crate::db::derived::evaluate::evaluate_type::evaluate_type;
  use crate::db::derived::name_resolver::file_symbol::file_symbol;
  use crate::db::fixtures::*;
  use crate::db::types::*;
  use crate::db::{QueryStorage, TypedownDatabase};

  fn empty_db() -> (TypedownDatabase, Project) {
    let db = TypedownDatabase {
      storage: QueryStorage::default(),
    };
    let project = Project::new(&db, PathBuf::new(), HashMap::new());

    (db, project)
  }

  #[test]
  fn serializes_string() {
    let (db, project) = empty_db();
    let obj = TdObjectEnum::from(make_str_obj(&db, "hello".to_string()));
    let value = to_json(&db, project, &obj).unwrap();
    assert_eq!(value, serde_json::Value::String("hello".to_string()));
  }

  #[test]
  fn serializes_number() {
    let (db, project) = empty_db();
    let obj = TdObjectEnum::from(make_num_obj(&db, 42.0_f64.to_bits()));
    let value = to_json(&db, project, &obj).unwrap();
    assert_eq!(value, serde_json::json!(42.0));
  }

  #[test]
  fn non_finite_float_serializes_to_null() {
    let (db, project) = empty_db();
    let obj = TdObjectEnum::from(make_num_obj(&db, f64::NAN.to_bits()));
    let value = to_json(&db, project, &obj).unwrap();
    assert_eq!(value, serde_json::Value::Null);
  }

  #[test]
  fn infinity_serializes_to_null() {
    let (db, project) = empty_db();
    let obj = TdObjectEnum::from(make_num_obj(&db, f64::INFINITY.to_bits()));
    let value = to_json(&db, project, &obj).unwrap();
    assert_eq!(value, serde_json::Value::Null);
  }

  #[test]
  fn serializes_bool() {
    let (db, project) = empty_db();
    let obj = TdObjectEnum::from(make_bool_obj(&db, true));
    let value = to_json(&db, project, &obj).unwrap();
    assert_eq!(value, serde_json::Value::Bool(true));
  }

  #[test]
  fn serializes_list() {
    let (db, project) = empty_db();
    let items = vec![
      Either::Right(TdObjectEnum::from(make_num_obj(&db, 1.0_f64.to_bits()))),
      Either::Right(TdObjectEnum::from(make_str_obj(&db, "two".to_string()))),
      Either::Right(TdObjectEnum::from(make_bool_obj(&db, false))),
    ];
    let obj = TdObjectEnum::from(make_list_obj(&db, items));
    let value = to_json(&db, project, &obj).unwrap();
    assert_eq!(value, serde_json::json!([1.0, "two", false]));
  }

  #[test]
  fn serializes_dict() {
    let (db, project) = empty_db();
    let entries = vec![
      (
        "x".to_string(),
        Either::Right(TdObjectEnum::from(make_num_obj(&db, 10.0_f64.to_bits()))),
      ),
      (
        "y".to_string(),
        Either::Right(TdObjectEnum::from(make_str_obj(&db, "hello".to_string()))),
      ),
    ];
    let obj = TdObjectEnum::from(make_dict_obj(&db, entries));
    let value = to_json(&db, project, &obj).unwrap();
    assert_eq!(value["x"], serde_json::json!(10.0));
    assert_eq!(value["y"], serde_json::json!("hello"));
  }

  #[test]
  fn serializes_math_as_string() {
    let (db, project) = empty_db();
    let obj = TdObjectEnum::from(make_math_obj(&db, "$E = mc^2$".to_string()));
    let value = to_json(&db, project, &obj).unwrap();
    assert_eq!(value, serde_json::Value::String("$E = mc^2$".to_string()));
  }

  #[test]
  fn serializes_datetime_as_string() {
    let (db, project) = empty_db();
    let obj = TdObjectEnum::from(make_datetime_obj(&db, "2024-01-15T10:30:00Z".to_string()));
    let value = to_json(&db, project, &obj).unwrap();
    assert_eq!(
      value,
      serde_json::Value::String("2024-01-15T10:30:00Z".to_string())
    );
  }

  #[test]
  fn serializes_date_as_string() {
    let (db, project) = empty_db();
    let obj = TdObjectEnum::from(make_date_obj(&db, "2024-01-15".to_string()));
    let value = to_json(&db, project, &obj).unwrap();
    assert_eq!(value, serde_json::Value::String("2024-01-15".to_string()));
  }

  #[test]
  fn serializes_time_as_string() {
    let (db, project) = empty_db();
    let obj = TdObjectEnum::from(make_time_obj(&db, "10:30:00".to_string()));
    let value = to_json(&db, project, &obj).unwrap();
    assert_eq!(value, serde_json::Value::String("10:30:00".to_string()));
  }

  #[test]
  fn serializes_product_fields() {
    let (db, project, file) = load_vault_fixture("evaluate/my_vault", "valid_person.td");
    let result = evaluate_resource(&db, file_symbol(&db, project, file).value(&db).unwrap());
    let obj = result.value(&db).expect("should evaluate resource");
    let value = to_json(&db, project, &obj).expect("should serialize without cycle");
    assert!(value.is_object(), "product should serialize to object");
    assert_eq!(
      value["name"],
      serde_json::Value::String("Alice".to_string())
    );
    assert_eq!(value["age"], serde_json::json!(30.0));
  }

  #[test]
  fn fref_field_serializes_as_resolved_ref() {
    let (db, project, file) = load_vault_fixture("evaluate/my_vault", "with_fref.td");
    let result = evaluate_resource(&db, file_symbol(&db, project, file).value(&db).unwrap());
    let obj = result.value(&db).expect("should evaluate resource");
    let value = to_json(&db, project, &obj).expect("should serialize");
    assert!(
      value["friend"]["$ref"].is_object(),
      "friend should have $ref: {value}"
    );
    assert!(
      value["friend"]["$ref"]["url"].is_string(),
      "ref should have url: {value}",
    );
    assert!(
      value["friend"]["$ref"]["name"].is_string(),
      "ref should have name: {value}",
    );
  }

  #[test]
  fn transitive_fref_serializes_as_resolved_ref() {
    let (db, project, file) = load_vault_fixture("evaluate/my_vault", "transitive_fref.td");
    let result = evaluate_resource(&db, file_symbol(&db, project, file).value(&db).unwrap());
    let obj = result.value(&db).expect("should evaluate resource");
    let value = to_json(&db, project, &obj).expect("should serialize");
    assert!(
      value["friend"]["$ref"]["url"].is_string(),
      "transitive ref should have url: {value}",
    );
    assert!(
      value["friend"]["$ref"]["name"].is_string(),
      "transitive ref should have name: {value}",
    );
  }

  #[test]
  fn nested_product_serializes_without_cycle() {
    let (db, project) = empty_db();
    let _product_type: TdTypeEnum = make_product_type(&db, None, vec![]).into();
    let str_type: TdTypeEnum = TdStrType::get(&db).into();
    let num_type: TdTypeEnum = TdNumType::get(&db).into();
    let inner = make_product_obj(&db, str_type, None, vec![]);
    let fields = vec![(
      "inner".to_string(),
      Either::Right(TdObjectEnum::from(inner)),
    )];
    let outer = make_product_obj(&db, num_type, None, fields);

    let result = to_json(&db, project, &TdObjectEnum::from(outer));
    assert!(result.is_ok(), "non-cyclic nested product should serialize");
  }

  #[test]
  fn serializes_product_type_as_field_type_map() {
    let (db, project, file) = load_vault_fixture("evaluate/my_vault", "_types/Person.td");
    let symbol = file_symbol(&db, project, file).value(&db).unwrap();
    let typ = evaluate_type(&db, symbol)
      .typ(&db)
      .expect("should have type");
    let obj = TdObjectEnum::from(typ);
    let value = to_json(&db, project, &obj).expect("should serialize");
    assert!(value.is_object(), "product type should serialize to object");
    assert_eq!(
      value["name"],
      serde_json::Value::String("string".to_string())
    );
    assert_eq!(
      value["age"],
      serde_json::Value::String("number".to_string())
    );
  }

  #[test]
  fn serializes_nested_product_type_recursively() {
    let (db, project, file) = load_vault_fixture("evaluate/my_vault", "_types/Event.td");
    let symbol = file_symbol(&db, project, file).value(&db).unwrap();
    let typ = evaluate_type(&db, symbol)
      .typ(&db)
      .expect("should have type");
    let obj = TdObjectEnum::from(typ);
    let value = to_json(&db, project, &obj).expect("should serialize");
    assert_eq!(
      value["title"],
      serde_json::Value::String("string".to_string())
    );
    // Nested product type expands inline rather than flattening to "Address"
    assert!(
      value["location"].is_object(),
      "nested schema should expand to object"
    );
    assert_eq!(
      value["location"]["street"],
      serde_json::Value::String("string".to_string())
    );
    assert_eq!(
      value["location"]["city"],
      serde_json::Value::String("string".to_string())
    );
  }

  #[test]
  fn non_product_type_serializes_to_null() {
    let (db, project) = empty_db();
    let obj = TdObjectEnum::from(TdStrType::get(&db));
    let value = to_json(&db, project, &obj).unwrap();
    assert_eq!(value, serde_json::Value::Null);
  }

  #[test]
  fn blob_includes_format_and_path() {
    let (db, project) = empty_db();
    let path = PathBuf::from("/vault/_assets/photo.png");
    let file = File::new(&db, FileHandle::Path(path.clone(), FileMetadata::default()));
    let blob = make_blob_obj(&db, AssetKind::Png, file);
    let obj = TdObjectEnum::from(blob);
    let value = to_json(&db, project, &obj).unwrap();
    assert_eq!(value["format"], "png");
    assert_eq!(value["handle"]["type"], "path");
    assert_eq!(value["handle"]["path"], "/vault/_assets/photo.png");
  }
}
