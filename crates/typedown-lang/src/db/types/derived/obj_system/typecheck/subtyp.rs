//! Shared type compatibility utilities for typechecking

use crate::db::TypedownDatabase;
use crate::db::derived::get_builtin_typs::{get_null_typ, get_obj_typ};
use crate::db::types::LitValue;
use crate::db::types::derived::obj_system::TdStaticTyp;
use crate::db::types::fields_compatible;
use crate::db::types::{
  LazyTyp, TdBoundVariableTyp, TdExistentialTyp, TdExistentialVariableTyp, TdFreeVariableTyp,
  TdSchemaTyp, TdSumTyp, TdTypEnum, TypParams,
};

use super::substitute_bound_variables;
use crate::syntax::diagnostic::*;
use std::collections::{HashMap, HashSet};
use typedown_incremental::Id;

/// Validate type arguments against type parameters for both arity and bounds
pub fn validate_typ_params(
  db: &TypedownDatabase,
  typ_params: Option<&TypParams>,
  args: &[LazyTyp],
) -> Vec<TdDiagnostic> {
  let expected_arity = typ_params.map_or(0, |param| param.len(db));
  if expected_arity != args.len() {
    return vec![
      TdDiagnosticKind::WrongTypArgCount {
        expected: expected_arity,
        got: args.len(),
      }
      .into_error(0, 0),
    ];
  }

  let mut diagnostics = Vec::new();
  if let Some(params) = typ_params {
    let params_vec = params.params(db);
    for (index, (param, arg)) in params_vec.iter().zip(args.iter()).enumerate() {
      if let Some(bound) = param.upper_bound(db).resolve(db)
        && bound.as_td_obj_typ().is_none()
        && let Some(arg_typ) = arg.resolve(db)
        && !is_subtyp_of(db, &arg_typ, &bound)
      {
        diagnostics.push(
          TdDiagnosticKind::TypArgBoundViolation {
            name: format!("T{}", index),
            expected_bound: bound.display_name(db),
            got: arg_typ.display_name(db),
          }
          .into_error(0, 0),
        );
      }
    }
  }
  diagnostics
}

/// Check if `subtype` is a subtype of `supertype`
pub fn is_subtyp_of(db: &TypedownDatabase, subtyp: &TdTypEnum, supertyp: &TdTypEnum) -> bool {
  let mut env = SubtypEnv::new();
  is_subtyp_of_env(db, subtyp, supertyp, &mut env)
}

// Witness environment for existential subtyping constraints
#[derive(Default)]
struct SubtypEnv<'db> {
  lower_bounds: HashMap<TdExistentialVariableTyp<'db>, Vec<TdTypEnum<'db>>>,
  upper_bounds: HashMap<TdExistentialVariableTyp<'db>, Vec<TdTypEnum<'db>>>,
}

impl<'db> SubtypEnv<'db> {
  fn new() -> Self {
    Self {
      lower_bounds: HashMap::new(),
      upper_bounds: HashMap::new(),
    }
  }

  fn add_lower_bound(
    &mut self,
    db: &'db TypedownDatabase,
    variable: TdExistentialVariableTyp<'db>,
    lower_bound: &TdTypEnum<'db>,
  ) -> bool {
    if let Some(upper_bounds) = self.upper_bounds.get(&variable).cloned() {
      for upper_bound in &upper_bounds {
        if !is_subtyp_of_env(db, lower_bound, upper_bound, self) {
          return false;
        }
      }
    }
    self
      .lower_bounds
      .entry(variable)
      .or_default()
      .push(lower_bound.clone());
    true
  }

  fn add_upper_bound(
    &mut self,
    db: &'db TypedownDatabase,
    variable: TdExistentialVariableTyp<'db>,
    upper_bound: &TdTypEnum<'db>,
  ) -> bool {
    if let Some(lower_bounds) = self.lower_bounds.get(&variable).cloned() {
      for lower_bound in &lower_bounds {
        if !is_subtyp_of_env(db, lower_bound, upper_bound, self) {
          return false;
        }
      }
    }
    self
      .upper_bounds
      .entry(variable)
      .or_default()
      .push(upper_bound.clone());
    true
  }
}

// Walk the nominal parent chain of `sub` to see if it reaches `sup` by identity
fn nominal_subtyp_of(db: &TypedownDatabase, subtyp: &TdSchemaTyp, supertyp: &TdSchemaTyp) -> bool {
  let target_id = supertyp.as_id();
  let mut current = subtyp.parent(db);
  while let Some(typ) = current {
    if let TdTypEnum::TdSchemaTyp(parent) = &typ {
      if parent.as_id() == target_id {
        return true;
      }
      current = parent.parent(db);
    } else {
      break;
    }
  }
  false
}

fn is_subtyp_of_env<'db>(
  db: &'db TypedownDatabase,
  subtyp: &TdTypEnum<'db>,
  supertyp: &TdTypEnum<'db>,
  env: &mut SubtypEnv<'db>,
) -> bool {
  // Phase 1: Check type constructor compatibility ignoring type arguments and parameter variance
  fn are_constructors_compatible<'db>(
    db: &'db TypedownDatabase,
    subtyp: &TdTypEnum<'db>, // INVARIANT: Due to sum type elimination, sub type cannot be a sum type here
    supertyp: &TdTypEnum<'db>,
    env: &mut SubtypEnv<'db>,
  ) -> bool {
    if subtyp.as_id() == supertyp.as_id() {
      return true;
    }
    debug_assert!(
      !subtyp.is_td_metatype_instance_typ()
        && !subtyp.is_td_free_variable_typ()
        && !subtyp.is_td_existential_typ(),
      "InstanceTyp/VariableTyp/ExistentialTyp must be handled in phase 0"
    );
    debug_assert!(
      !supertyp.is_td_metatype_instance_typ()
        && !supertyp.is_td_free_variable_typ()
        && !supertyp.is_td_existential_typ(),
      "InstanceTyp/VariableTyp/ExistentialTyp must be handled in phase 0"
    );
    match supertyp {
      TdTypEnum::TdObjTyp(_) | TdTypEnum::TdTypTyp(_) => true,
      TdTypEnum::TdNeverTyp(_) => false,
      // WARNING: This only works because subtype is not a sum type
      TdTypEnum::TdSumTyp(sum) => sum.members(db).iter().any(|member| {
        member
          .resolve(db)
          .is_some_and(|member| is_subtyp_of_env(db, subtyp, &member, env))
      }),
      TdTypEnum::TdLitTyp(_) => false,
      TdTypEnum::TdStringTyp(_) => {
        matches!(
          subtyp,
          TdTypEnum::TdLitTyp(lit)
            if matches!(lit.underlying_typ(db), TdTypEnum::TdStringTyp(_))
        ) || matches!(
          subtyp,
          TdTypEnum::TdDateTimeTyp(_)
            | TdTypEnum::TdDateTyp(_)
            | TdTypEnum::TdTimeTyp(_)
            | TdTypEnum::TdBlobTyp(_)
        )
      }
      TdTypEnum::TdNumberTyp(_) => matches!(
        subtyp,
        TdTypEnum::TdLitTyp(lit)
          if matches!(lit.underlying_typ(db), TdTypEnum::TdNumberTyp(_))
      ),
      TdTypEnum::TdBoolTyp(_) => matches!(
        subtyp,
        TdTypEnum::TdLitTyp(lit)
          if matches!(lit.underlying_typ(db), TdTypEnum::TdBoolTyp(_))
      ),
      TdTypEnum::TdListTyp(_) => matches!(subtyp, TdTypEnum::TdListTyp(_)),
      TdTypEnum::TdDictTyp(expected_dict) => match subtyp {
        TdTypEnum::TdDictTyp(_) => true,
        TdTypEnum::TdProductTyp(product) => {
          let value_typ = match expected_dict.value(db).and_then(|lit| lit.resolve(db)) {
            Some(value_typ) => value_typ,
            None => return true,
          };
          product.get_fields(db).values().all(|field_lazy| {
            field_lazy
              .resolve(db)
              .is_some_and(|field_typ| is_subtyp_of_env(db, &field_typ, &value_typ, env))
          })
        }
        TdTypEnum::TdSchemaTyp(schema) => {
          let value_typ = match expected_dict.value(db).and_then(|lit| lit.resolve(db)) {
            Some(value_typ) => value_typ,
            None => return true,
          };
          schema.get_fields(db).values().all(|field_lazy| {
            field_lazy
              .resolve(db)
              .is_some_and(|field_typ| is_subtyp_of_env(db, &field_typ, &value_typ, env))
          })
        }
        _ => false,
      },
      TdTypEnum::TdFuncTyp(_) => matches!(subtyp, TdTypEnum::TdFuncTyp(_)),
      TdTypEnum::TdProductTyp(expected_product_typ) => match subtyp {
        // Product to product: structural subtyping
        TdTypEnum::TdProductTyp(product_typ) => fields_compatible(
          db,
          &expected_product_typ.get_fields(db),
          &product_typ.get_fields(db),
        ),
        // Schema to product: allowed
        TdTypEnum::TdSchemaTyp(schema_typ) => fields_compatible(
          db,
          &expected_product_typ.get_fields(db),
          &schema_typ.get_fields(db),
        ),
        _ => false,
      },
      TdTypEnum::TdSchemaTyp(expected_schema_typ) => match subtyp {
        // Schema to schema: nominal via extends chain
        TdTypEnum::TdSchemaTyp(schema_typ) => {
          nominal_subtyp_of(db, schema_typ, expected_schema_typ)
        }
        // Product to schema: never
        _ => false,
      },
      TdTypEnum::TdBlobTyp(_) => {
        matches!(subtyp, TdTypEnum::TdBlobTyp(_) | TdTypEnum::TdStringTyp(_))
      }
      TdTypEnum::TdMathTyp(_)
      | TdTypEnum::TdDateTimeTyp(_)
      | TdTypEnum::TdDateTyp(_)
      | TdTypEnum::TdTimeTyp(_)
      | TdTypEnum::TdNullTyp(_) => false,
      TdTypEnum::TdIconTyp(_) => matches!(subtyp, TdTypEnum::TdIconTyp(_)),
      TdTypEnum::TdSchemaMetaTyp(_) => matches!(
        subtyp,
        TdTypEnum::TdLitTyp(lit)
          if matches!(lit.underlying_typ(db), TdTypEnum::TdSchemaMetaTyp(_))
      ),
      // Handled in phase 0, should not reach here
      TdTypEnum::TdMetatypeInstanceTyp(_)
      | TdTypEnum::TdFreeVariableTyp(_)
      | TdTypEnum::TdBoundVariableTyp(_)
      | TdTypEnum::TdExistentialTyp(_)
      | TdTypEnum::TdExistentialVariableTyp(_) => false,
    }
  }

  // Phase 2: Check type arguments and parameter variance between compatible constructors
  fn are_typ_args_compatible<'db>(
    db: &'db TypedownDatabase,
    subtyp: &TdTypEnum<'db>,
    supertyp: &TdTypEnum<'db>,
    env: &mut SubtypEnv<'db>,
  ) -> bool {
    match (subtyp, supertyp) {
      (TdTypEnum::TdListTyp(subtyp_list), TdTypEnum::TdListTyp(supertyp_list)) => {
        match (
          subtyp_list
            .element(db)
            .and_then(|element| element.resolve(db)),
          supertyp_list
            .element(db)
            .and_then(|element| element.resolve(db)),
        ) {
          (_, None) => true,
          (None, Some(_)) => false,
          (Some(subtyp_element), Some(supertyp_element)) => {
            is_subtyp_of_env(db, &subtyp_element, &supertyp_element, env)
          }
        }
      }
      (TdTypEnum::TdDictTyp(_), TdTypEnum::TdDictTyp(_)) => {
        let supertyp_args = supertyp.get_typ_args(db);
        if supertyp_args.is_empty() {
          return true;
        }
        let subtyp_args = subtyp.get_typ_args(db);
        if subtyp_args.is_empty() {
          return false;
        }
        subtyp_args
          .iter()
          .zip(supertyp_args.iter())
          .all(|(sub_arg, super_arg)| is_subtyp_of_env(db, sub_arg, super_arg, env))
      }
      (TdTypEnum::TdFuncTyp(subtyp_func), TdTypEnum::TdFuncTyp(supertyp_func)) => {
        let subtyp_signature = subtyp_func.signature(db);
        let supertyp_signature = supertyp_func.signature(db);
        let subtyp_params = subtyp_signature.params(db);
        let supertyp_params = supertyp_signature.params(db);
        if subtyp_params.len() != supertyp_params.len() {
          return false;
        }
        // Function parameters are contravariant: supertyp_param must be a subtype of subtyp_param
        for (subtyp_param, supertyp_param) in subtyp_params.iter().zip(supertyp_params.iter()) {
          if !is_subtyp_of_env(db, supertyp_param, subtyp_param, env) {
            return false;
          }
        }
        // Return type is covariant: subtype return type must be a subtype of supertype return type
        is_subtyp_of_env(
          db,
          &subtyp_signature.ret(db),
          &supertyp_signature.ret(db),
          env,
        )
      }
      _ => true,
    }
  }

  // Phase 0: Special pre-check (instance types, variables, existentials)
  match (subtyp, supertyp) {
    // metatype_instance<T> on sub side (including when both sides are instance)
    (TdTypEnum::TdMetatypeInstanceTyp(sub_instance_typ), _) => {
      match &sub_instance_typ.inner(db) {
        // metatype_instance<lit<X>>: unwrap to X
        TdTypEnum::TdLitTyp(lit) => match lit.value(db) {
          LitValue::Typ(unwrapped) => is_subtyp_of_env(db, &unwrapped, supertyp, env),
          _ => is_subtyp_of_env(db, &lit.underlying_typ(db), supertyp, env),
        },
        // metatype_instance<schema_meta>: sum of all schema types
        TdTypEnum::TdSchemaMetaTyp(_) => match supertyp {
          // subtype of obj and type (top types)
          TdTypEnum::TdObjTyp(_) | TdTypEnum::TdTypTyp(_) => true,
          // subtype of metatype_instance<schema_meta|type|obj> (same or wider)
          TdTypEnum::TdMetatypeInstanceTyp(super_instance_typ) => matches!(
            &super_instance_typ.inner(db),
            TdTypEnum::TdSchemaMetaTyp(_) | TdTypEnum::TdTypTyp(_) | TdTypEnum::TdObjTyp(_)
          ),
          _ => false,
        },
        // metatype_instance<type>: all values (every value has a type)
        // metatype_instance<obj>: all values (every value is an obj)
        // These are equivalent
        TdTypEnum::TdTypTyp(_) | TdTypEnum::TdObjTyp(_) => match supertyp {
          // subtype of obj and type (top types)
          TdTypEnum::TdObjTyp(_) | TdTypEnum::TdTypTyp(_) => true,
          // subtype of metatype_instance<type|obj> (equivalent)
          TdTypEnum::TdMetatypeInstanceTyp(super_instance_typ) => matches!(
            &super_instance_typ.inner(db),
            TdTypEnum::TdTypTyp(_) | TdTypEnum::TdObjTyp(_)
          ),
          _ => false,
        },
        // metatype_instance<never>: uninhabited, subtype of everything
        TdTypEnum::TdNeverTyp(_) => true,
        // metatype_instance<instance<T>>: recurse
        TdTypEnum::TdMetatypeInstanceTyp(_) => {
          is_subtyp_of_env(db, &sub_instance_typ.inner(db), supertyp, env)
        }
        // metatype_instance<variable/existential>: unresolvable, treat as obj
        TdTypEnum::TdFreeVariableTyp(_)
        | TdTypEnum::TdBoundVariableTyp(_)
        | TdTypEnum::TdExistentialTyp(_)
        | TdTypEnum::TdExistentialVariableTyp(_) => {
          is_subtyp_of_env(db, &get_obj_typ(db).into(), supertyp, env)
        }
        // metatype_instance of non-metatype is never (uninhabited), subtype of everything
        _ => true,
      }
    }
    // metatype_instance<T> on super side
    (_, TdTypEnum::TdMetatypeInstanceTyp(super_instance_typ)) => {
      match &super_instance_typ.inner(db) {
        // metatype_instance<lit<X>>: sub must be subtype of X
        TdTypEnum::TdLitTyp(lit) => match lit.value(db) {
          LitValue::Typ(unwrapped) => is_subtyp_of_env(db, subtyp, &unwrapped, env),
          _ => is_subtyp_of_env(db, subtyp, &lit.underlying_typ(db), env),
        },
        // metatype_instance<schema_meta>: sub must be a schema type
        TdTypEnum::TdSchemaMetaTyp(_) => subtyp.as_td_schema_typ().is_some(),
        // metatype_instance<type|obj>: all values (every value has a type, every type is an obj)
        TdTypEnum::TdTypTyp(_) | TdTypEnum::TdObjTyp(_) => true,
        // metatype_instance<never>: nothing
        TdTypEnum::TdNeverTyp(_) => false,
        // metatype_instance<instance<T>>: recurse
        TdTypEnum::TdMetatypeInstanceTyp(_) => {
          is_subtyp_of_env(db, subtyp, &super_instance_typ.inner(db), env)
        }
        // metatype_instance<variable/existential>: unresolvable
        TdTypEnum::TdFreeVariableTyp(_)
        | TdTypEnum::TdBoundVariableTyp(_)
        | TdTypEnum::TdExistentialTyp(_)
        | TdTypEnum::TdExistentialVariableTyp(_) => false,
        // metatype_instance of non-metatype is never, nothing is its subtype
        _ => false,
      }
    }
    /* Bound variables: delegate to upper bound */
    (TdTypEnum::TdBoundVariableTyp(bound_sub), _) => {
      if let Some(upper) = TdBoundVariableTyp::upper_bound(*bound_sub, db).resolve(db) {
        is_subtyp_of_env(db, &upper, supertyp, env)
      } else {
        false
      }
    }
    (_, TdTypEnum::TdBoundVariableTyp(_)) => false,

    /* Free variables: from closure param inference, delegate to upper bound */
    (TdTypEnum::TdFreeVariableTyp(free_sub), _) => {
      if subtyp == supertyp {
        return true;
      }
      if let Some(upper) = TdFreeVariableTyp::upper_bound(*free_sub, db).resolve(db) {
        is_subtyp_of_env(db, &upper, supertyp, env)
      } else {
        false
      }
    }
    (_, TdTypEnum::TdFreeVariableTyp(_)) => false,

    /* Existential variables: witness variables from opened existentials */
    (TdTypEnum::TdExistentialVariableTyp(existential_sub), _) => {
      if subtyp == supertyp {
        return true;
      }
      env.add_upper_bound(db, *existential_sub, supertyp)
    }
    (_, TdTypEnum::TdExistentialVariableTyp(existential_super)) => {
      env.add_lower_bound(db, *existential_super, subtyp)
    }

    /* Existential types */
    // (exists T1 <: S1. P1[T1]) <: (exists T2 <: S2. P2[T2])
    // Special case of the below, can be merged
    (TdTypEnum::TdExistentialTyp(existential_subtyp), TdTypEnum::TdExistentialTyp(_)) => {
      existential_subtyp
        .body(db)
        .and_then(|body| body.resolve(db))
        .is_some_and(|body| is_subtyp_of_env(db, &body, supertyp, env))
    }
    // (exists T1 <: S1. P1[T1]) <: P2[T2]
    // P1[T1] must be <: P2[T2] for all T1 <: S1
    // Body is already unwrapped due to the type variable invariant
    (TdTypEnum::TdExistentialTyp(existential_subtyp), supertyp) => existential_subtyp
      .body(db)
      .and_then(|body| body.resolve(db))
      .is_some_and(|body| is_subtyp_of_env(db, &body, supertyp, env)),
    (subtyp, TdTypEnum::TdExistentialTyp(existential_supertyp)) => {
      // Open the existential: instantiate bound variables to existential variables
      let params = TdExistentialTyp::typ_params(*existential_supertyp, db).params(db);
      let Some(body) = existential_supertyp
        .body(db)
        .and_then(|body| body.resolve(db))
      else {
        return false;
      };
      let mut bindings: hashbrown::HashMap<usize, LazyTyp<'db>> = hashbrown::HashMap::new();
      for bound_variable in &params {
        let index = TdBoundVariableTyp::index(*bound_variable, db);
        let upper_bound = TdBoundVariableTyp::upper_bound(*bound_variable, db);
        let existential_variable =
          TdExistentialVariableTyp::new(db, *existential_supertyp, index, upper_bound.clone());
        bindings.insert(index, LazyTyp::eager(existential_variable.into()));
        if let Some(resolved_bound) = upper_bound.resolve(db) {
          env
            .upper_bounds
            .entry(existential_variable)
            .or_default()
            .push(resolved_bound);
        }
      }
      let opened_body = substitute_bound_variables(db, &body, &bindings);
      is_subtyp_of_env(db, subtyp, &opened_body, env)
    }
    _ => {
      if subtyp.as_td_never_typ().is_some() {
        return true;
      }

      // Sum type elimination
      if let Some(sum) = subtyp.as_td_sum_typ() {
        return sum.members(db).iter().all(|member| {
          member
            .resolve(db)
            .is_some_and(|typ| is_subtyp_of_env(db, &typ, supertyp, env))
        });
      }

      // It's sensible that subtype cannot be a sum type here

      // Phase 1: Type constructor compatibility check
      if !are_constructors_compatible(db, subtyp, supertyp, env) {
        return false;
      }

      // Phase 2: Same-nature parameter and variance check
      are_typ_args_compatible(db, subtyp, supertyp, env)
    }
  }
}

// Wrap a type in Sum(T, null)
pub fn make_nullable<'db>(db: &'db TypedownDatabase, typ: TdTypEnum<'db>) -> TdTypEnum<'db> {
  let null_typ: TdTypEnum = get_null_typ(db).into();
  let members = HashSet::from([LazyTyp::eager(typ), LazyTyp::eager(null_typ)]);
  TdSumTyp::new(db, members).into()
}

// Check if a type includes null
pub fn is_nullable(db: &TypedownDatabase, typ: &TdTypEnum) -> bool {
  if typ.as_td_null_typ().is_some() {
    return true;
  }
  if let Some(sum) = typ.as_td_sum_typ() {
    return sum
      .members(db)
      .iter()
      .filter_map(|member| member.resolve(db))
      .any(|typ| is_nullable(db, &typ));
  }
  false
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::db::derived::evaluate::evaluate_typ::evaluate_typ;
  use crate::db::derived::get_builtin_typs::{
    get_bool_typ, get_date_typ, get_datetime_typ, get_dict_typ, get_list_typ, get_lit_typ,
    get_never_typ, get_null_typ, get_number_typ, get_obj_typ, get_schema_meta_typ, get_string_typ,
    get_sum_typ, get_time_typ, get_typ_typ,
  };
  use crate::db::derived::name_resolver::file_symbol::file_symbol;
  use crate::db::fixtures::load_vault_fixture;
  use crate::db::types::{
    LazyTyp, LitValue, TdBoundVariableTyp, TdExistentialTyp, TdFreeVariableTyp, TdFuncTyp,
    TdProductTyp, TdSchemaTyp, Variance, make_property_descriptors,
  };
  use crate::db::{QueryStorage, TypedownDatabase};
  use std::collections::BTreeMap;

  fn make_db() -> TypedownDatabase {
    TypedownDatabase {
      storage: QueryStorage::default(),
    }
  }

  fn create_lit_string<'a>(db: &'a TypedownDatabase, value: &str) -> TdTypEnum<'a> {
    get_lit_typ(db, LitValue::String(value.to_string())).into()
  }

  fn create_lit_number<'a>(db: &'a TypedownDatabase, value: &str) -> TdTypEnum<'a> {
    get_lit_typ(db, LitValue::Number(value.to_string())).into()
  }

  fn create_sum_typ<'a>(db: &'a TypedownDatabase, members: Vec<TdTypEnum<'a>>) -> TdTypEnum<'a> {
    get_sum_typ(db, members.into_iter().map(LazyTyp::eager).collect()).into()
  }

  // Simple vs Simple

  #[test]
  fn compatible_simple_same_typ() {
    let db = make_db();
    let string: TdTypEnum = get_string_typ(&db).into();
    assert!(is_subtyp_of(&db, &string, &string));
  }

  #[test]
  fn incompatible_simple_different_typ() {
    let db = make_db();
    let string: TdTypEnum = get_string_typ(&db).into();
    let number: TdTypEnum = get_number_typ(&db).into();
    assert!(!is_subtyp_of(&db, &number, &string));
  }

  // Lit vs Simple

  #[test]
  fn lit_compatible_with_base_simple() {
    let db = make_db();
    let string: TdTypEnum = get_string_typ(&db).into();
    let lit = create_lit_string(&db, "hello");
    assert!(is_subtyp_of(&db, &lit, &string));
  }

  #[test]
  fn lit_incompatible_with_wrong_simple() {
    let db = make_db();
    let number: TdTypEnum = get_number_typ(&db).into();
    let lit = create_lit_string(&db, "hello");
    assert!(!is_subtyp_of(&db, &lit, &number));
  }

  // Lit vs Lit

  #[test]
  fn lit_compatible_same_value() {
    let db = make_db();
    let lit1 = create_lit_string(&db, "draft");
    let lit2 = create_lit_string(&db, "draft");
    assert!(is_subtyp_of(&db, &lit2, &lit1));
  }

  #[test]
  fn lit_incompatible_different_value() {
    let db = make_db();
    let lit1 = create_lit_string(&db, "draft");
    let lit2 = create_lit_string(&db, "published");
    assert!(!is_subtyp_of(&db, &lit2, &lit1));
  }

  // Never is the bottom type: assignable to anything, but nothing is assignable to it

  #[test]
  fn never_is_bottom_typ() {
    let db = make_db();
    let string_typ: TdTypEnum = get_string_typ(&db).into();
    let never_typ: TdTypEnum = get_never_typ(&db).into();
    assert!(!is_subtyp_of(&db, &string_typ, &never_typ));
    assert!(is_subtyp_of(&db, &never_typ, &string_typ));
  }

  #[test]
  fn lit_numberber_compatible_with_number() {
    let db = make_db();
    let number_typ: TdTypEnum = get_number_typ(&db).into();
    let lit = create_lit_number(&db, "42");
    assert!(is_subtyp_of(&db, &lit, &number_typ));
  }

  #[test]
  fn lit_numberber_incompatible_with_string() {
    let db = make_db();
    let string_typ: TdTypEnum = get_string_typ(&db).into();
    let lit_typ = create_lit_number(&db, "42");
    assert!(!is_subtyp_of(&db, &lit_typ, &string_typ));
  }

  #[test]
  fn string_accepts_string_lit() {
    let db = make_db();
    let string_typ: TdTypEnum = get_string_typ(&db).into();
    let lit_typ: TdTypEnum = get_lit_typ(&db, LitValue::String("hello".to_string())).into();
    assert!(is_subtyp_of(&db, &lit_typ, &string_typ));
  }

  #[test]
  fn number_accepts_number_lit() {
    let db = make_db();
    let number_typ: TdTypEnum = get_number_typ(&db).into();
    let lit_typ: TdTypEnum = get_lit_typ(&db, LitValue::Number("42".to_string())).into();
    assert!(is_subtyp_of(&db, &lit_typ, &number_typ));
  }

  #[test]
  fn boolean_accepts_boolean_literal() {
    let db = make_db();
    let bool_typ: TdTypEnum = get_bool_typ(&db).into();
    let lit_typ: TdTypEnum = get_lit_typ(&db, LitValue::Bool(true)).into();
    assert!(is_subtyp_of(&db, &lit_typ, &bool_typ));
  }

  #[test]
  fn string_rejects_number_literal() {
    let db = make_db();
    let string_typ: TdTypEnum = get_string_typ(&db).into();
    let lit_typ: TdTypEnum = get_lit_typ(&db, LitValue::Number("42".to_string())).into();
    assert!(!is_subtyp_of(&db, &lit_typ, &string_typ));
  }

  #[test]
  fn literal_accepts_same_value() {
    let db = make_db();
    let lit_typ1: TdTypEnum = get_lit_typ(&db, LitValue::String("draft".to_string())).into();
    let lit_typ2: TdTypEnum = get_lit_typ(&db, LitValue::String("draft".to_string())).into();
    assert!(is_subtyp_of(&db, &lit_typ2, &lit_typ1));
  }

  #[test]
  fn literal_rejects_different_value() {
    let db = make_db();
    let lit_typ1: TdTypEnum = get_lit_typ(&db, LitValue::String("draft".to_string())).into();
    let lit_typ2: TdTypEnum = get_lit_typ(&db, LitValue::String("published".to_string())).into();
    assert!(!is_subtyp_of(&db, &lit_typ2, &lit_typ1));
  }

  // Sum type tests

  #[test]
  fn sum_accepts_member_typ() {
    let db = make_db();
    let string_or_number_typ = create_sum_typ(
      &db,
      vec![get_string_typ(&db).into(), get_number_typ(&db).into()],
    );
    let string_typ: TdTypEnum = get_string_typ(&db).into();
    assert!(is_subtyp_of(&db, &string_typ, &string_or_number_typ));
  }

  #[test]
  fn sum_rejects_non_member_typ() {
    let db = make_db();
    let string_or_number_typ = create_sum_typ(
      &db,
      vec![get_string_typ(&db).into(), get_number_typ(&db).into()],
    );
    let boolean_typ: TdTypEnum = get_bool_typ(&db).into();
    assert!(!is_subtyp_of(&db, &boolean_typ, &string_or_number_typ));
  }

  #[test]
  fn sum_accepts_lit_of_member_typ() {
    let db = make_db();
    let string_or_number_typ = create_sum_typ(
      &db,
      vec![get_string_typ(&db).into(), get_number_typ(&db).into()],
    );
    let lit = create_lit_string(&db, "hello");
    assert!(is_subtyp_of(&db, &lit, &string_or_number_typ));
  }

  #[test]
  fn sum_accepts_subsum() {
    let db = make_db();
    let string_or_number_typ = create_sum_typ(
      &db,
      vec![get_string_typ(&db).into(), get_number_typ(&db).into()],
    );
    let just_string_typ = create_sum_typ(&db, vec![get_string_typ(&db).into()]);
    assert!(is_subtyp_of(&db, &just_string_typ, &string_or_number_typ));
  }

  #[test]
  fn sum_rejects_wider_sum() {
    let db = make_db();
    let just_string_typ = create_sum_typ(&db, vec![get_string_typ(&db).into()]);
    let string_or_number_typ = create_sum_typ(
      &db,
      vec![get_string_typ(&db).into(), get_number_typ(&db).into()],
    );
    assert!(!is_subtyp_of(&db, &string_or_number_typ, &just_string_typ));
  }

  // Null type tests

  #[test]
  fn nullable_accepts_null() {
    let db = make_db();
    let nullable_string_typ = create_sum_typ(
      &db,
      vec![get_string_typ(&db).into(), get_null_typ(&db).into()],
    );
    let null_typ: TdTypEnum = get_null_typ(&db).into();
    assert!(is_subtyp_of(&db, &null_typ, &nullable_string_typ));
  }

  #[test]
  fn nullable_accepts_base_type() {
    let db = make_db();
    let nullable_string_typ = create_sum_typ(
      &db,
      vec![get_string_typ(&db).into(), get_null_typ(&db).into()],
    );
    let string_typ: TdTypEnum = get_string_typ(&db).into();
    assert!(is_subtyp_of(&db, &string_typ, &nullable_string_typ));
  }

  #[test]
  fn non_nullable_rejects_null() {
    let db = make_db();
    let string_typ: TdTypEnum = get_string_typ(&db).into();
    let null_typ: TdTypEnum = get_null_typ(&db).into();
    assert!(!is_subtyp_of(&db, &null_typ, &string_typ));
  }

  // Never type tests

  #[test]
  fn never_accepted_by_any_type() {
    let db = make_db();
    let never_typ: TdTypEnum = get_never_typ(&db).into();
    let string_typ: TdTypEnum = get_string_typ(&db).into();
    let number_typ: TdTypEnum = get_number_typ(&db).into();
    let boolean_typ: TdTypEnum = get_bool_typ(&db).into();
    assert!(is_subtyp_of(&db, &never_typ, &string_typ));
    assert!(is_subtyp_of(&db, &never_typ, &number_typ));
    assert!(is_subtyp_of(&db, &never_typ, &boolean_typ));
  }

  #[test]
  fn never_accepted_by_sum() {
    let db = make_db();
    let never_typ: TdTypEnum = get_never_typ(&db).into();
    let string_or_number_typ = create_sum_typ(
      &db,
      vec![get_string_typ(&db).into(), get_number_typ(&db).into()],
    );
    assert!(is_subtyp_of(&db, &never_typ, &string_or_number_typ));
  }

  #[test]
  fn nothing_accepted_by_never() {
    let db = make_db();
    let never_typ: TdTypEnum = get_never_typ(&db).into();
    let string_typ: TdTypEnum = get_string_typ(&db).into();
    assert!(!is_subtyp_of(&db, &string_typ, &never_typ));
  }

  // Lit sum (enum) tests

  #[test]
  fn literal_sum_accepts_matching_literal() {
    let db = make_db();
    let status_typ = create_sum_typ(
      &db,
      vec![
        create_lit_string(&db, "draft"),
        create_lit_string(&db, "published"),
      ],
    );
    let draft_typ = create_lit_string(&db, "draft");
    assert!(is_subtyp_of(&db, &draft_typ, &status_typ));
  }

  #[test]
  fn literal_sum_rejects_non_matching_literal() {
    let db = make_db();
    let status_typ = create_sum_typ(
      &db,
      vec![
        create_lit_string(&db, "draft"),
        create_lit_string(&db, "published"),
      ],
    );
    let archived_typ = create_lit_string(&db, "archived");
    assert!(!is_subtyp_of(&db, &archived_typ, &status_typ));
  }

  // String accepts sum of string literals
  #[test]
  fn string_accepts_sum_of_string_literals() {
    let db = make_db();
    let string_typ: TdTypEnum = get_string_typ(&db).into();
    let status_typ = create_sum_typ(
      &db,
      vec![
        create_lit_string(&db, "draft"),
        create_lit_string(&db, "published"),
      ],
    );
    assert!(is_subtyp_of(&db, &status_typ, &string_typ));
  }

  // String rejects sum with non-string member
  #[test]
  fn string_rejects_mixed_sum() {
    let db = make_db();
    let string_typ: TdTypEnum = get_string_typ(&db).into();
    let mixed_typ = create_sum_typ(
      &db,
      vec![create_lit_string(&db, "draft"), get_number_typ(&db).into()],
    );
    assert!(!is_subtyp_of(&db, &mixed_typ, &string_typ));
  }

  // List type tests

  #[test]
  fn list_accepts_same_element_typ() {
    let db = make_db();
    let list_string_typ: TdTypEnum = get_list_typ(&db)
      .instantiate(&db, vec![LazyTyp::eager(get_string_typ(&db).into())])
      .typ(&db);
    let list_string_typ2: TdTypEnum = get_list_typ(&db)
      .instantiate(&db, vec![LazyTyp::eager(get_string_typ(&db).into())])
      .typ(&db);
    assert!(is_subtyp_of(&db, &list_string_typ2, &list_string_typ));
  }

  #[test]
  fn list_accepts_covariant_element_typ() {
    let db = make_db();
    let list_string_typ: TdTypEnum = get_list_typ(&db)
      .instantiate(&db, vec![LazyTyp::eager(get_string_typ(&db).into())])
      .typ(&db);
    let list_lit_typ: TdTypEnum = get_list_typ(&db)
      .instantiate(&db, vec![LazyTyp::eager(create_lit_string(&db, "hello"))])
      .typ(&db);
    assert!(is_subtyp_of(&db, &list_lit_typ, &list_string_typ));
  }

  #[test]
  fn list_rejects_wider_element_typ() {
    let db = make_db();
    let list_string_typ: TdTypEnum = get_list_typ(&db)
      .instantiate(&db, vec![LazyTyp::eager(get_string_typ(&db).into())])
      .typ(&db);
    let list_lit_typ: TdTypEnum = get_list_typ(&db)
      .instantiate(&db, vec![LazyTyp::eager(create_lit_string(&db, "hello"))])
      .typ(&db);
    assert!(!is_subtyp_of(&db, &list_string_typ, &list_lit_typ));
  }

  #[test]
  fn list_rejects_different_element_typ() {
    let db = make_db();
    let list_string_typ: TdTypEnum = get_list_typ(&db)
      .instantiate(&db, vec![LazyTyp::eager(get_string_typ(&db).into())])
      .typ(&db);
    let list_number_typ: TdTypEnum = get_list_typ(&db)
      .instantiate(&db, vec![LazyTyp::eager(get_number_typ(&db).into())])
      .typ(&db);
    assert!(!is_subtyp_of(&db, &list_number_typ, &list_string_typ));
  }

  #[test]
  fn untyped_list_accepts_any_list() {
    let db = make_db();
    let untyped: TdTypEnum = get_list_typ(&db).into();
    let list_string_typ: TdTypEnum = get_list_typ(&db)
      .instantiate(&db, vec![LazyTyp::eager(get_string_typ(&db).into())])
      .typ(&db);
    assert!(is_subtyp_of(&db, &list_string_typ, &untyped));
  }

  #[test]
  fn typed_list_rejects_untyped_list() {
    let db = make_db();
    let untyped: TdTypEnum = get_list_typ(&db).into();
    let list_string_typ: TdTypEnum = get_list_typ(&db)
      .instantiate(&db, vec![LazyTyp::eager(get_string_typ(&db).into())])
      .typ(&db);
    assert!(!is_subtyp_of(&db, &untyped, &list_string_typ));
  }

  // Dict type tests

  #[test]
  fn dict_accepts_same_value_typ() {
    let db = make_db();
    let dict_string_typ: TdTypEnum = get_dict_typ(&db)
      .instantiate(
        &db,
        vec![
          LazyTyp::eager(get_string_typ(&db).into()),
          LazyTyp::eager(get_string_typ(&db).into()),
        ],
      )
      .typ(&db);
    let dict_string_typ2: TdTypEnum = get_dict_typ(&db)
      .instantiate(
        &db,
        vec![
          LazyTyp::eager(get_string_typ(&db).into()),
          LazyTyp::eager(get_string_typ(&db).into()),
        ],
      )
      .typ(&db);
    assert!(is_subtyp_of(&db, &dict_string_typ2, &dict_string_typ));
  }

  #[test]
  fn dict_accepts_covariant_key_and_value_typs() {
    let db = make_db();
    let dict_string_typ: TdTypEnum = get_dict_typ(&db)
      .instantiate(
        &db,
        vec![
          LazyTyp::eager(get_string_typ(&db).into()),
          LazyTyp::eager(get_string_typ(&db).into()),
        ],
      )
      .typ(&db);
    let dict_lit_typ: TdTypEnum = get_dict_typ(&db)
      .instantiate(
        &db,
        vec![
          LazyTyp::eager(create_lit_string(&db, "key")),
          LazyTyp::eager(create_lit_string(&db, "value")),
        ],
      )
      .typ(&db);
    assert!(is_subtyp_of(&db, &dict_lit_typ, &dict_string_typ));
  }

  #[test]
  fn dict_rejects_wider_value_type() {
    let db = make_db();
    let dict_string_typ: TdTypEnum = get_dict_typ(&db)
      .instantiate(
        &db,
        vec![
          LazyTyp::eager(get_string_typ(&db).into()),
          LazyTyp::eager(get_string_typ(&db).into()),
        ],
      )
      .typ(&db);
    let dict_lit_typ: TdTypEnum = get_dict_typ(&db)
      .instantiate(
        &db,
        vec![
          LazyTyp::eager(create_lit_string(&db, "key")),
          LazyTyp::eager(create_lit_string(&db, "value")),
        ],
      )
      .typ(&db);
    assert!(!is_subtyp_of(&db, &dict_string_typ, &dict_lit_typ));
  }

  #[test]
  fn dict_rejects_different_value_type() {
    let db = make_db();
    let dict_string_typ: TdTypEnum = get_dict_typ(&db)
      .instantiate(
        &db,
        vec![
          LazyTyp::eager(get_string_typ(&db).into()),
          LazyTyp::eager(get_string_typ(&db).into()),
        ],
      )
      .typ(&db);
    let dict_number_typ: TdTypEnum = get_dict_typ(&db)
      .instantiate(
        &db,
        vec![
          LazyTyp::eager(get_string_typ(&db).into()),
          LazyTyp::eager(get_number_typ(&db).into()),
        ],
      )
      .typ(&db);
    assert!(!is_subtyp_of(&db, &dict_number_typ, &dict_string_typ));
  }

  #[test]
  fn untyped_dict_accepts_typed_dict() {
    let db = make_db();
    let untyped: TdTypEnum = get_dict_typ(&db).into();
    let dict_string_typ: TdTypEnum = get_dict_typ(&db)
      .instantiate(
        &db,
        vec![
          LazyTyp::eager(get_string_typ(&db).into()),
          LazyTyp::eager(get_string_typ(&db).into()),
        ],
      )
      .typ(&db);
    assert!(is_subtyp_of(&db, &dict_string_typ, &untyped));
  }

  #[test]
  fn typed_dict_rejects_untyped_dict() {
    let db = make_db();
    let untyped: TdTypEnum = get_dict_typ(&db).into();
    let dict_string_typ: TdTypEnum = get_dict_typ(&db)
      .instantiate(
        &db,
        vec![
          LazyTyp::eager(get_string_typ(&db).into()),
          LazyTyp::eager(get_string_typ(&db).into()),
        ],
      )
      .typ(&db);
    assert!(!is_subtyp_of(&db, &untyped, &dict_string_typ));
  }

  // String accepts date/time subtypes

  #[test]
  fn string_accepts_date() {
    let db = make_db();
    let string_typ: TdTypEnum = get_string_typ(&db).into();
    let date_typ: TdTypEnum = get_date_typ(&db).into();
    assert!(is_subtyp_of(&db, &date_typ, &string_typ));
  }

  #[test]
  fn string_accepts_datetime() {
    let db = make_db();
    let string_typ: TdTypEnum = get_string_typ(&db).into();
    let datetime_typ: TdTypEnum = get_datetime_typ(&db).into();
    assert!(is_subtyp_of(&db, &datetime_typ, &string_typ));
  }

  #[test]
  fn string_accepts_time() {
    let db = make_db();
    let string_typ: TdTypEnum = get_string_typ(&db).into();
    let time_typ: TdTypEnum = get_time_typ(&db).into();
    assert!(is_subtyp_of(&db, &time_typ, &string_typ));
  }

  #[test]
  fn date_rejects_string() {
    let db = make_db();
    let date_typ: TdTypEnum = get_date_typ(&db).into();
    let string_typ: TdTypEnum = get_string_typ(&db).into();
    assert!(!is_subtyp_of(&db, &string_typ, &date_typ));
  }

  // Product type structural subtyping tests

  #[test]
  fn product_accepts_matching_product() {
    let db = make_db();
    let expected_typ: TdTypEnum = TdProductTyp::new(
      &db,
      None,
      BTreeMap::from([(
        "name".to_string(),
        LazyTyp::eager(get_string_typ(&db).into()),
      )]),
    )
    .into();
    let actual_typ: TdTypEnum = TdProductTyp::new(
      &db,
      None,
      BTreeMap::from([(
        "name".to_string(),
        LazyTyp::eager(get_string_typ(&db).into()),
      )]),
    )
    .into();
    assert!(is_subtyp_of(&db, &actual_typ, &expected_typ));
  }

  #[test]
  fn product_rejects_missing_required_field() {
    let db = make_db();
    let expected_typ: TdTypEnum = TdProductTyp::new(
      &db,
      None,
      BTreeMap::from([(
        "name".to_string(),
        LazyTyp::eager(get_string_typ(&db).into()),
      )]),
    )
    .into();
    let actual_typ: TdTypEnum = TdProductTyp::new(&db, None, BTreeMap::new()).into();
    assert!(!is_subtyp_of(&db, &actual_typ, &expected_typ));
  }

  #[test]
  fn product_accepts_missing_nullable_field() {
    let db = make_db();
    let nullable_string = create_sum_typ(
      &db,
      vec![get_string_typ(&db).into(), get_null_typ(&db).into()],
    );
    let expected_typ: TdTypEnum = TdProductTyp::new(
      &db,
      None,
      BTreeMap::from([("name".to_string(), LazyTyp::eager(nullable_string))]),
    )
    .into();
    let actual_typ: TdTypEnum = TdProductTyp::new(&db, None, BTreeMap::new()).into();
    assert!(is_subtyp_of(&db, &actual_typ, &expected_typ));
  }

  #[test]
  fn product_rejects_wrong_field_type() {
    let db = make_db();
    let expected_typ: TdTypEnum = TdProductTyp::new(
      &db,
      None,
      BTreeMap::from([(
        "name".to_string(),
        LazyTyp::eager(get_string_typ(&db).into()),
      )]),
    )
    .into();
    let actual_typ: TdTypEnum = TdProductTyp::new(
      &db,
      None,
      BTreeMap::from([(
        "name".to_string(),
        LazyTyp::eager(get_number_typ(&db).into()),
      )]),
    )
    .into();
    assert!(!is_subtyp_of(&db, &actual_typ, &expected_typ));
  }

  // Product accepts superset of fields
  #[test]
  fn product_accepts_superset_fields() {
    let db = make_db();
    let expected_typ: TdTypEnum = TdProductTyp::new(
      &db,
      None,
      BTreeMap::from([(
        "name".to_string(),
        LazyTyp::eager(get_string_typ(&db).into()),
      )]),
    )
    .into();
    let actual_typ: TdTypEnum = TdProductTyp::new(
      &db,
      None,
      BTreeMap::from([
        (
          "name".to_string(),
          LazyTyp::eager(get_string_typ(&db).into()),
        ),
        (
          "age".to_string(),
          LazyTyp::eager(get_number_typ(&db).into()),
        ),
      ]),
    )
    .into();
    assert!(is_subtyp_of(&db, &actual_typ, &expected_typ));
  }

  // Schema is assignable to matching product (one-way opaque)

  #[test]
  fn schema_accepts_matching_product() {
    let db = make_db();
    let schema_typ: TdTypEnum = TdSchemaTyp::new(
      &db,
      "Test".to_string(),
      BTreeMap::new(),
      make_property_descriptors(
        &db,
        BTreeMap::from([(
          "name".to_string(),
          LazyTyp::eager(get_string_typ(&db).into()),
        )]),
      ),
      BTreeMap::new(),
      None,
    )
    .into();
    let product_typ: TdTypEnum = TdProductTyp::new(
      &db,
      None,
      BTreeMap::from([(
        "name".to_string(),
        LazyTyp::eager(get_string_typ(&db).into()),
      )]),
    )
    .into();
    // Schema is assignable to a matching product
    assert!(is_subtyp_of(&db, &schema_typ, &product_typ));
    // Product is NOT assignable to schema
    assert!(!is_subtyp_of(&db, &product_typ, &schema_typ));
  }

  #[test]
  fn schema_rejects_product_with_wrong_field() {
    let db = make_db();
    let schema_typ: TdTypEnum = TdSchemaTyp::new(
      &db,
      "Test".to_string(),
      BTreeMap::new(),
      make_property_descriptors(
        &db,
        BTreeMap::from([(
          "name".to_string(),
          LazyTyp::eager(get_string_typ(&db).into()),
        )]),
      ),
      BTreeMap::new(),
      None,
    )
    .into();
    let product_typ: TdTypEnum = TdProductTyp::new(
      &db,
      None,
      BTreeMap::from([(
        "name".to_string(),
        LazyTyp::eager(get_string_typ(&db).into()),
      )]),
    )
    .into();
    // Schema -> product with matching fields is fine
    assert!(is_subtyp_of(&db, &schema_typ, &product_typ));
  }

  #[test]
  fn null_rejects_non_null() {
    let db = make_db();
    let null_typ: TdTypEnum = get_null_typ(&db).into();
    let string_typ: TdTypEnum = get_string_typ(&db).into();
    assert!(!is_subtyp_of(&db, &string_typ, &null_typ));
  }

  #[test]
  fn list_rejects_non_list() {
    let db = make_db();
    let list_string_typ: TdTypEnum = get_list_typ(&db)
      .instantiate(&db, vec![LazyTyp::eager(get_string_typ(&db).into())])
      .typ(&db);
    let string_typ: TdTypEnum = get_string_typ(&db).into();
    assert!(!is_subtyp_of(&db, &string_typ, &list_string_typ));
  }

  #[test]
  fn dict_rejects_non_dict() {
    let db = make_db();
    let dict_typ: TdTypEnum = get_dict_typ(&db)
      .instantiate(
        &db,
        vec![
          LazyTyp::eager(get_string_typ(&db).into()),
          LazyTyp::eager(get_string_typ(&db).into()),
        ],
      )
      .typ(&db);
    let string_typ: TdTypEnum = get_string_typ(&db).into();
    assert!(!is_subtyp_of(&db, &string_typ, &dict_typ));
  }

  // is_type tests

  #[test]
  fn builtin_typ_is_typ() {
    let db = make_db();
    let string_typ: TdTypEnum = get_string_typ(&db).into();
    assert!(!string_typ.is_typ(&db), "string is not a metatype");
  }

  #[test]
  fn typ_typ_is_typ() {
    let db = make_db();
    let typ_typ: TdTypEnum = get_typ_typ(&db).into();
    assert!(typ_typ.is_typ(&db), "type is a metatype");
  }

  #[test]
  fn schema_is_typ() {
    let db = make_db();
    let schema: TdTypEnum = get_schema_meta_typ(&db).into();
    assert!(schema.is_typ(&db), "schema is a metatype (subtype of type)");
  }

  // Function type variance

  #[test]
  fn func_accepts_same_signature() {
    let db = make_db();
    let string_typ: TdTypEnum = get_string_typ(&db).into();
    let number_typ: TdTypEnum = get_number_typ(&db).into();
    let func_typ = TdFuncTyp::get(&db, vec![string_typ.clone()], number_typ.clone());
    let func_typ2 = TdFuncTyp::get(&db, vec![string_typ], number_typ);
    let func_typ: TdTypEnum = func_typ.into();
    let func_typ2: TdTypEnum = func_typ2.into();
    assert!(is_subtyp_of(&db, &func_typ2, &func_typ));
  }

  #[test]
  fn func_accepts_covariant_return() {
    let db = make_db();
    let string_typ: TdTypEnum = get_string_typ(&db).into();
    let literal_typ: TdTypEnum = create_lit_string(&db, "hello");
    // fn(string) -> string should accept fn(string) -> literal "hello"
    let expected_typ = TdFuncTyp::get(&db, vec![string_typ.clone()], string_typ.clone());
    let actual_typ = TdFuncTyp::get(&db, vec![string_typ], literal_typ);
    let expected_typ: TdTypEnum = expected_typ.into();
    let actual_typ: TdTypEnum = actual_typ.into();
    assert!(is_subtyp_of(&db, &actual_typ, &expected_typ));
  }

  #[test]
  fn func_rejects_wider_return() {
    let db = make_db();
    let string_typ: TdTypEnum = get_string_typ(&db).into();
    let number_typ: TdTypEnum = get_number_typ(&db).into();
    // fn() -> string should reject fn() -> number
    let expected_typ = TdFuncTyp::get(&db, vec![], string_typ);
    let actual_typ = TdFuncTyp::get(&db, vec![], number_typ);
    let expected_typ: TdTypEnum = expected_typ.into();
    let actual_typ: TdTypEnum = actual_typ.into();
    assert!(!is_subtyp_of(&db, &actual_typ, &expected_typ));
  }

  #[test]
  fn func_accepts_contravariant_param() {
    let db = make_db();
    let string_typ: TdTypEnum = get_string_typ(&db).into();
    let literal_typ: TdTypEnum = create_lit_string(&db, "hello");
    let number_typ: TdTypEnum = get_number_typ(&db).into();
    // fn(literal "hello") -> number should accept fn(string) -> number
    // because string accepts literal "hello" (contravariant)
    let expected_typ = TdFuncTyp::get(&db, vec![literal_typ], number_typ.clone());
    let actual_typ = TdFuncTyp::get(&db, vec![string_typ], number_typ);
    let expected_typ: TdTypEnum = expected_typ.into();
    let actual_typ: TdTypEnum = actual_typ.into();
    assert!(is_subtyp_of(&db, &actual_typ, &expected_typ));
  }

  #[test]
  fn func_rejects_narrower_param() {
    let db = make_db();
    let string_typ: TdTypEnum = get_string_typ(&db).into();
    let literal_typ: TdTypEnum = create_lit_string(&db, "hello");
    let number_typ: TdTypEnum = get_number_typ(&db).into();
    // fn(string) -> number should reject fn(literal "hello") -> number
    // because literal "hello" does not accept string (too narrow)
    let expected_typ = TdFuncTyp::get(&db, vec![string_typ], number_typ.clone());
    let actual_typ = TdFuncTyp::get(&db, vec![literal_typ], number_typ);
    let expected_typ: TdTypEnum = expected_typ.into();
    let actual_typ: TdTypEnum = actual_typ.into();
    assert!(!is_subtyp_of(&db, &actual_typ, &expected_typ));
  }

  #[test]
  fn func_rejects_arity_mismatch() {
    let db = make_db();
    let string_typ: TdTypEnum = get_string_typ(&db).into();
    let number_typ: TdTypEnum = get_number_typ(&db).into();
    let expected_typ = TdFuncTyp::get(&db, vec![string_typ.clone()], number_typ.clone());
    let actual_typ = TdFuncTyp::get(&db, vec![string_typ, number_typ.clone()], number_typ);
    let expected_typ: TdTypEnum = expected_typ.into();
    let actual_typ: TdTypEnum = actual_typ.into();
    assert!(!is_subtyp_of(&db, &actual_typ, &expected_typ));
  }

  // Type variable (TdFreeVariableType) tests

  fn get_test_site(
    db: &TypedownDatabase,
    project: crate::db::types::Project,
    file: crate::db::types::File,
  ) -> crate::db::types::ProjectRedNode {
    crate::db::derived::parse_file::parse_file(db, project, file).ast(db)
  }

  #[test]
  fn typ_variable_strict_equality() {
    let (db, project, file) = load_vault_fixture("evaluate/my_vault", "valid_person.td");
    let site = get_test_site(&db, project, file);
    let string_typ: TdTypEnum = get_string_typ(&db).into();
    let number_typ: TdTypEnum = get_number_typ(&db).into();
    let variable1: TdTypEnum =
      TdFreeVariableTyp::new(&db, site.clone(), 0, LazyTyp::eager(string_typ)).into();
    let variable2: TdTypEnum =
      TdFreeVariableTyp::new(&db, site, 1, LazyTyp::eager(number_typ)).into();
    assert!(is_subtyp_of(&db, &variable1, &variable1));
    assert!(!is_subtyp_of(&db, &variable1, &variable2));
  }

  #[test]
  fn typ_variable_bounded_delegation() {
    let (db, project, file) = load_vault_fixture("evaluate/my_vault", "valid_person.td");
    let site = get_test_site(&db, project, file);
    let string_typ: TdTypEnum = get_string_typ(&db).into();
    let number_typ: TdTypEnum = get_number_typ(&db).into();
    let variable: TdTypEnum =
      TdFreeVariableTyp::new(&db, site, 0, LazyTyp::eager(string_typ.clone())).into();
    assert!(is_subtyp_of(&db, &variable, &string_typ));
    assert!(!is_subtyp_of(&db, &variable, &number_typ));
  }

  #[test]
  fn typ_variable_unbound_rejects_concrete_typ() {
    let (db, project, file) = load_vault_fixture("evaluate/my_vault", "valid_person.td");
    let site = get_test_site(&db, project, file);
    let string_typ: TdTypEnum = get_string_typ(&db).into();
    let variable: TdTypEnum =
      TdFreeVariableTyp::new(&db, site, 0, LazyTyp::eager(get_obj_typ(&db).into())).into();
    assert!(!is_subtyp_of(&db, &string_typ, &variable));
  }

  #[test]
  fn typ_variable_transitive_variable_bound() {
    let (db, project, file) = load_vault_fixture("evaluate/my_vault", "valid_person.td");
    let site = get_test_site(&db, project, file);
    let variable2: TdTypEnum = TdFreeVariableTyp::new(
      &db,
      site.clone(),
      1,
      LazyTyp::eager(get_obj_typ(&db).into()),
    )
    .into();
    let variable1: TdTypEnum =
      TdFreeVariableTyp::new(&db, site, 0, LazyTyp::eager(variable2.clone())).into();

    assert!(is_subtyp_of(&db, &variable1, &variable2));
    assert!(!is_subtyp_of(&db, &variable2, &variable1));
  }

  #[test]
  fn check_free_variable_different_sites_are_not_subtypes() {
    let (db, project, _) = load_vault_fixture("evaluate/my_vault", "valid_person.td");
    let files: Vec<_> = project.files(&db).values().copied().collect();
    let site_a = get_test_site(&db, project, files[0]);
    let site_b = get_test_site(&db, project, files[1]);
    let upper_bound = LazyTyp::eager(get_obj_typ(&db).into());

    // Same index and upper bound, different instantiation sites
    let variable_a: TdTypEnum = TdFreeVariableTyp::new(&db, site_a, 0, upper_bound.clone()).into();
    let variable_b: TdTypEnum = TdFreeVariableTyp::new(&db, site_b, 0, upper_bound).into();

    // Without instantiation site, these would collide and be subtypes of each other
    assert!(is_subtyp_of(&db, &variable_a, &variable_a));
    assert!(!is_subtyp_of(&db, &variable_a, &variable_b));
    assert!(!is_subtyp_of(&db, &variable_b, &variable_a));
  }

  // Existential type precheck test

  #[test]
  fn existential_supertyp_witnesses_body() {
    let db = make_db();
    let list_string_typ: TdTypEnum = get_list_typ(&db)
      .instantiate(&db, vec![LazyTyp::eager(get_string_typ(&db).into())])
      .typ(&db);
    let existential_typ: TdTypEnum = TdExistentialTyp::new(
      &db,
      TypParams::new(&db, vec![], vec![]),
      Some(LazyTyp::eager(list_string_typ.clone())),
    )
    .into();

    // matches body directly
    // List[string] <= exists. List[string]
    assert!(is_subtyp_of(&db, &list_string_typ, &existential_typ));
  }

  #[test]
  fn existential_subtyp_bounded_body() {
    let db = make_db();
    let string_typ: TdTypEnum = get_string_typ(&db).into();
    let object_typ: TdTypEnum = get_obj_typ(&db).into();

    // exists T0 <: string. T0
    let bound_variable_0_string = TdBoundVariableTyp::new(
      &db,
      0,
      LazyTyp::eager(string_typ.clone()),
      Variance::Covariant,
    );
    let existential_bound_typ: TdTypEnum = TdExistentialTyp::new(
      &db,
      TypParams::new(&db, vec![bound_variable_0_string], vec![]),
      Some(LazyTyp::eager(bound_variable_0_string.into())),
    )
    .into();

    // T0 delegates to upper bound string
    // exists T0 <: string. T0 <= string
    assert!(is_subtyp_of(&db, &existential_bound_typ, &string_typ));

    // exists T0 <: Object. List[T0]
    let bound_variable_0_obj = TdBoundVariableTyp::new(
      &db,
      0,
      LazyTyp::eager(object_typ.clone()),
      Variance::Covariant,
    );
    let list_bound_typ: TdTypEnum = get_list_typ(&db)
      .instantiate(&db, vec![LazyTyp::eager(bound_variable_0_obj.into())])
      .typ(&db);
    let existential_list_typ: TdTypEnum = TdExistentialTyp::new(
      &db,
      TypParams::new(&db, vec![bound_variable_0_obj], vec![]),
      Some(LazyTyp::eager(list_bound_typ)),
    )
    .into();
    let list_obj: TdTypEnum = get_list_typ(&db)
      .instantiate(&db, vec![LazyTyp::eager(object_typ)])
      .typ(&db);
    let list_string: TdTypEnum = get_list_typ(&db)
      .instantiate(&db, vec![LazyTyp::eager(string_typ)])
      .typ(&db);

    // Object <= Object holds
    // exists T0 <: Object. List[T0] <= List[Object]
    assert!(is_subtyp_of(&db, &existential_list_typ, &list_obj));

    // upper bound Object <= string fails
    // exists T0 <: Object. List[T0] <= List[string]
    assert!(!is_subtyp_of(&db, &existential_list_typ, &list_string));
  }

  #[test]
  fn existential_witness_bound_accumulation() {
    let db = make_db();
    let string_typ: TdTypEnum = get_string_typ(&db).into();
    let number_typ: TdTypEnum = get_number_typ(&db).into();
    let object_typ: TdTypEnum = get_obj_typ(&db).into();

    let bound_variable_0_obj = TdBoundVariableTyp::new(
      &db,
      0,
      LazyTyp::eager(object_typ.clone()),
      Variance::Covariant,
    );
    let variable: TdTypEnum = bound_variable_0_obj.into();

    let list_string: TdTypEnum = get_list_typ(&db)
      .instantiate(&db, vec![LazyTyp::eager(string_typ.clone())])
      .typ(&db);
    let list_variable: TdTypEnum = get_list_typ(&db)
      .instantiate(&db, vec![LazyTyp::eager(variable.clone())])
      .typ(&db);
    let existential_list_typ: TdTypEnum = TdExistentialTyp::new(
      &db,
      TypParams::new(&db, vec![bound_variable_0_obj], vec![]),
      Some(LazyTyp::eager(list_variable)),
    )
    .into();

    // solves witness T2 = string
    // List[string] <= exists T2 <: Object. List[T2]
    assert!(is_subtyp_of(&db, &list_string, &existential_list_typ));

    let bound_variable_0_num = TdBoundVariableTyp::new(
      &db,
      0,
      LazyTyp::eager(number_typ.clone()),
      Variance::Covariant,
    );
    let variable_number_bound: TdTypEnum = bound_variable_0_num.into();
    let list_variable_number: TdTypEnum = get_list_typ(&db)
      .instantiate(&db, vec![LazyTyp::eager(variable_number_bound)])
      .typ(&db);
    let existential_list_number_bound: TdTypEnum = TdExistentialTyp::new(
      &db,
      TypParams::new(&db, vec![bound_variable_0_num], vec![]),
      Some(LazyTyp::eager(list_variable_number)),
    )
    .into();

    // witness string violates bound number
    // List[string] <= exists T2 <: number. List[T2]
    assert!(!is_subtyp_of(
      &db,
      &list_string,
      &existential_list_number_bound
    ));

    let func_string_string: TdTypEnum =
      TdFuncTyp::get(&db, vec![string_typ.clone()], string_typ.clone()).into();
    let bound_variable_0_obj2 = TdBoundVariableTyp::new(
      &db,
      0,
      LazyTyp::eager(object_typ.clone()),
      Variance::Covariant,
    );
    let variable2: TdTypEnum = bound_variable_0_obj2.into();
    let func_variable_variable: TdTypEnum =
      TdFuncTyp::get(&db, vec![variable2.clone()], variable2.clone()).into();
    let existential_func: TdTypEnum = TdExistentialTyp::new(
      &db,
      TypParams::new(&db, vec![bound_variable_0_obj2], vec![]),
      Some(LazyTyp::eager(func_variable_variable)),
    )
    .into();

    // solves witness T2 = string across covariant & contravariant positions
    // func(string) -> string <= exists T2 <: Object. func(T2) -> T2
    assert!(is_subtyp_of(&db, &func_string_string, &existential_func));

    let func_number_string: TdTypEnum = TdFuncTyp::get(&db, vec![number_typ], string_typ).into();

    // lower bound string <= upper bound number is false
    // func(number) -> string <= exists T2 <: Object. func(T2) -> T2
    assert!(!is_subtyp_of(&db, &func_number_string, &existential_func));
  }

  #[test]
  fn existential_multi_param_and_nested() {
    let db = make_db();
    let string_typ: TdTypEnum = get_string_typ(&db).into();
    let number_typ: TdTypEnum = get_number_typ(&db).into();
    let object_typ: TdTypEnum = get_obj_typ(&db).into();

    let bound_variable_0_obj = TdBoundVariableTyp::new(
      &db,
      0,
      LazyTyp::eager(object_typ.clone()),
      Variance::Covariant,
    );
    let bound_variable_1_obj = TdBoundVariableTyp::new(
      &db,
      1,
      LazyTyp::eager(object_typ.clone()),
      Variance::Covariant,
    );
    let variable1: TdTypEnum = bound_variable_0_obj.into();
    let variable2: TdTypEnum = bound_variable_1_obj.into();

    let dict_variable: TdTypEnum = get_dict_typ(&db)
      .instantiate(
        &db,
        vec![LazyTyp::eager(variable1.clone()), LazyTyp::eager(variable2)],
      )
      .typ(&db);

    let existential_dict_typ: TdTypEnum = TdExistentialTyp::new(
      &db,
      TypParams::new(
        &db,
        vec![bound_variable_0_obj, bound_variable_1_obj],
        vec![],
      ),
      Some(LazyTyp::eager(dict_variable)),
    )
    .into();

    let dict_string_number: TdTypEnum = get_dict_typ(&db)
      .instantiate(
        &db,
        vec![
          LazyTyp::eager(string_typ.clone()),
          LazyTyp::eager(number_typ.clone()),
        ],
      )
      .typ(&db);

    // solves T1 = string and T2 = number
    // Dict[string, number] <= exists T1 <: Object, T2 <: Object. Dict[T1, T2]
    assert!(is_subtyp_of(
      &db,
      &dict_string_number,
      &existential_dict_typ
    ));

    let bound_variable_0_obj2 = TdBoundVariableTyp::new(
      &db,
      0,
      LazyTyp::eager(object_typ.clone()),
      Variance::Covariant,
    );
    let bound_variable_1_str = TdBoundVariableTyp::new(
      &db,
      1,
      LazyTyp::eager(string_typ.clone()),
      Variance::Covariant,
    );
    let variable1b: TdTypEnum = bound_variable_0_obj2.into();
    let variable2_string: TdTypEnum = bound_variable_1_str.into();
    let dict_variable_string_bound: TdTypEnum = get_dict_typ(&db)
      .instantiate(
        &db,
        vec![LazyTyp::eager(variable1b), LazyTyp::eager(variable2_string)],
      )
      .typ(&db);
    let existential_dict_string_bound: TdTypEnum = TdExistentialTyp::new(
      &db,
      TypParams::new(
        &db,
        vec![bound_variable_0_obj2, bound_variable_1_str],
        vec![],
      ),
      Some(LazyTyp::eager(dict_variable_string_bound)),
    )
    .into();

    // witness T2 = number violates bound string
    // Dict[string, number] <= exists T1 <: string, T2 <: string. Dict[T1, T2]
    assert!(!is_subtyp_of(
      &db,
      &dict_string_number,
      &existential_dict_string_bound
    ));
  }

  #[test]
  fn existential_vs_existential_subtyping() {
    let db = make_db();
    let string_typ: TdTypEnum = get_string_typ(&db).into();
    let object_typ: TdTypEnum = get_obj_typ(&db).into();

    let bound_variable_0_str = TdBoundVariableTyp::new(
      &db,
      0,
      LazyTyp::eager(string_typ.clone()),
      Variance::Covariant,
    );
    let variable_string: TdTypEnum = bound_variable_0_str.into();
    let list_variable_string: TdTypEnum = get_list_typ(&db)
      .instantiate(&db, vec![LazyTyp::eager(variable_string)])
      .typ(&db);

    let existential_string: TdTypEnum = TdExistentialTyp::new(
      &db,
      TypParams::new(&db, vec![bound_variable_0_str], vec![]),
      Some(LazyTyp::eager(list_variable_string)),
    )
    .into();

    let bound_variable_0_obj = TdBoundVariableTyp::new(
      &db,
      0,
      LazyTyp::eager(object_typ.clone()),
      Variance::Covariant,
    );
    let variable_obj: TdTypEnum = bound_variable_0_obj.into();
    let list_variable_obj: TdTypEnum = get_list_typ(&db)
      .instantiate(&db, vec![LazyTyp::eager(variable_obj)])
      .typ(&db);
    let existential_obj: TdTypEnum = TdExistentialTyp::new(
      &db,
      TypParams::new(&db, vec![bound_variable_0_obj], vec![]),
      Some(LazyTyp::eager(list_variable_obj)),
    )
    .into();

    // bound subsumption string <= Object holds
    // (exists T1 <: string. List[T1]) <= (exists T2 <: Object. List[T2])
    assert!(is_subtyp_of(&db, &existential_string, &existential_obj));

    // bound Object <= string is false
    // (exists T2 <: Object. List[T2]) <= (exists T1 <: string. List[T1])
    assert!(!is_subtyp_of(&db, &existential_obj, &existential_string));
  }

  #[test]
  fn bare_typ_and_existential_reciprocal_subtyping() {
    let db = make_db();
    let string_typ: TdTypEnum = get_string_typ(&db).into();
    let object_typ: TdTypEnum = get_obj_typ(&db).into();

    let bound_variable_0_obj = TdBoundVariableTyp::new(
      &db,
      0,
      LazyTyp::eager(object_typ.clone()),
      Variance::Covariant,
    );
    let variable_obj: TdTypEnum = bound_variable_0_obj.into();
    let list_variable_obj: TdTypEnum = get_list_typ(&db)
      .instantiate(&db, vec![LazyTyp::eager(variable_obj)])
      .typ(&db);
    let existential_obj: TdTypEnum = TdExistentialTyp::new(
      &db,
      TypParams::new(&db, vec![bound_variable_0_obj], vec![]),
      Some(LazyTyp::eager(list_variable_obj)),
    )
    .into();

    let list_string: TdTypEnum = get_list_typ(&db)
      .instantiate(&db, vec![LazyTyp::eager(string_typ.clone())])
      .typ(&db);

    // bare subtype solves witness T = string
    // List[string] <= exists T <: Object. List[T]
    assert!(is_subtyp_of(&db, &list_string, &existential_obj));

    let bound_variable_0_str = TdBoundVariableTyp::new(
      &db,
      0,
      LazyTyp::eager(string_typ.clone()),
      Variance::Covariant,
    );
    let variable_string: TdTypEnum = bound_variable_0_str.into();
    let list_variable_string: TdTypEnum = get_list_typ(&db)
      .instantiate(&db, vec![LazyTyp::eager(variable_string)])
      .typ(&db);
    let existential_string: TdTypEnum = TdExistentialTyp::new(
      &db,
      TypParams::new(&db, vec![bound_variable_0_str], vec![]),
      Some(LazyTyp::eager(list_variable_string)),
    )
    .into();

    let list_obj: TdTypEnum = get_list_typ(&db)
      .instantiate(&db, vec![LazyTyp::eager(object_typ)])
      .typ(&db);

    // existential subtype opens T with upper bound string <= Object
    // (exists T <: string. List[T]) <= List[Object]
    assert!(is_subtyp_of(&db, &existential_string, &list_obj));

    // witness Object violates bound string
    // List[Object] <= exists T <: string. List[T]
    assert!(!is_subtyp_of(&db, &list_obj, &existential_string));

    // open T bound Object <= string is false
    // (exists T <: Object. List[T]) <= List[string]
    assert!(!is_subtyp_of(&db, &existential_obj, &list_string));
  }

  /* lit<T> subtyping rules
  Rule 1: lit<A> <: lit<B> only if A == B
  Rule 2: lit<A> <: B iff A is an instance of B
  Rule 3: A <: lit<B> never (except identity or never) */

  fn create_lit_typ<'a>(db: &'a TypedownDatabase, typ: TdTypEnum<'a>) -> TdTypEnum<'a> {
    get_lit_typ(db, LitValue::Typ(typ)).into()
  }

  // Rule 1

  #[test]
  fn lit_typ_subtype_of_itself() {
    let db = make_db();
    let lit_string = create_lit_typ(&db, get_string_typ(&db).into());
    assert!(is_subtyp_of(&db, &lit_string, &lit_string));
  }

  #[test]
  fn lit_typ_not_subtype_of_different_lit_typ() {
    let db = make_db();
    let lit_string = create_lit_typ(&db, get_string_typ(&db).into());
    let lit_number = create_lit_typ(&db, get_number_typ(&db).into());
    assert!(!is_subtyp_of(&db, &lit_string, &lit_number));
    assert!(!is_subtyp_of(&db, &lit_number, &lit_string));
  }

  // Rule 2

  #[test]
  fn lit_typ_subtype_of_underlying() {
    let db = make_db();
    // string_typ has type TdTypTyp, so lit<string> <: type
    let lit_string = create_lit_typ(&db, get_string_typ(&db).into());
    let typ_typ: TdTypEnum = get_typ_typ(&db).into();
    assert!(is_subtyp_of(&db, &lit_string, &typ_typ));
  }

  #[test]
  fn lit_typ_subtype_of_obj() {
    let db = make_db();
    let lit_string = create_lit_typ(&db, get_string_typ(&db).into());
    let obj: TdTypEnum = get_obj_typ(&db).into();
    assert!(is_subtyp_of(&db, &lit_string, &obj));
  }

  #[test]
  fn lit_typ_not_subtype_of_unrelated() {
    let db = make_db();
    let lit_string = create_lit_typ(&db, get_string_typ(&db).into());
    let number: TdTypEnum = get_number_typ(&db).into();
    assert!(!is_subtyp_of(&db, &lit_string, &number));
  }

  #[test]
  fn lit_schema_meta_subtype_of_schema_meta_underlying() {
    let db = make_db();
    // schema_meta has type TdTypTyp, so lit<schema_meta> <: type
    let lit_schema_meta = create_lit_typ(&db, get_schema_meta_typ(&db).into());
    let typ_typ: TdTypEnum = get_typ_typ(&db).into();
    assert!(is_subtyp_of(&db, &lit_schema_meta, &typ_typ));
  }

  // Rule 3

  #[test]
  fn typ_not_subtype_of_lit_typ() {
    let db = make_db();
    let lit_string = create_lit_typ(&db, get_string_typ(&db).into());
    let typ_typ: TdTypEnum = get_typ_typ(&db).into();
    assert!(!is_subtyp_of(&db, &typ_typ, &lit_string));
  }

  #[test]
  fn obj_not_subtype_of_lit_typ() {
    let db = make_db();
    let lit_string = create_lit_typ(&db, get_string_typ(&db).into());
    let obj: TdTypEnum = get_obj_typ(&db).into();
    assert!(!is_subtyp_of(&db, &obj, &lit_string));
  }

  #[test]
  fn never_subtype_of_lit_typ() {
    let db = make_db();
    let lit_string = create_lit_typ(&db, get_string_typ(&db).into());
    let never: TdTypEnum = get_never_typ(&db).into();
    assert!(is_subtyp_of(&db, &never, &lit_string));
  }

  // Rule 2 with real schema: lit<Person> <: schema_meta

  #[test]
  fn lit_schema_subtype_of_schema_meta() {
    let (db, project, file) = load_vault_fixture("evaluate/my_vault", "_types/Person.td");
    let symbol = file_symbol(&db, project, file).value(&db).unwrap();
    let person_typ = evaluate_typ(&db, symbol).typ(&db).unwrap();

    let lit_person = create_lit_typ(&db, person_typ);
    let schema_meta: TdTypEnum = get_schema_meta_typ(&db).into();

    assert!(is_subtyp_of(&db, &lit_person, &schema_meta));
  }

  #[test]
  fn lit_schema_not_subtype_of_different_lit_schema() {
    let (db, project, _) = load_vault_fixture("evaluate/my_vault", "_types/Person.td");

    let person_file = *project
      .files(&db)
      .iter()
      .find(|(path, _)| path.ends_with("_types/Person.td"))
      .unwrap()
      .1;
    let student_file = *project
      .files(&db)
      .iter()
      .find(|(path, _)| path.ends_with("_types/Student.td"))
      .unwrap()
      .1;

    let person_symbol = file_symbol(&db, project, person_file).value(&db).unwrap();
    let student_symbol = file_symbol(&db, project, student_file).value(&db).unwrap();
    let person_typ = evaluate_typ(&db, person_symbol).typ(&db).unwrap();
    let student_typ = evaluate_typ(&db, student_symbol).typ(&db).unwrap();

    let lit_person = create_lit_typ(&db, person_typ);
    let lit_student = create_lit_typ(&db, student_typ);

    // lit<Person> != lit<Student> even though Student extends Person
    assert!(!is_subtyp_of(&db, &lit_person, &lit_student));
    assert!(!is_subtyp_of(&db, &lit_student, &lit_person));

    // but both are subtypes of schema_meta
    let schema_meta: TdTypEnum = get_schema_meta_typ(&db).into();
    assert!(is_subtyp_of(&db, &lit_person, &schema_meta));
    assert!(is_subtyp_of(&db, &lit_student, &schema_meta));
  }
}
