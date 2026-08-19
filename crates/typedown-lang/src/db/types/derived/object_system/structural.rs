use std::collections::HashMap;
use typedown_macros::query_derived;

use super::base::{TdObjectLike, TdObjectType, TdTypeLike, TdTypeType};
use super::func::TdFuncObj;
use super::{TdObjectEnum, TdTypeEnum};
use crate::db::TypedownDatabase;
use crate::db::types::{InstResult, LazyType};
use crate::db::utils::typecheck::is_nullable;
use typedown_incremental::Id;

// Anonymous structural type for typechecking, holds field name to type mappings
// This type never MATERIALIZES at runtime
#[query_derived]
pub struct TdStructuralType {
  pub fields: HashMap<String, LazyType>,
}

impl TdObjectLike for TdStructuralType {
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

impl TdTypeLike for TdStructuralType {
  fn arity(&self, _db: &TypedownDatabase) -> usize {
    0
  }
  fn get_supertype(&self, db: &TypedownDatabase) -> TdTypeEnum {
    TdObjectType::get(db).into()
  }
  fn get_vtable(&self, _db: &TypedownDatabase) -> HashMap<String, TdFuncObj> {
    HashMap::new()
  }
  fn get_owned_field_type(&self, db: &TypedownDatabase, name: &str) -> Option<TdTypeEnum> {
    self.fields(db).get(name)?.resolve(db)
  }
  fn instantiate(&self, db: &TypedownDatabase, args: Vec<LazyType>) -> InstResult {
    assert_eq!(args.len(), self.arity(db), "arity mismatch");
    InstResult::new(db, (*self).into(), vec![])
  }
  fn get_type_args(&self, _db: &TypedownDatabase) -> Vec<TdTypeEnum> {
    vec![]
  }
  fn accepts(&self, db: &TypedownDatabase, actual: &TdTypeEnum) -> bool {
    match actual {
      TdTypeEnum::TdNeverType(_) => true,
      TdTypeEnum::TdSumType(sum) => sum
        .members(db)
        .iter()
        .all(|m| m.resolve(db).is_some_and(|t| self.accepts(db, &t))),
      _ if self.as_id() == actual.as_id() => true,
      TdTypeEnum::TdProductType(product) => {
        fields_compatible(db, &self.fields(db), &product.fields(db))
      }
      TdTypeEnum::TdStructuralType(structural) => {
        fields_compatible(db, &self.fields(db), &structural.fields(db))
      }
      _ => false,
    }
  }
  fn construct(&self, _db: &TypedownDatabase, _args: Vec<TdObjectEnum>) -> Option<TdObjectEnum> {
    None
  }
  fn display_name(&self, db: &TypedownDatabase) -> String {
    let fields = self.fields(db);
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
        expected_type.accepts(db, &actual_type)
      }
      None => optional,
    }
  })
}
