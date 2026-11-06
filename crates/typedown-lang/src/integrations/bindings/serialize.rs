//! Serialize Typedown objects to JSON (_type-tagged, round-trippable)

use std::collections::HashSet;

use typedown_incremental::Id;
use typedown_types::either::Either;

use crate::db::TypedownDatabase;
use crate::db::derived::evaluate::evaluate_node::evaluate_node;
use crate::db::derived::name_resolver::file_symbol::file_symbol;
use crate::db::derived::name_resolver::scope::get_file_runtime_scope;
use crate::db::types::derived::obj_system::{TdRuntimeObj, TdStaticTyp};
use crate::db::types::{FileHandle, LazyTyp, Project, ProjectRedNode, TdObjEnum, TdTypEnum};

use super::file_ref::resolve_file_ref;

/// Evaluate a lazy field (either a deferred AST node or an already-evaluated object)
pub fn evaluate_lazy_field<'db>(
  db: &'db TypedownDatabase,
  field: Either<ProjectRedNode, TdObjEnum<'db>>,
) -> Option<TdObjEnum<'db>> {
  match field {
    Either::Right(obj) => Some(obj),
    Either::Left(project_node) => {
      let file_scope = get_file_runtime_scope(db, project_node.project, project_node.file);
      evaluate_node(db, project_node, file_scope).value(db)
    }
  }
}

/// Serialize a FileHandle to a JSON object
pub fn serialize_handle(handle: &FileHandle) -> serde_json::Value {
  let meta = handle.metadata();
  let metadata = serde_json::json!({
    "mtime": meta.mtime_epoch_secs(),
    "ctime": meta.ctime_epoch_secs(),
  });
  match handle {
    FileHandle::Path(path, _) => serde_json::json!({
      "type": "path",
      "path": path.to_string_lossy(),
      "metadata": metadata,
    }),
    FileHandle::Content(path, content, _) => serde_json::json!({
      "type": "content",
      "path": path.to_string_lossy(),
      "content": content,
      "metadata": metadata,
    }),
  }
}

pub fn serialize_value(
  db: &TypedownDatabase,
  project: Project,
  obj: &TdObjEnum,
) -> serde_json::Value {
  serialize(db, project, obj, &mut HashSet::new(), false)
}

fn serialize(
  db: &TypedownDatabase,
  project: Project,
  obj: &TdObjEnum,
  visiting: &mut HashSet<typedown_incremental::DepId>,
  nested: bool,
) -> serde_json::Value {
  match obj {
    TdObjEnum::TdNullObj(_) => serde_json::Value::Null,
    TdObjEnum::TdBoolObj(bool_obj) => serde_json::Value::Bool(bool_obj.value(db)),

    TdObjEnum::TdNumberObj(number_obj) => {
      let value = number_obj.value(db);
      if value.is_nan() {
        serde_json::json!({"_type": "nan"})
      } else if value == f64::INFINITY {
        serde_json::json!({"_type": "inf"})
      } else if value == f64::NEG_INFINITY {
        serde_json::json!({"_type": "-inf"})
      } else {
        serde_json::Value::Number(serde_json::Number::from_f64(value).unwrap())
      }
    }

    TdObjEnum::TdStringObj(string_obj) => serde_json::Value::String(string_obj.value(db)),

    TdObjEnum::TdMathObj(math_obj) => {
      serde_json::json!({"_type": "math", "value": math_obj.value(db)})
    }
    TdObjEnum::TdDateObj(date) => {
      serde_json::json!({"_type": "date", "value": date.value(db)})
    }
    TdObjEnum::TdDateTimeObj(datetime) => {
      serde_json::json!({"_type": "datetime", "value": datetime.value(db)})
    }
    TdObjEnum::TdTimeObj(time) => {
      serde_json::json!({"_type": "time", "value": time.value(db)})
    }

    TdObjEnum::TdIconObj(icon_obj) => serde_json::json!({
      "_type": "icon",
      "name": icon_obj.name(db),
      "lucide_name": icon_obj.lucide_name(db),
    }),

    TdObjEnum::TdListObj(list) => {
      let count = list.items(db).len();
      let mut items = Vec::with_capacity(count);
      for index in 0..count {
        match list.get(db, index) {
          Some(item) => items.push(serialize(db, project, &item, visiting, true)),
          None => items.push(serde_json::Value::Null),
        }
      }
      serde_json::Value::Array(items)
    }

    TdObjEnum::TdDictObj(dict) => {
      let mut map = serde_json::Map::new();
      for (key, entry) in dict.entries(db) {
        if let Some(item) = evaluate_lazy_field(db, entry) {
          map.insert(key, serialize(db, project, &item, visiting, true));
        }
      }
      serde_json::Value::Object(map)
    }

    TdObjEnum::TdSchemaObj(schema_obj) => {
      let schema_name = schema_obj.schema(db).display_name(db);

      // $ref for nested file-scoped objects when project is available
      if nested
        && true // project always present
        && let Some(symbol) = schema_obj.file_symbol(db)
        && let Some(resolved) = resolve_file_ref(db, project, &symbol)
      {
        let icon = schema_obj
          .get_builtin_field(db, "_icon")
          .and_then(|obj| obj.as_td_icon_obj().map(|icon| icon.lucide_name(db)));
        let mut ref_obj = serde_json::json!({
          "$kind": "ref",
          "schema": schema_name,
          "$ref": { "url": resolved.url, "name": resolved.name }
        });
        if let Some(icon_name) = icon {
          ref_obj["$ref"]["icon"] = serde_json::json!({ "name": icon_name });
        }
        return ref_obj;
      }

      let id = schema_obj.as_id();
      if !visiting.insert(id) {
        return serde_json::json!({"$kind": "ref", "schema": schema_name});
      }
      let fields = serialize_object_fields(
        db,
        project,
        schema_obj.builtins(db),
        schema_obj.fields(db),
        visiting,
      );
      visiting.remove(&id);
      serde_json::json!({
        "_type": "schema",
        "schema": schema_name,
        "fields": serde_json::Value::Object(fields),
      })
    }

    TdObjEnum::TdProductObj(product) => {
      let typ_name = product.product_typ(db).display_name(db);

      if nested
        && true // project always present
        && let Some(symbol) = product.file_symbol(db)
        && let Some(resolved) = resolve_file_ref(db, project, &symbol)
      {
        let icon = product
          .get_builtin_field(db, "_icon")
          .and_then(|obj| obj.as_td_icon_obj().map(|icon| icon.lucide_name(db)));
        let mut ref_obj = serde_json::json!({
          "$kind": "ref",
          "type": typ_name,
          "$ref": { "url": resolved.url, "name": resolved.name }
        });
        if let Some(icon_name) = icon {
          ref_obj["$ref"]["icon"] = serde_json::json!({ "name": icon_name });
        }
        return ref_obj;
      }

      let id = product.as_id();
      if !visiting.insert(id) {
        return serde_json::json!({"$kind": "ref", "type": typ_name});
      }
      let fields = serialize_object_fields(
        db,
        project,
        product.builtins(db),
        product.fields(db),
        visiting,
      );
      visiting.remove(&id);
      serde_json::json!({
        "_type": "product",
        "fields": serde_json::Value::Object(fields),
      })
    }

    TdObjEnum::TdBlobObj(blob) => {
      let format = blob.asset_kind(db).as_format_str();
      let file = blob.file(db);

      if nested
        && true // project always present
        && let Some(symbol) = file_symbol(db, project, file).value(db)
        && let Some(resolved) = resolve_file_ref(db, project, &symbol)
      {
        return serde_json::json!({
          "$kind": "ref",
          "$ref": { "url": resolved.url, "name": resolved.name, "format": format }
        });
      }

      let handle = serialize_handle(&file.handle(db));
      serde_json::json!({"_type": "blob", "format": format, "handle": handle})
    }

    TdObjEnum::TdTypObj(typ) => serialize_typ_helper(db, typ, visiting),

    _ => serde_json::Value::Null,
  }
}

/// Serialize builtins and fields from a schema or product object into a JSON map
fn serialize_object_fields<'db>(
  db: &'db TypedownDatabase,
  project: Project,
  builtins: impl IntoIterator<Item = (String, Either<ProjectRedNode, TdObjEnum<'db>>)>,
  fields: impl IntoIterator<Item = (String, Either<ProjectRedNode, TdObjEnum<'db>>)>,
  visiting: &mut HashSet<typedown_incremental::DepId>,
) -> serde_json::Map<String, serde_json::Value> {
  let mut map = serde_json::Map::new();
  for (key, entry) in builtins.into_iter().chain(fields) {
    if let Some(item) = evaluate_lazy_field(db, entry) {
      map.insert(key, serialize(db, project, &item, visiting, true));
    }
  }
  map
}

/// Serialize a type to its canonical JSON representation.
/// Uses `$kind: "type"` as the envelope with JSON Schema-compatible fields inside.
pub fn serialize_typ(db: &TypedownDatabase, typ: &TdTypEnum) -> serde_json::Value {
  serialize_typ_helper(db, typ, &mut HashSet::new())
}

fn serialize_typ_helper(
  db: &TypedownDatabase,
  typ: &TdTypEnum,
  visiting: &mut HashSet<typedown_incremental::DepId>,
) -> serde_json::Value {
  match typ {
    TdTypEnum::TdNullTyp(_) => serde_json::json!({"$kind": "type", "type": "null"}),
    TdTypEnum::TdBoolTyp(_) => serde_json::json!({"$kind": "type", "type": "boolean"}),
    TdTypEnum::TdNumberTyp(_) => serde_json::json!({"$kind": "type", "type": "number"}),
    TdTypEnum::TdStringTyp(_) => serde_json::json!({"$kind": "type", "type": "string"}),
    TdTypEnum::TdMathTyp(_) => serde_json::json!({"$kind": "type", "type": "math"}),
    TdTypEnum::TdDateTyp(_) => serde_json::json!({"$kind": "type", "type": "date"}),
    TdTypEnum::TdDateTimeTyp(_) => serde_json::json!({"$kind": "type", "type": "datetime"}),
    TdTypEnum::TdTimeTyp(_) => serde_json::json!({"$kind": "type", "type": "time"}),
    TdTypEnum::TdListTyp(list) => {
      if let Some(element) = list.element(db) {
        let items = serialize_lazy_typ(db, &element, visiting);
        serde_json::json!({"$kind": "type", "type": "array", "items": items})
      } else {
        serde_json::json!({"$kind": "type", "type": "array"})
      }
    }
    TdTypEnum::TdDictTyp(dict) => {
      if let Some(val) = dict.value(db) {
        let additional = serialize_lazy_typ(db, &val, visiting);
        serde_json::json!({"$kind": "type", "type": "object", "additionalProperties": additional})
      } else {
        serde_json::json!({"$kind": "type", "type": "object", "additionalProperties": true})
      }
    }
    TdTypEnum::TdSumTyp(sum) => {
      let mut members_lazy: Vec<_> = sum.members(db).into_iter().collect();
      members_lazy.sort_by(|left_member, right_member| {
        left_member
          .resolve(db)
          .map(|typ| typ.display_name(db))
          .cmp(&right_member.resolve(db).map(|typ| typ.display_name(db)))
      });
      let any_of: Vec<_> = members_lazy
        .iter()
        .map(|member| serialize_lazy_typ(db, member, visiting))
        .collect();
      serde_json::json!({"$kind": "type", "anyOf": any_of})
    }
    TdTypEnum::TdSchemaTyp(schema) => {
      // Serialize as a schema reference by name, properties are looked up from the live schema on deserialization rather than embedded inline
      serde_json::json!({"$kind": "type", "type": "schema", "name": schema.name(db)})
    }
    TdTypEnum::TdProductTyp(product) => {
      let mut properties = serde_json::Map::new();
      for (name, lazy) in product.fields(db) {
        properties.insert(name, serialize_lazy_typ(db, &lazy, visiting));
      }
      let mut obj = serde_json::Map::new();
      obj.insert("$kind".into(), serde_json::json!("type"));
      obj.insert("type".into(), serde_json::json!("object"));
      if let Some(name) = product.name(db) {
        obj.insert("name".into(), serde_json::Value::String(name));
      }
      obj.insert("properties".into(), serde_json::Value::Object(properties));
      serde_json::Value::Object(obj)
    }
    _ => serde_json::json!({"$kind": "type", "type": typ.display_name(db)}),
  }
}

fn serialize_lazy_typ(
  db: &TypedownDatabase,
  lazy: &LazyTyp,
  visiting: &mut HashSet<typedown_incremental::DepId>,
) -> serde_json::Value {
  match lazy.resolve(db) {
    Some(typ) => serialize_typ_helper(db, &typ, visiting),
    None => serde_json::json!({"$kind": "type", "type": "unknown"}),
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::db::derived::evaluate::evaluate_resource::evaluate_resource;
  use crate::db::fixtures::*;
  use crate::db::types::*;
  use std::path::PathBuf;
  use typedown_types::either::Either;

  #[test]
  fn nan_serializes_as_kind() {
    let (db, project, _) = load_vault_fixture("evaluate/my_vault", "valid_person.td");
    let obj = TdObjEnum::from(make_number_obj(&db, f64::NAN.to_bits()));
    let value = serialize_value(&db, project, &obj);
    assert_eq!(value["_type"], "nan");
  }

  #[test]
  fn infinity_serializes_as_kind() {
    let (db, project, _) = load_vault_fixture("evaluate/my_vault", "valid_person.td");
    let obj = TdObjEnum::from(make_number_obj(&db, f64::INFINITY.to_bits()));
    let value = serialize_value(&db, project, &obj);
    assert_eq!(value["_type"], "inf");
  }

  #[test]
  fn neg_infinity_serializes_as_kind() {
    let (db, project, _) = load_vault_fixture("evaluate/my_vault", "valid_person.td");
    let obj = TdObjEnum::from(make_number_obj(&db, f64::NEG_INFINITY.to_bits()));
    let value = serialize_value(&db, project, &obj);
    assert_eq!(value["_type"], "-inf");
  }

  #[test]
  fn serializes_math_as_kind() {
    let (db, project, _) = load_vault_fixture("evaluate/my_vault", "valid_person.td");
    let obj = TdObjEnum::from(make_math_obj(&db, "$E = mc^2$".to_string()));
    let value = serialize_value(&db, project, &obj);
    assert_eq!(value["_type"], "math");
    assert_eq!(value["value"], "$E = mc^2$");
  }

  #[test]
  fn serializes_product_with_kind() {
    let (db, project, _) = load_vault_fixture("evaluate/my_vault", "valid_person.td");
    let string_typ: TdTypEnum = TdStringTyp::get(&db).into();
    let number_typ: TdTypEnum = TdNumberTyp::get(&db).into();
    let inner = make_product_obj(&db, string_typ, None, vec![]);
    let fields = vec![("inner".to_string(), Either::Right(TdObjEnum::from(inner)))];
    let outer = make_product_obj(&db, number_typ, None, fields);
    let value = serialize_value(&db, project, &TdObjEnum::from(outer));
    assert_eq!(value["_type"], "product");
    assert!(value["fields"].is_object());
  }

  #[test]
  fn blob_serializes_with_kind() {
    let (db, project, _) = load_vault_fixture("evaluate/my_vault", "valid_person.td");
    let path = PathBuf::from("/vault/_assets/photo.png");
    let file = File::new(&db, FileHandle::Path(path.clone(), FileMetadata::default()));
    let blob = make_blob_obj(&db, AssetKind::Png, file);
    let obj = TdObjEnum::from(blob);
    let value = serialize_value(&db, project, &obj);
    assert_eq!(value["_type"], "blob");
    assert_eq!(value["format"], "png");
    assert_eq!(value["handle"]["path"], "/vault/_assets/photo.png");
  }

  #[test]
  fn fref_field_serializes_as_ref_with_project() {
    let (db, project, file) = load_vault_fixture("evaluate/my_vault", "with_fref.td");
    let result = evaluate_resource(&db, file_symbol(&db, project, file).value(&db).unwrap());
    let obj = result.value(&db).expect("should evaluate resource");
    let value = serialize_value(&db, project, &obj);
    assert_eq!(
      value["fields"]["friend"]["$kind"], "ref",
      "friend should be $ref: {value}"
    );
    assert!(
      value["fields"]["friend"]["$ref"]["url"].is_string(),
      "ref should have url: {value}",
    );
  }
}
