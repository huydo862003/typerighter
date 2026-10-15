//! Translate JSON Schema (from the component contract) into typedown types.

use std::collections::BTreeMap;

use crate::db::TypedownDatabase;
use crate::db::derived::get_builtin_typs::{
  get_bool_typ, get_dict_typ, get_list_typ, get_lit_typ, get_null_typ, get_number_typ,
  get_string_typ, get_sum_typ,
};
use crate::db::types::derived::obj_system::TdStaticTyp;
use crate::db::types::{LazyTyp, LitValue, TdProductTyp, TdTypEnum};

pub fn parse_json_schema<'db>(
  db: &'db TypedownDatabase,
  value: &serde_json::Value,
) -> Option<TdTypEnum<'db>> {
  // { enum: [...] }
  if let Some(members) = value.get("enum").and_then(|e| e.as_array()) {
    return parse_enum(db, members);
  }

  // { oneOf: [...] } or { anyOf: [...] }
  if let Some(schemas) = value
    .get("oneOf")
    .or_else(|| value.get("anyOf"))
    .and_then(|value| value.as_array())
  {
    return parse_schema_union(db, schemas);
  }

  // { type: "string" } or { type: ["string", "null"] }
  match value.get("type")? {
    serde_json::Value::Array(typs) => parse_typ_array(db, typs, value),
    serde_json::Value::String(typ) => parse_typ_string(db, typ, value),
    _ => None,
  }
}

fn parse_enum<'db>(
  db: &'db TypedownDatabase,
  members: &[serde_json::Value],
) -> Option<TdTypEnum<'db>> {
  let lazy_members: Vec<LazyTyp<'db>> = members
    .iter()
    .filter_map(|member| {
      let typ: TdTypEnum = match member {
        serde_json::Value::String(string) => {
          get_lit_typ(db, LitValue::String(string.clone())).into()
        }
        serde_json::Value::Number(number) => {
          get_lit_typ(db, LitValue::Number(number.to_string())).into()
        }
        serde_json::Value::Bool(bool) => get_lit_typ(db, LitValue::Bool(*bool)).into(),
        serde_json::Value::Null => get_null_typ(db).into(),
        _ => return None,
      };
      Some(LazyTyp::eager(typ))
    })
    .collect();
  if lazy_members.is_empty() {
    return None;
  }
  Some(get_sum_typ(db, lazy_members).into())
}

fn parse_schema_union<'db>(
  db: &'db TypedownDatabase,
  schemas: &[serde_json::Value],
) -> Option<TdTypEnum<'db>> {
  let members: Vec<LazyTyp<'db>> = schemas
    .iter()
    .filter_map(|s| parse_json_schema(db, s))
    .map(LazyTyp::eager)
    .collect();
  if members.is_empty() {
    return None;
  }
  if members.len() == 1 {
    return members.into_iter().next()?.resolve(db);
  }
  Some(get_sum_typ(db, members).into())
}

fn parse_typ_array<'db>(
  db: &'db TypedownDatabase,
  typs: &[serde_json::Value],
  schema: &serde_json::Value,
) -> Option<TdTypEnum<'db>> {
  let non_null: Vec<&str> = typs
    .iter()
    .filter_map(|typ| typ.as_str())
    .filter(|typ| *typ != "null" && *typ != "undefined")
    .collect();
  let has_null = typs.iter().any(|typ| {
    typ
      .as_str()
      .is_some_and(|string| string == "null" || string == "undefined")
  });

  if non_null.is_empty() {
    return None;
  }

  // Single concrete type + null -> Sum([T, null])
  if non_null.len() == 1 {
    let inner_typ = parse_typ_string(db, non_null[0], schema)?;
    if has_null {
      let null_typ: TdTypEnum = get_null_typ(db).into();
      return Some(
        get_sum_typ(
          db,
          vec![LazyTyp::eager(inner_typ), LazyTyp::eager(null_typ)],
        )
        .into(),
      );
    }
    return Some(inner_typ);
  }

  // Multiple concrete types -> Sum([T1, T2, ...]) optionally with null
  let mut members: Vec<LazyTyp<'db>> = non_null
    .iter()
    .filter_map(|t| parse_typ_string(db, t, schema))
    .map(LazyTyp::eager)
    .collect();
  if members.is_empty() {
    return None;
  }
  if has_null {
    members.push(LazyTyp::eager(get_null_typ(db).into()));
  }
  if members.len() == 1 {
    return members.into_iter().next()?.resolve(db);
  }
  Some(get_sum_typ(db, members).into())
}

// Check if any sum member resolves to null
pub fn is_optional(db: &TypedownDatabase, typ: &TdTypEnum) -> bool {
  typ.as_td_sum_typ().is_some_and(|sum| {
    sum
      .members(db)
      .iter()
      .any(|member| member.resolve(db).is_some_and(|typ| typ.is_td_null_typ()))
  })
}

// Strip null from Sum([T, null]) -> T, passthrough if not optional
pub fn unwrap_optional<'db>(db: &'db TypedownDatabase, typ: TdTypEnum<'db>) -> TdTypEnum<'db> {
  let Some(sum) = typ.as_td_sum_typ() else {
    return typ;
  };
  // Filter out null members and return the single remaining type
  let non_null: Vec<_> = sum
    .members(db)
    .into_iter()
    .filter(|member| !member.resolve(db).is_some_and(|typ| typ.is_td_null_typ()))
    .collect();
  if non_null.len() == 1
    && let Some(inner) = non_null[0].resolve(db)
  {
    return inner;
  }
  typ
}

// Map a concrete JSON Schema type string to a td type
fn parse_typ_string<'db>(
  db: &'db TypedownDatabase,
  type_str: &str,
  schema: &serde_json::Value,
) -> Option<TdTypEnum<'db>> {
  match type_str {
    "string" => Some(get_string_typ(db).into()),
    "number" => Some(get_number_typ(db).into()),
    "boolean" => Some(get_bool_typ(db).into()),
    "array" => {
      // Parse element schema, instantiate list[T]
      let element = parse_json_schema(db, schema.get("items")?)?;
      let list_typ: TdTypEnum = get_list_typ(db).into();
      Some(
        list_typ
          .instantiate(db, vec![LazyTyp::eager(element)])
          .typ(db),
      )
    }
    "object" => {
      if let Some(additional) = schema.get("additionalProperties").filter(|v| v.is_object()) {
        // additionalProperties is a schema -> dict[string, V]
        let value_typ = parse_json_schema(db, additional)?;
        let string_typ: TdTypEnum = get_string_typ(db).into();
        let dict_typ: TdTypEnum = get_dict_typ(db).into();
        Some(
          dict_typ
            .instantiate(
              db,
              vec![LazyTyp::eager(string_typ), LazyTyp::eager(value_typ)],
            )
            .typ(db),
        )
      } else if let Some(props) = schema.get("properties").and_then(|p| p.as_object()) {
        // Named properties -> anonymous product type
        let mut fields = BTreeMap::new();
        for (key, prop_schema) in props {
          let prop_typ = parse_json_schema(db, prop_schema)?;
          fields.insert(key.clone(), LazyTyp::eager(prop_typ));
        }
        Some(TdProductTyp::new(db, None, fields).into())
      } else {
        None
      }
    }
    _ => None,
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::db::{QueryStorage, TypedownDatabase};

  fn make_db() -> TypedownDatabase {
    TypedownDatabase {
      storage: QueryStorage::default(),
    }
  }

  #[test]
  fn primitive_required() {
    let db = make_db();
    let typ = parse_json_schema(&db, &serde_json::json!({ "type": "string" })).unwrap();
    assert!(
      typ.is_td_string_typ(),
      "expected string, got {}",
      typ.display_name(&db)
    );

    let typ = parse_json_schema(&db, &serde_json::json!({ "type": "number" })).unwrap();
    assert!(typ.is_td_number_typ());

    let typ = parse_json_schema(&db, &serde_json::json!({ "type": "boolean" })).unwrap();
    assert!(typ.is_td_bool_typ());
  }

  #[test]
  fn primitive_optional_null() {
    let db = make_db();
    let typ = parse_json_schema(&db, &serde_json::json!({ "type": ["string", "null"] })).unwrap();
    assert!(is_optional(&db, &typ));
    let inner = unwrap_optional(&db, typ);
    assert!(inner.is_td_string_typ());
  }

  #[test]
  fn primitive_optional_undefined() {
    let db = make_db();
    let typ =
      parse_json_schema(&db, &serde_json::json!({ "type": ["number", "undefined"] })).unwrap();
    assert!(is_optional(&db, &typ));
    let inner = unwrap_optional(&db, typ);
    assert!(inner.is_td_number_typ());
  }

  #[test]
  fn array_of_strings() {
    let db = make_db();
    let typ = parse_json_schema(
      &db,
      &serde_json::json!({ "type": "array", "items": { "type": "string" } }),
    )
    .unwrap();
    assert!(
      typ.is_td_list_typ(),
      "expected list, got {}",
      typ.display_name(&db)
    );
  }

  #[test]
  fn optional_array() {
    let db = make_db();
    let typ = parse_json_schema(
      &db,
      &serde_json::json!({ "type": ["array", "null"], "items": { "type": "number" } }),
    )
    .unwrap();
    assert!(is_optional(&db, &typ));
    let inner = unwrap_optional(&db, typ);
    assert!(inner.is_td_list_typ());
  }

  #[test]
  fn dict_object() {
    let db = make_db();
    let typ = parse_json_schema(
      &db,
      &serde_json::json!({ "type": "object", "additionalProperties": { "type": "string" } }),
    )
    .unwrap();
    assert!(
      typ.is_td_dict_typ(),
      "expected dict, got {}",
      typ.display_name(&db)
    );
  }

  #[test]
  fn product_object() {
    let db = make_db();
    let typ = parse_json_schema(
      &db,
      &serde_json::json!({
        "type": "object",
        "properties": { "name": { "type": "string" }, "age": { "type": "number" } },
        "additionalProperties": false
      }),
    )
    .unwrap();
    assert!(
      typ.is_td_product_typ(),
      "expected product, got {}",
      typ.display_name(&db)
    );
    let display = typ.display_name(&db);
    assert!(display.contains("name"), "{display}");
    assert!(display.contains("age"), "{display}");
  }

  #[test]
  fn enum_of_strings() {
    let db = make_db();
    let typ = parse_json_schema(
      &db,
      &serde_json::json!({ "enum": ["info", "warning", "error"] }),
    )
    .unwrap();
    assert!(
      typ.is_td_sum_typ(),
      "expected sum type, got {}",
      typ.display_name(&db)
    );
    assert!(!is_optional(&db, &typ));
  }

  #[test]
  fn enum_single_member() {
    let db = make_db();
    let typ = parse_json_schema(&db, &serde_json::json!({ "enum": ["only"] })).unwrap();
    assert!(typ.is_td_sum_typ());
    assert!(!is_optional(&db, &typ));
  }

  #[test]
  fn enum_nullable() {
    let db = make_db();
    let typ = parse_json_schema(&db, &serde_json::json!({ "enum": ["a", "b", null] })).unwrap();
    assert!(
      typ.is_td_sum_typ(),
      "expected sum type, got {}",
      typ.display_name(&db)
    );
    assert!(is_optional(&db, &typ));
  }

  #[test]
  fn enum_empty_returns_none() {
    let db = make_db();
    assert!(parse_json_schema(&db, &serde_json::json!({ "enum": [] })).is_none());
  }

  #[test]
  fn enum_mixed_types() {
    let db = make_db();
    let typ = parse_json_schema(&db, &serde_json::json!({ "enum": ["yes", true, 1] })).unwrap();
    assert!(typ.is_td_sum_typ());
  }

  #[test]
  fn multi_type_array() {
    let db = make_db();
    let typ = parse_json_schema(&db, &serde_json::json!({ "type": ["string", "number"] })).unwrap();
    assert!(typ.is_td_sum_typ());
    assert!(!is_optional(&db, &typ));
  }

  #[test]
  fn multi_type_array_nullable() {
    let db = make_db();
    let typ = parse_json_schema(
      &db,
      &serde_json::json!({ "type": ["string", "number", "null"] }),
    )
    .unwrap();
    assert!(typ.is_td_sum_typ());
    assert!(is_optional(&db, &typ));
  }

  #[test]
  fn one_of_schema() {
    let db = make_db();
    let typ = parse_json_schema(
      &db,
      &serde_json::json!({ "oneOf": [{ "type": "string" }, { "type": "number" }] }),
    )
    .unwrap();
    assert!(typ.is_td_sum_typ());
  }

  #[test]
  fn any_of_schema() {
    let db = make_db();
    let typ = parse_json_schema(
      &db,
      &serde_json::json!({ "anyOf": [{ "type": "string" }, { "type": "boolean" }] }),
    )
    .unwrap();
    assert!(typ.is_td_sum_typ());
  }

  #[test]
  fn one_of_single_passthrough() {
    let db = make_db();
    let typ =
      parse_json_schema(&db, &serde_json::json!({ "oneOf": [{ "type": "string" }] })).unwrap();
    assert!(typ.is_td_string_typ());
  }

  #[test]
  fn one_of_empty_returns_none() {
    let db = make_db();
    assert!(parse_json_schema(&db, &serde_json::json!({ "oneOf": [] })).is_none());
  }

  #[test]
  fn unknown_type_returns_none() {
    let db = make_db();
    assert!(parse_json_schema(&db, &serde_json::json!({ "type": "integer" })).is_none());
  }

  #[test]
  fn missing_type_returns_none() {
    let db = make_db();
    assert!(
      parse_json_schema(&db, &serde_json::json!({ "items": { "type": "string" } })).is_none()
    );
  }
}
