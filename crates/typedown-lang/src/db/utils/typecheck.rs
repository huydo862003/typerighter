//! Shared type compatibility utilities for typechecking

use crate::db::TypedownDatabase;
use crate::db::types::derived::object_system::TdStaticType;
use crate::db::types::fields_compatible;
use crate::db::types::{LazyType, TdTypeEnum, TypeParams};
use crate::syntax::diagnostic::Diagnostic;
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
      if let Some(bound) = p.bound(db).as_ref().and_then(|b| b.resolve(db))
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

/// Check if `subtype` is a subtype of `supertype` (subtyping check)
pub fn is_subtype_of(db: &TypedownDatabase, subtype: &TdTypeEnum, supertype: &TdTypeEnum) -> bool {
  /// Phase 1: Check type constructor compatibility ignoring type arguments and parameter variance
  fn are_constructors_compatible(
    db: &TypedownDatabase,
    subtype: &TdTypeEnum, // INVARIANT: Due to sum type elimination, sub type cannot be a sum type here
    supertype: &TdTypeEnum,
  ) -> bool {
    if subtype.as_id() == supertype.as_id() {
      return true;
    }
    match supertype {
      TdTypeEnum::TdTypeType(_) => true,
      TdTypeEnum::TdNeverType(_) => false,
      // WARNING: This only works because subtype is not a sum type
      TdTypeEnum::TdSumType(sum) => sum.members(db).iter().any(|member| {
        member
          .resolve(db)
          .is_some_and(|m| is_subtype_of(db, subtype, &m))
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
              .is_some_and(|ft| is_subtype_of(db, &ft, &value_type))
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
              .is_some_and(|ft| is_subtype_of(db, &ft, &value_type))
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
      TdTypeEnum::TdVariableType(_) => false,
    }
  }

  /// Phase 2: Check type arguments and parameter variance between compatible constructors
  fn are_type_args_compatible(
    db: &TypedownDatabase,
    subtype: &TdTypeEnum,
    supertype: &TdTypeEnum,
  ) -> bool {
    match (subtype, supertype) {
      (TdTypeEnum::TdListType(sub_list), TdTypeEnum::TdListType(super_list)) => {
        match (
          sub_list.elem(db).and_then(|e| e.resolve(db)),
          super_list.elem(db).and_then(|e| e.resolve(db)),
        ) {
          (_, None) => true,
          (None, Some(_)) => false,
          (Some(sub_elem), Some(super_elem)) => is_subtype_of(db, &sub_elem, &super_elem),
        }
      }
      (TdTypeEnum::TdDictType(_), TdTypeEnum::TdDictType(_)) => {
        let super_args = supertype.get_type_args(db);
        if super_args.is_empty() {
          return true;
        }
        let sub_args = subtype.get_type_args(db);
        if sub_args.is_empty() {
          return false;
        }
        sub_args
          .iter()
          .zip(super_args.iter())
          .all(|(s, p)| is_subtype_of(db, s, p))
      }
      (TdTypeEnum::TdFuncType(sub_func), TdTypeEnum::TdFuncType(super_func)) => {
        let sub_sig = sub_func.signature(db);
        let super_sig = super_func.signature(db);
        let sub_params = sub_sig.params(db);
        let super_params = super_sig.params(db);
        if sub_params.len() != super_params.len() {
          return false;
        }
        // Function parameters are contravariant: super_p must be a subtype of sub_p
        for (sub_p, super_p) in sub_params.iter().zip(super_params.iter()) {
          if !is_subtype_of(db, super_p, sub_p) {
            return false;
          }
        }
        // Return type is covariant: sub_ret must be a subtype of super_ret
        is_subtype_of(db, &sub_sig.ret(db), &super_sig.ret(db))
      }
      _ => true,
    }
  }

  // Phase 0: Special pre-check
  match (subtype, supertype) {
    (TdTypeEnum::TdVariableType(var_sub), TdTypeEnum::TdVariableType(var_super)) => {
      if subtype == supertype {
        return true;
      }
      let v_sub = var_sub.variable(db);
      let v_super = var_super.variable(db);
      if let Some(val_sub) = v_sub.value(db).and_then(|l| l.resolve(db))
        && let Some(val_super) = v_super.value(db).and_then(|l| l.resolve(db))
      {
        return is_subtype_of(db, &val_sub, &val_super);
      }
      false
    }
    (TdTypeEnum::TdVariableType(var_sub), _) => {
      let v_sub = var_sub.variable(db);
      if let Some(val) = v_sub.value(db).and_then(|l| l.resolve(db)) {
        is_subtype_of(db, &val, supertype)
      } else if let Some(bound) = v_sub.bound(db).and_then(|l| l.resolve(db)) {
        is_subtype_of(db, &bound, supertype)
      } else {
        false
      }
    }
    (_, TdTypeEnum::TdVariableType(var_super)) => {
      let v_super = var_super.variable(db);
      if let Some(val) = v_super.value(db).and_then(|l| l.resolve(db)) {
        is_subtype_of(db, subtype, &val)
      } else {
        false
      }
    }
    _ => {
      if subtype.as_td_never_type().is_some() {
        return true;
      }

      // Sum type elimination
      if let Some(sum) = subtype.as_td_sum_type() {
        return sum.members(db).iter().all(|m| {
          m.resolve(db)
            .is_some_and(|t| is_subtype_of(db, &t, supertype))
        });
      }

      // It's sensible that subtype cannot be a sum type here

      // Phase 1: Type constructor compatibility check
      if !are_constructors_compatible(db, subtype, supertype) {
        return false;
      }

      // Phase 2: Same-nature parameter and variance check
      are_type_args_compatible(db, subtype, supertype)
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
    get_literal_type, get_never_type, get_null_type, get_num_type, get_schema_type, get_str_type,
    get_sum_type, get_time_type, get_type_type,
  };
  use crate::db::types::{
    LazyType, LiteralValue, TdFuncType, TdProductType, TdStructuralType, TdVariableType,
    TypeVariable,
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
    let just_str = sum(&db, vec![get_str_type(&db).into()]);
    assert!(is_subtype_of(&db, &just_str, &str_or_num));
  }

  #[test]
  fn sum_rejects_wider_sum() {
    let db = db();
    let just_str = sum(&db, vec![get_str_type(&db).into()]);
    let str_or_num = sum(
      &db,
      vec![get_str_type(&db).into(), get_num_type(&db).into()],
    );
    assert!(!is_subtype_of(&db, &str_or_num, &just_str));
  }

  // Null type tests

  #[test]
  fn nullable_accepts_null() {
    let db = db();
    let nullable_str = sum(
      &db,
      vec![get_str_type(&db).into(), get_null_type(&db).into()],
    );
    let null: TdTypeEnum = get_null_type(&db).into();
    assert!(is_subtype_of(&db, &null, &nullable_str));
  }

  #[test]
  fn nullable_accepts_base_type() {
    let db = db();
    let nullable_str = sum(
      &db,
      vec![get_str_type(&db).into(), get_null_type(&db).into()],
    );
    let string: TdTypeEnum = get_str_type(&db).into();
    assert!(is_subtype_of(&db, &string, &nullable_str));
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
    let list_str: TdTypeEnum = get_list_type(&db)
      .instantiate(&db, vec![LazyType::eager(get_str_type(&db).into())])
      .typ(&db);
    let list_str2: TdTypeEnum = get_list_type(&db)
      .instantiate(&db, vec![LazyType::eager(get_str_type(&db).into())])
      .typ(&db);
    assert!(is_subtype_of(&db, &list_str2, &list_str));
  }

  #[test]
  fn list_accepts_covariant_elem_type() {
    let db = db();
    let list_str: TdTypeEnum = get_list_type(&db)
      .instantiate(&db, vec![LazyType::eager(get_str_type(&db).into())])
      .typ(&db);
    let list_lit: TdTypeEnum = get_list_type(&db)
      .instantiate(&db, vec![LazyType::eager(lit_str(&db, "hello"))])
      .typ(&db);
    assert!(is_subtype_of(&db, &list_lit, &list_str));
  }

  #[test]
  fn list_rejects_wider_elem_type() {
    let db = db();
    let list_str: TdTypeEnum = get_list_type(&db)
      .instantiate(&db, vec![LazyType::eager(get_str_type(&db).into())])
      .typ(&db);
    let list_lit: TdTypeEnum = get_list_type(&db)
      .instantiate(&db, vec![LazyType::eager(lit_str(&db, "hello"))])
      .typ(&db);
    assert!(!is_subtype_of(&db, &list_str, &list_lit));
  }

  #[test]
  fn list_rejects_different_elem_type() {
    let db = db();
    let list_str: TdTypeEnum = get_list_type(&db)
      .instantiate(&db, vec![LazyType::eager(get_str_type(&db).into())])
      .typ(&db);
    let list_num: TdTypeEnum = get_list_type(&db)
      .instantiate(&db, vec![LazyType::eager(get_num_type(&db).into())])
      .typ(&db);
    assert!(!is_subtype_of(&db, &list_num, &list_str));
  }

  #[test]
  fn untyped_list_accepts_any_list() {
    let db = db();
    let untyped: TdTypeEnum = get_list_type(&db).into();
    let list_str: TdTypeEnum = get_list_type(&db)
      .instantiate(&db, vec![LazyType::eager(get_str_type(&db).into())])
      .typ(&db);
    assert!(is_subtype_of(&db, &list_str, &untyped));
  }

  #[test]
  fn typed_list_rejects_untyped_list() {
    let db = db();
    let untyped: TdTypeEnum = get_list_type(&db).into();
    let list_str: TdTypeEnum = get_list_type(&db)
      .instantiate(&db, vec![LazyType::eager(get_str_type(&db).into())])
      .typ(&db);
    assert!(!is_subtype_of(&db, &untyped, &list_str));
  }

  // Dict type tests

  #[test]
  fn dict_accepts_same_value_type() {
    let db = db();
    let dict_str: TdTypeEnum = get_dict_type(&db)
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
    assert!(is_subtype_of(&db, &dict_str2, &dict_str));
  }

  #[test]
  fn dict_accepts_covariant_key_and_value_types() {
    let db = db();
    let dict_str: TdTypeEnum = get_dict_type(&db)
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
    assert!(is_subtype_of(&db, &dict_lit, &dict_str));
  }

  #[test]
  fn dict_rejects_wider_value_type() {
    let db = db();
    let dict_str: TdTypeEnum = get_dict_type(&db)
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
    assert!(!is_subtype_of(&db, &dict_str, &dict_lit));
  }

  #[test]
  fn dict_rejects_different_value_type() {
    let db = db();
    let dict_str: TdTypeEnum = get_dict_type(&db)
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
    assert!(!is_subtype_of(&db, &dict_num, &dict_str));
  }

  #[test]
  fn untyped_dict_accepts_typed_dict() {
    let db = db();
    let untyped: TdTypeEnum = get_dict_type(&db).into();
    let dict_str: TdTypeEnum = get_dict_type(&db)
      .instantiate(
        &db,
        vec![
          LazyType::eager(get_str_type(&db).into()),
          LazyType::eager(get_str_type(&db).into()),
        ],
      )
      .typ(&db);
    assert!(is_subtype_of(&db, &dict_str, &untyped));
  }

  #[test]
  fn typed_dict_rejects_untyped_dict() {
    let db = db();
    let untyped: TdTypeEnum = get_dict_type(&db).into();
    let dict_str: TdTypeEnum = get_dict_type(&db)
      .instantiate(
        &db,
        vec![
          LazyType::eager(get_str_type(&db).into()),
          LazyType::eager(get_str_type(&db).into()),
        ],
      )
      .typ(&db);
    assert!(!is_subtype_of(&db, &untyped, &dict_str));
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
    let nullable_str = sum(
      &db,
      vec![get_str_type(&db).into(), get_null_type(&db).into()],
    );
    let expected: TdTypeEnum = TdStructuralType::new(
      &db,
      HashMap::from([("name".to_string(), LazyType::eager(nullable_str))]),
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
    let list_str: TdTypeEnum = get_list_type(&db)
      .instantiate(&db, vec![LazyType::eager(get_str_type(&db).into())])
      .typ(&db);
    let string: TdTypeEnum = get_str_type(&db).into();
    assert!(!is_subtype_of(&db, &string, &list_str));
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
  fn type_var_strict_equality() {
    let db = db();
    let string: TdTypeEnum = get_str_type(&db).into();
    let number: TdTypeEnum = get_num_type(&db).into();
    let tv1 = TypeVariable::get(&db, Some(LazyType::eager(string)), None);
    let tv2 = TypeVariable::get(&db, Some(LazyType::eager(number)), None);
    let var1: TdTypeEnum = TdVariableType::new(&db, 0, tv1).into();
    let var2: TdTypeEnum = TdVariableType::new(&db, 1, tv2).into();
    assert!(is_subtype_of(&db, &var1, &var1));
    assert!(!is_subtype_of(&db, &var1, &var2));
  }

  #[test]
  fn type_var_instantiated_delegation() {
    let db = db();
    let string: TdTypeEnum = get_str_type(&db).into();
    let tv = TypeVariable::get(&db, None, Some(LazyType::eager(string.clone())));
    let var: TdTypeEnum = TdVariableType::new(&db, 0, tv).into();
    assert!(is_subtype_of(&db, &var, &string));
  }

  #[test]
  fn type_var_bounded_delegation() {
    let db = db();
    let string: TdTypeEnum = get_str_type(&db).into();
    let number: TdTypeEnum = get_num_type(&db).into();
    let tv = TypeVariable::get(&db, Some(LazyType::eager(string.clone())), None);
    let var: TdTypeEnum = TdVariableType::new(&db, 0, tv).into();
    assert!(is_subtype_of(&db, &var, &string));
    assert!(!is_subtype_of(&db, &var, &number));
  }

  #[test]
  fn type_var_both_instantiated_subtyping() {
    let db = db();
    let string: TdTypeEnum = get_str_type(&db).into();
    let literal: TdTypeEnum = lit_str(&db, "hello");
    let tv_lit = TypeVariable::get(&db, None, Some(LazyType::eager(literal)));
    let tv_str = TypeVariable::get(&db, None, Some(LazyType::eager(string.clone())));
    let var_lit: TdTypeEnum = TdVariableType::new(&db, 0, tv_lit).into();
    let var_str: TdTypeEnum = TdVariableType::new(&db, 1, tv_str).into();
    assert!(is_subtype_of(&db, &var_lit, &var_str));
    assert!(!is_subtype_of(&db, &var_str, &var_lit));
  }

  #[test]
  fn type_var_unbound_rejects_concrete_type() {
    let db = db();
    let string: TdTypeEnum = get_str_type(&db).into();
    let tv_unbound = TypeVariable::get(&db, None, None);
    let var: TdTypeEnum = TdVariableType::new(&db, 0, tv_unbound).into();
    assert!(!is_subtype_of(&db, &var, &string));
    assert!(!is_subtype_of(&db, &string, &var));
  }

  #[test]
  fn type_var_in_list_covariance() {
    let db = db();
    let string: TdTypeEnum = get_str_type(&db).into();
    let literal: TdTypeEnum = lit_str(&db, "hello");
    let tv_lit = TypeVariable::get(&db, None, Some(LazyType::eager(literal)));
    let var_lit: TdTypeEnum = TdVariableType::new(&db, 0, tv_lit).into();

    let list_var = get_list_type(&db)
      .instantiate(&db, vec![LazyType::eager(var_lit)])
      .typ(&db);
    let list_str = get_list_type(&db)
      .instantiate(&db, vec![LazyType::eager(string)])
      .typ(&db);

    assert!(is_subtype_of(&db, &list_var, &list_str));
    assert!(!is_subtype_of(&db, &list_str, &list_var));
  }
}
