use std::collections::HashMap;
use typedown_incremental::Id;
use typedown_macros::query_derived;

use super::base::{TdObjectLike, TdObjectType, TdTypeLike, TdTypeType};
use super::func::TdFuncObj;
use super::native_fn::NativeFnKind;
use super::str::TdStrType;
use super::{TdObjectEnum, TdTypeEnum};
use crate::db::TypedownDatabase;
use crate::db::derived::get_builtin_types::get_math_type;
use crate::db::types::{FuncSignature, InstResult, LazyType};

#[query_derived]
pub struct TdMathType {}

impl TdObjectLike for TdMathType {
  fn get_type(&self, db: &TypedownDatabase) -> TdTypeEnum {
    TdTypeType::get(db).into()
  }
  fn get_owned_field(&self, _db: &TypedownDatabase, _key: &str) -> Option<TdObjectEnum> {
    None
  }
  fn source_path(&self, _db: &TypedownDatabase) -> String {
    "@builtin::math".to_string()
  }
}

impl TdTypeLike for TdMathType {
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
      TdMathType::get(db).into(),
      sig,
      NativeFnKind::MathToString,
      None,
      None,
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
      _ => self.as_id() == actual.as_id(),
    }
  }
  fn construct(&self, _db: &TypedownDatabase, args: Vec<TdObjectEnum>) -> Option<TdObjectEnum> {
    let arg = args.into_iter().next()?;
    arg.as_td_math_obj()?;
    Some(arg)
  }
  fn display_name(&self, _db: &TypedownDatabase) -> String {
    "math".to_string()
  }
}

impl TdMathType {
  pub fn get(db: &TypedownDatabase) -> TdMathType {
    get_math_type(db)
  }
}

#[query_derived]
pub struct TdMathObj {
  pub value: String,
}

impl TdObjectLike for TdMathObj {
  fn get_type(&self, db: &TypedownDatabase) -> TdTypeEnum {
    TdMathType::get(db).into()
  }
  fn get_owned_field(&self, _db: &TypedownDatabase, _key: &str) -> Option<TdObjectEnum> {
    None
  }
  fn source_path(&self, db: &TypedownDatabase) -> String {
    self.get_type(db).source_path(db)
  }
}
