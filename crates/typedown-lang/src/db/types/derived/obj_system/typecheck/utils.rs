//! Shared type compatibility utilities for typechecking

use crate::db::TypedownDatabase;
use crate::db::derived::get_builtin_typs::get_null_typ;
use crate::db::types::derived::obj_system::TdStaticTyp;
use crate::db::types::fields_compatible;
use crate::db::types::{
  LazyTyp, TdExistentialTyp, TdSchemaTyp, TdSumTyp, TdTypEnum, TypParams, TypVariable,
};
use crate::syntax::diagnostic::Diagnostic;
use std::collections::{HashMap, HashSet};
use typedown_incremental::Id;

/// Validate type arguments against type parameters for both arity and bounds
pub fn validate_typ_params(
  db: &TypedownDatabase,
  type_params: Option<&TypParams>,
  args: &[LazyTyp],
) -> Vec<Diagnostic> {
  let expected_arity = type_params.map_or(0, |p| p.len(db));
  if expected_arity != args.len() {
    return vec![Diagnostic::WrongTypArgCount {
      expected: expected_arity,
      got: args.len(),
    }];
  }

  let mut diagnostics = Vec::new();
  if let Some(params) = type_params {
    let params_vec = params.params(db);
    for (index, (p, arg)) in params_vec.iter().zip(args.iter()).enumerate() {
      if let Some(bound) = p.upper_bound(db).resolve(db)
        && bound.as_td_obj_typ().is_none()
        && let Some(arg_type) = arg.resolve(db)
        && !is_subtype_of(db, &arg_type, &bound)
      {
        diagnostics.push(Diagnostic::TypArgBoundViolation {
          index,
          expected_bound: bound.display_name(db),
          got: arg_type.display_name(db),
        });
      }
    }
  }
  diagnostics
}

/// Check if `subtype` is a subtype of `supertype`
pub fn is_subtype_of(db: &TypedownDatabase, subtype: &TdTypEnum, supertype: &TdTypEnum) -> bool {
  let mut env = SubtypeEnv::new();
  is_subtype_of_env(db, subtype, supertype, &mut env)
}

/// Witness environment for existential subtyping constraints
#[derive(Default)]
struct SubtypeEnv<'db> {
  lower_bounds: HashMap<TypVariable<'db>, Vec<TdTypEnum<'db>>>,
  upper_bounds: HashMap<TypVariable<'db>, Vec<TdTypEnum<'db>>>,
  existential_variables: HashSet<TypVariable<'db>>,
}

impl<'db> SubtypeEnv<'db> {
  fn new() -> Self {
    Self {
      lower_bounds: HashMap::new(),
      upper_bounds: HashMap::new(),
      existential_variables: HashSet::new(),
    }
  }

  /// Register an existential variable in the environment and push its declared upper bound
  fn track_existential_variable(&mut self, db: &'db TypedownDatabase, variable: TypVariable<'db>) {
    self.existential_variables.insert(variable);
    if let Some(upper_bound) = variable.upper_bound(db).resolve(db) {
      self.add_upper_bound(db, variable, &upper_bound);
    }
  }

  /// Record a lower bound for an existential variable
  /// Returning `false` early if a conflict with any existing upper bound is detected (`lower <= upper` fails)
  fn add_lower_bound(
    &mut self,
    db: &'db TypedownDatabase,
    variable: TypVariable<'db>,
    lower_bound: &TdTypEnum<'db>,
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
    db: &'db TypedownDatabase,
    variable: TypVariable<'db>,
    upper_bound: &TdTypEnum<'db>,
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

// Walk the nominal parent chain of `sub` to see if it reaches `sup` by identity
fn nominal_subtype_of(
  db: &TypedownDatabase,
  subtype: &TdSchemaTyp,
  supertype: &TdSchemaTyp,
) -> bool {
  let target_id = supertype.as_id();
  let mut current = subtype.parent(db);
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

fn is_subtype_of_env<'db>(
  db: &'db TypedownDatabase,
  subtype: &TdTypEnum<'db>,
  supertype: &TdTypEnum<'db>,
  env: &mut SubtypeEnv<'db>,
) -> bool {
  // Phase 1: Check type constructor compatibility ignoring type arguments and parameter variance
  fn are_constructors_compatible<'db>(
    db: &'db TypedownDatabase,
    subtype: &TdTypEnum<'db>, // INVARIANT: Due to sum type elimination, sub type cannot be a sum type here
    supertype: &TdTypEnum<'db>,
    env: &mut SubtypeEnv<'db>,
  ) -> bool {
    if subtype.as_id() == supertype.as_id() {
      return true;
    }
    match supertype {
      TdTypEnum::TdObjTyp(_) | TdTypEnum::TdTypTyp(_) => true,
      TdTypEnum::TdNeverTyp(_) => false,
      // WARNING: This only works because subtype is not a sum type
      TdTypEnum::TdSumTyp(sum) => sum.members(db).iter().any(|member| {
        member
          .resolve(db)
          .is_some_and(|m| is_subtype_of_env(db, subtype, &m, env))
      }),
      TdTypEnum::TdLitTyp(_) => false,
      TdTypEnum::TdStringTyp(_) => {
        matches!(
          subtype,
          TdTypEnum::TdLitTyp(lit)
            if matches!(lit.underlying_typ(db), TdTypEnum::TdStringTyp(_))
        ) || matches!(
          subtype,
          TdTypEnum::TdDateTimeTyp(_)
            | TdTypEnum::TdDateTyp(_)
            | TdTypEnum::TdTimeTyp(_)
            | TdTypEnum::TdBlobTyp(_)
        )
      }
      TdTypEnum::TdNumberTyp(_) => matches!(
        subtype,
        TdTypEnum::TdLitTyp(lit)
          if matches!(lit.underlying_typ(db), TdTypEnum::TdNumberTyp(_))
      ),
      TdTypEnum::TdBoolTyp(_) => matches!(
        subtype,
        TdTypEnum::TdLitTyp(lit)
          if matches!(lit.underlying_typ(db), TdTypEnum::TdBoolTyp(_))
      ),
      TdTypEnum::TdListTyp(_) => matches!(subtype, TdTypEnum::TdListTyp(_)),
      TdTypEnum::TdDictTyp(expected_dict) => match subtype {
        TdTypEnum::TdDictTyp(_) => true,
        TdTypEnum::TdProductTyp(product) => {
          let value_type = match expected_dict.value(db).and_then(|l| l.resolve(db)) {
            Some(vt) => vt,
            None => return true,
          };
          product.get_fields(db).values().all(|field_lazy| {
            field_lazy
              .resolve(db)
              .is_some_and(|ft| is_subtype_of_env(db, &ft, &value_type, env))
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
              .is_some_and(|field_typ| is_subtype_of_env(db, &field_typ, &value_typ, env))
          })
        }
        _ => false,
      },
      TdTypEnum::TdFuncTyp(_) => matches!(subtype, TdTypEnum::TdFuncTyp(_)),
      TdTypEnum::TdProductTyp(expected_product_typ) => match subtype {
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
      TdTypEnum::TdSchemaTyp(expected_schema_typ) => match subtype {
        // Schema to schema: nominal via extends chain
        TdTypEnum::TdSchemaTyp(schema_typ) => {
          nominal_subtype_of(db, schema_typ, expected_schema_typ)
        }
        // Product to schema: never
        _ => false,
      },
      TdTypEnum::TdBlobTyp(_) => {
        matches!(subtype, TdTypEnum::TdBlobTyp(_) | TdTypEnum::TdStringTyp(_))
      }
      TdTypEnum::TdMathTyp(_)
      | TdTypEnum::TdDateTimeTyp(_)
      | TdTypEnum::TdDateTyp(_)
      | TdTypEnum::TdTimeTyp(_)
      | TdTypEnum::TdNullTyp(_) => false,
      TdTypEnum::TdIconTyp(_) => matches!(subtype, TdTypEnum::TdIconTyp(_)),
      TdTypEnum::TdSchemaMetaTyp(_)
      | TdTypEnum::TdVariableTyp(_)
      | TdTypEnum::TdExistentialTyp(_) => false,
    }
  }

  // Phase 2: Check type arguments and parameter variance between compatible constructors
  fn are_type_args_compatible<'db>(
    db: &'db TypedownDatabase,
    subtype: &TdTypEnum<'db>,
    supertype: &TdTypEnum<'db>,
    env: &mut SubtypeEnv<'db>,
  ) -> bool {
    match (subtype, supertype) {
      (TdTypEnum::TdListTyp(subtype_list), TdTypEnum::TdListTyp(supertype_list)) => {
        match (
          subtype_list.element(db).and_then(|e| e.resolve(db)),
          supertype_list.element(db).and_then(|e| e.resolve(db)),
        ) {
          (_, None) => true,
          (None, Some(_)) => false,
          (Some(subtype_element), Some(supertype_element)) => {
            is_subtype_of_env(db, &subtype_element, &supertype_element, env)
          }
        }
      }
      (TdTypEnum::TdDictTyp(_), TdTypEnum::TdDictTyp(_)) => {
        let supertype_args = supertype.get_typ_args(db);
        if supertype_args.is_empty() {
          return true;
        }
        let subtype_args = subtype.get_typ_args(db);
        if subtype_args.is_empty() {
          return false;
        }
        subtype_args
          .iter()
          .zip(supertype_args.iter())
          .all(|(sub_arg, super_arg)| is_subtype_of_env(db, sub_arg, super_arg, env))
      }
      (TdTypEnum::TdFuncTyp(subtype_func), TdTypEnum::TdFuncTyp(supertype_func)) => {
        let subtype_signature = subtype_func.signature(db);
        let supertype_signature = supertype_func.signature(db);
        let subtype_params = subtype_signature.params(db);
        let supertype_params = supertype_signature.params(db);
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
        is_subtype_of_env(
          db,
          &subtype_signature.ret(db),
          &supertype_signature.ret(db),
          env,
        )
      }
      _ => true,
    }
  }

  // Phase 0: Special pre-check (variables, existentials)
  match (subtype, supertype) {
    // Universal variables: T1 <: T2 requires identity or checking T1's declared upper bound
    (TdTypEnum::TdVariableTyp(variable_sub), TdTypEnum::TdVariableTyp(_variable_super)) => {
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
    (TdTypEnum::TdVariableTyp(variable_subtype), _) => {
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
    (_, TdTypEnum::TdVariableTyp(variable_supertype)) => {
      let variable_supertype = variable_supertype.variable(db);
      // Only existential variables on supertype accumulate lower bounds
      if env.existential_variables.contains(&variable_supertype) {
        env.add_lower_bound(db, variable_supertype, subtype)
      } else {
        false
      }
    }
    (TdTypEnum::TdExistentialTyp(existential_subtype), TdTypEnum::TdExistentialTyp(_)) => {
      // (exists T1 <: S1. P1[T1]) <: (exists T2 <: S2. P2[T2])
      // Well this is just a special case of the below case, technically can be merged...
      existential_subtype
        .body(db)
        .and_then(|b| b.resolve(db))
        .is_some_and(|body| is_subtype_of_env(db, &body, supertype, env))
    }
    (TdTypEnum::TdExistentialTyp(existential_subtype), supertype) => {
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
    (subtype, TdTypEnum::TdExistentialTyp(existential_supertype)) => {
      // P1[T1] <: (exists T2 <: S2. P2[T2])
      // iff
      // P1[T1] is a subtype of some P2[T2] where T2 <: S2
      // We just proceed product decomposition, then accumulate bounds
      let params = TdExistentialTyp::typ_params(*existential_supertype, db).params(db);
      for param in &params {
        env.track_existential_variable(db, *param);
      }
      existential_supertype
        .body(db)
        .and_then(|b| b.resolve(db))
        .is_some_and(|body| is_subtype_of_env(db, subtype, &body, env))
    }
    _ => {
      if subtype.as_td_never_typ().is_some() {
        return true;
      }

      // Sum type elimination
      if let Some(sum) = subtype.as_td_sum_typ() {
        return sum.members(db).iter().all(|member| {
          member
            .resolve(db)
            .is_some_and(|typ| is_subtype_of_env(db, &typ, supertype, env))
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
      .filter_map(|m| m.resolve(db))
      .any(|t| is_nullable(db, &t));
  }
  false
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::db::derived::get_builtin_typs::{
    get_bool_typ, get_date_typ, get_datetime_typ, get_dict_typ, get_list_typ, get_lit_typ,
    get_never_typ, get_null_typ, get_number_typ, get_obj_typ, get_schema_meta_typ, get_string_typ,
    get_sum_typ, get_time_typ, get_typ_typ,
  };
  use crate::db::types::{
    LazyTyp, LitValue, TdExistentialTyp, TdFuncTyp, TdProductTyp, TdSchemaTyp, TdVariableTyp,
    TypVariable, make_property_descriptors,
  };
  use crate::db::{QueryStorage, TypedownDatabase};
  use std::collections::BTreeMap;

  fn make_db() -> TypedownDatabase {
    TypedownDatabase {
      storage: QueryStorage::default(),
    }
  }

  fn create_lit_string<'a>(db: &'a TypedownDatabase, val: &str) -> TdTypEnum<'a> {
    get_lit_typ(db, LitValue::String(val.to_string())).into()
  }

  fn create_lit_number<'a>(db: &'a TypedownDatabase, val: &str) -> TdTypEnum<'a> {
    get_lit_typ(db, LitValue::Number(val.to_string())).into()
  }

  fn create_sum_typ<'a>(db: &'a TypedownDatabase, members: Vec<TdTypEnum<'a>>) -> TdTypEnum<'a> {
    get_sum_typ(db, members.into_iter().map(LazyTyp::eager).collect()).into()
  }

  // Simple vs Simple

  #[test]
  fn compatible_simple_same_typ() {
    let db = make_db();
    let string: TdTypEnum = get_string_typ(&db).into();
    assert!(is_subtype_of(&db, &string, &string));
  }

  #[test]
  fn incompatible_simple_different_typ() {
    let db = make_db();
    let string: TdTypEnum = get_string_typ(&db).into();
    let number: TdTypEnum = get_number_typ(&db).into();
    assert!(!is_subtype_of(&db, &number, &string));
  }

  // Lit vs Simple

  #[test]
  fn lit_compatible_with_base_simple() {
    let db = make_db();
    let string: TdTypEnum = get_string_typ(&db).into();
    let lit = create_lit_string(&db, "hello");
    assert!(is_subtype_of(&db, &lit, &string));
  }

  #[test]
  fn lit_incompatible_with_wrong_simple() {
    let db = make_db();
    let number: TdTypEnum = get_number_typ(&db).into();
    let lit = create_lit_string(&db, "hello");
    assert!(!is_subtype_of(&db, &lit, &number));
  }

  // Lit vs Lit

  #[test]
  fn lit_compatible_same_value() {
    let db = make_db();
    let lit1 = create_lit_string(&db, "draft");
    let lit2 = create_lit_string(&db, "draft");
    assert!(is_subtype_of(&db, &lit2, &lit1));
  }

  #[test]
  fn lit_incompatible_different_value() {
    let db = make_db();
    let lit1 = create_lit_string(&db, "draft");
    let lit2 = create_lit_string(&db, "published");
    assert!(!is_subtype_of(&db, &lit2, &lit1));
  }

  // Never is the bottom type: assignable to anything, but nothing is assignable to it

  #[test]
  fn never_is_bottom_typ() {
    let db = make_db();
    let string_typ: TdTypEnum = get_string_typ(&db).into();
    let never_typ: TdTypEnum = get_never_typ(&db).into();
    assert!(!is_subtype_of(&db, &string_typ, &never_typ));
    assert!(is_subtype_of(&db, &never_typ, &string_typ));
  }

  #[test]
  fn lit_number_compatible_with_number() {
    let db = make_db();
    let number_typ: TdTypEnum = get_number_typ(&db).into();
    let lit = create_lit_number(&db, "42");
    assert!(is_subtype_of(&db, &lit, &number_typ));
  }

  #[test]
  fn lit_number_incompatible_with_string() {
    let db = make_db();
    let string_typ: TdTypEnum = get_string_typ(&db).into();
    let lit_typ = create_lit_number(&db, "42");
    assert!(!is_subtype_of(&db, &lit_typ, &string_typ));
  }

  #[test]
  fn string_accepts_string_lit() {
    let db = make_db();
    let string_typ: TdTypEnum = get_string_typ(&db).into();
    let lit_typ: TdTypEnum = get_lit_typ(&db, LitValue::String("hello".to_string())).into();
    assert!(is_subtype_of(&db, &lit_typ, &string_typ));
  }

  #[test]
  fn number_accepts_number_lit() {
    let db = make_db();
    let number_typ: TdTypEnum = get_number_typ(&db).into();
    let lit_typ: TdTypEnum = get_lit_typ(&db, LitValue::Number("42".to_string())).into();
    assert!(is_subtype_of(&db, &lit_typ, &number_typ));
  }

  #[test]
  fn boolean_accepts_boolean_literal() {
    let db = make_db();
    let bool_typ: TdTypEnum = get_bool_typ(&db).into();
    let lit_typ: TdTypEnum = get_lit_typ(&db, LitValue::Bool(true)).into();
    assert!(is_subtype_of(&db, &lit_typ, &bool_typ));
  }

  #[test]
  fn string_rejects_number_literal() {
    let db = make_db();
    let string_typ: TdTypEnum = get_string_typ(&db).into();
    let lit_typ: TdTypEnum = get_lit_typ(&db, LitValue::Number("42".to_string())).into();
    assert!(!is_subtype_of(&db, &lit_typ, &string_typ));
  }

  #[test]
  fn literal_accepts_same_value() {
    let db = make_db();
    let lit_typ1: TdTypEnum = get_lit_typ(&db, LitValue::String("draft".to_string())).into();
    let lit_typ2: TdTypEnum = get_lit_typ(&db, LitValue::String("draft".to_string())).into();
    assert!(is_subtype_of(&db, &lit_typ2, &lit_typ1));
  }

  #[test]
  fn literal_rejects_different_value() {
    let db = make_db();
    let lit_typ1: TdTypEnum = get_lit_typ(&db, LitValue::String("draft".to_string())).into();
    let lit_typ2: TdTypEnum = get_lit_typ(&db, LitValue::String("published".to_string())).into();
    assert!(!is_subtype_of(&db, &lit_typ2, &lit_typ1));
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
    assert!(is_subtype_of(&db, &string_typ, &string_or_number_typ));
  }

  #[test]
  fn sum_rejects_non_member_typ() {
    let db = make_db();
    let string_or_number_typ = create_sum_typ(
      &db,
      vec![get_string_typ(&db).into(), get_number_typ(&db).into()],
    );
    let boolean_typ: TdTypEnum = get_bool_typ(&db).into();
    assert!(!is_subtype_of(&db, &boolean_typ, &string_or_number_typ));
  }

  #[test]
  fn sum_accepts_lit_of_member_typ() {
    let db = make_db();
    let string_or_number_typ = create_sum_typ(
      &db,
      vec![get_string_typ(&db).into(), get_number_typ(&db).into()],
    );
    let lit = create_lit_string(&db, "hello");
    assert!(is_subtype_of(&db, &lit, &string_or_number_typ));
  }

  #[test]
  fn sum_accepts_subsum() {
    let db = make_db();
    let string_or_number_typ = create_sum_typ(
      &db,
      vec![get_string_typ(&db).into(), get_number_typ(&db).into()],
    );
    let just_string_typ = create_sum_typ(&db, vec![get_string_typ(&db).into()]);
    assert!(is_subtype_of(&db, &just_string_typ, &string_or_number_typ));
  }

  #[test]
  fn sum_rejects_wider_sum() {
    let db = make_db();
    let just_string_typ = create_sum_typ(&db, vec![get_string_typ(&db).into()]);
    let string_or_number_typ = create_sum_typ(
      &db,
      vec![get_string_typ(&db).into(), get_number_typ(&db).into()],
    );
    assert!(!is_subtype_of(&db, &string_or_number_typ, &just_string_typ));
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
    assert!(is_subtype_of(&db, &null_typ, &nullable_string_typ));
  }

  #[test]
  fn nullable_accepts_base_type() {
    let db = make_db();
    let nullable_string_typ = create_sum_typ(
      &db,
      vec![get_string_typ(&db).into(), get_null_typ(&db).into()],
    );
    let string_typ: TdTypEnum = get_string_typ(&db).into();
    assert!(is_subtype_of(&db, &string_typ, &nullable_string_typ));
  }

  #[test]
  fn non_nullable_rejects_null() {
    let db = make_db();
    let string_typ: TdTypEnum = get_string_typ(&db).into();
    let null_typ: TdTypEnum = get_null_typ(&db).into();
    assert!(!is_subtype_of(&db, &null_typ, &string_typ));
  }

  // Never type tests

  #[test]
  fn never_accepted_by_any_type() {
    let db = make_db();
    let never_typ: TdTypEnum = get_never_typ(&db).into();
    let string_typ: TdTypEnum = get_string_typ(&db).into();
    let number_typ: TdTypEnum = get_number_typ(&db).into();
    let boolean_typ: TdTypEnum = get_bool_typ(&db).into();
    assert!(is_subtype_of(&db, &never_typ, &string_typ));
    assert!(is_subtype_of(&db, &never_typ, &number_typ));
    assert!(is_subtype_of(&db, &never_typ, &boolean_typ));
  }

  #[test]
  fn never_accepted_by_sum() {
    let db = make_db();
    let never_typ: TdTypEnum = get_never_typ(&db).into();
    let string_or_number_typ = create_sum_typ(
      &db,
      vec![get_string_typ(&db).into(), get_number_typ(&db).into()],
    );
    assert!(is_subtype_of(&db, &never_typ, &string_or_number_typ));
  }

  #[test]
  fn nothing_accepted_by_never() {
    let db = make_db();
    let never_typ: TdTypEnum = get_never_typ(&db).into();
    let string_typ: TdTypEnum = get_string_typ(&db).into();
    assert!(!is_subtype_of(&db, &string_typ, &never_typ));
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
    assert!(is_subtype_of(&db, &draft_typ, &status_typ));
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
    assert!(!is_subtype_of(&db, &archived_typ, &status_typ));
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
    assert!(is_subtype_of(&db, &status_typ, &string_typ));
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
    assert!(!is_subtype_of(&db, &mixed_typ, &string_typ));
  }

  // List type tests

  #[test]
  fn list_accepts_same_elem_type() {
    let db = make_db();
    let list_string_typ: TdTypEnum = get_list_typ(&db)
      .instantiate(&db, vec![LazyTyp::eager(get_string_typ(&db).into())])
      .typ(&db);
    let list_string_typ2: TdTypEnum = get_list_typ(&db)
      .instantiate(&db, vec![LazyTyp::eager(get_string_typ(&db).into())])
      .typ(&db);
    assert!(is_subtype_of(&db, &list_string_typ2, &list_string_typ));
  }

  #[test]
  fn list_accepts_covariant_elem_type() {
    let db = make_db();
    let list_string_typ: TdTypEnum = get_list_typ(&db)
      .instantiate(&db, vec![LazyTyp::eager(get_string_typ(&db).into())])
      .typ(&db);
    let list_lit_typ: TdTypEnum = get_list_typ(&db)
      .instantiate(&db, vec![LazyTyp::eager(create_lit_string(&db, "hello"))])
      .typ(&db);
    assert!(is_subtype_of(&db, &list_lit_typ, &list_string_typ));
  }

  #[test]
  fn list_rejects_wider_elem_type() {
    let db = make_db();
    let list_string_typ: TdTypEnum = get_list_typ(&db)
      .instantiate(&db, vec![LazyTyp::eager(get_string_typ(&db).into())])
      .typ(&db);
    let list_lit_typ: TdTypEnum = get_list_typ(&db)
      .instantiate(&db, vec![LazyTyp::eager(create_lit_string(&db, "hello"))])
      .typ(&db);
    assert!(!is_subtype_of(&db, &list_string_typ, &list_lit_typ));
  }

  #[test]
  fn list_rejects_different_elem_type() {
    let db = make_db();
    let list_string_typ: TdTypEnum = get_list_typ(&db)
      .instantiate(&db, vec![LazyTyp::eager(get_string_typ(&db).into())])
      .typ(&db);
    let list_number_typ: TdTypEnum = get_list_typ(&db)
      .instantiate(&db, vec![LazyTyp::eager(get_number_typ(&db).into())])
      .typ(&db);
    assert!(!is_subtype_of(&db, &list_number_typ, &list_string_typ));
  }

  #[test]
  fn untyped_list_accepts_any_list() {
    let db = make_db();
    let untyped: TdTypEnum = get_list_typ(&db).into();
    let list_string_typ: TdTypEnum = get_list_typ(&db)
      .instantiate(&db, vec![LazyTyp::eager(get_string_typ(&db).into())])
      .typ(&db);
    assert!(is_subtype_of(&db, &list_string_typ, &untyped));
  }

  #[test]
  fn typed_list_rejects_untyped_list() {
    let db = make_db();
    let untyped: TdTypEnum = get_list_typ(&db).into();
    let list_string_typ: TdTypEnum = get_list_typ(&db)
      .instantiate(&db, vec![LazyTyp::eager(get_string_typ(&db).into())])
      .typ(&db);
    assert!(!is_subtype_of(&db, &untyped, &list_string_typ));
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
    assert!(is_subtype_of(&db, &dict_string_typ2, &dict_string_typ));
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
    assert!(is_subtype_of(&db, &dict_lit_typ, &dict_string_typ));
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
    assert!(!is_subtype_of(&db, &dict_string_typ, &dict_lit_typ));
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
    assert!(!is_subtype_of(&db, &dict_number_typ, &dict_string_typ));
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
    assert!(is_subtype_of(&db, &dict_string_typ, &untyped));
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
    assert!(!is_subtype_of(&db, &untyped, &dict_string_typ));
  }

  // String accepts date/time subtypes

  #[test]
  fn string_accepts_date() {
    let db = make_db();
    let string_typ: TdTypEnum = get_string_typ(&db).into();
    let date_typ: TdTypEnum = get_date_typ(&db).into();
    assert!(is_subtype_of(&db, &date_typ, &string_typ));
  }

  #[test]
  fn string_accepts_datetime() {
    let db = make_db();
    let string_typ: TdTypEnum = get_string_typ(&db).into();
    let datetime_typ: TdTypEnum = get_datetime_typ(&db).into();
    assert!(is_subtype_of(&db, &datetime_typ, &string_typ));
  }

  #[test]
  fn string_accepts_time() {
    let db = make_db();
    let string_typ: TdTypEnum = get_string_typ(&db).into();
    let time_typ: TdTypEnum = get_time_typ(&db).into();
    assert!(is_subtype_of(&db, &time_typ, &string_typ));
  }

  #[test]
  fn date_rejects_string() {
    let db = make_db();
    let date_typ: TdTypEnum = get_date_typ(&db).into();
    let string_typ: TdTypEnum = get_string_typ(&db).into();
    assert!(!is_subtype_of(&db, &string_typ, &date_typ));
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
    assert!(is_subtype_of(&db, &actual_typ, &expected_typ));
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
    assert!(!is_subtype_of(&db, &actual_typ, &expected_typ));
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
    assert!(is_subtype_of(&db, &actual_typ, &expected_typ));
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
    assert!(!is_subtype_of(&db, &actual_typ, &expected_typ));
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
    assert!(is_subtype_of(&db, &actual_typ, &expected_typ));
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
    assert!(is_subtype_of(&db, &schema_typ, &product_typ));
    // Product is NOT assignable to schema
    assert!(!is_subtype_of(&db, &product_typ, &schema_typ));
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
    assert!(is_subtype_of(&db, &schema_typ, &product_typ));
  }

  #[test]
  fn null_rejects_non_null() {
    let db = make_db();
    let null_typ: TdTypEnum = get_null_typ(&db).into();
    let string_typ: TdTypEnum = get_string_typ(&db).into();
    assert!(!is_subtype_of(&db, &string_typ, &null_typ));
  }

  #[test]
  fn list_rejects_non_list() {
    let db = make_db();
    let list_string_typ: TdTypEnum = get_list_typ(&db)
      .instantiate(&db, vec![LazyTyp::eager(get_string_typ(&db).into())])
      .typ(&db);
    let string_typ: TdTypEnum = get_string_typ(&db).into();
    assert!(!is_subtype_of(&db, &string_typ, &list_string_typ));
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
    assert!(!is_subtype_of(&db, &string_typ, &dict_typ));
  }

  // is_type tests

  #[test]
  fn builtin_type_is_typ() {
    let db = make_db();
    let string_type: TdTypEnum = get_string_typ(&db).into();
    assert!(!string_type.is_typ(&db), "string is not a metatype");
  }

  #[test]
  fn type_type_is_typ() {
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
    assert!(is_subtype_of(&db, &func_typ2, &func_typ));
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
    assert!(is_subtype_of(&db, &actual_typ, &expected_typ));
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
    assert!(!is_subtype_of(&db, &actual_typ, &expected_typ));
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
    assert!(is_subtype_of(&db, &actual_typ, &expected_typ));
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
    assert!(!is_subtype_of(&db, &actual_typ, &expected_typ));
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
    assert!(!is_subtype_of(&db, &actual_typ, &expected_typ));
  }

  // Type variable (TdVariableType) tests

  #[test]
  fn type_variable_strict_equality() {
    let db = make_db();
    let string_typ: TdTypEnum = get_string_typ(&db).into();
    let number_typ: TdTypEnum = get_number_typ(&db).into();
    let typ_variable1 = TypVariable::get(&db, Some(LazyTyp::eager(string_typ)));
    let typ_variable2 = TypVariable::get(&db, Some(LazyTyp::eager(number_typ)));
    let variable1: TdTypEnum = TdVariableTyp::new(&db, 0, typ_variable1).into();
    let variable2: TdTypEnum = TdVariableTyp::new(&db, 1, typ_variable2).into();
    assert!(is_subtype_of(&db, &variable1, &variable1));
    assert!(!is_subtype_of(&db, &variable1, &variable2));
  }

  #[test]
  fn type_variable_bounded_delegation() {
    let db = make_db();
    let string_typ: TdTypEnum = get_string_typ(&db).into();
    let number_typ: TdTypEnum = get_number_typ(&db).into();
    let typ_variable = TypVariable::get(&db, Some(LazyTyp::eager(string_typ.clone())));
    let variable: TdTypEnum = TdVariableTyp::new(&db, 0, typ_variable).into();
    assert!(is_subtype_of(&db, &variable, &string_typ));
    assert!(!is_subtype_of(&db, &variable, &number_typ));
  }

  #[test]
  fn type_variable_unbound_rejects_concrete_type() {
    let db = make_db();
    let string_typ: TdTypEnum = get_string_typ(&db).into();
    let typ_variable_unbound = TypVariable::get(&db, None);
    let variable: TdTypEnum = TdVariableTyp::new(&db, 0, typ_variable_unbound).into();
    assert!(!is_subtype_of(&db, &string_typ, &variable));
  }

  #[test]
  fn type_variable_transitive_variable_bound() {
    let db = make_db();
    let typ_variable2 = TypVariable::get(&db, None);
    let variable2: TdTypEnum = TdVariableTyp::new(&db, 1, typ_variable2).into();
    let typ_variable1 = TypVariable::get(&db, Some(LazyTyp::eager(variable2.clone())));
    let variable1: TdTypEnum = TdVariableTyp::new(&db, 0, typ_variable1).into();

    assert!(is_subtype_of(&db, &variable1, &variable2));
    assert!(!is_subtype_of(&db, &variable2, &variable1));
  }

  // Existential type precheck test

  #[test]
  fn existential_supertype_witnesses_body() {
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
    assert!(is_subtype_of(&db, &list_string_typ, &existential_typ));
  }

  #[test]
  fn existential_subtype_bounded_body() {
    let db = make_db();
    let string_typ: TdTypEnum = get_string_typ(&db).into();
    let object_typ: TdTypEnum = get_obj_typ(&db).into();

    let typ_variable_string = TypVariable::get(&db, Some(LazyTyp::eager(string_typ.clone())));
    let variable_string_typ: TdTypEnum = TdVariableTyp::new(&db, 0, typ_variable_string).into();
    let existential_variable_typ: TdTypEnum = TdExistentialTyp::new(
      &db,
      TypParams::new(&db, vec![typ_variable_string], vec![]),
      Some(LazyTyp::eager(variable_string_typ)),
    )
    .into();

    // T1 delegates to upper bound string
    // exists T1 <: string. T1 <= string
    assert!(is_subtype_of(&db, &existential_variable_typ, &string_typ));

    let typ_variable_obj = TypVariable::get(&db, Some(LazyTyp::eager(object_typ.clone())));
    let variable_obj_typ: TdTypEnum = TdVariableTyp::new(&db, 0, typ_variable_obj).into();
    let list_variable_typ: TdTypEnum = get_list_typ(&db)
      .instantiate(&db, vec![LazyTyp::eager(variable_obj_typ)])
      .typ(&db);
    let existential_list_typ: TdTypEnum = TdExistentialTyp::new(
      &db,
      TypParams::new(&db, vec![typ_variable_obj], vec![]),
      Some(LazyTyp::eager(list_variable_typ)),
    )
    .into();
    let list_obj: TdTypEnum = get_list_typ(&db)
      .instantiate(&db, vec![LazyTyp::eager(object_typ)])
      .typ(&db);
    let list_string: TdTypEnum = get_list_typ(&db)
      .instantiate(&db, vec![LazyTyp::eager(string_typ)])
      .typ(&db);

    // Object <= Object holds
    // exists T1 <: Object. List[T1] <= List[Object]
    assert!(is_subtype_of(&db, &existential_list_typ, &list_obj));

    // upper bound Object <= string fails
    // exists T1 <: Object. List[T1] <= List[string]
    assert!(!is_subtype_of(&db, &existential_list_typ, &list_string));
  }

  #[test]
  fn existential_witness_bound_accumulation() {
    let db = make_db();
    let string_typ: TdTypEnum = get_string_typ(&db).into();
    let number_typ: TdTypEnum = get_number_typ(&db).into();
    let object_typ: TdTypEnum = get_obj_typ(&db).into();

    let typ_variable = TypVariable::get(&db, Some(LazyTyp::eager(object_typ.clone())));
    let variable: TdTypEnum = TdVariableTyp::new(&db, 0, typ_variable).into();

    let list_string: TdTypEnum = get_list_typ(&db)
      .instantiate(&db, vec![LazyTyp::eager(string_typ.clone())])
      .typ(&db);
    let list_variable: TdTypEnum = get_list_typ(&db)
      .instantiate(&db, vec![LazyTyp::eager(variable.clone())])
      .typ(&db);
    let existential_list_typ: TdTypEnum = TdExistentialTyp::new(
      &db,
      TypParams::new(&db, vec![typ_variable], vec![]),
      Some(LazyTyp::eager(list_variable)),
    )
    .into();

    // solves witness T2 = string
    // List[string] <= exists T2 <: Object. List[T2]
    assert!(is_subtype_of(&db, &list_string, &existential_list_typ));

    let typ_variable_num_bound = TypVariable::get(&db, Some(LazyTyp::eager(number_typ.clone())));
    let variable_num_bound: TdTypEnum = TdVariableTyp::new(&db, 0, typ_variable_num_bound).into();
    let list_variable_num: TdTypEnum = get_list_typ(&db)
      .instantiate(&db, vec![LazyTyp::eager(variable_num_bound)])
      .typ(&db);
    let existential_list_num_bound: TdTypEnum = TdExistentialTyp::new(
      &db,
      TypParams::new(&db, vec![typ_variable_num_bound], vec![]),
      Some(LazyTyp::eager(list_variable_num)),
    )
    .into();

    // witness string violates bound number
    // List[string] <= exists T2 <: number. List[T2]
    assert!(!is_subtype_of(
      &db,
      &list_string,
      &existential_list_num_bound
    ));

    let func_string_string: TdTypEnum =
      TdFuncTyp::get(&db, vec![string_typ.clone()], string_typ.clone()).into();
    let func_variable_variable: TdTypEnum =
      TdFuncTyp::get(&db, vec![variable.clone()], variable.clone()).into();
    let existential_func: TdTypEnum = TdExistentialTyp::new(
      &db,
      TypParams::new(&db, vec![typ_variable], vec![]),
      Some(LazyTyp::eager(func_variable_variable)),
    )
    .into();

    // solves witness T2 = string across covariant & contravariant positions
    // func(string) -> string <= exists T2 <: Object. func(T2) -> T2
    assert!(is_subtype_of(&db, &func_string_string, &existential_func));

    let func_number_string: TdTypEnum = TdFuncTyp::get(&db, vec![number_typ], string_typ).into();

    // lower bound string <= upper bound number is false
    // func(number) -> string <= exists T2 <: Object. func(T2) -> T2
    assert!(!is_subtype_of(&db, &func_number_string, &existential_func));
  }

  #[test]
  fn existential_multi_param_and_nested() {
    let db = make_db();
    let string_typ: TdTypEnum = get_string_typ(&db).into();
    let number_typ: TdTypEnum = get_number_typ(&db).into();
    let object_typ: TdTypEnum = get_obj_typ(&db).into();

    let typ_variable1 = TypVariable::get(&db, Some(LazyTyp::eager(object_typ.clone())));
    let typ_variable2 = TypVariable::get(&db, Some(LazyTyp::eager(object_typ.clone())));
    let variable1: TdTypEnum = TdVariableTyp::new(&db, 0, typ_variable1).into();
    let variable2: TdTypEnum = TdVariableTyp::new(&db, 1, typ_variable2).into();

    let dict_variable: TdTypEnum = get_dict_typ(&db)
      .instantiate(
        &db,
        vec![LazyTyp::eager(variable1.clone()), LazyTyp::eager(variable2)],
      )
      .typ(&db);

    let existential_dict_typ: TdTypEnum = TdExistentialTyp::new(
      &db,
      TypParams::new(&db, vec![typ_variable1, typ_variable2], vec![]),
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
    assert!(is_subtype_of(
      &db,
      &dict_string_number,
      &existential_dict_typ
    ));

    let typ_variable2_string = TypVariable::get(&db, Some(LazyTyp::eager(string_typ.clone())));
    let variable2_string: TdTypEnum = TdVariableTyp::new(&db, 1, typ_variable2_string).into();
    let dict_variable_string_bound: TdTypEnum = get_dict_typ(&db)
      .instantiate(
        &db,
        vec![LazyTyp::eager(variable1), LazyTyp::eager(variable2_string)],
      )
      .typ(&db);
    let existential_dict_string_bound: TdTypEnum = TdExistentialTyp::new(
      &db,
      TypParams::new(&db, vec![typ_variable1, typ_variable2_string], vec![]),
      Some(LazyTyp::eager(dict_variable_string_bound)),
    )
    .into();

    // witness T2 = number violates bound string
    // Dict[string, number] <= exists T1 <: string, T2 <: string. Dict[T1, T2]
    assert!(!is_subtype_of(
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

    let typ_variable_string = TypVariable::get(&db, Some(LazyTyp::eager(string_typ.clone())));
    let variable_string: TdTypEnum = TdVariableTyp::new(&db, 0, typ_variable_string).into();
    let list_variable_string: TdTypEnum = get_list_typ(&db)
      .instantiate(&db, vec![LazyTyp::eager(variable_string)])
      .typ(&db);

    let existential_string: TdTypEnum = TdExistentialTyp::new(
      &db,
      TypParams::new(&db, vec![typ_variable_string], vec![]),
      Some(LazyTyp::eager(list_variable_string)),
    )
    .into();

    let typ_variable_obj = TypVariable::get(&db, Some(LazyTyp::eager(object_typ.clone())));
    let variable_obj: TdTypEnum = TdVariableTyp::new(&db, 0, typ_variable_obj).into();
    let list_variable_obj: TdTypEnum = get_list_typ(&db)
      .instantiate(&db, vec![LazyTyp::eager(variable_obj)])
      .typ(&db);
    let existential_obj: TdTypEnum = TdExistentialTyp::new(
      &db,
      TypParams::new(&db, vec![typ_variable_obj], vec![]),
      Some(LazyTyp::eager(list_variable_obj)),
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
    let db = make_db();
    let string_typ: TdTypEnum = get_string_typ(&db).into();
    let object_typ: TdTypEnum = get_obj_typ(&db).into();

    let typ_variable_obj = TypVariable::get(&db, Some(LazyTyp::eager(object_typ.clone())));
    let variable_obj: TdTypEnum = TdVariableTyp::new(&db, 0, typ_variable_obj).into();
    let list_variable_obj: TdTypEnum = get_list_typ(&db)
      .instantiate(&db, vec![LazyTyp::eager(variable_obj)])
      .typ(&db);
    let existential_obj: TdTypEnum = TdExistentialTyp::new(
      &db,
      TypParams::new(&db, vec![typ_variable_obj], vec![]),
      Some(LazyTyp::eager(list_variable_obj)),
    )
    .into();

    let list_string: TdTypEnum = get_list_typ(&db)
      .instantiate(&db, vec![LazyTyp::eager(string_typ.clone())])
      .typ(&db);

    // bare subtype solves witness T = string
    // List[string] <= exists T <: Object. List[T]
    assert!(is_subtype_of(&db, &list_string, &existential_obj));

    let typ_variable_string = TypVariable::get(&db, Some(LazyTyp::eager(string_typ.clone())));
    let variable_string: TdTypEnum = TdVariableTyp::new(&db, 0, typ_variable_string).into();
    let list_variable_string: TdTypEnum = get_list_typ(&db)
      .instantiate(&db, vec![LazyTyp::eager(variable_string)])
      .typ(&db);
    let existential_string: TdTypEnum = TdExistentialTyp::new(
      &db,
      TypParams::new(&db, vec![typ_variable_string], vec![]),
      Some(LazyTyp::eager(list_variable_string)),
    )
    .into();

    let list_obj: TdTypEnum = get_list_typ(&db)
      .instantiate(&db, vec![LazyTyp::eager(object_typ)])
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
