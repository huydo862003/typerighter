use typedown_macros::query_derived;

use super::base::{TdRuntimeObj, TdStaticTyp, TdTypTyp};
use super::native_func::FuncKind;
use super::variable::TdVariableTyp;
use super::{TdObjEnum, TdTypEnum};
use crate::db::TypedownDatabase;
use crate::db::derived::evaluate::evaluate_node::evaluate_node;
use crate::db::derived::get_builtin_typs::{get_func_typ, get_obj_typ};
use crate::db::types::{FuncSignature, LazyTyp, Project, ProjectRedNode, RuntimeScope};
use crate::syntax::ast::{AstNode, ClosureExpr, IdentLit};
use crate::syntax::diagnostic::TdDiagnostic;
use typedown_types::either::Either;

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
      .map(|param| param.display_name(db))
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
    arg_typs: Vec<TdTypEnum<'db>>,
  ) -> Option<FuncSignature<'db>> {
    let signature = self.signature(db);
    let Some(typ_params) = signature.typ_params(db) else {
      return Some(signature);
    };

    // Infer type variable bindings from argument types
    let params = signature.params(db);
    let number_variables = typ_params.len(db);
    let mut bindings: Vec<Option<TdTypEnum<'db>>> = vec![None; number_variables];

    for (param_typ, arg_typ) in params.iter().zip(arg_typs.iter()) {
      infer_bindings(db, param_typ, arg_typ, &mut bindings);
    }

    // Fill unresolved bindings with upper bounds
    let upper_bounds: Vec<LazyTyp<'db>> = typ_params
      .params(db)
      .iter()
      .map(|variable| variable.upper_bound(db))
      .collect();
    let resolved: Vec<LazyTyp<'db>> = bindings
      .into_iter()
      .enumerate()
      .map(|(index, binding)| match binding {
        Some(typ) => LazyTyp::eager(typ),
        None => upper_bounds
          .get(index)
          .cloned()
          .unwrap_or_else(|| LazyTyp::eager(get_obj_typ(db).into())),
      })
      .collect();

    // Substitute variables in params and ret
    let new_params = params
      .iter()
      .map(|param| substitute_variables(db, param, &resolved))
      .collect();
    let new_ret = substitute_variables(db, &signature.ret(db), &resolved);

    Some(FuncSignature::new(db, None, new_params, new_ret))
  }
}

impl<'db> TdFuncTyp<'db> {
  pub fn get(
    db: &'db TypedownDatabase,
    params: Vec<TdTypEnum<'db>>,
    ret: TdTypEnum<'db>,
  ) -> TdFuncTyp<'db> {
    get_func_typ(db, FuncSignature::new(db, None, params, ret))
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
  ) -> Result<TdObjEnum<'db>, Vec<TdDiagnostic>> {
    match self.func(db) {
      FuncKind::Native(kind) => (kind.resolve())(db, project, this, args),
      FuncKind::UserDefined(closure_project_node, defining_scope) => {
        let Some(closure) = ClosureExpr::cast(closure_project_node.node.clone()) else {
          return Err(vec![]);
        };
        let param_idents: Vec<IdentLit> = match closure.params() {
          Some(Either::Left(param_list)) => param_list.params().collect(),
          Some(Either::Right(ident)) => vec![ident],
          None => vec![],
        };
        let params: Vec<String> = param_idents
          .into_iter()
          .filter_map(|id| id.value())
          .collect();
        let bindings: Vec<(String, TdObjEnum)> = params.into_iter().zip(args).collect();
        // Chain the defining scope as parent so nested closures can resolve outer params
        let runtime_scope = RuntimeScope::new(
          db,
          defining_scope.scope(db),
          bindings,
          Some(Box::new(defining_scope)),
        );
        let Some(body_expr) = closure.body() else {
          return Err(vec![]);
        };
        let body_project_node = ProjectRedNode::new(
          closure_project_node.project,
          closure_project_node.file,
          body_expr.syntax().clone(),
        );
        let result = evaluate_node(db, body_project_node, runtime_scope);
        if let Some(value) = result.value(db) {
          Ok(value)
        } else {
          Err(result.diagnostics(db).clone())
        }
      }
    }
  }
}

/// Infer type variable bindings by matching a parameter type against an argument type
fn infer_bindings<'db>(
  db: &'db TypedownDatabase,
  param: &TdTypEnum<'db>,
  arg: &TdTypEnum<'db>,
  bindings: &mut [Option<TdTypEnum<'db>>],
) {
  match param {
    TdTypEnum::TdVariableTyp(variable) => {
      let index = TdVariableTyp::index(*variable, db);
      if index < bindings.len() && bindings[index].is_none() {
        bindings[index] = Some(arg.clone());
      }
    }
    TdTypEnum::TdFuncTyp(func_param) => {
      // Recurse into function param/ret: (T) -> U matched against (String) -> Number
      if let TdTypEnum::TdFuncTyp(func_arg) = arg {
        let param_signature = func_param.signature(db);
        let arg_signature = func_arg.signature(db);
        for (param, arg) in param_signature.params(db).iter().zip(arg_signature.params(db).iter()) {
          infer_bindings(db, param, arg, bindings);
        }
        infer_bindings(db, &param_signature.ret(db), &arg_signature.ret(db), bindings);
      }
    }
    TdTypEnum::TdListTyp(list_param) => {
      // list[T] matched against list[String]
      if let TdTypEnum::TdListTyp(list_arg) = arg {
        let param_args = list_param.get_typ_args(db);
        let arg_args = list_arg.get_typ_args(db);
        for (param, arg) in param_args.iter().zip(arg_args.iter()) {
          infer_bindings(db, param, arg, bindings);
        }
      }
    }
    TdTypEnum::TdDictTyp(dict_param) => {
      // dict[K, V] matched against dict[String, Number]
      if let TdTypEnum::TdDictTyp(dict_arg) = arg {
        let param_args = dict_param.get_typ_args(db);
        let arg_args = dict_arg.get_typ_args(db);
        for (param, arg) in param_args.iter().zip(arg_args.iter()) {
          infer_bindings(db, param, arg, bindings);
        }
      }
    }
    _ => {}
  }
}

/// Substitute type variables with their resolved bindings.
fn substitute_variables<'db>(
  db: &'db TypedownDatabase,
  typ: &TdTypEnum<'db>,
  bindings: &[LazyTyp<'db>],
) -> TdTypEnum<'db> {
  match typ {
    TdTypEnum::TdVariableTyp(variable) => {
      let index = TdVariableTyp::index(*variable, db);
      if let Some(binding) = bindings.get(index) {
        binding.resolve(db).unwrap_or_else(|| typ.clone())
      } else {
        typ.clone()
      }
    }
    TdTypEnum::TdFuncTyp(func) => {
      let signature = func.signature(db);
      let new_params: Vec<TdTypEnum<'db>> = signature
        .params(db)
        .iter()
        .map(|param| substitute_variables(db, param, bindings))
        .collect();
      let new_ret = substitute_variables(db, &signature.ret(db), bindings);
      TdFuncTyp::get(db, new_params, new_ret).into()
    }
    _ => typ.clone(),
  }
}
