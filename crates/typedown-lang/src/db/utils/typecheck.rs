//! Shared type compatibility utilities for typechecking

use crate::db::TypedownDatabase;
use crate::db::types::TdTypeEnum;

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
    TdTypeLike,
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
    assert!(string.accepts(&db, &string));
  }

  #[test]
  fn incompatible_simple_different_type() {
    let db = db();
    let string: TdTypeEnum = get_str_type(&db).into();
    let number: TdTypeEnum = get_num_type(&db).into();
    assert!(!string.accepts(&db, &number));
  }

  // Literal vs Simple

  #[test]
  fn literal_compatible_with_base_simple() {
    let db = db();
    let string: TdTypeEnum = get_str_type(&db).into();
    let lit = lit_str(&db, "hello");
    assert!(string.accepts(&db, &lit));
  }

  #[test]
  fn literal_incompatible_with_wrong_simple() {
    let db = db();
    let number: TdTypeEnum = get_num_type(&db).into();
    let lit = lit_str(&db, "hello");
    assert!(!number.accepts(&db, &lit));
  }

  // Literal vs Literal

  #[test]
  fn literal_compatible_same_value() {
    let db = db();
    let lit1 = lit_str(&db, "draft");
    let lit2 = lit_str(&db, "draft");
    assert!(lit1.accepts(&db, &lit2));
  }

  #[test]
  fn literal_incompatible_different_value() {
    let db = db();
    let lit1 = lit_str(&db, "draft");
    let lit2 = lit_str(&db, "published");
    assert!(!lit1.accepts(&db, &lit2));
  }

  // Never is the bottom type: assignable to anything, but nothing is assignable to it

  #[test]
  fn never_is_bottom_type() {
    let db = db();
    let string: TdTypeEnum = get_str_type(&db).into();
    let never: TdTypeEnum = get_never_type(&db).into();
    assert!(!never.accepts(&db, &string));
    assert!(string.accepts(&db, &never));
  }

  #[test]
  fn literal_num_compatible_with_number() {
    let db = db();
    let number: TdTypeEnum = get_num_type(&db).into();
    let lit = lit_num(&db, "42");
    assert!(number.accepts(&db, &lit));
  }

  #[test]
  fn literal_num_incompatible_with_string() {
    let db = db();
    let string: TdTypeEnum = get_str_type(&db).into();
    let lit = lit_num(&db, "42");
    assert!(!string.accepts(&db, &lit));
  }

  #[test]
  fn string_accepts_string_literal() {
    let db = db();
    let string: TdTypeEnum = get_str_type(&db).into();
    let lit: TdTypeEnum = get_literal_type(&db, LiteralValue::Str("hello".to_string())).into();
    assert!(string.accepts(&db, &lit));
  }

  #[test]
  fn number_accepts_number_literal() {
    let db = db();
    let number: TdTypeEnum = get_num_type(&db).into();
    let lit: TdTypeEnum = get_literal_type(&db, LiteralValue::Num("42".to_string())).into();
    assert!(number.accepts(&db, &lit));
  }

  #[test]
  fn boolean_accepts_boolean_literal() {
    let db = db();
    let boolean: TdTypeEnum = get_bool_type(&db).into();
    let lit: TdTypeEnum = get_literal_type(&db, LiteralValue::Bool(true)).into();
    assert!(boolean.accepts(&db, &lit));
  }

  #[test]
  fn string_rejects_number_literal() {
    let db = db();
    let string: TdTypeEnum = get_str_type(&db).into();
    let lit: TdTypeEnum = get_literal_type(&db, LiteralValue::Num("42".to_string())).into();
    assert!(!string.accepts(&db, &lit));
  }

  #[test]
  fn literal_accepts_same_value() {
    let db = db();
    let lit1: TdTypeEnum = get_literal_type(&db, LiteralValue::Str("draft".to_string())).into();
    let lit2: TdTypeEnum = get_literal_type(&db, LiteralValue::Str("draft".to_string())).into();
    assert!(lit1.accepts(&db, &lit2));
  }

  #[test]
  fn literal_rejects_different_value() {
    let db = db();
    let lit1: TdTypeEnum = get_literal_type(&db, LiteralValue::Str("draft".to_string())).into();
    let lit2: TdTypeEnum = get_literal_type(&db, LiteralValue::Str("published".to_string())).into();
    assert!(!lit1.accepts(&db, &lit2));
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
    assert!(str_or_num.accepts(&db, &string));
  }

  #[test]
  fn sum_rejects_non_member_type() {
    let db = db();
    let str_or_num = sum(
      &db,
      vec![get_str_type(&db).into(), get_num_type(&db).into()],
    );
    let boolean: TdTypeEnum = get_bool_type(&db).into();
    assert!(!str_or_num.accepts(&db, &boolean));
  }

  #[test]
  fn sum_accepts_literal_of_member_type() {
    let db = db();
    let str_or_num = sum(
      &db,
      vec![get_str_type(&db).into(), get_num_type(&db).into()],
    );
    let lit = lit_str(&db, "hello");
    assert!(str_or_num.accepts(&db, &lit));
  }

  #[test]
  fn sum_accepts_sub_sum() {
    let db = db();
    let str_or_num = sum(
      &db,
      vec![get_str_type(&db).into(), get_num_type(&db).into()],
    );
    let just_str = sum(&db, vec![get_str_type(&db).into()]);
    assert!(str_or_num.accepts(&db, &just_str));
  }

  #[test]
  fn sum_rejects_wider_sum() {
    let db = db();
    let just_str = sum(&db, vec![get_str_type(&db).into()]);
    let str_or_num = sum(
      &db,
      vec![get_str_type(&db).into(), get_num_type(&db).into()],
    );
    assert!(!just_str.accepts(&db, &str_or_num));
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
    assert!(nullable_str.accepts(&db, &null));
  }

  #[test]
  fn nullable_accepts_base_type() {
    let db = db();
    let nullable_str = sum(
      &db,
      vec![get_str_type(&db).into(), get_null_type(&db).into()],
    );
    let string: TdTypeEnum = get_str_type(&db).into();
    assert!(nullable_str.accepts(&db, &string));
  }

  #[test]
  fn non_nullable_rejects_null() {
    let db = db();
    let string: TdTypeEnum = get_str_type(&db).into();
    let null: TdTypeEnum = get_null_type(&db).into();
    assert!(!string.accepts(&db, &null));
  }

  // Never type tests

  #[test]
  fn never_accepted_by_any_type() {
    let db = db();
    let never: TdTypeEnum = get_never_type(&db).into();
    let string: TdTypeEnum = get_str_type(&db).into();
    let number: TdTypeEnum = get_num_type(&db).into();
    let boolean: TdTypeEnum = get_bool_type(&db).into();
    assert!(string.accepts(&db, &never));
    assert!(number.accepts(&db, &never));
    assert!(boolean.accepts(&db, &never));
  }

  #[test]
  fn never_accepted_by_sum() {
    let db = db();
    let never: TdTypeEnum = get_never_type(&db).into();
    let str_or_num = sum(
      &db,
      vec![get_str_type(&db).into(), get_num_type(&db).into()],
    );
    assert!(str_or_num.accepts(&db, &never));
  }

  #[test]
  fn nothing_accepted_by_never() {
    let db = db();
    let never: TdTypeEnum = get_never_type(&db).into();
    let string: TdTypeEnum = get_str_type(&db).into();
    assert!(!never.accepts(&db, &string));
  }

  // Literal sum (enum) tests

  #[test]
  fn literal_sum_accepts_matching_literal() {
    let db = db();
    let status = sum(&db, vec![lit_str(&db, "draft"), lit_str(&db, "published")]);
    let draft = lit_str(&db, "draft");
    assert!(status.accepts(&db, &draft));
  }

  #[test]
  fn literal_sum_rejects_non_matching_literal() {
    let db = db();
    let status = sum(&db, vec![lit_str(&db, "draft"), lit_str(&db, "published")]);
    let archived = lit_str(&db, "archived");
    assert!(!status.accepts(&db, &archived));
  }

  // String accepts sum of string literals
  #[test]
  fn string_accepts_sum_of_string_literals() {
    let db = db();
    let string: TdTypeEnum = get_str_type(&db).into();
    let status = sum(&db, vec![lit_str(&db, "draft"), lit_str(&db, "published")]);
    assert!(string.accepts(&db, &status));
  }

  // String rejects sum with non-string member
  #[test]
  fn string_rejects_mixed_sum() {
    let db = db();
    let string: TdTypeEnum = get_str_type(&db).into();
    let mixed = sum(&db, vec![lit_str(&db, "draft"), get_num_type(&db).into()]);
    assert!(!string.accepts(&db, &mixed));
  }

  // List type tests

  #[test]
  fn list_accepts_same_elem_type() {
    let db = db();
    let list_str: TdTypeEnum =
      TdListType::new(&db, Some(LazyType::eager(get_str_type(&db).into()))).into();
    let list_str2: TdTypeEnum =
      TdListType::new(&db, Some(LazyType::eager(get_str_type(&db).into()))).into();
    assert!(list_str.accepts(&db, &list_str2));
  }

  #[test]
  fn list_rejects_different_elem_type() {
    let db = db();
    let list_str: TdTypeEnum =
      TdListType::new(&db, Some(LazyType::eager(get_str_type(&db).into()))).into();
    let list_num: TdTypeEnum =
      TdListType::new(&db, Some(LazyType::eager(get_num_type(&db).into()))).into();
    assert!(!list_str.accepts(&db, &list_num));
  }

  #[test]
  fn untyped_list_accepts_any_list() {
    let db = db();
    let untyped: TdTypeEnum = get_list_type(&db).into();
    let list_str: TdTypeEnum =
      TdListType::new(&db, Some(LazyType::eager(get_str_type(&db).into()))).into();
    assert!(untyped.accepts(&db, &list_str));
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
    assert!(dict_str.accepts(&db, &dict_str2));
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
    assert!(!dict_str.accepts(&db, &dict_num));
  }

  // String accepts date/time subtypes

  #[test]
  fn string_accepts_date() {
    let db = db();
    let string: TdTypeEnum = get_str_type(&db).into();
    let date: TdTypeEnum = get_date_type(&db).into();
    assert!(string.accepts(&db, &date));
  }

  #[test]
  fn string_accepts_datetime() {
    let db = db();
    let string: TdTypeEnum = get_str_type(&db).into();
    let datetime: TdTypeEnum = get_datetime_type(&db).into();
    assert!(string.accepts(&db, &datetime));
  }

  #[test]
  fn string_accepts_time() {
    let db = db();
    let string: TdTypeEnum = get_str_type(&db).into();
    let time: TdTypeEnum = get_time_type(&db).into();
    assert!(string.accepts(&db, &time));
  }

  #[test]
  fn date_rejects_string() {
    let db = db();
    let date: TdTypeEnum = get_date_type(&db).into();
    let string: TdTypeEnum = get_str_type(&db).into();
    assert!(!date.accepts(&db, &string));
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
    assert!(expected.accepts(&db, &actual));
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
    assert!(!expected.accepts(&db, &actual));
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
    assert!(expected.accepts(&db, &actual));
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
    assert!(!expected.accepts(&db, &actual));
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
    assert!(expected.accepts(&db, &actual));
  }

  // Product accepts structural

  #[test]
  fn product_accepts_matching_structural() {
    let db = db();
    let product: TdTypeEnum = TdProductType::new(
      &db,
      Some("Test".to_string()),
      get_type_type(&db).into(),
      None,
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
    assert!(product.accepts(&db, &structural));
  }

  #[test]
  fn product_rejects_structural_with_wrong_field() {
    let db = db();
    let product: TdTypeEnum = TdProductType::new(
      &db,
      Some("Test".to_string()),
      get_type_type(&db).into(),
      None,
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
    assert!(!product.accepts(&db, &structural));
  }

  #[test]
  fn product_rejects_structural_with_missing_required_field() {
    let db = db();
    let product: TdTypeEnum = TdProductType::new(
      &db,
      Some("Test".to_string()),
      get_type_type(&db).into(),
      None,
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
    assert!(!product.accepts(&db, &structural));
  }

  #[test]
  fn null_rejects_non_null() {
    let db = db();
    let null: TdTypeEnum = get_null_type(&db).into();
    let string: TdTypeEnum = get_str_type(&db).into();
    assert!(!null.accepts(&db, &string));
  }

  #[test]
  fn list_rejects_non_list() {
    let db = db();
    let list_str: TdTypeEnum =
      TdListType::new(&db, Some(LazyType::eager(get_str_type(&db).into()))).into();
    let string: TdTypeEnum = get_str_type(&db).into();
    assert!(!list_str.accepts(&db, &string));
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
    assert!(!dict.accepts(&db, &string));
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
    assert!(func_type.accepts(&db, &func2_type));
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
    assert!(expected_type.accepts(&db, &actual_type));
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
    assert!(!expected_type.accepts(&db, &actual_type));
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
    assert!(expected_type.accepts(&db, &actual_type));
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
    assert!(!expected_type.accepts(&db, &actual_type));
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
    assert!(!expected_type.accepts(&db, &actual_type));
  }
}
