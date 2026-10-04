//! Static type formatting utilities

use std::collections::BTreeMap;

use crate::db::TypedownDatabase;
use crate::db::types::LazyTyp;
use crate::db::types::derived::obj_system::TdStaticTyp;

// Format a field map as "{ name: type, ... }"
pub fn format_field_map(db: &TypedownDatabase, fields: &BTreeMap<String, LazyTyp>) -> String {
  if fields.is_empty() {
    return "{}".to_string();
  }
  let mut parts: Vec<String> = fields
    .iter()
    .filter_map(|(name, lazy)| {
      lazy
        .resolve(db)
        .map(|typ| format!("{}: {}", name, typ.display_name(db)))
    })
    .collect();
  parts.sort();
  format!("{{ {} }}", parts.join(", "))
}
