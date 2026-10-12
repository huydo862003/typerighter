//! Export schema property descriptors for the client

use super::json;
use crate::db::TypedownDatabase;
use crate::db::derived::evaluate::evaluate_typ::evaluate_typ;
use crate::db::derived::name_resolver::file_symbol::file_symbol;
use crate::db::types::{File, LazyTyp, LitValue, Project, TdTypEnum};

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
pub fn export_prop_descriptors(
  db: &TypedownDatabase,
  project: Project,
  file: File,
) -> Option<serde_json::Value> {
  let file_sym = file_symbol(db, project, file);
  let symbol = file_sym.value(db)?;
  let eval_result = evaluate_typ(db, symbol);
  let typ = eval_result.typ(db)?;

  let schema = typ.as_td_schema_typ()?;
  let fields = schema.fields(db);

  let mut props = serde_json::Map::new();

  for (name, prop_desc) in fields {
    let mut prop_json = export_lazy_to_descriptor(db, &prop_desc.field_typ);
    if let Some(ref definition_obj) = prop_desc.default_value
      && let Ok(definition_json) = json::serialize_to_json(db, project, definition_obj)
      && let serde_json::Value::Object(ref mut map) = prop_json
    {
      map.insert("default".to_string(), definition_json);
    }
    props.insert(name, prop_json);
  }

  Some(serde_json::Value::Object(props))
}

// Map a LazyType to a property descriptor with a widget hint
fn export_lazy_to_descriptor(db: &TypedownDatabase, lazy: &LazyTyp) -> serde_json::Value {
  let Some(typ) = lazy.resolve(db) else {
    return serde_json::json!({ "type": "string" });
  };

  // Sum of string literals is a select; filter out TdNullType for nullable T? types
  if let Some(sum) = typ.as_td_sum_typ() {
    let members = sum.members(db);
    let non_null_members: Vec<LazyTyp> = members
      .iter()
      .filter(|m| {
        if let Some(m_typ) = m.resolve(db) {
          !m_typ.is_td_null_typ()
        } else {
          true
        }
      })
      .cloned()
      .collect();

    if non_null_members.len() == 1 {
      return export_lazy_to_descriptor(db, &non_null_members[0]);
    }

    let mut lits: Vec<String> = non_null_members
      .iter()
      .filter_map(|m| {
        if let Some(TdTypEnum::TdLitTyp(lit)) = m.resolve(db)
          && let LitValue::String(s) = lit.value(db)
        {
          Some(s)
        } else {
          None
        }
      })
      .collect();
    lits.sort();

    if !lits.is_empty() && lits.len() == non_null_members.len() {
      return serde_json::json!({ "widget": Widget::Select, "options": lits });
    }
    return serde_json::json!({ "widget": Widget::Text });
  }

  // List type: check if elem is a sum of string literals (multi_select)
  if let Some(list) = typ.as_td_list_typ() {
    if let Some(element_lazy) = list.element(db)
      && let Some(element_typ) = element_lazy.resolve(db)
    {
      if let Some(sum) = element_typ.as_td_sum_typ() {
        let members = sum.members(db);
        let mut lits: Vec<String> = members
          .iter()
          .filter_map(|m| {
            if let Some(TdTypEnum::TdLitTyp(lit)) = m.resolve(db)
              && let LitValue::String(s) = lit.value(db)
            {
              Some(s)
            } else {
              None
            }
          })
          .collect();
        lits.sort();

        if lits.len() == members.len() && !lits.is_empty() {
          return serde_json::json!({ "widget": Widget::MultiSelect, "options": lits });
        }
        if members.len() == 1 {
          let first_member = members.iter().next().unwrap();
          let inner = export_lazy_to_descriptor(db, first_member);
          return serde_json::json!({ "widget": Widget::List, "items": inner });
        }
      } else {
        let inner = export_lazy_to_descriptor(db, &element_lazy);
        return serde_json::json!({ "widget": Widget::List, "items": inner });
      }
    }
    return serde_json::json!({ "widget": Widget::Text });
  }

  export_simple_typ_to_descriptor(db, &typ)
}

fn export_simple_typ_to_descriptor(db: &TypedownDatabase, typ: &TdTypEnum) -> serde_json::Value {
  match typ {
    TdTypEnum::TdStringTyp(_) => serde_json::json!({ "widget": Widget::Text }),
    TdTypEnum::TdNumberTyp(_) => serde_json::json!({ "widget": Widget::Number }),
    TdTypEnum::TdBoolTyp(_) => serde_json::json!({ "widget": Widget::Checkbox }),
    TdTypEnum::TdDateTyp(_) => serde_json::json!({ "widget": Widget::Date }),
    TdTypEnum::TdDateTimeTyp(_) => serde_json::json!({ "widget": Widget::Date }),
    TdTypEnum::TdTimeTyp(_) => serde_json::json!({ "widget": Widget::Text }),
    TdTypEnum::TdListTyp(list) => match list.element(db).and_then(|element| element.resolve(db)) {
      Some(element) => {
        let inner = export_simple_typ_to_descriptor(db, &element);
        serde_json::json!({ "widget": Widget::List, "items": inner })
      }
      None => serde_json::json!({ "widget": Widget::List }),
    },
    TdTypEnum::TdSchemaTyp(schema) => {
      serde_json::json!({ "widget": Widget::Relation, "schema": schema.name(db) })
    }
    _ => serde_json::json!({ "widget": Widget::Text }),
  }
}
