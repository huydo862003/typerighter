use std::collections::HashMap;
use typedown_incremental::Id;
use typedown_macros::query_derived;

use super::base::{TdObjectLike, TdObjectType, TdTypeLike, TdTypeType};
use super::native_fn::NativeFnKind;
use super::str::TdStrType;
use super::{TdObjectEnum, TdTypeEnum};
use crate::db::TypedownDatabase;
use crate::db::derived::get_builtin_types::get_func_type;
use crate::db::types::{FuncSignature, InstResult, LazyType, TypeMember};

#[query_derived]
pub struct TdFuncType {
  #[id]
  pub signature: FuncSignature,
}

impl TdObjectLike for TdFuncType {
  fn get_type(&self, db: &TypedownDatabase) -> TdTypeEnum {
    TdTypeType::get(db).into()
  }
  fn get_owned_field(&self, _db: &TypedownDatabase, _key: &str) -> Option<TdObjectEnum> {
    None
  }
  fn source_path(&self, db: &TypedownDatabase) -> String {
    let sig = self.signature(db);
    let params: Vec<String> = sig
      .params(db)
      .iter()
      .map(|param| param.source_path(db))
      .collect();
    let ret = sig.ret(db).source_path(db);
    format!("@builtin::function[({}) -> {}]", params.join(", "), ret)
  }
}

impl TdTypeLike for TdFuncType {
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
      (*self).into(),
      sig,
      NativeFnKind::FuncToString,
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
  fn construct(&self, _db: &TypedownDatabase, _args: Vec<TdObjectEnum>) -> Option<TdObjectEnum> {
    None
  }
  fn display_name(&self, _db: &TypedownDatabase) -> String {
    "function".to_string()
  }
}

impl TdFuncType {
  pub fn get(db: &TypedownDatabase, params: Vec<TdTypeEnum>, ret: TdTypeEnum) -> TdFuncType {
    get_func_type(db, FuncSignature::new(db, params, ret))
  }
}

#[query_derived]
pub struct TdFuncObj {
  #[id]
  pub name: String,
  #[id]
  pub typ: TdTypeEnum,
  #[id]
  pub signature: FuncSignature,
  pub func: NativeFnKind,
}

impl TdFuncObj {
  pub fn call(
    &self,
    db: &TypedownDatabase,
    this: TdObjectEnum,
    args: Vec<TdObjectEnum>,
  ) -> Option<TdObjectEnum> {
    (self.func(db).resolve())(db, this, args)
  }
}

impl TdObjectLike for TdFuncObj {
  fn get_type(&self, db: &TypedownDatabase) -> TdTypeEnum {
    get_func_type(db, self.signature(db)).into()
  }
  fn get_owned_field(&self, _db: &TypedownDatabase, _key: &str) -> Option<TdObjectEnum> {
    None
  }
  fn source_path(&self, db: &TypedownDatabase) -> String {
    self.get_type(db).source_path(db)
  }
}
