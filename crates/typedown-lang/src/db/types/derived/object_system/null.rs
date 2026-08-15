use std::collections::HashMap;
use typedown_macros::query_derived;

use super::base::{TdObjectLike, TdObjectType, TdTypeLike, TdTypeType};
use super::func::TdFuncObj;
use super::{TdObjectEnum, TdTypeEnum};
use crate::db::TypedownDatabase;
use crate::db::derived::get_builtin_types::{get_null_obj, get_null_type};
use crate::db::types::{InstResult, LazyType};
use typedown_incremental::Id;

#[query_derived]
pub struct TdNullType {}

impl TdObjectLike for TdNullType {
  fn get_type(&self, db: &TypedownDatabase) -> TdTypeEnum {
    TdTypeType::get(db).into()
  }
  fn get_owned_field(&self, _db: &TypedownDatabase, _key: &str) -> Option<TdObjectEnum> {
    None
  }
  fn source_path(&self, _db: &TypedownDatabase) -> String {
    "@builtin::null".to_string()
  }
}

impl TdTypeLike for TdNullType {
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
      TdTypeEnum::TdSumType(sum) => sum
        .members(db)
        .iter()
        .all(|m| m.resolve(db).is_some_and(|t| self.accepts(db, &t))),
      _ => self.as_id() == actual.as_id(),
    }
  }
  fn construct(&self, db: &TypedownDatabase, _args: Vec<TdObjectEnum>) -> Option<TdObjectEnum> {
    Some(TdNullObj::get(db).into())
  }
  fn display_name(&self, _db: &TypedownDatabase) -> String {
    "null".to_string()
  }
}

impl TdNullType {
  pub fn get(db: &TypedownDatabase) -> TdNullType {
    get_null_type(db)
  }
}

#[query_derived]
pub struct TdNullObj {}

impl TdObjectLike for TdNullObj {
  fn get_type(&self, db: &TypedownDatabase) -> TdTypeEnum {
    TdNullType::get(db).into()
  }
  fn get_owned_field(&self, _db: &TypedownDatabase, _key: &str) -> Option<TdObjectEnum> {
    None
  }
  fn source_path(&self, db: &TypedownDatabase) -> String {
    self.get_type(db).source_path(db)
  }
  fn eq(&self, _db: &TypedownDatabase, other: &TdObjectEnum) -> bool {
    other.as_td_null_obj().is_some()
  }
}

impl TdNullObj {
  pub fn get(db: &TypedownDatabase) -> TdNullObj {
    get_null_obj(db)
  }
}
