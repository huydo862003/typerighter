use std::collections::{HashMap, HashSet};
use typedown_incremental::StableCompare;
use typedown_macros::query_derived;

use super::base::{TdObjectLike, TdObjectType, TdTypeLike, TdTypeType};
use super::func::TdFuncObj;
use super::{TdObjectEnum, TdTypeEnum};
use crate::db::TypedownDatabase;
use crate::db::types::{InstResult, LazyType};

// A union type: accepts any of its member types
#[query_derived]
pub struct TdSumType {
  pub members: HashSet<LazyType>,
}

impl TdObjectLike for TdSumType {
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

impl TdTypeLike for TdSumType {
  fn arity(&self, _db: &TypedownDatabase) -> usize {
    0
  }
  fn get_supertype(&self, db: &TypedownDatabase) -> TdTypeEnum {
    TdObjectType::get(db).into()
  }
  fn get_vtable(&self, _db: &TypedownDatabase) -> HashMap<String, TdFuncObj> {
    HashMap::new()
  }
  fn get_owned_field_type(&self, _db: &TypedownDatabase, _name: &str) -> Option<TdTypeEnum> {
    None
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
      TdTypeEnum::TdSumType(actual_sum) => {
        // Every member of actual must be accepted by some member of self
        actual_sum.members(db).iter().all(|actual_member| {
          actual_member
            .resolve(db)
            .is_some_and(|actual_type| self.accepts(db, &actual_type))
        })
      }
      _ => {
        // Actual must be accepted by at least one member
        self.members(db).iter().any(|member| {
          member
            .resolve(db)
            .is_some_and(|member_type| member_type.accepts(db, actual))
        })
      }
    }
  }
  fn construct(&self, _db: &TypedownDatabase, _args: Vec<TdObjectEnum>) -> Option<TdObjectEnum> {
    None
  }
  fn display_name(&self, db: &TypedownDatabase) -> String {
    let mut members: Vec<_> = self.members(db).into_iter().collect();
    members.sort_by(|a, b| a.stable_cmp(db, b));
    let parts: Vec<String> = members
      .iter()
      .filter_map(|m| m.resolve(db).map(|t| t.display_name(db)))
      .collect();
    parts.join(" | ")
  }
}
