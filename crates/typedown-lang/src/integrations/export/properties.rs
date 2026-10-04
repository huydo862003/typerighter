//! Export schema property descriptors for the client

use super::json;
use crate::db::TypedownDatabase;
use crate::db::derived::evaluate::evaluate_type::evaluate_type;
use crate::db::derived::name_resolver::file_symbol::file_symbol;
use crate::db::types::{File, LazyType, LiteralValue, Project, TdTypeEnum};

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub enum Widget {
  Text,
  Number,
  Checkbox,
  Date,
  Select,
  MultiSelect,
  Relation,
  List,
}

/// Export schema property descriptors as structured JSON for the client
pub fn export_property_descriptors(
  db: &TypedownDatabase,
  project: Project,
  file: File,
) -> Option<serde_json::Value> {
  let file_sym = file_symbol(db, project, file);
  let symbol = file_sym.value(db)?;
  let eval_result = evaluate_type(db, symbol);
  let typ = eval_result.typ(db)?;

  let schema = typ.as_td_schema_type()?;
  let fields = schema.fields(db);

  let mut properties = serde_json::Map::new();

  for (name, prop_desc) in fields {
    let mut prop_json = export_lazy_to_descriptor(db, &prop_desc.field_type);
    if let Some(ref def_obj) = prop_desc.default_value
      && let Ok(def_json) = json::serialize_to_json(db, project, def_obj)
      && let serde_json::Value::Object(ref mut map) = prop_json
    {
      map.insert("default".to_string(), def_json);
    }
    properties.insert(name, prop_json);
  }

  Some(serde_json::Value::Object(properties))
}

// Map a LazyType to a property descriptor with a widget hint
fn export_lazy_to_descriptor(db: &TypedownDatabase, lazy: &LazyType) -> serde_json::Value {
  let Some(typ) = lazy.resolve(db) else {
    return serde_json::json!({ "type": "string" });
  };

  // Sum of string literals is a select; filter out TdNullType for nullable T? types
  if let Some(sum) = typ.as_td_sum_type() {
    let members = sum.members(db);
    let non_null_members: Vec<LazyType> = members
      .iter()
      .filter(|m| {
        if let Some(m_typ) = m.resolve(db) {
          !m_typ.is_td_null_type()
        } else {
          true
        }
      })
      .cloned()
      .collect();

    if non_null_members.len() == 1 {
      return export_lazy_to_descriptor(db, &non_null_members[0]);
    }

    let mut literals: Vec<String> = non_null_members
      .iter()
      .filter_map(|m| {
        if let Some(TdTypeEnum::TdLiteralType(lit)) = m.resolve(db)
          && let LiteralValue::Str(s) = lit.value(db)
        {
          Some(s)
        } else {
          None
        }
      })
      .collect();
    literals.sort();

    if !literals.is_empty() && literals.len() == non_null_members.len() {
      return serde_json::json!({ "widget": Widget::Select, "options": literals });
    }
    return serde_json::json!({ "widget": Widget::Text });
  }

  // List type: check if elem is a sum of string literals (multi_select)
  if let Some(list) = typ.as_td_list_type() {
    if let Some(elem_lazy) = list.elem(db)
      && let Some(elem_typ) = elem_lazy.resolve(db)
    {
      if let Some(sum) = elem_typ.as_td_sum_type() {
        let members = sum.members(db);
        let mut literals: Vec<String> = members
          .iter()
          .filter_map(|m| {
            if let Some(TdTypeEnum::TdLiteralType(lit)) = m.resolve(db)
              && let LiteralValue::Str(s) = lit.value(db)
            {
              Some(s)
            } else {
              None
            }
          })
          .collect();
        literals.sort();

        if literals.len() == members.len() && !literals.is_empty() {
          return serde_json::json!({ "widget": Widget::MultiSelect, "options": literals });
        }
        if members.len() == 1 {
          let first_member = members.iter().next().unwrap();
          let inner = export_lazy_to_descriptor(db, first_member);
          return serde_json::json!({ "widget": Widget::List, "items": inner });
        }
      } else {
        let inner = export_lazy_to_descriptor(db, &elem_lazy);
        return serde_json::json!({ "widget": Widget::List, "items": inner });
      }
    }
    return serde_json::json!({ "widget": Widget::Text });
  }

  export_simple_type_to_descriptor(db, &typ)
}

fn export_simple_type_to_descriptor(db: &TypedownDatabase, typ: &TdTypeEnum) -> serde_json::Value {
  match typ {
    TdTypeEnum::TdStrType(_) => serde_json::json!({ "widget": Widget::Text }),
    TdTypeEnum::TdNumType(_) => serde_json::json!({ "widget": Widget::Number }),
    TdTypeEnum::TdBoolType(_) => serde_json::json!({ "widget": Widget::Checkbox }),
    TdTypeEnum::TdDateType(_) => serde_json::json!({ "widget": Widget::Date }),
    TdTypeEnum::TdDateTimeType(_) => serde_json::json!({ "widget": Widget::Date }),
    TdTypeEnum::TdTimeType(_) => serde_json::json!({ "widget": Widget::Text }),
    TdTypeEnum::TdListType(list) => match list.elem(db).and_then(|e| e.resolve(db)) {
      Some(elem) => {
        let inner = export_simple_type_to_descriptor(db, &elem);
        serde_json::json!({ "widget": Widget::List, "items": inner })
      }
      None => serde_json::json!({ "widget": Widget::List }),
    },
    TdTypeEnum::TdSchemaType(schema) => {
      serde_json::json!({ "widget": Widget::Relation, "schema": schema.name(db) })
    }
    _ => serde_json::json!({ "widget": Widget::Text }),
  }
}
