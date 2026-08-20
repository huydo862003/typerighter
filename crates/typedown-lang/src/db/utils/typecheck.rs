//! Shared type compatibility utilities for typechecking

use crate::db::TypedownDatabase;
use crate::db::types::TdTypeEnum;
use crate::db::types::derived::object_system::TdStaticType;
use crate::db::types::fields_compatible;
use typedown_incremental::Id;

// Check if expected type accepts actual type
pub fn is_assignable_from(
  db: &TypedownDatabase,
  expected: &TdTypeEnum,
  actual: &TdTypeEnum,
) -> bool {
  if actual.as_td_never_type().is_some() {
    return true;
  }
  if let Some(sum) = actual.as_td_sum_type() {
    return sum.members(db).iter().all(|m| {
      m.resolve(db)
        .is_some_and(|t| is_assignable_from(db, expected, &t))
    });
  }
  if expected.as_id() == actual.as_id() {
    return true;
  }
  match expected {
    TdTypeEnum::TdTypeType(_) => true,
    TdTypeEnum::TdNeverType(_) => false,
    TdTypeEnum::TdSumType(sum) => sum.members(db).iter().any(|member| {
      member
        .resolve(db)
        .is_some_and(|member_type| is_assignable_from(db, &member_type, actual))
    }),
    TdTypeEnum::TdLiteralType(_) => false,
    TdTypeEnum::TdStrType(_) => {
      matches!(
        actual,
        TdTypeEnum::TdLiteralType(lit)
          if matches!(lit.underlying_type(db), TdTypeEnum::TdStrType(_))
      ) || matches!(
        actual,
        TdTypeEnum::TdDateTimeType(_) | TdTypeEnum::TdDateType(_) | TdTypeEnum::TdTimeType(_)
      )
    }
    TdTypeEnum::TdNumType(_) => matches!(
      actual,
      TdTypeEnum::TdLiteralType(lit)
        if matches!(lit.underlying_type(db), TdTypeEnum::TdNumType(_))
    ),
    TdTypeEnum::TdBoolType(_) => matches!(
      actual,
      TdTypeEnum::TdLiteralType(lit)
        if matches!(lit.underlying_type(db), TdTypeEnum::TdBoolType(_))
    ),
    TdTypeEnum::TdFuncType(expected_func) => {
      if let TdTypeEnum::TdFuncType(actual_func) = actual {
        let expected_sig = expected_func.signature(db);
        let actual_sig = actual_func.signature(db);
        let expected_params = expected_sig.params(db);
        let actual_params = actual_sig.params(db);
        if expected_params.len() != actual_params.len() {
          return false;
        }
        for (ep, ap) in expected_params.iter().zip(actual_params.iter()) {
          if !is_assignable_from(db, ap, ep) {
            return false;
          }
        }
        is_assignable_from(db, &expected_sig.ret(db), &actual_sig.ret(db))
      } else {
        false
      }
    }
    TdTypeEnum::TdListType(expected_list) => {
      if let TdTypeEnum::TdListType(actual_list) = actual {
        match (
          expected_list.elem(db).and_then(|e| e.resolve(db)),
          actual_list.elem(db).and_then(|e| e.resolve(db)),
        ) {
          (None, _) => true,
          (Some(_), None) => false,
          (Some(exp_elem), Some(act_elem)) => is_assignable_from(db, &exp_elem, &act_elem),
        }
      } else {
        false
      }
    }
    TdTypeEnum::TdDictType(expected_dict) => match actual {
      TdTypeEnum::TdDictType(_) => {
        let self_args = expected.get_type_args(db);
        if self_args.is_empty() {
          return true;
        }
        let actual_args = actual.get_type_args(db);
        if actual_args.is_empty() {
          return false;
        }
        self_args
          .iter()
          .zip(actual_args.iter())
          .all(|(s, a)| is_assignable_from(db, s, a))
      }
      TdTypeEnum::TdProductType(product) => {
        let value_type = match expected_dict.value(db).and_then(|l| l.resolve(db)) {
          Some(vt) => vt,
          None => return true,
        };
        product.fields(db).values().all(|field_lazy| {
          field_lazy
            .resolve(db)
            .is_some_and(|ft| is_assignable_from(db, &value_type, &ft))
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
            .is_some_and(|ft| is_assignable_from(db, &value_type, &ft))
        })
      }
      _ => false,
    },
    TdTypeEnum::TdProductType(expected_product) => {
      if let TdTypeEnum::TdStructuralType(structural) = actual {
        fields_compatible(db, &expected_product.fields(db), &structural.fields(db))
      } else {
        false
      }
    }
    TdTypeEnum::TdStructuralType(expected_structural) => match actual {
      TdTypeEnum::TdProductType(product) => {
        fields_compatible(db, &expected_structural.fields(db), &product.fields(db))
      }
      TdTypeEnum::TdStructuralType(structural) => {
        fields_compatible(db, &expected_structural.fields(db), &structural.fields(db))
      }
      _ => false,
    },
    _ => false,
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
    get_bool_type, get_date_type, get_datetime_type, get_list_type, get_literal_type,
    get_never_type, get_null_type, get_num_type, get_schema_type, get_str_type, get_sum_type,
    get_time_type, get_type_type,
  };
  use crate::db::types::{
    LazyType, LiteralValue, TdDictType, TdFuncType, TdListType, TdProductType, TdStructuralType,
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
    assert!(is_assignable_from(&db, &string, &string));
  }

  #[test]
  fn incompatible_simple_different_type() {
    let db = db();
    let string: TdTypeEnum = get_str_type(&db).into();
    let number: TdTypeEnum = get_num_type(&db).into();
    assert!(!is_assignable_from(&db, &string, &number));
  }

  // Literal vs Simple

  #[test]
  fn literal_compatible_with_base_simple() {
    let db = db();
    let string: TdTypeEnum = get_str_type(&db).into();
    let lit = lit_str(&db, "hello");
    assert!(is_assignable_from(&db, &string, &lit));
  }

  #[test]
  fn literal_incompatible_with_wrong_simple() {
    let db = db();
    let number: TdTypeEnum = get_num_type(&db).into();
    let lit = lit_str(&db, "hello");
    assert!(!is_assignable_from(&db, &number, &lit));
  }

  // Literal vs Literal

  #[test]
  fn literal_compatible_same_value() {
    let db = db();
    let lit1 = lit_str(&db, "draft");
    let lit2 = lit_str(&db, "draft");
    assert!(is_assignable_from(&db, &lit1, &lit2));
  }

  #[test]
  fn literal_incompatible_different_value() {
    let db = db();
    let lit1 = lit_str(&db, "draft");
    let lit2 = lit_str(&db, "published");
    assert!(!is_assignable_from(&db, &lit1, &lit2));
  }

  // Never is the bottom type: assignable to anything, but nothing is assignable to it

  #[test]
  fn never_is_bottom_type() {
    let db = db();
    let string: TdTypeEnum = get_str_type(&db).into();
    let never: TdTypeEnum = get_never_type(&db).into();
    assert!(!is_assignable_from(&db, &never, &string));
    assert!(is_assignable_from(&db, &string, &never));
  }

  #[test]
  fn literal_num_compatible_with_number() {
    let db = db();
    let number: TdTypeEnum = get_num_type(&db).into();
    let lit = lit_num(&db, "42");
    assert!(is_assignable_from(&db, &number, &lit));
  }

  #[test]
  fn literal_num_incompatible_with_string() {
    let db = db();
    let string: TdTypeEnum = get_str_type(&db).into();
    let lit = lit_num(&db, "42");
    assert!(!is_assignable_from(&db, &string, &lit));
  }

  #[test]
  fn string_accepts_string_literal() {
    let db = db();
    let string: TdTypeEnum = get_str_type(&db).into();
    let lit: TdTypeEnum = get_literal_type(&db, LiteralValue::Str("hello".to_string())).into();
    assert!(is_assignable_from(&db, &string, &lit));
  }

  #[test]
  fn number_accepts_number_literal() {
    let db = db();
    let number: TdTypeEnum = get_num_type(&db).into();
    let lit: TdTypeEnum = get_literal_type(&db, LiteralValue::Num("42".to_string())).into();
    assert!(is_assignable_from(&db, &number, &lit));
  }

  #[test]
  fn boolean_accepts_boolean_literal() {
    let db = db();
    let boolean: TdTypeEnum = get_bool_type(&db).into();
    let lit: TdTypeEnum = get_literal_type(&db, LiteralValue::Bool(true)).into();
    assert!(is_assignable_from(&db, &boolean, &lit));
  }

  #[test]
  fn string_rejects_number_literal() {
    let db = db();
    let string: TdTypeEnum = get_str_type(&db).into();
    let lit: TdTypeEnum = get_literal_type(&db, LiteralValue::Num("42".to_string())).into();
    assert!(!is_assignable_from(&db, &string, &lit));
  }

  #[test]
  fn literal_accepts_same_value() {
    let db = db();
    let lit1: TdTypeEnum = get_literal_type(&db, LiteralValue::Str("draft".to_string())).into();
    let lit2: TdTypeEnum = get_literal_type(&db, LiteralValue::Str("draft".to_string())).into();
    assert!(is_assignable_from(&db, &lit1, &lit2));
  }

  #[test]
  fn literal_rejects_different_value() {
    let db = db();
    let lit1: TdTypeEnum = get_literal_type(&db, LiteralValue::Str("draft".to_string())).into();
    let lit2: TdTypeEnum = get_literal_type(&db, LiteralValue::Str("published".to_string())).into();
    assert!(!is_assignable_from(&db, &lit1, &lit2));
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
    assert!(is_assignable_from(&db, &str_or_num, &string));
  }

  #[test]
  fn sum_rejects_non_member_type() {
    let db = db();
    let str_or_num = sum(
      &db,
      vec![get_str_type(&db).into(), get_num_type(&db).into()],
    );
    let boolean: TdTypeEnum = get_bool_type(&db).into();
    assert!(!is_assignable_from(&db, &str_or_num, &boolean));
  }

  #[test]
  fn sum_accepts_literal_of_member_type() {
    let db = db();
    let str_or_num = sum(
      &db,
      vec![get_str_type(&db).into(), get_num_type(&db).into()],
    );
    let lit = lit_str(&db, "hello");
    assert!(is_assignable_from(&db, &str_or_num, &lit));
  }

  #[test]
  fn sum_accepts_sub_sum() {
    let db = db();
    let str_or_num = sum(
      &db,
      vec![get_str_type(&db).into(), get_num_type(&db).into()],
    );
    let just_str = sum(&db, vec![get_str_type(&db).into()]);
    assert!(is_assignable_from(&db, &str_or_num, &just_str));
  }

  #[test]
  fn sum_rejects_wider_sum() {
    let db = db();
    let just_str = sum(&db, vec![get_str_type(&db).into()]);
    let str_or_num = sum(
      &db,
      vec![get_str_type(&db).into(), get_num_type(&db).into()],
    );
    assert!(!is_assignable_from(&db, &just_str, &str_or_num));
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
    assert!(is_assignable_from(&db, &nullable_str, &null));
  }

  #[test]
  fn nullable_accepts_base_type() {
    let db = db();
    let nullable_str = sum(
      &db,
      vec![get_str_type(&db).into(), get_null_type(&db).into()],
    );
    let string: TdTypeEnum = get_str_type(&db).into();
    assert!(is_assignable_from(&db, &nullable_str, &string));
  }

  #[test]
  fn non_nullable_rejects_null() {
    let db = db();
    let string: TdTypeEnum = get_str_type(&db).into();
    let null: TdTypeEnum = get_null_type(&db).into();
    assert!(!is_assignable_from(&db, &string, &null));
  }

  // Never type tests

  #[test]
  fn never_accepted_by_any_type() {
    let db = db();
    let never: TdTypeEnum = get_never_type(&db).into();
    let string: TdTypeEnum = get_str_type(&db).into();
    let number: TdTypeEnum = get_num_type(&db).into();
    let boolean: TdTypeEnum = get_bool_type(&db).into();
    assert!(is_assignable_from(&db, &string, &never));
    assert!(is_assignable_from(&db, &number, &never));
    assert!(is_assignable_from(&db, &boolean, &never));
  }

  #[test]
  fn never_accepted_by_sum() {
    let db = db();
    let never: TdTypeEnum = get_never_type(&db).into();
    let str_or_num = sum(
      &db,
      vec![get_str_type(&db).into(), get_num_type(&db).into()],
    );
    assert!(is_assignable_from(&db, &str_or_num, &never));
  }

  #[test]
  fn nothing_accepted_by_never() {
    let db = db();
    let never: TdTypeEnum = get_never_type(&db).into();
    let string: TdTypeEnum = get_str_type(&db).into();
    assert!(!is_assignable_from(&db, &never, &string));
  }

  // Literal sum (enum) tests

  #[test]
  fn literal_sum_accepts_matching_literal() {
    let db = db();
    let status = sum(&db, vec![lit_str(&db, "draft"), lit_str(&db, "published")]);
    let draft = lit_str(&db, "draft");
    assert!(is_assignable_from(&db, &status, &draft));
  }

  #[test]
  fn literal_sum_rejects_non_matching_literal() {
    let db = db();
    let status = sum(&db, vec![lit_str(&db, "draft"), lit_str(&db, "published")]);
    let archived = lit_str(&db, "archived");
    assert!(!is_assignable_from(&db, &status, &archived));
  }

  // String accepts sum of string literals
  #[test]
  fn string_accepts_sum_of_string_literals() {
    let db = db();
    let string: TdTypeEnum = get_str_type(&db).into();
    let status = sum(&db, vec![lit_str(&db, "draft"), lit_str(&db, "published")]);
    assert!(is_assignable_from(&db, &string, &status));
  }

  // String rejects sum with non-string member
  #[test]
  fn string_rejects_mixed_sum() {
    let db = db();
    let string: TdTypeEnum = get_str_type(&db).into();
    let mixed = sum(&db, vec![lit_str(&db, "draft"), get_num_type(&db).into()]);
    assert!(!is_assignable_from(&db, &string, &mixed));
  }

  // List type tests

  #[test]
  fn list_accepts_same_elem_type() {
    let db = db();
    let list_str: TdTypeEnum =
      TdListType::new(&db, Some(LazyType::eager(get_str_type(&db).into()))).into();
    let list_str2: TdTypeEnum =
      TdListType::new(&db, Some(LazyType::eager(get_str_type(&db).into()))).into();
    assert!(is_assignable_from(&db, &list_str, &list_str2));
  }

  #[test]
  fn list_rejects_different_elem_type() {
    let db = db();
    let list_str: TdTypeEnum =
      TdListType::new(&db, Some(LazyType::eager(get_str_type(&db).into()))).into();
    let list_num: TdTypeEnum =
      TdListType::new(&db, Some(LazyType::eager(get_num_type(&db).into()))).into();
    assert!(!is_assignable_from(&db, &list_str, &list_num));
  }

  #[test]
  fn untyped_list_accepts_any_list() {
    let db = db();
    let untyped: TdTypeEnum = get_list_type(&db).into();
    let list_str: TdTypeEnum =
      TdListType::new(&db, Some(LazyType::eager(get_str_type(&db).into()))).into();
    assert!(is_assignable_from(&db, &untyped, &list_str));
  }

  // Dict type tests

  #[test]
  fn dict_accepts_same_value_type() {
    let db = db();
    let dict_str: TdTypeEnum = TdDictType::new(
      &db,
      Some(LazyType::eager(get_str_type(&db).into())),
      Some(LazyType::eager(get_str_type(&db).into())),
    )
    .into();
    let dict_str2: TdTypeEnum = TdDictType::new(
      &db,
      Some(LazyType::eager(get_str_type(&db).into())),
      Some(LazyType::eager(get_str_type(&db).into())),
    )
    .into();
    assert!(is_assignable_from(&db, &dict_str, &dict_str2));
  }

  #[test]
  fn dict_rejects_different_value_type() {
    let db = db();
    let dict_str: TdTypeEnum = TdDictType::new(
      &db,
      Some(LazyType::eager(get_str_type(&db).into())),
      Some(LazyType::eager(get_str_type(&db).into())),
    )
    .into();
    let dict_num: TdTypeEnum = TdDictType::new(
      &db,
      Some(LazyType::eager(get_str_type(&db).into())),
      Some(LazyType::eager(get_num_type(&db).into())),
    )
    .into();
    assert!(!is_assignable_from(&db, &dict_str, &dict_num));
  }

  // String accepts date/time subtypes

  #[test]
  fn string_accepts_date() {
    let db = db();
    let string: TdTypeEnum = get_str_type(&db).into();
    let date: TdTypeEnum = get_date_type(&db).into();
    assert!(is_assignable_from(&db, &string, &date));
  }

  #[test]
  fn string_accepts_datetime() {
    let db = db();
    let string: TdTypeEnum = get_str_type(&db).into();
    let datetime: TdTypeEnum = get_datetime_type(&db).into();
    assert!(is_assignable_from(&db, &string, &datetime));
  }

  #[test]
  fn string_accepts_time() {
    let db = db();
    let string: TdTypeEnum = get_str_type(&db).into();
    let time: TdTypeEnum = get_time_type(&db).into();
    assert!(is_assignable_from(&db, &string, &time));
  }

  #[test]
  fn date_rejects_string() {
    let db = db();
    let date: TdTypeEnum = get_date_type(&db).into();
    let string: TdTypeEnum = get_str_type(&db).into();
    assert!(!is_assignable_from(&db, &date, &string));
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
    assert!(is_assignable_from(&db, &expected, &actual));
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
    assert!(!is_assignable_from(&db, &expected, &actual));
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
    assert!(is_assignable_from(&db, &expected, &actual));
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
    assert!(!is_assignable_from(&db, &expected, &actual));
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
    assert!(is_assignable_from(&db, &expected, &actual));
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
    assert!(is_assignable_from(&db, &product, &structural));
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
    assert!(!is_assignable_from(&db, &product, &structural));
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
    assert!(!is_assignable_from(&db, &product, &structural));
  }

  #[test]
  fn null_rejects_non_null() {
    let db = db();
    let null: TdTypeEnum = get_null_type(&db).into();
    let string: TdTypeEnum = get_str_type(&db).into();
    assert!(!is_assignable_from(&db, &null, &string));
  }

  #[test]
  fn list_rejects_non_list() {
    let db = db();
    let list_str: TdTypeEnum =
      TdListType::new(&db, Some(LazyType::eager(get_str_type(&db).into()))).into();
    let string: TdTypeEnum = get_str_type(&db).into();
    assert!(!is_assignable_from(&db, &list_str, &string));
  }

  #[test]
  fn dict_rejects_non_dict() {
    let db = db();
    let dict: TdTypeEnum = TdDictType::new(
      &db,
      Some(LazyType::eager(get_str_type(&db).into())),
      Some(LazyType::eager(get_str_type(&db).into())),
    )
    .into();
    let string: TdTypeEnum = get_str_type(&db).into();
    assert!(!is_assignable_from(&db, &dict, &string));
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
    assert!(is_assignable_from(&db, &func_type, &func2_type));
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
    assert!(is_assignable_from(&db, &expected_type, &actual_type));
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
    assert!(!is_assignable_from(&db, &expected_type, &actual_type));
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
    assert!(is_assignable_from(&db, &expected_type, &actual_type));
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
    assert!(!is_assignable_from(&db, &expected_type, &actual_type));
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
    assert!(!is_assignable_from(&db, &expected_type, &actual_type));
  }
}
