use typedown_macros::query_derived;

use super::base::{TdRuntimeObject, TdStaticType, TdTypeType};
use super::native_fn::FnKind;
use super::{TdObjectEnum, TdTypeEnum};
use crate::db::TypedownDatabase;
use crate::db::derived::evaluate::evaluate_node::evaluate_node;
use crate::db::derived::get_builtin_types::get_func_type;
use crate::db::types::{FuncSignature, HirValueKind, Project, RuntimeScope};
use crate::syntax::diagnostic::Diagnostic;

#[query_derived]
pub struct TdFuncType<'db> {
  #[id]
  pub signature: FuncSignature<'db>,
}

impl TdRuntimeObject for TdFuncType<'_> {
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

impl TdStaticType for TdFuncType<'_> {
  fn display_name(&self, db: &TypedownDatabase) -> String {
    let sig = self.signature(db);
    let params: Vec<String> = sig.params(db).iter().map(|p| p.display_name(db)).collect();
    let ret = sig.ret(db).display_name(db);
    format!("fn({}) -> {}", params.join(", "), ret)
  }

  fn runtime_type(&self, _db: &TypedownDatabase) -> Option<TdTypeEnum> {
    Some((*self).into())
  }
  fn call_type(&self, db: &TypedownDatabase, _arg_types: Vec<TdTypeEnum>) -> Option<FuncSignature> {
    Some(self.signature(db))
  }
}

impl TdFuncType<'_> {
  pub fn get(db: &TypedownDatabase, params: Vec<TdTypeEnum>, ret: TdTypeEnum) -> TdFuncType {
    get_func_type(db, FuncSignature::new(db, params, ret))
  }
}

#[query_derived]
pub struct TdFuncObj<'db> {
  #[id]
  pub name: String,
  #[id]
  pub signature: FuncSignature<'db>,
  pub func: FnKind<'db>,
}

impl TdRuntimeObject for TdFuncObj<'_> {
  fn get_type(&self, db: &TypedownDatabase) -> TdTypeEnum {
    get_func_type(db, self.signature(db)).into()
  }
  fn get_owned_field(&self, _db: &TypedownDatabase, _key: &str) -> Option<TdObjectEnum> {
    None
  }
  fn source_path(&self, db: &TypedownDatabase) -> String {
    self.get_type(db).source_path(db)
  }
  fn to_display_string(&self, db: &TypedownDatabase) -> String {
    self.name(db)
  }
  fn call(
    &self,
    db: &TypedownDatabase,
    project: Project,
    this: Option<TdObjectEnum>,
    args: Vec<TdObjectEnum>,
  ) -> Result<TdObjectEnum, Vec<Diagnostic>> {
    match self.func(db) {
      FnKind::Native(kind) => (kind.resolve())(db, project, this, args),
      FnKind::UserDefined(closure_hir, defining_scope) => {
        let HirValueKind::Closure { params, body } = closure_hir.kind(db) else {
          return Err(vec![]);
        };
        let bindings: Vec<(String, TdObjectEnum)> = params.into_iter().zip(args).collect();
        // Chain the defining scope as parent so nested closures can resolve outer params
        let runtime_scope = RuntimeScope::new(
          db,
          defining_scope.scope(db),
          bindings,
          Some(Box::new(defining_scope)),
        );
        let res = evaluate_node(db, *body, runtime_scope);
        if let Some(val) = res.value(db) {
          Ok(val)
        } else {
          Err(res.diagnostics(db).clone())
        }
      }
    }
  }
}
