use typedown_macros::query_derived;

use super::base::{TdRuntimeObj, TdStaticTyp, TdTypTyp};
use super::native_func::FuncKind;
use super::typecheck::{contains_variable, infer_bindings, is_subtyp_of, substitute_variables};
use super::{TdObjEnum, TdTypEnum};
use crate::db::TypedownDatabase;
use crate::db::derived::evaluate::evaluate_node::evaluate_node;
use crate::db::derived::get_builtin_typs::get_func_typ;
use crate::db::types::CallTypResult;
use crate::db::types::{FuncSignature, LazyTyp, Project, ProjectRedNode, RuntimeScope};
use crate::syntax::ast::{AstNode, ClosureExpr, IdentLit};
use crate::syntax::diagnostic::TdDiagnostic;
use crate::syntax::diagnostic::TdDiagnosticKind;
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
  ) -> CallTypResult<'db> {
    let signature = self.signature(db);
    let Some(typ_params) = signature.typ_params(db) else {
      return CallTypResult {
        signature: Some(signature),
        diagnostics: vec![],
      };
    };

    let params = signature.params(db);
    let variables = typ_params.params(db);
    let num_variables = variables.len();

    // Iterative inference:
    // 1. Infer from original params
    // 2. Substitute known bindings
    // 3. Re-infer from the original params to discover new bindings from args whose types are now concrete after substitution
    let mut bindings = infer_bindings(db, &params, &arg_typs);
    for _ in 0..num_variables {
      // Substitute known bindings into the arg types (for closures whose param types depend on earlier inferred variables)
      let resolved_typ: Vec<LazyTyp<'db>> = (0..num_variables)
        .map(|index| {
          bindings
            .get(&index)
            .map(|typ| LazyTyp::eager(typ.clone()))
            .unwrap_or_else(|| variables[index].upper_bound(db))
        })
        .collect();
      let substituted_params: Vec<TdTypEnum<'db>> = params
        .iter()
        .map(|param| substitute_variables(db, param, &resolved_typ))
        .collect();
      let new_bindings = infer_bindings(db, &substituted_params, &arg_typs);
      let mut merged = bindings.clone();
      let mut changed = false;
      for (index, typ) in &new_bindings {
        if !merged.contains_key(index) {
          merged.insert(*index, typ.clone());
          changed = true;
        }
      }
      if !changed {
        break;
      }
      bindings = merged;
    }

    // Build final resolved bindings with upper bound fallback
    let mut diagnostics = vec![];
    let resolved: Vec<LazyTyp<'db>> = (0..num_variables)
      .map(|index| {
        let upper_bound = variables[index].upper_bound(db);
        match bindings.get(&index) {
          Some(inferred) => {
            if let Some(bound) = upper_bound.resolve(db)
              && !is_subtyp_of(db, inferred, &bound)
            {
              diagnostics.push(TdDiagnosticKind::TypArgBoundViolation {
                index,
                expected_bound: bound.display_name(db),
                got: inferred.display_name(db),
              });
              return upper_bound;
            }
            LazyTyp::eager(inferred.clone())
          }
          None => upper_bound,
        }
      })
      .collect();

    let new_params: Vec<TdTypEnum<'db>> = params
      .iter()
      .map(|param| substitute_variables(db, param, &resolved))
      .collect();
    let new_ret = substitute_variables(db, &signature.ret(db), &resolved);

    if contains_variable(&new_ret) {
      diagnostics.push(TdDiagnosticKind::UnresolvedGenericReturnTyp {
        ret: new_ret.display_name(db),
      });
      return CallTypResult {
        signature: None,
        diagnostics,
      };
    }

    CallTypResult {
      signature: Some(FuncSignature::new(db, None, new_params, new_ret)),
      diagnostics,
    }
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
