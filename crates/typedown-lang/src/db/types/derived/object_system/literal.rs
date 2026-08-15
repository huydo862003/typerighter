use std::collections::HashMap;
use typedown_macros::query_derived;

use super::base::{TdObjectLike, TdTypeLike, TdTypeType};
use super::bool::TdBoolObj;
use super::func::TdFuncObj;
use super::num::TdNumObj;
use super::str::TdStrObj;
use super::{TdObjectEnum, TdTypeEnum};
use crate::db::TypedownDatabase;
use crate::db::derived::get_builtin_types::{get_bool_type, get_num_type, get_str_type};
use crate::db::types::{InstResult, LazyType, LiteralValue};

#[query_derived]
pub struct TdLiteralType {
  pub value: LiteralValue,
}

impl TdObjectLike for TdLiteralType {
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

impl TdTypeLike for TdLiteralType {
  fn arity(&self, _db: &TypedownDatabase) -> usize {
    0
  }
  // Literal types are subtypes of their base type
  fn get_supertype(&self, db: &TypedownDatabase) -> TdTypeEnum {
    match self.value(db) {
      LiteralValue::Str(_) => get_str_type(db).into(),
      LiteralValue::Num(_) => get_num_type(db).into(),
      LiteralValue::Bool(_) => get_bool_type(db).into(),
    }
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
      TdTypeEnum::TdLiteralType(other) => self.value(db) == other.value(db),
      _ => false,
    }
  }
  fn construct(&self, db: &TypedownDatabase, _args: Vec<TdObjectEnum>) -> Option<TdObjectEnum> {
    match self.value(db) {
      LiteralValue::Str(s) => Some(TdStrObj::new(db, s).into()),
      LiteralValue::Num(n) => {
        let num: f64 = n.parse().unwrap_or(0.0);
        Some(TdNumObj::new(db, num).into())
      }
      LiteralValue::Bool(b) => Some(TdBoolObj::new(db, b).into()),
    }
  }
  fn display_name(&self, db: &TypedownDatabase) -> String {
    match self.value(db) {
      LiteralValue::Str(s) => format!("\"{}\"", s),
      LiteralValue::Num(n) => n,
      LiteralValue::Bool(b) => b.to_string(),
    }
  }
}

impl TdLiteralType {
  pub fn underlying_type(&self, db: &TypedownDatabase) -> TdTypeEnum {
    match self.value(db) {
      LiteralValue::Str(_) => get_str_type(db).into(),
      LiteralValue::Num(_) => get_num_type(db).into(),
      LiteralValue::Bool(_) => get_bool_type(db).into(),
    }
  }
}
