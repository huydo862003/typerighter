// Tracked query to load and parse the component contract from .typedown/components.json

use typedown_macros::query_derived;

use crate::db::TypedownDatabase;
use crate::db::types::{
  CustomComponentContractResult, CustomComponentProp, CustomComponentTyp, File, FileHandle,
};
use typedown_incremental::QueryDatabase;

#[query_derived]
pub fn get_component_contract<'db>(
  db: &'db TypedownDatabase,
  file: File,
) -> CustomComponentContractResult<'db> {
  let content = read_handle_content(&file.handle(db)).unwrap_or_default();
  CustomComponentContractResult::new(db, parse_contract_entries(&content))
}

fn read_handle_content(handle: &FileHandle) -> Option<String> {
  match handle {
    // Re-reads from disk on each call, but get_component_contract is memoized on the File input (which includes mtime), so this only runs when the contract file changes
    FileHandle::Path(path, _) => std::fs::read_to_string(path).ok(),
    FileHandle::Content(_, content, _) => Some(content.clone()),
  }
}

fn parse_contract_entries(json: &str) -> Vec<CustomComponentTyp> {
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
          .filter_map(|(prop_name, prop)| {
            let type_name = prop.get("type")?.as_str()?.to_string();
            let required = prop
              .get("required")
              .and_then(|required| required.as_bool())
              .unwrap_or(false);
            Some(CustomComponentProp {
              name: prop_name.clone(),
              type_name,
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

  #[test]
  fn parse_empty_json() {
    assert!(parse_contract_entries("{}").is_empty());
  }

  #[test]
  fn parse_invalid_json() {
    assert!(parse_contract_entries("not json").is_empty());
  }

  #[test]
  fn parse_component_with_props() {
    let json = r#"{"Chart":{"source":"./Chart.vue","props":{"data":{"type":"list[number]","required":true},"title":{"type":"string","required":false}}}}"#;
    let entries = parse_contract_entries(json);
    assert_eq!(entries.len(), 1);
    let entry = &entries[0];
    assert_eq!(entry.name, "Chart");
    assert_eq!(entry.source, "./Chart.vue");
    let props = entry.props.as_ref().unwrap();
    assert_eq!(props.len(), 2);
    let data_prop = props.iter().find(|p| p.name == "data").unwrap();
    assert_eq!(data_prop.type_name, "list[number]");
    assert!(data_prop.required);
    let title_prop = props.iter().find(|p| p.name == "title").unwrap();
    assert!(!title_prop.required);
  }

  #[test]
  fn parse_component_no_props() {
    let json = r#"{"Toc":{"source":"./Toc.vue","props":{}}}"#;
    let entries = parse_contract_entries(json);
    assert_eq!(entries.len(), 1);
    assert!(entries[0].props.as_ref().unwrap().is_empty());
  }

  #[test]
  fn parse_skips_missing_source() {
    let json = r#"{"Bad":{"props":{}}}"#;
    assert!(parse_contract_entries(json).is_empty());
  }

  #[test]
  fn parse_component_without_props_key() {
    let json = r#"{"Badge":{"source":"./Badge.vue"}}"#;
    let entries = parse_contract_entries(json);
    assert_eq!(entries.len(), 1);
    assert!(entries[0].props.is_none());
  }
}
