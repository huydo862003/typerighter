// Tracked query to load and parse the component contract from .typedown/components.json

use typedown_macros::query_derived;

use crate::db::TypedownDatabase;
use crate::db::types::derived::obj_system::TdStaticTyp;
use crate::db::types::{
  CustomComponentContractResult, CustomComponentProp, CustomComponentTyp, File, FileHandle,
};
use crate::integrations::json_schema::{is_optional, parse_json_schema, unwrap_optional};
use typedown_incremental::QueryDatabase;

#[query_derived]
pub fn get_component_contract<'db>(
  db: &'db TypedownDatabase,
  file: File,
) -> CustomComponentContractResult<'db> {
  let content = read_handle_content(&file.handle(db)).unwrap_or_default();
  CustomComponentContractResult::new(db, parse_contract_entries(db, &content))
}

// Read raw JSON from disk or editor buffer
fn read_handle_content(handle: &FileHandle) -> Option<String> {
  match handle {
    // Re-reads from disk on each call, but get_component_contract is memoized on the File input (which includes mtime), so this only runs when the contract file changes
    FileHandle::Path(path, _) => std::fs::read_to_string(path).ok(),
    FileHandle::Content(_, content, _) => Some(content.clone()),
  }
}

// Deserialize contract JSON, parse each prop's JSON Schema into a td type
fn parse_contract_entries(db: &TypedownDatabase, json: &str) -> Vec<CustomComponentTyp> {
  let Ok(map) = serde_json::from_str::<serde_json::Value>(json) else {
    return vec![];
  };
  let Some(obj) = map.as_object() else {
    return vec![];
  };

  obj
    .iter()
    .filter_map(|(name, entry)| {
      let source = entry.get("source")?.as_str()?.to_string();
      let props_obj = entry.get("props").and_then(|p| p.as_object());

      // None means props key is absent: skip prop validation for this component
      let props = props_obj.map(|prop| {
        prop
          .iter()
          .filter_map(|(prop_name, schema)| {
            // Parse JSON Schema -> td type, derive required from optionality
            let typ = parse_json_schema(db, schema)?;
            let required = !is_optional(db, &typ);
            let base = unwrap_optional(db, typ);
            Some(CustomComponentProp {
              name: prop_name.clone(),
              typ_name: base.display_name(db),
              required,
            })
          })
          .collect()
      });

      Some(CustomComponentTyp {
        name: name.clone(),
        source,
        props,
      })
    })
    .collect()
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::db::QueryStorage;

  fn make_db() -> TypedownDatabase {
    TypedownDatabase {
      storage: QueryStorage::default(),
    }
  }

  #[test]
  fn parse_empty_json() {
    let db = make_db();
    assert!(parse_contract_entries(&db, "{}").is_empty());
  }

  #[test]
  fn parse_invalid_json() {
    let db = make_db();
    assert!(parse_contract_entries(&db, "not json").is_empty());
  }

  #[test]
  fn parse_component_with_props() {
    let db = make_db();
    let json = r#"{"Chart":{"source":"./Chart.vue","props":{"data":{"type":"array","items":{"type":"number"}},"title":{"type":["string","null"]}}}}"#;
    let entries = parse_contract_entries(&db, json);
    assert_eq!(entries.len(), 1);
    let entry = &entries[0];
    assert_eq!(entry.name, "Chart");
    assert_eq!(entry.source, "./Chart.vue");
    let props = entry.props.as_ref().unwrap();
    assert_eq!(props.len(), 2);
    let data_prop = props.iter().find(|p| p.name == "data").unwrap();
    assert!(data_prop.required);
    let title_prop = props.iter().find(|p| p.name == "title").unwrap();
    assert!(!title_prop.required);
  }

  #[test]
  fn parse_component_no_props() {
    let db = make_db();
    let json = r#"{"Toc":{"source":"./Toc.vue","props":{}}}"#;
    let entries = parse_contract_entries(&db, json);
    assert_eq!(entries.len(), 1);
    assert!(entries[0].props.as_ref().unwrap().is_empty());
  }

  #[test]
  fn parse_skips_missing_source() {
    let db = make_db();
    assert!(parse_contract_entries(&db, r#"{"Bad":{"props":{}}}"#).is_empty());
  }

  #[test]
  fn parse_component_without_props_key() {
    let db = make_db();
    let json = r#"{"Badge":{"source":"./Badge.vue"}}"#;
    let entries = parse_contract_entries(&db, json);
    assert_eq!(entries.len(), 1);
    assert!(entries[0].props.is_none());
  }
}
