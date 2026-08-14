mod utils;

use std::collections::HashMap;
use typedown_macros::query_derived;

use super::base::{TdObjectLike, TdTypeLike, TdTypeType};
use super::func::TdFuncObj;
use super::native_fn::NativeFnKind;
use super::str::TdStrType;
use super::{TdObjectEnum, TdTypeEnum};
use crate::db::TypedownDatabase;
use crate::db::derived::get_builtin_types::{get_date_type, get_datetime_type, get_time_type};
use crate::db::types::{FuncSignature, InstResult, LazyType, TypeMember};
use typedown_incremental::Id;
pub(crate) use utils::{is_valid_iso_date, is_valid_iso_datetime, is_valid_iso_time};

// DateTime

#[query_derived]
pub struct TdDateTimeType {}

impl TdObjectLike for TdDateTimeType {
  fn get_type(&self, db: &TypedownDatabase) -> TdTypeEnum {
    TdTypeType::get(db).into()
  }
  fn get_owned_field(&self, _db: &TypedownDatabase, _key: &str) -> Option<TdObjectEnum> {
    None
  }
  fn source_path(&self, _db: &TypedownDatabase) -> String {
    "@builtin::datetime".to_string()
  }
}

impl TdTypeLike for TdDateTimeType {
  fn arity(&self, _db: &TypedownDatabase) -> usize {
    0
  }
  fn get_supertype(&self, db: &TypedownDatabase) -> TdTypeEnum {
    TdStrType::get(db).into()
  }
  fn get_vtable(&self, db: &TypedownDatabase) -> HashMap<String, TdFuncObj> {
    let sig = FuncSignature::new(db, vec![], TdStrType::get(db).into());
    let func_obj = TdFuncObj::new(
      db,
      "to_string".to_string(),
      TdDateTimeType::get(db).into(),
      sig,
      NativeFnKind::DateTimeToString,
    );
    HashMap::from([("to_string".to_string(), func_obj)])
  }
  fn get_owned_field_type_member(&self, _db: &TypedownDatabase, _name: &str) -> Option<TypeMember> {
    None
  }
  fn instantiate(&self, db: &TypedownDatabase, args: Vec<LazyType>) -> InstResult {
    assert_eq!(args.len(), self.arity(db), "arity mismatch");
    InstResult::new(db, (*self).into(), vec![])
  }
  fn get_type_args(&self, _db: &TypedownDatabase) -> Vec<TdTypeEnum> {
    vec![]
  }
  fn is_compatible_with(&self, _db: &TypedownDatabase, actual: &TdTypeEnum) -> bool {
    self.as_id() == actual.as_id()
  }
  fn construct(&self, db: &TypedownDatabase, args: Vec<TdObjectEnum>) -> Option<TdObjectEnum> {
    let arg = args.into_iter().next()?;
    let str_obj = arg.as_td_str_obj()?;
    let val = str_obj.value(db);
    if is_valid_iso_datetime(&val) {
      return Some(TdDateTimeObj::new(db, val).into());
    }
    None
  }
  fn display_name(&self, _db: &TypedownDatabase) -> String {
    "datetime".to_string()
  }
}

impl TdDateTimeType {
  pub fn get(db: &TypedownDatabase) -> TdDateTimeType {
    get_datetime_type(db)
  }
}

#[query_derived]
pub struct TdDateTimeObj {
  pub value: String,
}

impl TdObjectLike for TdDateTimeObj {
  fn get_type(&self, db: &TypedownDatabase) -> TdTypeEnum {
    TdDateTimeType::get(db).into()
  }
  fn get_owned_field(&self, _db: &TypedownDatabase, _key: &str) -> Option<TdObjectEnum> {
    None
  }
  fn source_path(&self, db: &TypedownDatabase) -> String {
    self.get_type(db).source_path(db)
  }
  fn eq(&self, db: &TypedownDatabase, other: &TdObjectEnum) -> bool {
    if let TdObjectEnum::TdDateTimeObj(other) = other {
      self.value(db) == other.value(db)
    } else {
      self.as_id() == other.as_id()
    }
  }
  fn lt(&self, db: &TypedownDatabase, other: &TdObjectEnum) -> bool {
    if let TdObjectEnum::TdDateTimeObj(other) = other {
      self.value(db) < other.value(db)
    } else {
      self.as_id() < other.as_id()
    }
  }
  fn gt(&self, db: &TypedownDatabase, other: &TdObjectEnum) -> bool {
    if let TdObjectEnum::TdDateTimeObj(other) = other {
      self.value(db) > other.value(db)
    } else {
      self.as_id() > other.as_id()
    }
  }
  fn le(&self, db: &TypedownDatabase, other: &TdObjectEnum) -> bool {
    if let TdObjectEnum::TdDateTimeObj(other) = other {
      self.value(db) <= other.value(db)
    } else {
      self.as_id() <= other.as_id()
    }
  }
  fn ge(&self, db: &TypedownDatabase, other: &TdObjectEnum) -> bool {
    if let TdObjectEnum::TdDateTimeObj(other) = other {
      self.value(db) >= other.value(db)
    } else {
      self.as_id() >= other.as_id()
    }
  }
}

// Date

#[query_derived]
pub struct TdDateType {}

impl TdObjectLike for TdDateType {
  fn get_type(&self, db: &TypedownDatabase) -> TdTypeEnum {
    TdTypeType::get(db).into()
  }
  fn get_owned_field(&self, _db: &TypedownDatabase, _key: &str) -> Option<TdObjectEnum> {
    None
  }
  fn source_path(&self, _db: &TypedownDatabase) -> String {
    "@builtin::date".to_string()
  }
}

impl TdTypeLike for TdDateType {
  fn arity(&self, _db: &TypedownDatabase) -> usize {
    0
  }
  fn get_supertype(&self, db: &TypedownDatabase) -> TdTypeEnum {
    TdStrType::get(db).into()
  }
  fn get_vtable(&self, db: &TypedownDatabase) -> HashMap<String, TdFuncObj> {
    let sig = FuncSignature::new(db, vec![], TdStrType::get(db).into());
    let func_obj = TdFuncObj::new(
      db,
      "to_string".to_string(),
      TdDateType::get(db).into(),
      sig,
      NativeFnKind::DateToString,
    );
    HashMap::from([("to_string".to_string(), func_obj)])
  }
  fn get_owned_field_type_member(&self, _db: &TypedownDatabase, _name: &str) -> Option<TypeMember> {
    None
  }
  fn instantiate(&self, db: &TypedownDatabase, args: Vec<LazyType>) -> InstResult {
    assert_eq!(args.len(), self.arity(db), "arity mismatch");
    InstResult::new(db, (*self).into(), vec![])
  }
  fn get_type_args(&self, _db: &TypedownDatabase) -> Vec<TdTypeEnum> {
    vec![]
  }
  fn is_compatible_with(&self, _db: &TypedownDatabase, actual: &TdTypeEnum) -> bool {
    self.as_id() == actual.as_id()
  }
  fn construct(&self, db: &TypedownDatabase, args: Vec<TdObjectEnum>) -> Option<TdObjectEnum> {
    let arg = args.into_iter().next()?;
    let str_obj = arg.as_td_str_obj()?;
    let val = str_obj.value(db);
    if is_valid_iso_date(&val) {
      return Some(TdDateObj::new(db, val).into());
    }
    None
  }
  fn display_name(&self, _db: &TypedownDatabase) -> String {
    "date".to_string()
  }
}

impl TdDateType {
  pub fn get(db: &TypedownDatabase) -> TdDateType {
    get_date_type(db)
  }
}

#[query_derived]
pub struct TdDateObj {
  pub value: String,
}

impl TdObjectLike for TdDateObj {
  fn get_type(&self, db: &TypedownDatabase) -> TdTypeEnum {
    TdDateType::get(db).into()
  }
  fn get_owned_field(&self, _db: &TypedownDatabase, _key: &str) -> Option<TdObjectEnum> {
    None
  }
  fn source_path(&self, db: &TypedownDatabase) -> String {
    self.get_type(db).source_path(db)
  }
  fn eq(&self, db: &TypedownDatabase, other: &TdObjectEnum) -> bool {
    if let TdObjectEnum::TdDateObj(other) = other {
      self.value(db) == other.value(db)
    } else {
      self.as_id() == other.as_id()
    }
  }
  fn lt(&self, db: &TypedownDatabase, other: &TdObjectEnum) -> bool {
    if let TdObjectEnum::TdDateObj(other) = other {
      self.value(db) < other.value(db)
    } else {
      self.as_id() < other.as_id()
    }
  }
  fn gt(&self, db: &TypedownDatabase, other: &TdObjectEnum) -> bool {
    if let TdObjectEnum::TdDateObj(other) = other {
      self.value(db) > other.value(db)
    } else {
      self.as_id() > other.as_id()
    }
  }
  fn le(&self, db: &TypedownDatabase, other: &TdObjectEnum) -> bool {
    if let TdObjectEnum::TdDateObj(other) = other {
      self.value(db) <= other.value(db)
    } else {
      self.as_id() <= other.as_id()
    }
  }
  fn ge(&self, db: &TypedownDatabase, other: &TdObjectEnum) -> bool {
    if let TdObjectEnum::TdDateObj(other) = other {
      self.value(db) >= other.value(db)
    } else {
      self.as_id() >= other.as_id()
    }
  }
}

// Time

#[query_derived]
pub struct TdTimeType {}

impl TdObjectLike for TdTimeType {
  fn get_type(&self, db: &TypedownDatabase) -> TdTypeEnum {
    TdTypeType::get(db).into()
  }
  fn get_owned_field(&self, _db: &TypedownDatabase, _key: &str) -> Option<TdObjectEnum> {
    None
  }
  fn source_path(&self, _db: &TypedownDatabase) -> String {
    "@builtin::time".to_string()
  }
}

impl TdTypeLike for TdTimeType {
  fn arity(&self, _db: &TypedownDatabase) -> usize {
    0
  }
  fn get_supertype(&self, db: &TypedownDatabase) -> TdTypeEnum {
    TdStrType::get(db).into()
  }
  fn get_vtable(&self, db: &TypedownDatabase) -> HashMap<String, TdFuncObj> {
    let sig = FuncSignature::new(db, vec![], TdStrType::get(db).into());
    let func_obj = TdFuncObj::new(
      db,
      "to_string".to_string(),
      TdTimeType::get(db).into(),
      sig,
      NativeFnKind::TimeToString,
    );
    HashMap::from([("to_string".to_string(), func_obj)])
  }
  fn get_owned_field_type_member(&self, _db: &TypedownDatabase, _name: &str) -> Option<TypeMember> {
    None
  }
  fn instantiate(&self, db: &TypedownDatabase, args: Vec<LazyType>) -> InstResult {
    assert_eq!(args.len(), self.arity(db), "arity mismatch");
    InstResult::new(db, (*self).into(), vec![])
  }
  fn get_type_args(&self, _db: &TypedownDatabase) -> Vec<TdTypeEnum> {
    vec![]
  }
  fn is_compatible_with(&self, _db: &TypedownDatabase, actual: &TdTypeEnum) -> bool {
    self.as_id() == actual.as_id()
  }
  fn construct(&self, db: &TypedownDatabase, args: Vec<TdObjectEnum>) -> Option<TdObjectEnum> {
    let arg = args.into_iter().next()?;
    let str_obj = arg.as_td_str_obj()?;
    let val = str_obj.value(db);
    if is_valid_iso_time(&val) {
      return Some(TdTimeObj::new(db, val).into());
    }
    None
  }
  fn display_name(&self, _db: &TypedownDatabase) -> String {
    "time".to_string()
  }
}

impl TdTimeType {
  pub fn get(db: &TypedownDatabase) -> TdTimeType {
    get_time_type(db)
  }
}

#[query_derived]
pub struct TdTimeObj {
  pub value: String,
}

impl TdObjectLike for TdTimeObj {
  fn get_type(&self, db: &TypedownDatabase) -> TdTypeEnum {
    TdTimeType::get(db).into()
  }
  fn get_owned_field(&self, _db: &TypedownDatabase, _key: &str) -> Option<TdObjectEnum> {
    None
  }
  fn source_path(&self, db: &TypedownDatabase) -> String {
    self.get_type(db).source_path(db)
  }
  fn eq(&self, db: &TypedownDatabase, other: &TdObjectEnum) -> bool {
    if let TdObjectEnum::TdTimeObj(other) = other {
      self.value(db) == other.value(db)
    } else {
      self.as_id() == other.as_id()
    }
  }
  fn lt(&self, db: &TypedownDatabase, other: &TdObjectEnum) -> bool {
    if let TdObjectEnum::TdTimeObj(other) = other {
      self.value(db) < other.value(db)
    } else {
      self.as_id() < other.as_id()
    }
  }
  fn gt(&self, db: &TypedownDatabase, other: &TdObjectEnum) -> bool {
    if let TdObjectEnum::TdTimeObj(other) = other {
      self.value(db) > other.value(db)
    } else {
      self.as_id() > other.as_id()
    }
  }
  fn le(&self, db: &TypedownDatabase, other: &TdObjectEnum) -> bool {
    if let TdObjectEnum::TdTimeObj(other) = other {
      self.value(db) <= other.value(db)
    } else {
      self.as_id() <= other.as_id()
    }
  }
  fn ge(&self, db: &TypedownDatabase, other: &TdObjectEnum) -> bool {
    if let TdObjectEnum::TdTimeObj(other) = other {
      self.value(db) >= other.value(db)
    } else {
      self.as_id() >= other.as_id()
    }
  }
}
