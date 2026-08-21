//! Shared type compatibility utilities for typechecking

use crate::db::TypedownDatabase;
use crate::db::types::derived::object_system::TdStaticType;
use crate::db::types::fields_compatible;
use crate::db::types::{LazyType, TdTypeEnum, TypeParams, TypeVariable};
use crate::syntax::diagnostic::Diagnostic;
use std::collections::{HashMap, HashSet};
use typedown_incremental::Id;

/// Validate type arguments against type parameters for both arity and bounds
pub fn validate_type_params(
  db: &TypedownDatabase,
  type_params: Option<&TypeParams>,
  args: &[LazyType],
) -> Vec<Diagnostic> {
  let expected_arity = type_params.map_or(0, |p| p.len(db));
  if expected_arity != args.len() {
    return vec![Diagnostic::WrongTypeArgCount {
      expected: expected_arity,
      got: args.len(),
    }];
  }

  let mut diagnostics = Vec::new();
  if let Some(params) = type_params {
    let params_vec = params.params(db);
    for (idx, (p, arg)) in params_vec.iter().zip(args.iter()).enumerate() {
      if let Some(bound) = p.upper_bound(db).resolve(db)
        && bound.as_td_object_type().is_none()
        && let Some(arg_type) = arg.resolve(db)
        && !is_subtype_of(db, &arg_type, &bound)
      {
        diagnostics.push(Diagnostic::TypeArgBoundViolation {
          index: idx,
          expected_bound: bound.display_name(db),
          got: arg_type.display_name(db),
        });
      }
    }
  }
  diagnostics
}

/// Check if `subtype` is a subtype of `supertype`
pub fn is_subtype_of(db: &TypedownDatabase, subtype: &TdTypeEnum, supertype: &TdTypeEnum) -> bool {
  let mut env = SubtypeEnv::new();
  is_subtype_of_env(db, subtype, supertype, &mut env)
}

/// Witness environment for existential subtyping constraints
#[derive(Default)]
struct SubtypeEnv {
  lower_bounds: HashMap<TypeVariable, Vec<TdTypeEnum>>,
  upper_bounds: HashMap<TypeVariable, Vec<TdTypeEnum>>,
  existential_variables: HashSet<TypeVariable>,
}

impl SubtypeEnv {
  fn new() -> Self {
    Self {
      lower_bounds: HashMap::new(),
      upper_bounds: HashMap::new(),
      existential_variables: HashSet::new(),
    }
  }

  /// Register an existential variable in the environment and push its declared upper bound
  fn track_existential_variable(&mut self, db: &TypedownDatabase, variable: TypeVariable) {
    self.existential_variables.insert(variable);
    if let Some(upper_bound) = variable.upper_bound(db).resolve(db) {
      self.add_upper_bound(db, variable, &upper_bound);
    }
  }

  /// Record a lower bound for an existential variable
  /// Returning `false` early if a conflict with any existing upper bound is detected (`lower <= upper` fails)
  fn add_lower_bound(
    &mut self,
    db: &TypedownDatabase,
    variable: TypeVariable,
    lower_bound: &TdTypeEnum,
  ) -> bool {
    if let Some(upper_bounds) = self.upper_bounds.get(&variable).cloned() {
      for upper_bound in &upper_bounds {
        if !is_subtype_of_env(db, lower_bound, upper_bound, self) {
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

  /// Record an upper bound for an existential variable
  /// Returning `false` early if a conflict with any existing lower bound is detected (`lower <= upper` fails)
  fn add_upper_bound(
    &mut self,
    db: &TypedownDatabase,
    variable: TypeVariable,
    upper_bound: &TdTypeEnum,
  ) -> bool {
    if let Some(lower_bounds) = self.lower_bounds.get(&variable).cloned() {
      for lower_bound in &lower_bounds {
        if !is_subtype_of_env(db, lower_bound, upper_bound, self) {
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

fn is_subtype_of_env(
  db: &TypedownDatabase,
  subtype: &TdTypeEnum,
  supertype: &TdTypeEnum,
  env: &mut SubtypeEnv,
) -> bool {
  // Phase 1: Check type constructor compatibility ignoring type arguments and parameter variance
  fn are_constructors_compatible(
    db: &TypedownDatabase,
    subtype: &TdTypeEnum, // INVARIANT: Due to sum type elimination, sub type cannot be a sum type here
    supertype: &TdTypeEnum,
    env: &mut SubtypeEnv,
  ) -> bool {
    if subtype.as_id() == supertype.as_id() {
      return true;
    }
    match supertype {
      TdTypeEnum::TdObjectType(_) | TdTypeEnum::TdTypeType(_) => true,
      TdTypeEnum::TdNeverType(_) => false,
      // WARNING: This only works because subtype is not a sum type
      TdTypeEnum::TdSumType(sum) => sum.members(db).iter().any(|member| {
        member
          .resolve(db)
          .is_some_and(|m| is_subtype_of_env(db, subtype, &m, env))
      }),
      TdTypeEnum::TdLiteralType(_) => false,
      TdTypeEnum::TdStrType(_) => {
        matches!(
          subtype,
          TdTypeEnum::TdLiteralType(lit)
            if matches!(lit.underlying_type(db), TdTypeEnum::TdStrType(_))
        ) || matches!(
          subtype,
          TdTypeEnum::TdDateTimeType(_) | TdTypeEnum::TdDateType(_) | TdTypeEnum::TdTimeType(_)
        )
      }
      TdTypeEnum::TdNumType(_) => matches!(
        subtype,
        TdTypeEnum::TdLiteralType(lit)
          if matches!(lit.underlying_type(db), TdTypeEnum::TdNumType(_))
      ),
      TdTypeEnum::TdBoolType(_) => matches!(
        subtype,
        TdTypeEnum::TdLiteralType(lit)
          if matches!(lit.underlying_type(db), TdTypeEnum::TdBoolType(_))
      ),
      TdTypeEnum::TdListType(_) => matches!(subtype, TdTypeEnum::TdListType(_)),
      TdTypeEnum::TdDictType(expected_dict) => match subtype {
        TdTypeEnum::TdDictType(_) => true,
        TdTypeEnum::TdProductType(product) => {
          let value_type = match expected_dict.value(db).and_then(|l| l.resolve(db)) {
            Some(vt) => vt,
            None => return true,
          };
          product.fields(db).values().all(|field_lazy| {
            field_lazy
              .resolve(db)
              .is_some_and(|ft| is_subtype_of_env(db, &ft, &value_type, env))
          })
        }
        TdTypeEnum::TdStructuralType(structural) => {
          let value_type = match expected_dict.value(db).and_then(|l| l.resolve(db)) {
            Some(vt) => vt,
            None => return true,
          };
          structural.fields(db).values().all(|field_lazy| {
            field_lazy
              .resolve(db)
              .is_some_and(|ft| is_subtype_of_env(db, &ft, &value_type, env))
          })
        }
        _ => false,
      },
      TdTypeEnum::TdFuncType(_) => matches!(subtype, TdTypeEnum::TdFuncType(_)),
      TdTypeEnum::TdProductType(expected_product) => match subtype {
        TdTypeEnum::TdProductType(product) => {
          fields_compatible(db, &expected_product.fields(db), &product.fields(db))
        }
        TdTypeEnum::TdStructuralType(structural) => {
          fields_compatible(db, &expected_product.fields(db), &structural.fields(db))
        }
        _ => false,
      },
      TdTypeEnum::TdStructuralType(expected_structural) => match subtype {
        TdTypeEnum::TdProductType(product) => {
          fields_compatible(db, &expected_structural.fields(db), &product.fields(db))
        }
        TdTypeEnum::TdStructuralType(structural) => {
          fields_compatible(db, &expected_structural.fields(db), &structural.fields(db))
        }
        _ => false,
      },
      TdTypeEnum::TdMathType(_)
      | TdTypeEnum::TdDateTimeType(_)
      | TdTypeEnum::TdDateType(_)
      | TdTypeEnum::TdTimeType(_)
      | TdTypeEnum::TdBlobType(_)
      | TdTypeEnum::TdNullType(_) => false,
      TdTypeEnum::TdVariableType(_) | TdTypeEnum::TdExistentialType(_) => false,
    }
  }

  // Phase 2: Check type arguments and parameter variance between compatible constructors
  fn are_type_args_compatible(
    db: &TypedownDatabase,
    subtype: &TdTypeEnum,
    supertype: &TdTypeEnum,
    env: &mut SubtypeEnv,
  ) -> bool {
    match (subtype, supertype) {
      (TdTypeEnum::TdListType(subtype_list), TdTypeEnum::TdListType(supertype_list)) => {
        match (
          subtype_list.elem(db).and_then(|e| e.resolve(db)),
          supertype_list.elem(db).and_then(|e| e.resolve(db)),
        ) {
          (_, None) => true,
          (None, Some(_)) => false,
          (Some(subtype_element), Some(supertype_element)) => {
            is_subtype_of_env(db, &subtype_element, &supertype_element, env)
          }
        }
      }
      (TdTypeEnum::TdDictType(_), TdTypeEnum::TdDictType(_)) => {
        let supertype_args = supertype.get_type_args(db);
        if supertype_args.is_empty() {
          return true;
        }
        let subtype_args = subtype.get_type_args(db);
        if subtype_args.is_empty() {
          return false;
        }
        subtype_args
          .iter()
          .zip(supertype_args.iter())
          .all(|(sub_arg, super_arg)| is_subtype_of_env(db, sub_arg, super_arg, env))
      }
      (TdTypeEnum::TdFuncType(subtype_func), TdTypeEnum::TdFuncType(supertype_func)) => {
        let subtype_sig = subtype_func.signature(db);
        let supertype_sig = supertype_func.signature(db);
        let subtype_params = subtype_sig.params(db);
        let supertype_params = supertype_sig.params(db);
        if subtype_params.len() != supertype_params.len() {
          return false;
        }
        // Function parameters are contravariant: supertype_param must be a subtype of subtype_param
        for (subtype_param, supertype_param) in subtype_params.iter().zip(supertype_params.iter()) {
          if !is_subtype_of_env(db, supertype_param, subtype_param, env) {
            return false;
          }
        }
        // Return type is covariant: subtype return type must be a subtype of supertype return type
        is_subtype_of_env(db, &subtype_sig.ret(db), &supertype_sig.ret(db), env)
      }
      _ => true,
    }
  }

  // Phase 0: Special pre-check (variables, existentials)
  match (subtype, supertype) {
    // Universal variables: T1 <: T2 requires identity or checking T1's declared upper bound.
    (TdTypeEnum::TdVariableType(variable_sub), TdTypeEnum::TdVariableType(_variable_super)) => {
      if subtype == supertype {
        return true;
      }
      let variable_sub = variable_sub.variable(db);
      if let Some(upper_bound) = variable_sub.upper_bound(db).resolve(db) {
        is_subtype_of_env(db, &upper_bound, supertype, env)
      } else {
        false
      }
    }
    // Subtype candidate is variable
    (TdTypeEnum::TdVariableType(variable_subtype), _) => {
      let variable_subtype = variable_subtype.variable(db);
      if env.existential_variables.contains(&variable_subtype) {
        // If existential variable, accumulate upper bound
        env.add_upper_bound(db, variable_subtype, supertype)
      } else if let Some(upper_bound) = variable_subtype.upper_bound(db).resolve(db) {
        // If parameterized type variable, proceed as normal type checking
        is_subtype_of_env(db, &upper_bound, supertype, env)
      } else {
        false
      }
    }
    // Supertype is variable
    (_, TdTypeEnum::TdVariableType(variable_supertype)) => {
      let variable_supertype = variable_supertype.variable(db);
      // Only existential variables on supertype accumulate lower bounds
      if env.existential_variables.contains(&variable_supertype) {
        env.add_lower_bound(db, variable_supertype, subtype)
      } else {
        false
      }
    }
    (TdTypeEnum::TdExistentialType(existential_subtype), TdTypeEnum::TdExistentialType(_)) => {
      // (exists T1 <: S1. P1[T1]) <: (exists T2 <: S2. P2[T2])
      // Well this is just a special case of the below case, technically can be merged...
      existential_subtype
        .body(db)
        .and_then(|b| b.resolve(db))
        .is_some_and(|body| is_subtype_of_env(db, &body, supertype, env))
    }
    (TdTypeEnum::TdExistentialType(existential_subtype), supertype) => {
      // (exists T1 <: S1. P1[T1]) <: P2[T2]
      // iff
      // - A value x of candidate type means that x is of type P1[T1] for some T1 <: S1
      // - This should imply that x is also of type P2[T2]
      // So basically, P1[T1] must be <: P2[T2] for all T1 <: S1 regardless

      // Well this is already resolved due to the type variable invariant listed above
      existential_subtype
        .body(db)
        .and_then(|b| b.resolve(db))
        .is_some_and(|body| is_subtype_of_env(db, &body, supertype, env))
    }
    (subtype, TdTypeEnum::TdExistentialType(existential_supertype)) => {
      // P1[T1] <: (exists T2 <: S2. P2[T2])
      // iff
      // P1[T1] is a subtype of some P2[T2] where T2 <: S2
      // We just proceed structural decomposition, then accumulate bounds
      let params = existential_supertype.type_params(db).params(db);
      for param in &params {
        env.track_existential_variable(db, *param);
      }
      existential_supertype
        .body(db)
        .and_then(|b| b.resolve(db))
        .is_some_and(|body| is_subtype_of_env(db, subtype, &body, env))
    }
    _ => {
      if subtype.as_td_never_type().is_some() {
        return true;
      }

      // Sum type elimination
      if let Some(sum) = subtype.as_td_sum_type() {
        return sum.members(db).iter().all(|m| {
          m.resolve(db)
            .is_some_and(|t| is_subtype_of_env(db, &t, supertype, env))
        });
      }

      // It's sensible that subtype cannot be a sum type here

      // Phase 1: Type constructor compatibility check
      if !are_constructors_compatible(db, subtype, supertype, env) {
        return false;
      }

      // Phase 2: Same-nature parameter and variance check
      are_type_args_compatible(db, subtype, supertype, env)
    }
  }
}

// Check if a type includes null
pub fn is_nullable(db: &TypedownDatabase, typ: &TdTypeEnum) -> bool {
  if typ.as_td_null_type().is_some() {
    return true;
  }
  if let Some(sum) = typ.as_td_sum_type() {
    return sum
      .members(db)
      .iter()
      .filter_map(|m| m.resolve(db))
      .any(|t| is_nullable(db, &t));
  }
  false
}

#[cfg(test)]
mod tests {
  use std::collections::HashMap;

  use super::*;
  use crate::db::derived::get_builtin_types::{
    get_bool_type, get_date_type, get_datetime_type, get_dict_type, get_list_type,
    get_literal_type, get_never_type, get_null_type, get_num_type, get_object_type,
    get_schema_type, get_str_type, get_sum_type, get_time_type, get_type_type,
  };
  use crate::db::types::{
    LazyType, LiteralValue, TdExistentialType, TdFuncType, TdProductType, TdStructuralType,
    TdVariableType, TypeVariable,
  };
  use crate::db::{QueryStorage, TypedownDatabase};

  fn db() -> TypedownDatabase {
    TypedownDatabase {
      storage: QueryStorage::default(),
    }
  }

  fn lit_str(db: &TypedownDatabase, val: &str) -> TdTypeEnum {
    get_literal_type(db, LiteralValue::Str(val.to_string())).into()
  }

  fn lit_num(db: &TypedownDatabase, val: &str) -> TdTypeEnum {
    get_literal_type(db, LiteralValue::Num(val.to_string())).into()
  }

  fn sum(db: &TypedownDatabase, members: Vec<TdTypeEnum>) -> TdTypeEnum {
    get_sum_type(db, members.into_iter().map(LazyType::eager).collect()).into()
  }

  // Simple vs Simple

  #[test]
  fn compatible_simple_same_type() {
    let db = db();
    let string: TdTypeEnum = get_str_type(&db).into();
    assert!(is_subtype_of(&db, &string, &string));
  }

  #[test]
  fn incompatible_simple_different_type() {
    let db = db();
    let string: TdTypeEnum = get_str_type(&db).into();
    let number: TdTypeEnum = get_num_type(&db).into();
    assert!(!is_subtype_of(&db, &number, &string));
  }

  // Literal vs Simple

  #[test]
  fn literal_compatible_with_base_simple() {
    let db = db();
    let string: TdTypeEnum = get_str_type(&db).into();
    let lit = lit_str(&db, "hello");
    assert!(is_subtype_of(&db, &lit, &string));
  }

  #[test]
  fn literal_incompatible_with_wrong_simple() {
    let db = db();
    let number: TdTypeEnum = get_num_type(&db).into();
    let lit = lit_str(&db, "hello");
    assert!(!is_subtype_of(&db, &lit, &number));
  }

  // Literal vs Literal

  #[test]
  fn literal_compatible_same_value() {
    let db = db();
    let lit1 = lit_str(&db, "draft");
    let lit2 = lit_str(&db, "draft");
    assert!(is_subtype_of(&db, &lit2, &lit1));
  }

  #[test]
  fn literal_incompatible_different_value() {
    let db = db();
    let lit1 = lit_str(&db, "draft");
    let lit2 = lit_str(&db, "published");
    assert!(!is_subtype_of(&db, &lit2, &lit1));
  }

  // Never is the bottom type: assignable to anything, but nothing is assignable to it

  #[test]
  fn never_is_bottom_type() {
    let db = db();
    let string: TdTypeEnum = get_str_type(&db).into();
    let never: TdTypeEnum = get_never_type(&db).into();
    assert!(!is_subtype_of(&db, &string, &never));
    assert!(is_subtype_of(&db, &never, &string));
  }

  #[test]
  fn literal_num_compatible_with_number() {
    let db = db();
    let number: TdTypeEnum = get_num_type(&db).into();
    let lit = lit_num(&db, "42");
    assert!(is_subtype_of(&db, &lit, &number));
  }

  #[test]
  fn literal_num_incompatible_with_string() {
    let db = db();
    let string: TdTypeEnum = get_str_type(&db).into();
    let lit = lit_num(&db, "42");
    assert!(!is_subtype_of(&db, &lit, &string));
  }

  #[test]
  fn string_accepts_string_literal() {
    let db = db();
    let string: TdTypeEnum = get_str_type(&db).into();
    let lit: TdTypeEnum = get_literal_type(&db, LiteralValue::Str("hello".to_string())).into();
    assert!(is_subtype_of(&db, &lit, &string));
  }

  #[test]
  fn number_accepts_number_literal() {
    let db = db();
    let number: TdTypeEnum = get_num_type(&db).into();
    let lit: TdTypeEnum = get_literal_type(&db, LiteralValue::Num("42".to_string())).into();
    assert!(is_subtype_of(&db, &lit, &number));
  }

  #[test]
  fn boolean_accepts_boolean_literal() {
    let db = db();
    let boolean: TdTypeEnum = get_bool_type(&db).into();
    let lit: TdTypeEnum = get_literal_type(&db, LiteralValue::Bool(true)).into();
    assert!(is_subtype_of(&db, &lit, &boolean));
  }

  #[test]
  fn string_rejects_number_literal() {
    let db = db();
    let string: TdTypeEnum = get_str_type(&db).into();
    let lit: TdTypeEnum = get_literal_type(&db, LiteralValue::Num("42".to_string())).into();
    assert!(!is_subtype_of(&db, &lit, &string));
  }

  #[test]
  fn literal_accepts_same_value() {
    let db = db();
    let lit1: TdTypeEnum = get_literal_type(&db, LiteralValue::Str("draft".to_string())).into();
    let lit2: TdTypeEnum = get_literal_type(&db, LiteralValue::Str("draft".to_string())).into();
    assert!(is_subtype_of(&db, &lit2, &lit1));
  }

  #[test]
  fn literal_rejects_different_value() {
    let db = db();
    let lit1: TdTypeEnum = get_literal_type(&db, LiteralValue::Str("draft".to_string())).into();
    let lit2: TdTypeEnum = get_literal_type(&db, LiteralValue::Str("published".to_string())).into();
    assert!(!is_subtype_of(&db, &lit2, &lit1));
  }

  // Sum type tests

  #[test]
  fn sum_accepts_member_type() {
    let db = db();
    let str_or_num = sum(
      &db,
      vec![get_str_type(&db).into(), get_num_type(&db).into()],
    );
    let string: TdTypeEnum = get_str_type(&db).into();
    assert!(is_subtype_of(&db, &string, &str_or_num));
  }

  #[test]
  fn sum_rejects_non_member_type() {
    let db = db();
    let str_or_num = sum(
      &db,
      vec![get_str_type(&db).into(), get_num_type(&db).into()],
    );
    let boolean: TdTypeEnum = get_bool_type(&db).into();
    assert!(!is_subtype_of(&db, &boolean, &str_or_num));
  }

  #[test]
  fn sum_accepts_literal_of_member_type() {
    let db = db();
    let str_or_num = sum(
      &db,
      vec![get_str_type(&db).into(), get_num_type(&db).into()],
    );
    let lit = lit_str(&db, "hello");
    assert!(is_subtype_of(&db, &lit, &str_or_num));
  }

  #[test]
  fn sum_accepts_sub_sum() {
    let db = db();
    let str_or_num = sum(
      &db,
      vec![get_str_type(&db).into(), get_num_type(&db).into()],
    );
    let just_string = sum(&db, vec![get_str_type(&db).into()]);
    assert!(is_subtype_of(&db, &just_string, &str_or_num));
  }

  #[test]
  fn sum_rejects_wider_sum() {
    let db = db();
    let just_string = sum(&db, vec![get_str_type(&db).into()]);
    let str_or_num = sum(
      &db,
      vec![get_str_type(&db).into(), get_num_type(&db).into()],
    );
    assert!(!is_subtype_of(&db, &str_or_num, &just_string));
  }

  // Null type tests

  #[test]
  fn nullable_accepts_null() {
    let db = db();
    let nullable_string = sum(
      &db,
      vec![get_str_type(&db).into(), get_null_type(&db).into()],
    );
    let null: TdTypeEnum = get_null_type(&db).into();
    assert!(is_subtype_of(&db, &null, &nullable_string));
  }

  #[test]
  fn nullable_accepts_base_type() {
    let db = db();
    let nullable_string = sum(
      &db,
      vec![get_str_type(&db).into(), get_null_type(&db).into()],
    );
    let string: TdTypeEnum = get_str_type(&db).into();
    assert!(is_subtype_of(&db, &string, &nullable_string));
  }

  #[test]
  fn non_nullable_rejects_null() {
    let db = db();
    let string: TdTypeEnum = get_str_type(&db).into();
    let null: TdTypeEnum = get_null_type(&db).into();
    assert!(!is_subtype_of(&db, &null, &string));
  }

  // Never type tests

  #[test]
  fn never_accepted_by_any_type() {
    let db = db();
    let never: TdTypeEnum = get_never_type(&db).into();
    let string: TdTypeEnum = get_str_type(&db).into();
    let number: TdTypeEnum = get_num_type(&db).into();
    let boolean: TdTypeEnum = get_bool_type(&db).into();
    assert!(is_subtype_of(&db, &never, &string));
    assert!(is_subtype_of(&db, &never, &number));
    assert!(is_subtype_of(&db, &never, &boolean));
  }

  #[test]
  fn never_accepted_by_sum() {
    let db = db();
    let never: TdTypeEnum = get_never_type(&db).into();
    let str_or_num = sum(
      &db,
      vec![get_str_type(&db).into(), get_num_type(&db).into()],
    );
    assert!(is_subtype_of(&db, &never, &str_or_num));
  }

  #[test]
  fn nothing_accepted_by_never() {
    let db = db();
    let never: TdTypeEnum = get_never_type(&db).into();
    let string: TdTypeEnum = get_str_type(&db).into();
    assert!(!is_subtype_of(&db, &string, &never));
  }

  // Literal sum (enum) tests

  #[test]
  fn literal_sum_accepts_matching_literal() {
    let db = db();
    let status = sum(&db, vec![lit_str(&db, "draft"), lit_str(&db, "published")]);
    let draft = lit_str(&db, "draft");
    assert!(is_subtype_of(&db, &draft, &status));
  }

  #[test]
  fn literal_sum_rejects_non_matching_literal() {
    let db = db();
    let status = sum(&db, vec![lit_str(&db, "draft"), lit_str(&db, "published")]);
    let archived = lit_str(&db, "archived");
    assert!(!is_subtype_of(&db, &archived, &status));
  }

  // String accepts sum of string literals
  #[test]
  fn string_accepts_sum_of_string_literals() {
    let db = db();
    let string: TdTypeEnum = get_str_type(&db).into();
    let status = sum(&db, vec![lit_str(&db, "draft"), lit_str(&db, "published")]);
    assert!(is_subtype_of(&db, &status, &string));
  }

  // String rejects sum with non-string member
  #[test]
  fn string_rejects_mixed_sum() {
    let db = db();
    let string: TdTypeEnum = get_str_type(&db).into();
    let mixed = sum(&db, vec![lit_str(&db, "draft"), get_num_type(&db).into()]);
    assert!(!is_subtype_of(&db, &mixed, &string));
  }

  // List type tests

  #[test]
  fn list_accepts_same_elem_type() {
    let db = db();
    let list_string: TdTypeEnum = get_list_type(&db)
      .instantiate(&db, vec![LazyType::eager(get_str_type(&db).into())])
      .typ(&db);
    let list_str2: TdTypeEnum = get_list_type(&db)
      .instantiate(&db, vec![LazyType::eager(get_str_type(&db).into())])
      .typ(&db);
    assert!(is_subtype_of(&db, &list_str2, &list_string));
  }

  #[test]
  fn list_accepts_covariant_elem_type() {
    let db = db();
    let list_string: TdTypeEnum = get_list_type(&db)
      .instantiate(&db, vec![LazyType::eager(get_str_type(&db).into())])
      .typ(&db);
    let list_lit: TdTypeEnum = get_list_type(&db)
      .instantiate(&db, vec![LazyType::eager(lit_str(&db, "hello"))])
      .typ(&db);
    assert!(is_subtype_of(&db, &list_lit, &list_string));
  }

  #[test]
  fn list_rejects_wider_elem_type() {
    let db = db();
    let list_string: TdTypeEnum = get_list_type(&db)
      .instantiate(&db, vec![LazyType::eager(get_str_type(&db).into())])
      .typ(&db);
    let list_lit: TdTypeEnum = get_list_type(&db)
      .instantiate(&db, vec![LazyType::eager(lit_str(&db, "hello"))])
      .typ(&db);
    assert!(!is_subtype_of(&db, &list_string, &list_lit));
  }

  #[test]
  fn list_rejects_different_elem_type() {
    let db = db();
    let list_string: TdTypeEnum = get_list_type(&db)
      .instantiate(&db, vec![LazyType::eager(get_str_type(&db).into())])
      .typ(&db);
    let list_num: TdTypeEnum = get_list_type(&db)
      .instantiate(&db, vec![LazyType::eager(get_num_type(&db).into())])
      .typ(&db);
    assert!(!is_subtype_of(&db, &list_num, &list_string));
  }

  #[test]
  fn untyped_list_accepts_any_list() {
    let db = db();
    let untyped: TdTypeEnum = get_list_type(&db).into();
    let list_string: TdTypeEnum = get_list_type(&db)
      .instantiate(&db, vec![LazyType::eager(get_str_type(&db).into())])
      .typ(&db);
    assert!(is_subtype_of(&db, &list_string, &untyped));
  }

  #[test]
  fn typed_list_rejects_untyped_list() {
    let db = db();
    let untyped: TdTypeEnum = get_list_type(&db).into();
    let list_string: TdTypeEnum = get_list_type(&db)
      .instantiate(&db, vec![LazyType::eager(get_str_type(&db).into())])
      .typ(&db);
    assert!(!is_subtype_of(&db, &untyped, &list_string));
  }

  // Dict type tests

  #[test]
  fn dict_accepts_same_value_type() {
    let db = db();
    let dict_string: TdTypeEnum = get_dict_type(&db)
      .instantiate(
        &db,
        vec![
          LazyType::eager(get_str_type(&db).into()),
          LazyType::eager(get_str_type(&db).into()),
        ],
      )
      .typ(&db);
    let dict_str2: TdTypeEnum = get_dict_type(&db)
      .instantiate(
        &db,
        vec![
          LazyType::eager(get_str_type(&db).into()),
          LazyType::eager(get_str_type(&db).into()),
        ],
      )
      .typ(&db);
    assert!(is_subtype_of(&db, &dict_str2, &dict_string));
  }

  #[test]
  fn dict_accepts_covariant_key_and_value_types() {
    let db = db();
    let dict_string: TdTypeEnum = get_dict_type(&db)
      .instantiate(
        &db,
        vec![
          LazyType::eager(get_str_type(&db).into()),
          LazyType::eager(get_str_type(&db).into()),
        ],
      )
      .typ(&db);
    let dict_lit: TdTypeEnum = get_dict_type(&db)
      .instantiate(
        &db,
        vec![
          LazyType::eager(lit_str(&db, "key")),
          LazyType::eager(lit_str(&db, "value")),
        ],
      )
      .typ(&db);
    assert!(is_subtype_of(&db, &dict_lit, &dict_string));
  }

  #[test]
  fn dict_rejects_wider_value_type() {
    let db = db();
    let dict_string: TdTypeEnum = get_dict_type(&db)
      .instantiate(
        &db,
        vec![
          LazyType::eager(get_str_type(&db).into()),
          LazyType::eager(get_str_type(&db).into()),
        ],
      )
      .typ(&db);
    let dict_lit: TdTypeEnum = get_dict_type(&db)
      .instantiate(
        &db,
        vec![
          LazyType::eager(lit_str(&db, "key")),
          LazyType::eager(lit_str(&db, "value")),
        ],
      )
      .typ(&db);
    assert!(!is_subtype_of(&db, &dict_string, &dict_lit));
  }

  #[test]
  fn dict_rejects_different_value_type() {
    let db = db();
    let dict_string: TdTypeEnum = get_dict_type(&db)
      .instantiate(
        &db,
        vec![
          LazyType::eager(get_str_type(&db).into()),
          LazyType::eager(get_str_type(&db).into()),
        ],
      )
      .typ(&db);
    let dict_num: TdTypeEnum = get_dict_type(&db)
      .instantiate(
        &db,
        vec![
          LazyType::eager(get_str_type(&db).into()),
          LazyType::eager(get_num_type(&db).into()),
        ],
      )
      .typ(&db);
    assert!(!is_subtype_of(&db, &dict_num, &dict_string));
  }

  #[test]
  fn untyped_dict_accepts_typed_dict() {
    let db = db();
    let untyped: TdTypeEnum = get_dict_type(&db).into();
    let dict_string: TdTypeEnum = get_dict_type(&db)
      .instantiate(
        &db,
        vec![
          LazyType::eager(get_str_type(&db).into()),
          LazyType::eager(get_str_type(&db).into()),
        ],
      )
      .typ(&db);
    assert!(is_subtype_of(&db, &dict_string, &untyped));
  }

  #[test]
  fn typed_dict_rejects_untyped_dict() {
    let db = db();
    let untyped: TdTypeEnum = get_dict_type(&db).into();
    let dict_string: TdTypeEnum = get_dict_type(&db)
      .instantiate(
        &db,
        vec![
          LazyType::eager(get_str_type(&db).into()),
          LazyType::eager(get_str_type(&db).into()),
        ],
      )
      .typ(&db);
    assert!(!is_subtype_of(&db, &untyped, &dict_string));
  }

  // String accepts date/time subtypes

  #[test]
  fn string_accepts_date() {
    let db = db();
    let string: TdTypeEnum = get_str_type(&db).into();
    let date: TdTypeEnum = get_date_type(&db).into();
    assert!(is_subtype_of(&db, &date, &string));
  }

  #[test]
  fn string_accepts_datetime() {
    let db = db();
    let string: TdTypeEnum = get_str_type(&db).into();
    let datetime: TdTypeEnum = get_datetime_type(&db).into();
    assert!(is_subtype_of(&db, &datetime, &string));
  }

  #[test]
  fn string_accepts_time() {
    let db = db();
    let string: TdTypeEnum = get_str_type(&db).into();
    let time: TdTypeEnum = get_time_type(&db).into();
    assert!(is_subtype_of(&db, &time, &string));
  }

  #[test]
  fn date_rejects_string() {
    let db = db();
    let date: TdTypeEnum = get_date_type(&db).into();
    let string: TdTypeEnum = get_str_type(&db).into();
    assert!(!is_subtype_of(&db, &string, &date));
  }

  // Structural type tests

  #[test]
  fn structural_accepts_matching_structural() {
    let db = db();
    let expected: TdTypeEnum = TdStructuralType::new(
      &db,
      HashMap::from([(
        "name".to_string(),
        LazyType::eager(get_str_type(&db).into()),
      )]),
    )
    .into();
    let actual: TdTypeEnum = TdStructuralType::new(
      &db,
      HashMap::from([(
        "name".to_string(),
        LazyType::eager(get_str_type(&db).into()),
      )]),
    )
    .into();
    assert!(is_subtype_of(&db, &actual, &expected));
  }

  #[test]
  fn structural_rejects_missing_required_field() {
    let db = db();
    let expected: TdTypeEnum = TdStructuralType::new(
      &db,
      HashMap::from([(
        "name".to_string(),
        LazyType::eager(get_str_type(&db).into()),
      )]),
    )
    .into();
    let actual: TdTypeEnum = TdStructuralType::new(&db, HashMap::new()).into();
    assert!(!is_subtype_of(&db, &actual, &expected));
  }

  #[test]
  fn structural_accepts_missing_nullable_field() {
    let db = db();
    let nullable_string = sum(
      &db,
      vec![get_str_type(&db).into(), get_null_type(&db).into()],
    );
    let expected: TdTypeEnum = TdStructuralType::new(
      &db,
      HashMap::from([("name".to_string(), LazyType::eager(nullable_string))]),
    )
    .into();
    let actual: TdTypeEnum = TdStructuralType::new(&db, HashMap::new()).into();
    assert!(is_subtype_of(&db, &actual, &expected));
  }

  #[test]
  fn structural_rejects_wrong_field_type() {
    let db = db();
    let expected: TdTypeEnum = TdStructuralType::new(
      &db,
      HashMap::from([(
        "name".to_string(),
        LazyType::eager(get_str_type(&db).into()),
      )]),
    )
    .into();
    let actual: TdTypeEnum = TdStructuralType::new(
      &db,
      HashMap::from([(
        "name".to_string(),
        LazyType::eager(get_num_type(&db).into()),
      )]),
    )
    .into();
    assert!(!is_subtype_of(&db, &actual, &expected));
  }

  // Structural accepts superset of fields
  #[test]
  fn structural_accepts_superset_fields() {
    let db = db();
    let expected: TdTypeEnum = TdStructuralType::new(
      &db,
      HashMap::from([(
        "name".to_string(),
        LazyType::eager(get_str_type(&db).into()),
      )]),
    )
    .into();
    let actual: TdTypeEnum = TdStructuralType::new(
      &db,
      HashMap::from([
        (
          "name".to_string(),
          LazyType::eager(get_str_type(&db).into()),
        ),
        ("age".to_string(), LazyType::eager(get_num_type(&db).into())),
      ]),
    )
    .into();
    assert!(is_subtype_of(&db, &actual, &expected));
  }

  // Product accepts structural

  #[test]
  fn product_accepts_matching_structural() {
    let db = db();
    let product: TdTypeEnum = TdProductType::new(
      &db,
      Some("Test".to_string()),
      get_type_type(&db).into(),
      HashMap::from([(
        "name".to_string(),
        LazyType::eager(get_str_type(&db).into()),
      )]),
      HashMap::new(),
    )
    .into();
    let structural: TdTypeEnum = TdStructuralType::new(
      &db,
      HashMap::from([(
        "name".to_string(),
        LazyType::eager(get_str_type(&db).into()),
      )]),
    )
    .into();
    assert!(is_subtype_of(&db, &structural, &product));
  }

  #[test]
  fn product_rejects_structural_with_wrong_field() {
    let db = db();
    let product: TdTypeEnum = TdProductType::new(
      &db,
      Some("Test".to_string()),
      get_type_type(&db).into(),
      HashMap::from([(
        "name".to_string(),
        LazyType::eager(get_str_type(&db).into()),
      )]),
      HashMap::new(),
    )
    .into();
    let structural: TdTypeEnum = TdStructuralType::new(
      &db,
      HashMap::from([(
        "name".to_string(),
        LazyType::eager(get_num_type(&db).into()),
      )]),
    )
    .into();
    assert!(!is_subtype_of(&db, &structural, &product));
  }

  #[test]
  fn product_rejects_structural_with_missing_required_field() {
    let db = db();
    let product: TdTypeEnum = TdProductType::new(
      &db,
      Some("Test".to_string()),
      get_type_type(&db).into(),
      HashMap::from([
        (
          "name".to_string(),
          LazyType::eager(get_str_type(&db).into()),
        ),
        ("age".to_string(), LazyType::eager(get_num_type(&db).into())),
      ]),
      HashMap::new(),
    )
    .into();
    let structural: TdTypeEnum = TdStructuralType::new(
      &db,
      HashMap::from([(
        "name".to_string(),
        LazyType::eager(get_str_type(&db).into()),
      )]),
    )
    .into();
    assert!(!is_subtype_of(&db, &structural, &product));
  }

  #[test]
  fn null_rejects_non_null() {
    let db = db();
    let null: TdTypeEnum = get_null_type(&db).into();
    let string: TdTypeEnum = get_str_type(&db).into();
    assert!(!is_subtype_of(&db, &string, &null));
  }

  #[test]
  fn list_rejects_non_list() {
    let db = db();
    let list_string: TdTypeEnum = get_list_type(&db)
      .instantiate(&db, vec![LazyType::eager(get_str_type(&db).into())])
      .typ(&db);
    let string: TdTypeEnum = get_str_type(&db).into();
    assert!(!is_subtype_of(&db, &string, &list_string));
  }

  #[test]
  fn dict_rejects_non_dict() {
    let db = db();
    let dict: TdTypeEnum = get_dict_type(&db)
      .instantiate(
        &db,
        vec![
          LazyType::eager(get_str_type(&db).into()),
          LazyType::eager(get_str_type(&db).into()),
        ],
      )
      .typ(&db);
    let string: TdTypeEnum = get_str_type(&db).into();
    assert!(!is_subtype_of(&db, &string, &dict));
  }

  // is_type tests

  #[test]
  fn builtin_type_is_type() {
    let db = db();
    let str_type: TdTypeEnum = get_str_type(&db).into();
    assert!(!str_type.is_type(&db), "string is not a metatype");
  }

  #[test]
  fn type_type_is_type() {
    let db = db();
    let type_type: TdTypeEnum = get_type_type(&db).into();
    assert!(type_type.is_type(&db), "type is a metatype");
  }

  #[test]
  fn schema_is_type() {
    let db = db();
    let schema: TdTypeEnum = get_schema_type(&db).into();
    assert!(
      schema.is_type(&db),
      "schema is a metatype (subtype of type)"
    );
  }

  // Function type variance

  #[test]
  fn func_accepts_same_signature() {
    let db = db();
    let string: TdTypeEnum = get_str_type(&db).into();
    let number: TdTypeEnum = get_num_type(&db).into();
    let func = TdFuncType::get(&db, vec![string.clone()], number.clone());
    let func2 = TdFuncType::get(&db, vec![string], number);
    let func_type: TdTypeEnum = func.into();
    let func2_type: TdTypeEnum = func2.into();
    assert!(is_subtype_of(&db, &func2_type, &func_type));
  }

  #[test]
  fn func_accepts_covariant_return() {
    let db = db();
    let string: TdTypeEnum = get_str_type(&db).into();
    let literal: TdTypeEnum = lit_str(&db, "hello");
    // fn(string) -> string should accept fn(string) -> literal "hello"
    let expected = TdFuncType::get(&db, vec![string.clone()], string.clone());
    let actual = TdFuncType::get(&db, vec![string], literal);
    let expected_type: TdTypeEnum = expected.into();
    let actual_type: TdTypeEnum = actual.into();
    assert!(is_subtype_of(&db, &actual_type, &expected_type));
  }

  #[test]
  fn func_rejects_wider_return() {
    let db = db();
    let string: TdTypeEnum = get_str_type(&db).into();
    let number: TdTypeEnum = get_num_type(&db).into();
    // fn() -> string should reject fn() -> number
    let expected = TdFuncType::get(&db, vec![], string);
    let actual = TdFuncType::get(&db, vec![], number);
    let expected_type: TdTypeEnum = expected.into();
    let actual_type: TdTypeEnum = actual.into();
    assert!(!is_subtype_of(&db, &actual_type, &expected_type));
  }

  #[test]
  fn func_accepts_contravariant_param() {
    let db = db();
    let string: TdTypeEnum = get_str_type(&db).into();
    let literal: TdTypeEnum = lit_str(&db, "hello");
    let number: TdTypeEnum = get_num_type(&db).into();
    // fn(literal "hello") -> number should accept fn(string) -> number
    // because string accepts literal "hello" (contravariant)
    let expected = TdFuncType::get(&db, vec![literal], number.clone());
    let actual = TdFuncType::get(&db, vec![string], number);
    let expected_type: TdTypeEnum = expected.into();
    let actual_type: TdTypeEnum = actual.into();
    assert!(is_subtype_of(&db, &actual_type, &expected_type));
  }

  #[test]
  fn func_rejects_narrower_param() {
    let db = db();
    let string: TdTypeEnum = get_str_type(&db).into();
    let literal: TdTypeEnum = lit_str(&db, "hello");
    let number: TdTypeEnum = get_num_type(&db).into();
    // fn(string) -> number should reject fn(literal "hello") -> number
    // because literal "hello" does not accept string (too narrow)
    let expected = TdFuncType::get(&db, vec![string], number.clone());
    let actual = TdFuncType::get(&db, vec![literal], number);
    let expected_type: TdTypeEnum = expected.into();
    let actual_type: TdTypeEnum = actual.into();
    assert!(!is_subtype_of(&db, &actual_type, &expected_type));
  }

  #[test]
  fn func_rejects_arity_mismatch() {
    let db = db();
    let string: TdTypeEnum = get_str_type(&db).into();
    let number: TdTypeEnum = get_num_type(&db).into();
    let expected = TdFuncType::get(&db, vec![string.clone()], number.clone());
    let actual = TdFuncType::get(&db, vec![string, number.clone()], number);
    let expected_type: TdTypeEnum = expected.into();
    let actual_type: TdTypeEnum = actual.into();
    assert!(!is_subtype_of(&db, &actual_type, &expected_type));
  }

  // Type variable (TdVariableType) tests

  #[test]
  fn type_variable_strict_equality() {
    let db = db();
    let string: TdTypeEnum = get_str_type(&db).into();
    let number: TdTypeEnum = get_num_type(&db).into();
    let type_variable_1 = TypeVariable::get(&db, Some(LazyType::eager(string)));
    let type_variable_2 = TypeVariable::get(&db, Some(LazyType::eager(number)));
    let variable_1: TdTypeEnum = TdVariableType::new(&db, 0, type_variable_1).into();
    let variable_2: TdTypeEnum = TdVariableType::new(&db, 1, type_variable_2).into();
    assert!(is_subtype_of(&db, &variable_1, &variable_1));
    assert!(!is_subtype_of(&db, &variable_1, &variable_2));
  }

  #[test]
  fn type_variable_bounded_delegation() {
    let db = db();
    let string: TdTypeEnum = get_str_type(&db).into();
    let number: TdTypeEnum = get_num_type(&db).into();
    let tv = TypeVariable::get(&db, Some(LazyType::eager(string.clone())));
    let var: TdTypeEnum = TdVariableType::new(&db, 0, tv).into();
    assert!(is_subtype_of(&db, &var, &string));
    assert!(!is_subtype_of(&db, &var, &number));
  }

  #[test]
  fn type_variable_unbound_rejects_concrete_type() {
    let db = db();
    let string: TdTypeEnum = get_str_type(&db).into();
    let type_variable_unbound = TypeVariable::get(&db, None);
    let variable: TdTypeEnum = TdVariableType::new(&db, 0, type_variable_unbound).into();
    assert!(!is_subtype_of(&db, &string, &variable));
  }

  #[test]
  fn type_variable_transitive_variable_bound() {
    let db = db();
    let type_variable_2 = TypeVariable::get(&db, None);
    let variable_2: TdTypeEnum = TdVariableType::new(&db, 1, type_variable_2).into();
    let type_variable_1 = TypeVariable::get(&db, Some(LazyType::eager(variable_2.clone())));
    let variable_1: TdTypeEnum = TdVariableType::new(&db, 0, type_variable_1).into();

    assert!(is_subtype_of(&db, &variable_1, &variable_2));
    assert!(!is_subtype_of(&db, &variable_2, &variable_1));
  }

  // Existential type precheck test

  #[test]
  fn existential_supertype_witnesses_body() {
    let db = db();
    let list_string: TdTypeEnum = get_list_type(&db)
      .instantiate(&db, vec![LazyType::eager(get_str_type(&db).into())])
      .typ(&db);
    let existential_type: TdTypeEnum = TdExistentialType::new(
      &db,
      TypeParams::new(&db, vec![], vec![]),
      Some(LazyType::eager(list_string.clone())),
    )
    .into();

    // matches body directly
    // List[string] <= exists. List[string]
    assert!(is_subtype_of(&db, &list_string, &existential_type));
  }

  #[test]
  fn existential_subtype_bounded_body() {
    let db = db();
    let string: TdTypeEnum = get_str_type(&db).into();
    let object: TdTypeEnum = get_object_type(&db).into();

    let type_variable_string = TypeVariable::get(&db, Some(LazyType::eager(string.clone())));
    let variable_string: TdTypeEnum = TdVariableType::new(&db, 0, type_variable_string).into();
    let existential_variable: TdTypeEnum = TdExistentialType::new(
      &db,
      TypeParams::new(&db, vec![type_variable_string], vec![]),
      Some(LazyType::eager(variable_string)),
    )
    .into();

    // T1 delegates to upper bound string
    // exists T1 <: string. T1 <= string
    assert!(is_subtype_of(&db, &existential_variable, &string));

    let type_variable_obj = TypeVariable::get(&db, Some(LazyType::eager(object.clone())));
    let variable_obj: TdTypeEnum = TdVariableType::new(&db, 0, type_variable_obj).into();
    let list_variable: TdTypeEnum = get_list_type(&db)
      .instantiate(&db, vec![LazyType::eager(variable_obj)])
      .typ(&db);
    let existential_list: TdTypeEnum = TdExistentialType::new(
      &db,
      TypeParams::new(&db, vec![type_variable_obj], vec![]),
      Some(LazyType::eager(list_variable)),
    )
    .into();
    let list_obj: TdTypeEnum = get_list_type(&db)
      .instantiate(&db, vec![LazyType::eager(object)])
      .typ(&db);
    let list_string: TdTypeEnum = get_list_type(&db)
      .instantiate(&db, vec![LazyType::eager(string)])
      .typ(&db);

    // Object <= Object holds
    // exists T1 <: Object. List[T1] <= List[Object]
    assert!(is_subtype_of(&db, &existential_list, &list_obj));

    // upper bound Object <= string fails
    // exists T1 <: Object. List[T1] <= List[string]
    assert!(!is_subtype_of(&db, &existential_list, &list_string));
  }

  #[test]
  fn existential_witness_bound_accumulation() {
    let db = db();
    let string: TdTypeEnum = get_str_type(&db).into();
    let number: TdTypeEnum = get_num_type(&db).into();
    let object: TdTypeEnum = get_object_type(&db).into();

    let type_variable = TypeVariable::get(&db, Some(LazyType::eager(object.clone())));
    let variable: TdTypeEnum = TdVariableType::new(&db, 0, type_variable).into();

    let list_string: TdTypeEnum = get_list_type(&db)
      .instantiate(&db, vec![LazyType::eager(string.clone())])
      .typ(&db);
    let list_variable: TdTypeEnum = get_list_type(&db)
      .instantiate(&db, vec![LazyType::eager(variable.clone())])
      .typ(&db);
    let existential_list: TdTypeEnum = TdExistentialType::new(
      &db,
      TypeParams::new(&db, vec![type_variable], vec![]),
      Some(LazyType::eager(list_variable)),
    )
    .into();

    // solves witness T2 = string
    // List[string] <= exists T2 <: Object. List[T2]
    assert!(is_subtype_of(&db, &list_string, &existential_list));

    let type_variable_num_bound = TypeVariable::get(&db, Some(LazyType::eager(number.clone())));
    let variable_num_bound: TdTypeEnum =
      TdVariableType::new(&db, 0, type_variable_num_bound).into();
    let list_variable_num: TdTypeEnum = get_list_type(&db)
      .instantiate(&db, vec![LazyType::eager(variable_num_bound)])
      .typ(&db);
    let existential_list_num_bound: TdTypeEnum = TdExistentialType::new(
      &db,
      TypeParams::new(&db, vec![type_variable_num_bound], vec![]),
      Some(LazyType::eager(list_variable_num)),
    )
    .into();

    // witness string violates bound number
    // List[string] <= exists T2 <: number. List[T2]
    assert!(!is_subtype_of(&db, &list_string, &existential_list_num_bound));

    let func_str_string: TdTypeEnum =
      TdFuncType::get(&db, vec![string.clone()], string.clone()).into();
    let func_variable_variable: TdTypeEnum =
      TdFuncType::get(&db, vec![variable.clone()], variable.clone()).into();
    let existential_func: TdTypeEnum = TdExistentialType::new(
      &db,
      TypeParams::new(&db, vec![type_variable], vec![]),
      Some(LazyType::eager(func_variable_variable)),
    )
    .into();

    // solves witness T2 = string across covariant & contravariant positions
    // func(string) -> string <= exists T2 <: Object. func(T2) -> T2
    assert!(is_subtype_of(&db, &func_str_string, &existential_func));

    let func_num_string: TdTypeEnum = TdFuncType::get(&db, vec![number], string).into();

    // lower bound string <= upper bound number is false
    // func(number) -> string <= exists T2 <: Object. func(T2) -> T2
    assert!(!is_subtype_of(&db, &func_num_string, &existential_func));
  }

  #[test]
  fn existential_multi_param_and_nested() {
    let db = db();
    let string: TdTypeEnum = get_str_type(&db).into();
    let number: TdTypeEnum = get_num_type(&db).into();
    let object: TdTypeEnum = get_object_type(&db).into();

    let type_variable_1 = TypeVariable::get(&db, Some(LazyType::eager(object.clone())));
    let type_variable_2 = TypeVariable::get(&db, Some(LazyType::eager(object.clone())));
    let variable_1: TdTypeEnum = TdVariableType::new(&db, 0, type_variable_1).into();
    let variable_2: TdTypeEnum = TdVariableType::new(&db, 1, type_variable_2).into();

    let dict_variable: TdTypeEnum = get_dict_type(&db)
      .instantiate(
        &db,
        vec![
          LazyType::eager(variable_1.clone()),
          LazyType::eager(variable_2),
        ],
      )
      .typ(&db);

    let existential_dict: TdTypeEnum = TdExistentialType::new(
      &db,
      TypeParams::new(&db, vec![type_variable_1, type_variable_2], vec![]),
      Some(LazyType::eager(dict_variable)),
    )
    .into();

    let dict_str_num: TdTypeEnum = get_dict_type(&db)
      .instantiate(
        &db,
        vec![
          LazyType::eager(string.clone()),
          LazyType::eager(number.clone()),
        ],
      )
      .typ(&db);

    // solves T1 = string and T2 = number
    // Dict[string, number] <= exists T1 <: Object, T2 <: Object. Dict[T1, T2]
    assert!(is_subtype_of(&db, &dict_str_num, &existential_dict));

    let type_variable_2_string = TypeVariable::get(&db, Some(LazyType::eager(string.clone())));
    let variable_2_string: TdTypeEnum = TdVariableType::new(&db, 1, type_variable_2_string).into();
    let dict_variable_str_bound: TdTypeEnum = get_dict_type(&db)
      .instantiate(
        &db,
        vec![LazyType::eager(variable_1), LazyType::eager(variable_2_string)],
      )
      .typ(&db);
    let existential_dict_str_bound: TdTypeEnum = TdExistentialType::new(
      &db,
      TypeParams::new(&db, vec![type_variable_1, type_variable_2_string], vec![]),
      Some(LazyType::eager(dict_variable_str_bound)),
    )
    .into();

    // witness T2 = number violates bound string
    // Dict[string, number] <= exists T1 <: string, T2 <: string. Dict[T1, T2]
    assert!(!is_subtype_of(
      &db,
      &dict_str_num,
      &existential_dict_str_bound
    ));
  }

  #[test]
  fn existential_vs_existential_subtyping() {
    let db = db();
    let string: TdTypeEnum = get_str_type(&db).into();
    let object: TdTypeEnum = get_object_type(&db).into();

    let type_variable_string = TypeVariable::get(&db, Some(LazyType::eager(string.clone())));
    let variable_string: TdTypeEnum = TdVariableType::new(&db, 0, type_variable_string).into();
    let list_variable_string: TdTypeEnum = get_list_type(&db)
      .instantiate(&db, vec![LazyType::eager(variable_string)])
      .typ(&db);

    let existential_string: TdTypeEnum = TdExistentialType::new(
      &db,
      TypeParams::new(&db, vec![type_variable_string], vec![]),
      Some(LazyType::eager(list_variable_string)),
    )
    .into();

    let type_variable_obj = TypeVariable::get(&db, Some(LazyType::eager(object.clone())));
    let variable_obj: TdTypeEnum = TdVariableType::new(&db, 0, type_variable_obj).into();
    let list_variable_obj: TdTypeEnum = get_list_type(&db)
      .instantiate(&db, vec![LazyType::eager(variable_obj)])
      .typ(&db);
    let existential_obj: TdTypeEnum = TdExistentialType::new(
      &db,
      TypeParams::new(&db, vec![type_variable_obj], vec![]),
      Some(LazyType::eager(list_variable_obj)),
    )
    .into();

    // bound subsumption string <= Object holds
    // (exists T1 <: string. List[T1]) <= (exists T2 <: Object. List[T2])
    assert!(is_subtype_of(&db, &existential_string, &existential_obj));

    // bound Object <= string is false
    // (exists T2 <: Object. List[T2]) <= (exists T1 <: string. List[T1])
    assert!(!is_subtype_of(&db, &existential_obj, &existential_string));
  }

  #[test]
  fn bare_type_and_existential_reciprocal_subtyping() {
    let db = db();
    let string: TdTypeEnum = get_str_type(&db).into();
    let object: TdTypeEnum = get_object_type(&db).into();

    let type_variable_obj = TypeVariable::get(&db, Some(LazyType::eager(object.clone())));
    let variable_obj: TdTypeEnum = TdVariableType::new(&db, 0, type_variable_obj).into();
    let list_variable_obj: TdTypeEnum = get_list_type(&db)
      .instantiate(&db, vec![LazyType::eager(variable_obj)])
      .typ(&db);
    let existential_obj: TdTypeEnum = TdExistentialType::new(
      &db,
      TypeParams::new(&db, vec![type_variable_obj], vec![]),
      Some(LazyType::eager(list_variable_obj)),
    )
    .into();

    let list_string: TdTypeEnum = get_list_type(&db)
      .instantiate(&db, vec![LazyType::eager(string.clone())])
      .typ(&db);

    // bare subtype solves witness T = string
    // List[string] <= exists T <: Object. List[T]
    assert!(is_subtype_of(&db, &list_string, &existential_obj));

    let type_variable_string = TypeVariable::get(&db, Some(LazyType::eager(string.clone())));
    let variable_string: TdTypeEnum = TdVariableType::new(&db, 0, type_variable_string).into();
    let list_variable_string: TdTypeEnum = get_list_type(&db)
      .instantiate(&db, vec![LazyType::eager(variable_string)])
      .typ(&db);
    let existential_string: TdTypeEnum = TdExistentialType::new(
      &db,
      TypeParams::new(&db, vec![type_variable_string], vec![]),
      Some(LazyType::eager(list_variable_string)),
    )
    .into();

    let list_obj: TdTypeEnum = get_list_type(&db)
      .instantiate(&db, vec![LazyType::eager(object)])
      .typ(&db);

    // existential subtype opens T with upper bound string <= Object
    // (exists T <: string. List[T]) <= List[Object]
    assert!(is_subtype_of(&db, &existential_string, &list_obj));

    // witness Object violates bound string
    // List[Object] <= exists T <: string. List[T]
    assert!(!is_subtype_of(&db, &list_obj, &existential_string));

    // open T bound Object <= string is false
    // (exists T <: Object. List[T]) <= List[string]
    assert!(!is_subtype_of(&db, &existential_obj, &list_string));
  }
}
