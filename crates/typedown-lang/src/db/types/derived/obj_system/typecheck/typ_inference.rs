//! Type inference primitives: binding inference and bound variable substitution

use std::collections::BTreeMap;

use hashbrown::HashMap;

use crate::db::TypedownDatabase;
use crate::db::derived::get_builtin_typs::get_sum_typ;
use crate::db::types::LazyTyp;

use crate::db::types::derived::obj_system::bound_variable::TdBoundVariableTyp;
use crate::db::types::derived::obj_system::dict::TdDictTyp;
use crate::db::types::derived::obj_system::func::TdFuncTyp;
use crate::db::types::derived::obj_system::list::TdListTyp;
use crate::db::types::derived::obj_system::product::TdProductTyp;
use crate::db::types::derived::obj_system::{TdStaticTyp, TdTypEnum};

pub(crate) fn substitute_bound_variables<'db>(
  db: &'db TypedownDatabase,
  typ: &TdTypEnum<'db>,
  bindings: &HashMap<usize, LazyTyp<'db>>,
) -> TdTypEnum<'db> {
  match typ {
    TdTypEnum::TdBoundVariableTyp(bound_variable) => {
      let index = TdBoundVariableTyp::index(*bound_variable, db);
      if let Some(binding) = bindings.get(&index) {
        binding.resolve(db).unwrap_or_else(|| typ.clone())
      } else {
        TdBoundVariableTyp::upper_bound(*bound_variable, db)
          .resolve(db)
          .unwrap_or_else(|| typ.clone())
      }
    }
    TdTypEnum::TdFreeVariableTyp(_) | TdTypEnum::TdExistentialVariableTyp(_) => typ.clone(),
    TdTypEnum::TdFuncTyp(func) => {
      let signature = func.signature(db);
      let new_params: Vec<TdTypEnum<'db>> = signature
        .params(db)
        .iter()
        .map(|param| substitute_bound_variables(db, param, bindings))
        .collect();
      let new_ret = substitute_bound_variables(db, &signature.ret(db), bindings);
      TdFuncTyp::get(db, new_params, new_ret).into()
    }
    TdTypEnum::TdListTyp(list) => {
      if let Some(element) = list.element(db)
        && let Some(resolved_element) = element.resolve(db)
      {
        let substituted = substitute_bound_variables(db, &resolved_element, bindings);
        return TdListTyp::new(db, Some(LazyTyp::eager(substituted))).into();
      }
      typ.clone()
    }
    TdTypEnum::TdDictTyp(dict) => {
      let new_key = dict
        .key(db)
        .and_then(|key| key.resolve(db))
        .map(|key| LazyTyp::eager(substitute_bound_variables(db, &key, bindings)));
      let new_value = dict
        .value(db)
        .and_then(|value| value.resolve(db))
        .map(|value| LazyTyp::eager(substitute_bound_variables(db, &value, bindings)));
      if new_key.is_some() || new_value.is_some() {
        return TdDictTyp::new(db, new_key, new_value).into();
      }
      typ.clone()
    }
    TdTypEnum::TdSumTyp(sum) => {
      let new_members: Vec<LazyTyp<'db>> = sum
        .members(db)
        .iter()
        .map(|member| {
          member
            .resolve(db)
            .map(|resolved| LazyTyp::eager(substitute_bound_variables(db, &resolved, bindings)))
            .unwrap_or_else(|| member.clone())
        })
        .collect();
      get_sum_typ(db, new_members).into()
    }
    TdTypEnum::TdProductTyp(product) => {
      let new_fields: BTreeMap<String, LazyTyp<'db>> = product
        .fields(db)
        .into_iter()
        .map(|(name, lazy)| {
          let substituted = lazy
            .resolve(db)
            .map(|resolved| LazyTyp::eager(substitute_bound_variables(db, &resolved, bindings)))
            .unwrap_or(lazy);
          (name, substituted)
        })
        .collect();
      TdProductTyp::new(db, None, new_fields).into()
    }
    // Existentials bind their own variables, substituting outer bindings would be incorrect
    TdTypEnum::TdExistentialTyp(_) => typ.clone(),
    // Metatype instances wrap a type constructor, not a variable carrier
    TdTypEnum::TdMetatypeInstanceTyp(_) => typ.clone(),
    // Primitives, no internal type structure
    _ => typ.clone(),
  }
}

// Infer type variable bindings by matching parameter types against argument types
pub(crate) fn infer_bindings<'db>(
  db: &'db TypedownDatabase,
  params: &[TdTypEnum<'db>],
  args: &[TdTypEnum<'db>],
) -> HashMap<usize, TdTypEnum<'db>> {
  fn infer_from_pair<'db>(
    db: &'db TypedownDatabase,
    param: &TdTypEnum<'db>,
    arg: &TdTypEnum<'db>,
    bindings: &mut HashMap<usize, TdTypEnum<'db>>,
  ) {
    match param {
      TdTypEnum::TdBoundVariableTyp(bound_variable) => {
        let index = TdBoundVariableTyp::index(*bound_variable, db);
        bindings
          .entry(index)
          .and_modify(|existing| {
            if *existing != *arg {
              *existing = get_sum_typ(
                db,
                vec![
                  LazyTyp::eager(existing.clone()),
                  LazyTyp::eager(arg.clone()),
                ],
              )
              .into();
            }
          })
          .or_insert_with(|| arg.clone());
      }
      TdTypEnum::TdFreeVariableTyp(_) | TdTypEnum::TdExistentialVariableTyp(_) => {}
      TdTypEnum::TdFuncTyp(func_param) => {
        if let TdTypEnum::TdFuncTyp(func_arg) = arg {
          let param_signature = func_param.signature(db);
          let arg_signature = func_arg.signature(db);
          for (param, arg) in param_signature
            .params(db)
            .iter()
            .zip(arg_signature.params(db).iter())
          {
            infer_from_pair(db, param, arg, bindings);
          }
          infer_from_pair(
            db,
            &param_signature.ret(db),
            &arg_signature.ret(db),
            bindings,
          );
        }
      }
      TdTypEnum::TdListTyp(list_param) => {
        if let TdTypEnum::TdListTyp(list_arg) = arg {
          for (param, arg) in list_param
            .get_typ_args(db)
            .iter()
            .zip(list_arg.get_typ_args(db).iter())
          {
            infer_from_pair(db, param, arg, bindings);
          }
        }
      }
      TdTypEnum::TdDictTyp(dict_param) => {
        if let TdTypEnum::TdDictTyp(dict_arg) = arg {
          for (param, arg) in dict_param
            .get_typ_args(db)
            .iter()
            .zip(dict_arg.get_typ_args(db).iter())
          {
            infer_from_pair(db, param, arg, bindings);
          }
        }
      }
      _ => {}
    }
  }

  let mut bindings = HashMap::new();
  for (param, arg) in params.iter().zip(args.iter()) {
    infer_from_pair(db, param, arg, &mut bindings);
  }
  bindings
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::db::QueryStorage;
  use crate::db::derived::get_builtin_typs::{
    get_bool_typ, get_func_typ, get_number_typ, get_obj_typ, get_string_typ,
  };
  use crate::db::types::{FuncSignature, LazyTyp, TdBoundVariableTyp, TypParams, Variance};
  use crate::syntax::diagnostic::TdDiagnosticKind;

  fn make_db() -> TypedownDatabase {
    TypedownDatabase {
      storage: QueryStorage::default(),
    }
  }

  #[test]
  fn infer_simple_variable() {
    let db = make_db();
    let string_typ: TdTypEnum = get_string_typ(&db).into();
    let bound_variable = TdBoundVariableTyp::new(
      &db,
      0,
      LazyTyp::eager(get_obj_typ(&db).into()),
      Variance::Covariant,
    );

    let bindings = infer_bindings(
      &db,
      &[bound_variable.into()],
      std::slice::from_ref(&string_typ),
    );
    assert_eq!(bindings.get(&0), Some(&string_typ));
  }

  #[test]
  fn infer_multiple_same_variable_widens_to_sum() {
    let db = make_db();
    let string_typ: TdTypEnum = get_string_typ(&db).into();
    let number_typ: TdTypEnum = get_number_typ(&db).into();
    let bound_variable = TdBoundVariableTyp::new(
      &db,
      0,
      LazyTyp::eager(get_obj_typ(&db).into()),
      Variance::Covariant,
    );
    let variable: TdTypEnum = bound_variable.into();

    let bindings = infer_bindings(
      &db,
      &[variable.clone(), variable],
      &[string_typ, number_typ],
    );
    let inferred = bindings.get(&0).unwrap();
    // Should be a sum type (String | Number)
    assert!(
      matches!(inferred, TdTypEnum::TdSumTyp(_)),
      "expected sum type, got: {}",
      inferred.display_name(&db)
    );
  }

  #[test]
  fn infer_same_type_no_widen() {
    let db = make_db();
    let string_typ: TdTypEnum = get_string_typ(&db).into();
    let bound_variable = TdBoundVariableTyp::new(
      &db,
      0,
      LazyTyp::eager(get_obj_typ(&db).into()),
      Variance::Covariant,
    );
    let variable: TdTypEnum = bound_variable.into();

    let bindings = infer_bindings(
      &db,
      &[variable.clone(), variable],
      &[string_typ.clone(), string_typ.clone()],
    );
    assert_eq!(bindings.get(&0), Some(&string_typ));
  }

  #[test]
  fn infer_from_function_arg() {
    let db = make_db();
    let string_typ: TdTypEnum = get_string_typ(&db).into();
    let number_typ: TdTypEnum = get_number_typ(&db).into();
    let bound_variable_t = TdBoundVariableTyp::new(
      &db,
      0,
      LazyTyp::eager(get_obj_typ(&db).into()),
      Variance::Covariant,
    );
    let bound_variable_u = TdBoundVariableTyp::new(
      &db,
      1,
      LazyTyp::eager(get_obj_typ(&db).into()),
      Variance::Covariant,
    );

    // param: (T) -> U, arg: (String) -> Number
    let param_func =
      TdFuncTyp::get(&db, vec![bound_variable_t.into()], bound_variable_u.into()).into();
    let arg_func: TdTypEnum =
      TdFuncTyp::get(&db, vec![string_typ.clone()], number_typ.clone()).into();

    let bindings = infer_bindings(&db, &[param_func], &[arg_func]);
    assert_eq!(bindings.get(&0), Some(&string_typ));
    assert_eq!(bindings.get(&1), Some(&number_typ));
  }

  #[test]
  fn substitute_simple() {
    let db = make_db();
    let string_typ: TdTypEnum = get_string_typ(&db).into();
    let bound_variable = TdBoundVariableTyp::new(
      &db,
      0,
      LazyTyp::eager(get_obj_typ(&db).into()),
      Variance::Covariant,
    );
    let bindings: HashMap<usize, LazyTyp> =
      HashMap::from_iter([(0, LazyTyp::eager(string_typ.clone()))]);

    let result = substitute_bound_variables(&db, &bound_variable.into(), &bindings);
    assert_eq!(result, string_typ);
  }

  #[test]
  fn substitute_in_function_return() {
    let db = make_db();
    let string_typ: TdTypEnum = get_string_typ(&db).into();
    let bound_variable = TdBoundVariableTyp::new(
      &db,
      0,
      LazyTyp::eager(get_obj_typ(&db).into()),
      Variance::Covariant,
    );
    let bindings: HashMap<usize, LazyTyp> =
      HashMap::from_iter([(0, LazyTyp::eager(string_typ.clone()))]);

    // fn(String) -> T, substitute T=String
    let func_typ: TdTypEnum =
      TdFuncTyp::get(&db, vec![string_typ.clone()], bound_variable.into()).into();
    let result = substitute_bound_variables(&db, &func_typ, &bindings);

    if let TdTypEnum::TdFuncTyp(func) = result {
      assert_eq!(func.signature(&db).ret(&db), string_typ);
    } else {
      panic!("expected function type");
    }
  }

  #[test]
  fn substitute_in_list() {
    let db = make_db();
    let string_typ: TdTypEnum = get_string_typ(&db).into();
    let bound_variable = TdBoundVariableTyp::new(
      &db,
      0,
      LazyTyp::eager(get_obj_typ(&db).into()),
      Variance::Covariant,
    );
    let bindings: HashMap<usize, LazyTyp> =
      HashMap::from_iter([(0, LazyTyp::eager(string_typ.clone()))]);

    // list[T], substitute T=String
    let list_typ: TdTypEnum =
      TdListTyp::new(&db, Some(LazyTyp::eager(bound_variable.into()))).into();
    let result = substitute_bound_variables(&db, &list_typ, &bindings);

    if let TdTypEnum::TdListTyp(list) = result {
      let args = list.get_typ_args(&db);
      assert_eq!(args.len(), 1);
      assert_eq!(args[0], string_typ);
    } else {
      panic!("expected list type");
    }
  }

  #[test]
  fn call_typ_infers_and_substitutes() {
    let db = make_db();
    let string_typ: TdTypEnum = get_string_typ(&db).into();
    let bound_variable_t = TdBoundVariableTyp::new(
      &db,
      0,
      LazyTyp::eager(get_obj_typ(&db).into()),
      Variance::Covariant,
    );
    let bound_variable_t_typ: TdTypEnum = bound_variable_t.into();

    // Generic function: <T>(T) -> T
    let typ_params = TypParams::new(&db, vec![bound_variable_t], vec![]);
    let signature = FuncSignature::new(
      &db,
      Some(typ_params),
      vec![bound_variable_t_typ.clone()],
      bound_variable_t_typ,
    );
    let func = get_func_typ(&db, signature);

    // Call with String arg
    let call_result = func.call_typ(&db, vec![string_typ.clone()]);
    assert!(call_result.diagnostics.is_empty());
    let instantiated = call_result.signature.unwrap();
    assert_eq!(instantiated.ret(&db), string_typ);
    assert_eq!(instantiated.params(&db)[0], string_typ);
  }

  #[test]
  fn call_typ_bound_violation_emits_diagnostic() {
    let db = make_db();
    let string_typ: TdTypEnum = get_string_typ(&db).into();
    let number_typ: TdTypEnum = get_number_typ(&db).into();

    // T <: String
    let bound_variable_t = TdBoundVariableTyp::new(
      &db,
      0,
      LazyTyp::eager(string_typ.clone()),
      Variance::Covariant,
    );
    let bound_variable_t_typ: TdTypEnum = bound_variable_t.into();

    // Generic function: <T <: String>(T) -> T
    let typ_params = TypParams::new(&db, vec![bound_variable_t], vec![]);
    let signature = FuncSignature::new(
      &db,
      Some(typ_params),
      vec![bound_variable_t_typ.clone()],
      bound_variable_t_typ,
    );
    let func = get_func_typ(&db, signature);

    // Call with Number arg (violates T <: String) - should emit diagnostic
    let call_result = func.call_typ(&db, vec![number_typ]);
    assert!(
      !call_result.diagnostics.is_empty(),
      "expected bound violation diagnostic"
    );
    assert!(
      matches!(
        call_result.diagnostics[0],
        TdDiagnosticKind::TypArgBoundViolation { .. }
      ),
      "expected TypArgBoundViolation, got: {:?}",
      call_result.diagnostics[0]
    );
  }

  #[test]
  fn call_typ_uninferred_variable_falls_back_to_upper_bound() {
    let db = make_db();
    let string_typ: TdTypEnum = get_string_typ(&db).into();
    let obj_typ: TdTypEnum = get_obj_typ(&db).into();
    let bound_variable_t = TdBoundVariableTyp::new(
      &db,
      0,
      LazyTyp::eager(get_obj_typ(&db).into()),
      Variance::Covariant,
    );
    let bound_variable_u = TdBoundVariableTyp::new(
      &db,
      1,
      LazyTyp::eager(get_obj_typ(&db).into()),
      Variance::Covariant,
    );
    let bound_variable_t_typ: TdTypEnum = bound_variable_t.into();
    let bound_variable_u_typ: TdTypEnum = bound_variable_u.into();

    // Generic function: <T, U>(T) -> U  (U cannot be inferred from args)
    let typ_params = TypParams::new(&db, vec![bound_variable_t, bound_variable_u], vec![]);
    let signature = FuncSignature::new(
      &db,
      Some(typ_params),
      vec![bound_variable_t_typ],
      bound_variable_u_typ,
    );
    let func = get_func_typ(&db, signature);

    // U falls back to upper bound (Obj)
    let call_result = func.call_typ(&db, vec![string_typ]);
    assert!(call_result.diagnostics.is_empty());
    assert_eq!(call_result.signature.unwrap().ret(&db), obj_typ);
  }

  // Simulates: filter: <T>(list[T], (T) -> bool) -> list[T]
  // Call: filter(list[string], (item) -> true)
  // Round 1: infer T=string from list[string]; closure param T not yet concrete
  // Round 2: substitute T=string into params, confirm T=string from (string)->bool
  // Expected: instantiated to (list[string], (string)->bool) -> list[string]
  #[test]
  fn call_typ_iterative_inference_closure_param_from_list_arg() {
    let db = make_db();
    let string_typ: TdTypEnum = get_string_typ(&db).into();
    let bool_typ: TdTypEnum = get_bool_typ(&db).into();
    let bound_variable_t = TdBoundVariableTyp::new(
      &db,
      0,
      LazyTyp::eager(get_obj_typ(&db).into()),
      Variance::Covariant,
    );
    let bound_variable_t_typ: TdTypEnum = bound_variable_t.into();

    let list_of_t: TdTypEnum =
      TdListTyp::new(&db, Some(LazyTyp::eager(bound_variable_t_typ.clone()))).into();
    let list_of_string: TdTypEnum =
      TdListTyp::new(&db, Some(LazyTyp::eager(string_typ.clone()))).into();
    // closure param: (T) -> bool
    let closure_of_t: TdTypEnum =
      TdFuncTyp::get(&db, vec![bound_variable_t_typ.clone()], bool_typ.clone()).into();
    // concrete closure arg: (string) -> bool
    let closure_of_string: TdTypEnum =
      TdFuncTyp::get(&db, vec![string_typ.clone()], bool_typ.clone()).into();

    // filter: <T>(list[T], (T) -> bool) -> list[T]
    let typ_params = TypParams::new(&db, vec![bound_variable_t], vec![]);
    let signature = FuncSignature::new(
      &db,
      Some(typ_params),
      vec![list_of_t.clone(), closure_of_t],
      list_of_t,
    );
    let func = get_func_typ(&db, signature);

    // filter(list[string], (item: string) -> true)
    let call_result = func.call_typ(&db, vec![list_of_string.clone(), closure_of_string]);
    assert!(
      call_result.diagnostics.is_empty(),
      "unexpected diagnostics: {:?}",
      call_result.diagnostics
    );
    let instantiated = call_result.signature.unwrap();
    assert_eq!(
      instantiated.ret(&db).display_name(&db),
      list_of_string.display_name(&db)
    );
    assert_eq!(
      instantiated.params(&db)[0].display_name(&db),
      list_of_string.display_name(&db)
    );
    assert_eq!(
      instantiated.params(&db)[1].display_name(&db),
      TdTypEnum::from(TdFuncTyp::get(
        &db,
        vec![string_typ.clone()],
        bool_typ.clone()
      ))
      .display_name(&db)
    );
  }

  // Simulates: filter: <T>((T) -> bool, list[T]) -> list[T]  (args in reverse order)
  // Call: filter((item) -> true, list[string])
  // Round 1: infer T from closure (string)->bool (no T in concrete form since arg is concrete), then from list[string] -> T=string
  // This tests that iterative inference discovers T even when the list arg is second
  #[test]
  fn call_typ_iterative_inference_closure_first_list_second() {
    let db = make_db();
    let string_typ: TdTypEnum = get_string_typ(&db).into();
    let bool_typ: TdTypEnum = get_bool_typ(&db).into();
    let bound_variable_t = TdBoundVariableTyp::new(
      &db,
      0,
      LazyTyp::eager(get_obj_typ(&db).into()),
      Variance::Covariant,
    );
    let bound_variable_t_typ: TdTypEnum = bound_variable_t.into();

    let list_of_t: TdTypEnum =
      TdListTyp::new(&db, Some(LazyTyp::eager(bound_variable_t_typ.clone()))).into();
    let list_of_string: TdTypEnum =
      TdListTyp::new(&db, Some(LazyTyp::eager(string_typ.clone()))).into();
    let closure_of_t: TdTypEnum =
      TdFuncTyp::get(&db, vec![bound_variable_t_typ.clone()], bool_typ.clone()).into();
    let closure_of_string: TdTypEnum =
      TdFuncTyp::get(&db, vec![string_typ.clone()], bool_typ.clone()).into();

    // filter: <T>((T) -> bool, list[T]) -> list[T]
    let typ_params = TypParams::new(&db, vec![bound_variable_t], vec![]);
    let signature = FuncSignature::new(
      &db,
      Some(typ_params),
      vec![closure_of_t, list_of_t.clone()],
      list_of_t,
    );
    let func = get_func_typ(&db, signature);

    // filter((item: string) -> true, list[string])
    let call_result = func.call_typ(&db, vec![closure_of_string, list_of_string.clone()]);
    assert!(
      call_result.diagnostics.is_empty(),
      "unexpected diagnostics: {:?}",
      call_result.diagnostics
    );
    let instantiated = call_result.signature.unwrap();
    assert_eq!(
      instantiated.ret(&db).display_name(&db),
      list_of_string.display_name(&db)
    );
  }
}
