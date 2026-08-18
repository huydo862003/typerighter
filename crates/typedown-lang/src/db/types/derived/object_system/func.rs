use std::collections::HashMap;
use typedown_incremental::Id;
use typedown_macros::query_derived;

use super::base::{TdObjectLike, TdObjectType, TdTypeLike, TdTypeType};
use super::native_fn::{FnKind, NativeFnKind};
use super::str::TdStrType;
use super::{TdObjectEnum, TdTypeEnum};
use crate::db::TypedownDatabase;
use crate::db::derived::evaluate::evaluate_node::evaluate_node;
use crate::db::derived::get_builtin_types::get_func_type;
use crate::db::types::{FuncSignature, HirValueKind, InstResult, LazyType, RuntimeScope};

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
      FnKind::Native(NativeFnKind::FuncToString),
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
      TdTypeEnum::TdFuncType(actual_func) => {
        let expected_sig = self.signature(db);
        let actual_sig = actual_func.signature(db);
        let expected_params = expected_sig.params(db);
        let actual_params = actual_sig.params(db);
        // Arity must match
        if expected_params.len() != actual_params.len() {
          return false;
        }
        // Params are contravariant: actual param must accept expected param
        for (expected_param, actual_param) in expected_params.iter().zip(actual_params.iter()) {
          if !actual_param.accepts(db, expected_param) {
            return false;
          }
        }
        // Return type is covariant: expected return must accept actual return
        expected_sig.ret(db).accepts(db, &actual_sig.ret(db))
      }
      _ => self.as_id() == actual.as_id(),
    }
  }
  fn construct(&self, _db: &TypedownDatabase, _args: Vec<TdObjectEnum>) -> Option<TdObjectEnum> {
    None
  }
  fn display_name(&self, db: &TypedownDatabase) -> String {
    let sig = self.signature(db);
    let params: Vec<String> = sig.params(db).iter().map(|p| p.display_name(db)).collect();
    let ret = sig.ret(db).display_name(db);
    format!("fn({}) -> {}", params.join(", "), ret)
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
  pub func: FnKind,
}

impl TdFuncObj {
  pub fn call(
    &self,
    db: &TypedownDatabase,
    this: TdObjectEnum,
    args: Vec<TdObjectEnum>,
  ) -> Option<TdObjectEnum> {
    match self.func(db) {
      FnKind::Native(kind) => (kind.resolve())(db, this, args),
      FnKind::UserDefined(closure_hir, defining_scope) => {
        let HirValueKind::Closure { params, body } = closure_hir.kind(db) else {
          return None;
        };
        let bindings: Vec<(String, TdObjectEnum)> = params.into_iter().zip(args).collect();
        // Chain the defining scope as parent so nested closures can resolve outer params
        let runtime_scope = RuntimeScope::new(
          db,
          defining_scope.scope(db),
          bindings,
          Some(Box::new(defining_scope)),
        );
        evaluate_node(db, *body, runtime_scope).value(db)
      }
    }
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
