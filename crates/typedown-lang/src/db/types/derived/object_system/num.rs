use std::collections::HashMap;
use typedown_macros::query_derived;

use super::base::{TdObjectLike, TdObjectType, TdTypeLike, TdTypeType};
use super::func::TdFuncObj;
use super::native_fn::NativeFnKind;
use super::str::TdStrType;
use super::{TdObjectEnum, TdTypeEnum};
use crate::db::TypedownDatabase;
use crate::db::derived::get_builtin_types::get_num_type;
use crate::db::types::{FuncSignature, InstResult, LazyType};
use typedown_incremental::Id;

#[query_derived]
pub struct TdNumType {}

impl TdObjectLike for TdNumType {
  fn get_type(&self, db: &TypedownDatabase) -> TdTypeEnum {
    TdTypeType::get(db).into()
  }
  fn get_owned_field(&self, _db: &TypedownDatabase, _key: &str) -> Option<TdObjectEnum> {
    None
  }
  fn source_path(&self, _db: &TypedownDatabase) -> String {
    "@builtin::number".to_string()
  }
}

impl TdTypeLike for TdNumType {
  fn arity(&self, _db: &TypedownDatabase) -> usize {
    0
  }
  fn get_supertype(&self, db: &TypedownDatabase) -> TdTypeEnum {
    TdObjectType::get(db).into()
  }
  fn get_vtable(&self, db: &TypedownDatabase) -> HashMap<String, TdFuncObj> {
    let sig = FuncSignature::new(db, vec![], TdStrType::get(db).into());
    let func_obj = TdFuncObj::new(
      db,
      "to_string".to_string(),
      TdNumType::get(db).into(),
      sig,
      NativeFnKind::NumToString,
    );
    HashMap::from([("to_string".to_string(), func_obj)])
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
      TdTypeEnum::TdNumType(_) => true,
      TdTypeEnum::TdLiteralType(lit) => lit.underlying_type(db).as_id() == self.as_id(),
      _ => false,
    }
  }
  fn construct(&self, _db: &TypedownDatabase, args: Vec<TdObjectEnum>) -> Option<TdObjectEnum> {
    let arg = args.into_iter().next()?;
    arg.as_td_num_obj()?;
    Some(arg)
  }
  fn display_name(&self, _db: &TypedownDatabase) -> String {
    "number".to_string()
  }
}

impl TdNumType {
  pub fn get(db: &TypedownDatabase) -> TdNumType {
    get_num_type(db)
  }
}

#[query_derived]
pub struct TdNumObj {
  pub value: f64,
}

impl TdObjectLike for TdNumObj {
  fn get_type(&self, db: &TypedownDatabase) -> TdTypeEnum {
    TdNumType::get(db).into()
  }
  fn get_owned_field(&self, _db: &TypedownDatabase, _key: &str) -> Option<TdObjectEnum> {
    None
  }
  fn source_path(&self, db: &TypedownDatabase) -> String {
    self.get_type(db).source_path(db)
  }
  fn eq(&self, db: &TypedownDatabase, other: &TdObjectEnum) -> bool {
    if let TdObjectEnum::TdNumObj(other) = other {
      self.value(db) == other.value(db)
    } else {
      self.as_id() == other.as_id()
    }
  }
  fn lt(&self, db: &TypedownDatabase, other: &TdObjectEnum) -> bool {
    if let TdObjectEnum::TdNumObj(other) = other {
      self.value(db) < other.value(db)
    } else {
      self.as_id() < other.as_id()
    }
  }
  fn gt(&self, db: &TypedownDatabase, other: &TdObjectEnum) -> bool {
    if let TdObjectEnum::TdNumObj(other) = other {
      self.value(db) > other.value(db)
    } else {
      self.as_id() > other.as_id()
    }
  }
  fn le(&self, db: &TypedownDatabase, other: &TdObjectEnum) -> bool {
    if let TdObjectEnum::TdNumObj(other) = other {
      self.value(db) <= other.value(db)
    } else {
      self.as_id() <= other.as_id()
    }
  }
  fn ge(&self, db: &TypedownDatabase, other: &TdObjectEnum) -> bool {
    if let TdObjectEnum::TdNumObj(other) = other {
      self.value(db) >= other.value(db)
    } else {
      self.as_id() >= other.as_id()
    }
  }
}
