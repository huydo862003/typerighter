use std::collections::HashMap;
use typedown_macros::query_derived;

use super::base::{TdRuntimeObject, TdStaticType, TdTypeType};
use super::{TdObjectEnum, TdTypeEnum};
use crate::db::TypedownDatabase;
use crate::db::typecheck::utils::{is_nullable, is_subtype_of};
use crate::db::types::LazyType;
use crate::db::utils::static_type::format_field_map;

// Anonymous structural type for typechecking, holds field name to type mappings
// This type never MATERIALIZES at runtime
#[query_derived]
pub struct TdStructuralType {
  pub fields: HashMap<String, LazyType>,
}

impl TdStaticType for TdStructuralType {
  fn display_name(&self, db: &TypedownDatabase) -> String {
    format_field_map(db, &self.fields(db))
  }
  fn get_fields(&self, db: &TypedownDatabase) -> HashMap<String, LazyType> {
    self.fields(db)
  }
}

impl TdRuntimeObject for TdStructuralType {
  fn get_type(&self, db: &TypedownDatabase) -> TdTypeEnum {
    TdTypeType::get(db).into()
  }
  fn get_owned_field(&self, _db: &TypedownDatabase, _key: &str) -> Option<TdObjectEnum> {
    None
  }
  fn source_path(&self, db: &TypedownDatabase) -> String {
    self.display_name(db)
  }
}

// Check if expected fields are compatible with actual fields
pub fn fields_compatible(
  db: &TypedownDatabase,
  expected_fields: &HashMap<String, LazyType>,
  actual_fields: &HashMap<String, LazyType>,
) -> bool {
  expected_fields.iter().all(|(name, expected_lazy)| {
    let optional = expected_lazy
      .resolve(db)
      .is_some_and(|t| is_nullable(db, &t));
    match actual_fields.get(name) {
      Some(actual_lazy) => {
        let Some(expected_type) = expected_lazy.resolve(db) else {
          return false;
        };
        let Some(actual_type) = actual_lazy.resolve(db) else {
          return false;
        };
        is_subtype_of(db, &actual_type, &expected_type)
      }
      None => optional,
    }
  })
}
