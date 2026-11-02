//! Deserialize JSON values back into Typedown objects (round-trip support)
//!
//! Handles both plain POD values and $kind-tagged values

use std::collections::BTreeMap;
use typedown_types::either::Either;

use crate::db::TypedownDatabase;
use crate::db::types::TdObjEnum;
use crate::db::types::derived::obj_system::{
  TdBoolObj, TdDateObj, TdDateTimeObj, TdDictObj, TdIconObj, TdListObj, TdMathObj, TdNullObj,
  TdNumberObj, TdStringObj, TdTimeObj,
};

/// Deserialize a JSON value into a Typedown object
pub fn deserialize_value<'db>(
  db: &'db TypedownDatabase,
  value: serde_json::Value,
) -> Option<TdObjEnum<'db>> {
  match value {
    serde_json::Value::Null => Some(TdNullObj::get(db).into()),
    serde_json::Value::Bool(bool_value) => Some(TdBoolObj::new(db, bool_value).into()),
    serde_json::Value::Number(number) => {
      let float_value = number.as_f64()?;
      Some(TdNumberObj::new(db, float_value).into())
    }
    serde_json::Value::String(string_value) => Some(TdStringObj::new(db, string_value).into()),
    serde_json::Value::Array(array) => {
      let items: Vec<_> = array
        .into_iter()
        .map(|item| deserialize_value(db, item).map(Either::Right))
        .collect::<Option<_>>()?;
      Some(TdListObj::new(db, items).into())
    }
    serde_json::Value::Object(map) => {
      // Check for $kind tag
      if let Some(kind) = map
        .get("$kind")
        .and_then(|value| value.as_str())
        .map(|string| string.to_string())
      {
        return deserialize_tagged(db, &kind, map);
      }
      // Plain object -> TdDictObj
      let entries: BTreeMap<String, _> = map
        .into_iter()
        .map(|(key, value)| deserialize_value(db, value).map(|obj| (key, Either::Right(obj))))
        .collect::<Option<Vec<_>>>()?
        .into_iter()
        .collect();
      Some(TdDictObj::new(db, entries).into())
    }
  }
}

/// Deserialize a $kind-tagged JSON object
fn deserialize_tagged<'db>(
  db: &'db TypedownDatabase,
  kind: &str,
  map: serde_json::Map<String, serde_json::Value>,
) -> Option<TdObjEnum<'db>> {
  match kind {
    // Non-finite numbers
    "nan" => Some(TdNumberObj::new(db, f64::NAN).into()),
    "inf" => Some(TdNumberObj::new(db, f64::INFINITY).into()),
    "-inf" => Some(TdNumberObj::new(db, f64::NEG_INFINITY).into()),

    // Typed strings
    "math" => {
      let value = map.get("value")?.as_str()?;
      Some(TdMathObj::new(db, value.to_string()).into())
    }
    "date" => {
      let value = map.get("value")?.as_str()?;
      Some(TdDateObj::new(db, value.to_string()).into())
    }
    "datetime" => {
      let value = map.get("value")?.as_str()?;
      Some(TdDateTimeObj::new(db, value.to_string()).into())
    }
    "time" => {
      let value = map.get("value")?.as_str()?;
      Some(TdTimeObj::new(db, value.to_string()).into())
    }

    // Icon
    "icon" => {
      let name = map.get("name")?.as_str()?;
      let lucide_name = map.get("lucide_name")?.as_str()?;
      Some(TdIconObj::new(db, name.to_string(), lucide_name.to_string()).into())
    }

    // Schema and product objects -> TdDictObj (fields only, no type binding)
    "schema" | "product" => {
      let fields_value = map.get("fields")?;
      let fields_map = fields_value.as_object()?;
      let entries: BTreeMap<String, _> = fields_map
        .iter()
        .map(|(key, value)| {
          deserialize_value(db, value.clone()).map(|obj| (key.clone(), Either::Right(obj)))
        })
        .collect::<Option<Vec<_>>>()?
        .into_iter()
        .collect();
      Some(TdDictObj::new(db, entries).into())
    }

    // Blob: not round-trippable without filesystem access, return as dict
    "blob" => {
      let mut entries = BTreeMap::new();
      if let Some(format) = map.get("format")
        && let Some(obj) = deserialize_value(db, format.clone())
      {
        entries.insert("format".to_string(), Either::Right(obj));
      }
      if let Some(handle) = map.get("handle")
        && let Some(obj) = deserialize_value(db, handle.clone())
      {
        entries.insert("handle".to_string(), Either::Right(obj));
      }
      Some(TdDictObj::new(db, entries).into())
    }

    // Type descriptors: return as dict with $kind preserved
    _ if kind.starts_with("type:") => {
      let mut entries = BTreeMap::new();
      for (key, value) in &map {
        if let Some(obj) = deserialize_value(db, value.clone()) {
          entries.insert(key.clone(), Either::Right(obj));
        }
      }
      Some(TdDictObj::new(db, entries).into())
    }

    // Unknown $kind: return as plain dict
    _ => {
      let entries: BTreeMap<String, _> = map
        .into_iter()
        .map(|(key, value)| deserialize_value(db, value).map(|obj| (key, Either::Right(obj))))
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

#[cfg(test)]
mod tests {
  use super::*;
  use crate::db::types::TdRuntimeObj;
  use crate::db::{QueryStorage, TypedownDatabase};
  use typedown_incremental::{QueryDatabase, query_derived};

  fn make_db() -> TypedownDatabase {
    TypedownDatabase {
      storage: QueryStorage::default(),
    }
  }

  // Query-context wrappers: #[query_derived] requires a specific derived return type.

  #[query_derived]
  fn deserialize_null_fixture<'db>(db: &'db TypedownDatabase) -> TdNullObj<'db> {
    *deserialize_value(db, serde_json::Value::Null)
      .unwrap()
      .as_td_null_obj()
      .unwrap()
  }

  #[query_derived]
  fn deserialize_bool_true<'db>(db: &'db TypedownDatabase) -> TdBoolObj<'db> {
    *deserialize_value(db, serde_json::json!(true))
      .unwrap()
      .as_td_bool_obj()
      .unwrap()
  }

  #[query_derived]
  fn deserialize_number_314<'db>(db: &'db TypedownDatabase) -> TdNumberObj<'db> {
    *deserialize_value(db, serde_json::json!(std::f64::consts::PI))
      .unwrap()
      .as_td_number_obj()
      .unwrap()
  }

  #[query_derived]
  fn deserialize_string_hello<'db>(db: &'db TypedownDatabase) -> TdStringObj<'db> {
    *deserialize_value(db, serde_json::json!("hello"))
      .unwrap()
      .as_td_string_obj()
      .unwrap()
  }

  #[query_derived]
  fn deserialize_array_mixed<'db>(db: &'db TypedownDatabase) -> TdListObj<'db> {
    *deserialize_value(db, serde_json::json!([1, "two", false]))
      .unwrap()
      .as_td_list_obj()
      .unwrap()
  }

  #[query_derived]
  fn deserialize_object_xy<'db>(db: &'db TypedownDatabase) -> TdDictObj<'db> {
    *deserialize_value(db, serde_json::json!({"x": 1, "y": "hello"}))
      .unwrap()
      .as_td_dict_obj()
      .unwrap()
  }

  #[query_derived]
  fn deserialize_nested<'db>(db: &'db TypedownDatabase) -> TdDictObj<'db> {
    *deserialize_value(
      db,
      serde_json::json!({"items": [1, 2, 3], "meta": {"ok": true}}),
    )
    .unwrap()
    .as_td_dict_obj()
    .unwrap()
  }

  #[query_derived]
  fn deserialize_math_expr<'db>(db: &'db TypedownDatabase) -> TdMathObj<'db> {
    *deserialize_math(db, r"E = mc^2".to_string())
      .as_td_math_obj()
      .unwrap()
  }

  // $kind round-trip wrappers

  #[query_derived]
  fn deserialize_kind_nan<'db>(db: &'db TypedownDatabase) -> TdNumberObj<'db> {
    *deserialize_value(db, serde_json::json!({"$kind": "nan"}))
      .unwrap()
      .as_td_number_obj()
      .unwrap()
  }

  #[query_derived]
  fn deserialize_kind_inf<'db>(db: &'db TypedownDatabase) -> TdNumberObj<'db> {
    *deserialize_value(db, serde_json::json!({"$kind": "inf"}))
      .unwrap()
      .as_td_number_obj()
      .unwrap()
  }

  #[query_derived]
  fn deserialize_kind_neg_inf<'db>(db: &'db TypedownDatabase) -> TdNumberObj<'db> {
    *deserialize_value(db, serde_json::json!({"$kind": "-inf"}))
      .unwrap()
      .as_td_number_obj()
      .unwrap()
  }

  #[query_derived]
  fn deserialize_kind_math<'db>(db: &'db TypedownDatabase) -> TdMathObj<'db> {
    *deserialize_value(db, serde_json::json!({"$kind": "math", "value": "x^2"}))
      .unwrap()
      .as_td_math_obj()
      .unwrap()
  }

  #[query_derived]
  fn deserialize_kind_date<'db>(db: &'db TypedownDatabase) -> TdDateObj<'db> {
    *deserialize_value(
      db,
      serde_json::json!({"$kind": "date", "value": "2024-01-15"}),
    )
    .unwrap()
    .as_td_date_obj()
    .unwrap()
  }

  #[query_derived]
  fn deserialize_kind_datetime<'db>(db: &'db TypedownDatabase) -> TdDateTimeObj<'db> {
    *deserialize_value(
      db,
      serde_json::json!({"$kind": "datetime", "value": "2024-01-15T10:30:00Z"}),
    )
    .unwrap()
    .as_td_date_time_obj()
    .unwrap()
  }

  #[query_derived]
  fn deserialize_kind_time<'db>(db: &'db TypedownDatabase) -> TdTimeObj<'db> {
    *deserialize_value(
      db,
      serde_json::json!({"$kind": "time", "value": "10:30:00"}),
    )
    .unwrap()
    .as_td_time_obj()
    .unwrap()
  }

  #[query_derived]
  fn deserialize_kind_icon<'db>(db: &'db TypedownDatabase) -> TdIconObj<'db> {
    *deserialize_value(
      db,
      serde_json::json!({"$kind": "icon", "name": "icon.book", "lucide_name": "book"}),
    )
    .unwrap()
    .as_td_icon_obj()
    .unwrap()
  }

  #[query_derived]
  fn deserialize_kind_schema_fields<'db>(db: &'db TypedownDatabase) -> TdDictObj<'db> {
    *deserialize_value(
      db,
      serde_json::json!({"$kind": "schema", "schema": "Person", "fields": {"name": "Alice", "age": 30}}),
    )
    .unwrap()
    .as_td_dict_obj()
    .unwrap()
  }

  // Tests

  #[test]
  fn deserializes_null() {
    let db = make_db();
    let _ = deserialize_null_fixture(&db);
  }

  #[test]
  fn deserializes_bool() {
    let db = make_db();
    let bool_obj = deserialize_bool_true(&db);
    assert!(bool_obj.value(&db));
  }

  #[test]
  fn deserializes_number() {
    let db = make_db();
    let number = deserialize_number_314(&db);
    let delta = (number.value(&db) - std::f64::consts::PI).abs();
    assert!(
      delta < f64::EPSILON,
      "expected ~PI, got {}",
      number.value(&db)
    );
  }

  #[test]
  fn deserializes_string() {
    let db = make_db();
    let string = deserialize_string_hello(&db);
    assert_eq!(string.value(&db), "hello");
  }

  #[test]
  fn deserializes_array() {
    let db = make_db();
    let list = deserialize_array_mixed(&db);
    assert_eq!(list.len(&db), 3);
    assert!(list.get(&db, 0).unwrap().is_td_number_obj());
    assert!(list.get(&db, 1).unwrap().is_td_string_obj());
    assert!(list.get(&db, 2).unwrap().is_td_bool_obj());
  }

  #[test]
  fn deserializes_object() {
    let db = make_db();
    let dict = deserialize_object_xy(&db);
    assert!(dict.get_owned_field(&db, "x").unwrap().is_td_number_obj());
    assert!(dict.get_owned_field(&db, "y").unwrap().is_td_string_obj());
  }

  #[test]
  fn deserializes_nested() {
    let db = make_db();
    let dict = deserialize_nested(&db);
    assert!(dict.get_owned_field(&db, "items").unwrap().is_td_list_obj());
    assert!(dict.get_owned_field(&db, "meta").unwrap().is_td_dict_obj());
  }

  #[test]
  fn deserializes_math() {
    let db = make_db();
    let _ = deserialize_math_expr(&db);
  }

  // $kind round-trip tests

  #[test]
  fn round_trips_nan() {
    let db = make_db();
    let number = deserialize_kind_nan(&db);
    assert!(number.value(&db).is_nan());
  }

  #[test]
  fn round_trips_inf() {
    let db = make_db();
    let number = deserialize_kind_inf(&db);
    assert_eq!(number.value(&db), f64::INFINITY);
  }

  #[test]
  fn round_trips_neg_inf() {
    let db = make_db();
    let number = deserialize_kind_neg_inf(&db);
    assert_eq!(number.value(&db), f64::NEG_INFINITY);
  }

  #[test]
  fn round_trips_math() {
    let db = make_db();
    let math = deserialize_kind_math(&db);
    assert_eq!(math.value(&db), "x^2");
  }

  #[test]
  fn round_trips_date() {
    let db = make_db();
    let date = deserialize_kind_date(&db);
    assert_eq!(date.value(&db), "2024-01-15");
  }

  #[test]
  fn round_trips_datetime() {
    let db = make_db();
    let datetime = deserialize_kind_datetime(&db);
    assert_eq!(datetime.value(&db), "2024-01-15T10:30:00Z");
  }

  #[test]
  fn round_trips_time() {
    let db = make_db();
    let time = deserialize_kind_time(&db);
    assert_eq!(time.value(&db), "10:30:00");
  }

  #[test]
  fn round_trips_icon() {
    let db = make_db();
    let icon = deserialize_kind_icon(&db);
    assert_eq!(icon.name(&db), "icon.book");
    assert_eq!(icon.lucide_name(&db), "book");
  }

  #[test]
  fn schema_fields_deserialize_as_dict() {
    let db = make_db();
    let dict = deserialize_kind_schema_fields(&db);
    assert!(
      dict
        .get_owned_field(&db, "name")
        .unwrap()
        .is_td_string_obj()
    );
    assert!(dict.get_owned_field(&db, "age").unwrap().is_td_number_obj());
  }
}
