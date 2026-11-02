//! Serialize Typedown objects to typed JSON ($kind-tagged, round-trippable)
//!
//! Entry point: `serialize_value(db, project, obj) -> TdValue`
//!
//! Encoding rules:
//!   POD (plain JSON, no tag):  null, bool, finite number, string, array
//!   $kind-tagged:              nan/inf/-inf, math/date/datetime/time, icon, blob,
//!                              dict, schema, product, type:*
//!   $ref:                      nested file-scoped schema/product/blob objects

use std::collections::{BTreeMap, HashSet};

use typedown_incremental::Id;
use typedown_types::either::Either;

use crate::db::TypedownDatabase;
use crate::db::derived::evaluate::evaluate_node::evaluate_node;
use crate::db::derived::name_resolver::file_symbol::file_symbol;
use crate::db::derived::name_resolver::scope::get_file_runtime_scope;
use crate::db::types::derived::obj_system::{TdRuntimeObj, TdStaticTyp};
use crate::db::types::{FileHandle, LazyTyp, Project, ProjectRedNode, TdObjEnum, TdTypEnum};

use super::file_ref::resolve_file_ref;
use super::types::*;

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

/// Serialize a FileHandle to a typed handle
pub fn serialize_handle(handle: &FileHandle) -> TdHandle {
  let meta = handle.metadata();
  let metadata = TdHandleMetadata {
    mtime: meta.mtime_epoch_secs(),
    ctime: meta.ctime_epoch_secs(),
  };
  match handle {
    FileHandle::Path(path, _) => TdHandle::Path {
      path: path.to_string_lossy().into_owned(),
      metadata,
    },
    FileHandle::Content(path, content, _) => TdHandle::Content {
      path: path.to_string_lossy().into_owned(),
      content: content.clone(),
      metadata,
    },
  }
}

/// Serialize a Typedown object to a typed `TdValue`
pub fn serialize_value(db: &TypedownDatabase, project: Project, obj: &TdObjEnum) -> TdValue {
  serialize(db, project, obj, &mut HashSet::new(), false)
}

fn serialize(
  db: &TypedownDatabase,
  project: Project,
  obj: &TdObjEnum,
  visiting: &mut HashSet<typedown_incremental::DepId>,
  nested: bool,
) -> TdValue {
  match obj {
    TdObjEnum::TdNullObj(_) => TdValue::Null,
    TdObjEnum::TdBoolObj(bool_obj) => TdValue::Bool(bool_obj.value(db)),

    TdObjEnum::TdNumberObj(number_obj) => {
      let value = number_obj.value(db);
      if value.is_nan() {
        TdValue::Tagged(TdTaggedValue::Nan)
      } else if value == f64::INFINITY {
        TdValue::Tagged(TdTaggedValue::Inf)
      } else if value == f64::NEG_INFINITY {
        TdValue::Tagged(TdTaggedValue::NegInf)
      } else {
        TdValue::Number(FiniteF64::new(value))
      }
    }

    TdObjEnum::TdStringObj(string_obj) => TdValue::String(string_obj.value(db)),

    TdObjEnum::TdMathObj(math_obj) => TdValue::Tagged(TdTaggedValue::Math {
      value: math_obj.value(db),
    }),
    TdObjEnum::TdDateObj(date) => TdValue::Tagged(TdTaggedValue::Date {
      value: date.value(db),
    }),
    TdObjEnum::TdDateTimeObj(datetime) => TdValue::Tagged(TdTaggedValue::DateTime {
      value: datetime.value(db),
    }),
    TdObjEnum::TdTimeObj(time) => TdValue::Tagged(TdTaggedValue::Time {
      value: time.value(db),
    }),

    TdObjEnum::TdIconObj(icon_obj) => TdValue::Tagged(TdTaggedValue::Icon {
      name: icon_obj.name(db),
      lucide_name: icon_obj.lucide_name(db),
    }),

    TdObjEnum::TdListObj(list) => {
      let count = list.items(db).len();
      let mut items = Vec::with_capacity(count);
      for index in 0..count {
        match list.get(db, index) {
          Some(item) => items.push(serialize(db, project, &item, visiting, true)),
          None => items.push(TdValue::Null),
        }
      }
      TdValue::Array(items)
    }

    TdObjEnum::TdDictObj(dict) => {
      let mut map = BTreeMap::new();
      for (key, entry) in dict.entries(db) {
        if let Some(item) = evaluate_lazy_field(db, entry) {
          map.insert(key, serialize(db, project, &item, visiting, true));
        }
      }
      TdValue::Tagged(TdTaggedValue::Dict { entries: map })
    }

    TdObjEnum::TdSchemaObj(schema_obj) => {
      let schema_name = schema_obj.schema(db).display_name(db);

      // $ref for nested file-scoped objects
      if nested
        && let Some(symbol) = schema_obj.file_symbol(db)
        && let Some(resolved) = resolve_file_ref(db, project, &symbol)
      {
        let icon = schema_obj
          .get_builtin_field(db, "_icon")
          .and_then(|obj| obj.as_td_icon_obj().map(|icon| icon.lucide_name(db)));
        return TdValue::Tagged(TdTaggedValue::Ref(TdRef {
          schema: Some(schema_name),
          type_name: None,
          resolved: Some(TdResolvedLink {
            url: resolved.url,
            name: resolved.name,
            icon: icon.map(|name| TdRefIcon { name }),
            format: None,
          }),
        }));
      }

      let id = schema_obj.as_id();
      if !visiting.insert(id) {
        return TdValue::Tagged(TdTaggedValue::Ref(TdRef {
          schema: Some(schema_name),
          type_name: None,
          resolved: None,
        }));
      }
      let fields = serialize_object_fields(
        db,
        project,
        schema_obj.builtins(db),
        schema_obj.fields(db),
        visiting,
      );
      visiting.remove(&id);
      TdValue::Tagged(TdTaggedValue::Schema {
        schema: schema_name,
        fields,
      })
    }

    TdObjEnum::TdProductObj(product) => {
      let typ_name = product.product_typ(db).display_name(db);

      if nested
        && let Some(symbol) = product.file_symbol(db)
        && let Some(resolved) = resolve_file_ref(db, project, &symbol)
      {
        let icon = product
          .get_builtin_field(db, "_icon")
          .and_then(|obj| obj.as_td_icon_obj().map(|icon| icon.lucide_name(db)));
        return TdValue::Tagged(TdTaggedValue::Ref(TdRef {
          schema: None,
          type_name: Some(typ_name),
          resolved: Some(TdResolvedLink {
            url: resolved.url,
            name: resolved.name,
            icon: icon.map(|name| TdRefIcon { name }),
            format: None,
          }),
        }));
      }

      let id = product.as_id();
      if !visiting.insert(id) {
        return TdValue::Tagged(TdTaggedValue::Ref(TdRef {
          schema: None,
          type_name: Some(typ_name),
          resolved: None,
        }));
      }
      let fields = serialize_object_fields(
        db,
        project,
        product.builtins(db),
        product.fields(db),
        visiting,
      );
      visiting.remove(&id);
      TdValue::Tagged(TdTaggedValue::Product { fields })
    }

    TdObjEnum::TdBlobObj(blob) => {
      let format = blob.asset_kind(db).as_format_str().to_string();
      let file = blob.file(db);

      if nested
        && let Some(symbol) = file_symbol(db, project, file).value(db)
        && let Some(resolved) = resolve_file_ref(db, project, &symbol)
      {
        return TdValue::Tagged(TdTaggedValue::Ref(TdRef {
          schema: None,
          type_name: None,
          resolved: Some(TdResolvedLink {
            url: resolved.url,
            name: resolved.name,
            icon: None,
            format: Some(format),
          }),
        }));
      }

      let handle = serialize_handle(&file.handle(db));
      TdValue::Tagged(TdTaggedValue::Blob { format, handle })
    }

    TdObjEnum::TdTypObj(typ) => serialize_typ_enum(db, typ, visiting),

    _ => TdValue::Null,
  }
}

/// Serialize builtins and fields into a BTreeMap
fn serialize_object_fields<'db>(
  db: &'db TypedownDatabase,
  project: Project,
  builtins: impl IntoIterator<Item = (String, Either<ProjectRedNode, TdObjEnum<'db>>)>,
  fields: impl IntoIterator<Item = (String, Either<ProjectRedNode, TdObjEnum<'db>>)>,
  visiting: &mut HashSet<typedown_incremental::DepId>,
) -> BTreeMap<String, TdValue> {
  let mut map = BTreeMap::new();
  for (key, entry) in builtins.into_iter().chain(fields) {
    if let Some(item) = evaluate_lazy_field(db, entry) {
      map.insert(key, serialize(db, project, &item, visiting, true));
    }
  }
  map
}

/// Serialize a type enum to a $kind-tagged value
fn serialize_typ_enum(
  db: &TypedownDatabase,
  typ: &TdTypEnum,
  visiting: &mut HashSet<typedown_incremental::DepId>,
) -> TdValue {
  match typ {
    TdTypEnum::TdNullTyp(_) => TdValue::Tagged(TdTaggedValue::TypeNull),
    TdTypEnum::TdBoolTyp(_) => TdValue::Tagged(TdTaggedValue::TypeBool),
    TdTypEnum::TdNumberTyp(_) => TdValue::Tagged(TdTaggedValue::TypeNumber),
    TdTypEnum::TdStringTyp(_) => TdValue::Tagged(TdTaggedValue::TypeString),
    TdTypEnum::TdMathTyp(_) => TdValue::Tagged(TdTaggedValue::TypeMath),
    TdTypEnum::TdDateTyp(_) => TdValue::Tagged(TdTaggedValue::TypeDate),
    TdTypEnum::TdDateTimeTyp(_) => TdValue::Tagged(TdTaggedValue::TypeDateTime),
    TdTypEnum::TdTimeTyp(_) => TdValue::Tagged(TdTaggedValue::TypeTime),
    TdTypEnum::TdListTyp(list) => {
      let items = list
        .element(db)
        .map(|elem| Box::new(serialize_lazy_typ(db, &elem, visiting)));
      TdValue::Tagged(TdTaggedValue::TypeArray { items })
    }
    TdTypEnum::TdDictTyp(dict) => {
      let values = dict
        .value(db)
        .map(|val| Box::new(serialize_lazy_typ(db, &val, visiting)));
      TdValue::Tagged(TdTaggedValue::TypeDict { values })
    }
    TdTypEnum::TdSumTyp(sum) => {
      let mut members_lazy: Vec<_> = sum.members(db).into_iter().collect();
      members_lazy.sort_by(|left, right| {
        left
          .resolve(db)
          .map(|typ| typ.display_name(db))
          .cmp(&right.resolve(db).map(|typ| typ.display_name(db)))
      });
      let members: Vec<_> = members_lazy
        .iter()
        .map(|member| serialize_lazy_typ(db, member, visiting))
        .collect();
      TdValue::Tagged(TdTaggedValue::TypeUnion { members })
    }
    TdTypEnum::TdSchemaTyp(schema) => {
      let id = schema.as_id();
      if !visiting.insert(id) {
        return TdValue::Tagged(TdTaggedValue::TypeSchema {
          name: schema.name(db),
          fields: None,
        });
      }
      let mut fields = BTreeMap::new();
      for (name, prop) in schema.fields(db) {
        fields.insert(name, serialize_lazy_typ(db, &prop.field_typ, visiting));
      }
      visiting.remove(&id);
      TdValue::Tagged(TdTaggedValue::TypeSchema {
        name: schema.name(db),
        fields: Some(fields),
      })
    }
    TdTypEnum::TdProductTyp(product) => {
      let mut fields = BTreeMap::new();
      for (name, lazy) in product.fields(db) {
        fields.insert(name, serialize_lazy_typ(db, &lazy, visiting));
      }
      TdValue::Tagged(TdTaggedValue::TypeProduct {
        name: product.name(db),
        fields,
      })
    }
    _ => TdValue::Tagged(TdTaggedValue::TypeUnknown),
  }
}

fn serialize_lazy_typ(
  db: &TypedownDatabase,
  lazy: &LazyTyp,
  visiting: &mut HashSet<typedown_incremental::DepId>,
) -> TdValue {
  match lazy.resolve(db) {
    Some(typ) => serialize_typ_enum(db, &typ, visiting),
    None => TdValue::Tagged(TdTaggedValue::TypeUnknown),
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::db::derived::evaluate::evaluate_resource::evaluate_resource;
  use crate::db::fixtures::*;
  use crate::db::types::*;
  use crate::db::{QueryStorage, TypedownDatabase};
  use std::collections::HashMap;
  use std::path::PathBuf;
  use typedown_types::either::Either;

  fn empty_db() -> (TypedownDatabase, Project) {
    let db = TypedownDatabase {
      storage: QueryStorage::default(),
    };
    let project = Project::new(&db, PathBuf::new(), HashMap::new(), None);
    (db, project)
  }

  fn to_json(value: &TdValue) -> serde_json::Value {
    serde_json::to_value(value).unwrap()
  }

  #[test]
  fn nan_serializes_as_kind() {
    let (db, project) = empty_db();
    let obj = TdObjEnum::from(make_number_obj(&db, f64::NAN.to_bits()));
    let json = to_json(&serialize_value(&db, project, &obj));
    assert_eq!(json["$kind"], "nan");
  }

  #[test]
  fn infinity_serializes_as_kind() {
    let (db, project) = empty_db();
    let obj = TdObjEnum::from(make_number_obj(&db, f64::INFINITY.to_bits()));
    let json = to_json(&serialize_value(&db, project, &obj));
    assert_eq!(json["$kind"], "inf");
  }

  #[test]
  fn neg_infinity_serializes_as_kind() {
    let (db, project) = empty_db();
    let obj = TdObjEnum::from(make_number_obj(&db, f64::NEG_INFINITY.to_bits()));
    let json = to_json(&serialize_value(&db, project, &obj));
    assert_eq!(json["$kind"], "-inf");
  }

  #[test]
  fn serializes_math_as_kind() {
    let (db, project) = empty_db();
    let obj = TdObjEnum::from(make_math_obj(&db, "$E = mc^2$".to_string()));
    let json = to_json(&serialize_value(&db, project, &obj));
    assert_eq!(json["$kind"], "math");
    assert_eq!(json["value"], "$E = mc^2$");
  }

  #[test]
  fn serializes_product_with_kind() {
    let (db, project) = empty_db();
    let string_typ: TdTypEnum = TdStringTyp::get(&db).into();
    let number_typ: TdTypEnum = TdNumberTyp::get(&db).into();
    let inner = make_product_obj(&db, string_typ, None, vec![]);
    let fields = vec![("inner".to_string(), Either::Right(TdObjEnum::from(inner)))];
    let outer = make_product_obj(&db, number_typ, None, fields);
    let json = to_json(&serialize_value(&db, project, &TdObjEnum::from(outer)));
    assert_eq!(json["$kind"], "product");
    assert!(json["fields"].is_object());
  }

  #[test]
  fn blob_serializes_with_kind() {
    let (db, project) = empty_db();
    let path = PathBuf::from("/vault/_assets/photo.png");
    let file = File::new(&db, FileHandle::Path(path, FileMetadata::default()));
    let blob = make_blob_obj(&db, AssetKind::Png, file);
    let json = to_json(&serialize_value(&db, project, &TdObjEnum::from(blob)));
    assert_eq!(json["$kind"], "blob");
    assert_eq!(json["format"], "png");
    assert_eq!(json["handle"]["path"], "/vault/_assets/photo.png");
  }

  #[test]
  fn fref_field_serializes_as_ref() {
    let (db, project, file) = load_vault_fixture("evaluate/my_vault", "with_fref.td");
    let result = evaluate_resource(&db, file_symbol(&db, project, file).value(&db).unwrap());
    let obj = result.value(&db).expect("should evaluate resource");
    let json = to_json(&serialize_value(&db, project, &obj));
    assert_eq!(
      json["fields"]["friend"]["$kind"], "ref",
      "friend should be $ref: {json}"
    );
    assert!(
      json["fields"]["friend"]["$ref"]["url"].is_string(),
      "ref should have url: {json}"
    );
  }
}
