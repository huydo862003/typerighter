use typedown_macros::query_derived;

use super::base::{TdRuntimeObj, TdStaticTyp, TdTypTyp};
use super::native_func::FuncKind;
use super::{TdObjEnum, TdTypEnum};
use crate::db::TypedownDatabase;
use crate::db::derived::evaluate::evaluate_node::evaluate_node;
use crate::db::derived::get_builtin_typs::get_func_typ;
use crate::db::types::{FuncSignature, HirValueKind, Project, RuntimeScope};
use crate::syntax::diagnostic::Diagnostic;

#[query_derived]
pub struct TdFuncTyp<'db> {
  #[id]
  pub signature: FuncSignature<'db>,
}

impl<'db> TdRuntimeObj<'db> for TdFuncTyp<'db> {
  fn get_typ(&self, db: &'db TypedownDatabase) -> TdTypEnum<'db> {
    TdTypTyp::get(db).into()
  }
  fn get_owned_field(&self, _db: &'db TypedownDatabase, _key: &str) -> Option<TdObjEnum<'db>> {
    None
  }
  fn get_builtin_field(&self, _db: &'db TypedownDatabase, _key: &str) -> Option<TdObjEnum<'db>> {
    None
  }
  fn source_path(&self, db: &'db TypedownDatabase) -> String {
    let signature = self.signature(db);
    let params: Vec<String> = signature
      .params(db)
      .iter()
      .map(|param| param.source_path(db))
      .collect();
    let ret = signature.ret(db).source_path(db);
    format!("@builtin::function[({}) -> {}]", params.join(", "), ret)
  }
}

impl<'db> TdStaticTyp<'db> for TdFuncTyp<'db> {
  fn display_name(&self, db: &'db TypedownDatabase) -> String {
    let signature = self.signature(db);
    let params: Vec<String> = signature
      .params(db)
      .iter()
      .map(|p| p.display_name(db))
      .collect();
    let ret = signature.ret(db).display_name(db);
    format!("fn({}) -> {}", params.join(", "), ret)
  }

  fn runtime_typ(&self, _db: &'db TypedownDatabase) -> Option<TdTypEnum<'db>> {
    Some((*self).into())
  }
  fn call_typ(
    &self,
    db: &'db TypedownDatabase,
    _arg_types: Vec<TdTypEnum<'db>>,
  ) -> Option<FuncSignature<'db>> {
    Some(self.signature(db))
  }
}

impl<'db> TdFuncTyp<'db> {
  pub fn get(
    db: &'db TypedownDatabase,
    params: Vec<TdTypEnum<'db>>,
    ret: TdTypEnum<'db>,
  ) -> TdFuncTyp<'db> {
    get_func_typ(db, FuncSignature::new(db, params, ret))
  }
}

#[query_derived]
pub struct TdFuncObj<'db> {
  #[id]
  pub name: String,
  #[id]
  pub signature: FuncSignature<'db>,
  pub func: FuncKind<'db>,
}

impl<'db> TdRuntimeObj<'db> for TdFuncObj<'db> {
  fn get_typ(&self, db: &'db TypedownDatabase) -> TdTypEnum<'db> {
    get_func_typ(db, self.signature(db)).into()
  }
  fn get_owned_field(&self, _db: &'db TypedownDatabase, _key: &str) -> Option<TdObjEnum<'db>> {
    None
  }
  fn get_builtin_field(&self, _db: &'db TypedownDatabase, _key: &str) -> Option<TdObjEnum<'db>> {
    None
  }
  fn source_path(&self, db: &'db TypedownDatabase) -> String {
    self.get_typ(db).source_path(db)
  }
  fn to_display_string(&self, db: &'db TypedownDatabase) -> String {
    self.name(db)
  }
  fn call(
    &self,
    db: &'db TypedownDatabase,
    project: Project,
    this: Option<TdObjEnum<'db>>,
    args: Vec<TdObjEnum<'db>>,
  ) -> Result<TdObjEnum<'db>, Vec<Diagnostic>> {
    match self.func(db) {
      FuncKind::Native(kind) => (kind.resolve())(db, project, this, args),
      FuncKind::UserDefined(closure_hir, defining_scope) => {
        let HirValueKind::Closure { params, body } = closure_hir.kind(db) else {
          return Err(vec![]);
        };
        let bindings: Vec<(String, TdObjEnum)> = params.into_iter().zip(args).collect();
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
