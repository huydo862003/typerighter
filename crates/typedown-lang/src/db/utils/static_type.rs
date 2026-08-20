//! Static type formatting utilities

use std::collections::HashMap;

use crate::db::TypedownDatabase;
use crate::db::types::LazyType;
use crate::db::types::derived::object_system::TdStaticType;

// Format a field map as "{ name: type, ... }"
pub fn format_field_map(db: &TypedownDatabase, fields: &HashMap<String, LazyType>) -> String {
  if fields.is_empty() {
    return "{}".to_string();
  }
  let mut parts: Vec<String> = fields
    .iter()
    .filter_map(|(name, lazy)| {
      lazy
        .resolve(db)
        .map(|t| format!("{}: {}", name, t.display_name(db)))
    })
    .collect();
  parts.sort();
  format!("{{ {} }}", parts.join(", "))
}
