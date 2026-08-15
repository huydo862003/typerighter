//! Shared type compatibility utilities for typechecking

use crate::db::TypedownDatabase;
use crate::db::derived::get_builtin_types::{
  get_bool_type, get_dict_type, get_list_type, get_num_type, get_str_type,
};
use crate::db::types::{
  HirValue, HirValueKind, LazyType, LiteralValue, MemberType, TdTypeEnum, TdTypeLike,
  TypeMemberResult,
};

/// Extract a TdTypeEnum from a TypeMemberResult
pub fn lift_type_member_result(
  db: &TypedownDatabase,
  result: &TypeMemberResult,
) -> Option<TdTypeEnum> {
  let member = result.member(db)?;
  lift_member_type(db, &member.typ(db))
}

/// Lift a MemberType to a TdTypeEnum
/// NOTE: This causes loss of specificity
pub fn lift_member_type(db: &TypedownDatabase, member_type: &MemberType) -> Option<TdTypeEnum> {
  match member_type {
    MemberType::Simple(lazy) => lazy.resolve(db),
    MemberType::Literal(lit) => Some(literal_base_type(db, lit)),
    MemberType::ListOfSum(_) => Some(get_list_type(db).into()),
    MemberType::DictOfSum(_) => Some(get_dict_type(db).into()),
    MemberType::Sum(arms) => {
      // Return the first arm's type as a rough approximation
      let first = arms.first()?;
      lift_member_type(db, &first.typ(db))
    }
    MemberType::Structural(_) => None,
    MemberType::Never => None,
  }
}

/// Get the base TdTypeEnum for a literal value
pub fn literal_base_type(db: &TypedownDatabase, lit: &LiteralValue) -> TdTypeEnum {
  match lit {
    LiteralValue::Str(_) => get_str_type(db).into(),
    LiteralValue::Num(_) => get_num_type(db).into(),
    LiteralValue::Bool(_) => get_bool_type(db).into(),
  }
}

/// Check if actual MemberType can be assigned to expected MemberType
pub fn member_types_compatible(
  db: &TypedownDatabase,
  expected: &MemberType,
  actual: &MemberType,
) -> bool {
  match (expected, actual) {
    (MemberType::Simple(expected_lazy), MemberType::Simple(actual_lazy)) => {
      let exp_type = match expected_lazy.resolve(db) {
        Some(t) => t,
        None => return false,
      };
      let actual_type = match actual_lazy.resolve(db) {
        Some(t) => t,
        None => return false,
      };
      exp_type.accepts(db, &actual_type)
    }
    (MemberType::Sum(exp_arms), MemberType::Sum(act_arms))
    | (MemberType::ListOfSum(exp_arms), MemberType::ListOfSum(act_arms))
    | (MemberType::DictOfSum(exp_arms), MemberType::DictOfSum(act_arms)) => {
      // Every arm in actual must be compatible with some arm in expected
      act_arms.iter().all(|act_arm| {
        let act_typ = act_arm.typ(db);
        exp_arms
          .iter()
          .any(|exp_arm| member_types_compatible(db, &exp_arm.typ(db), &act_typ))
      })
    }

    // Literal is a subtype of its base simple type
    (MemberType::Simple(expected_lazy), MemberType::Literal(act_val)) => {
      let exp_type = match expected_lazy.resolve(db) {
        Some(t) => t,
        None => return false,
      };
      let base = literal_base_type(db, act_val);
      exp_type.accepts(db, &base)
    }

    // An actual is assignable to a Sum expected if some arm matches
    (MemberType::Sum(exp_arms), _) => exp_arms
      .iter()
      .any(|exp_arm| member_types_compatible(db, &exp_arm.typ(db), actual)),

    // ListOfSum is compatible with a Simple only if the simple is a list type whose elem type is compatible with some arm
    (MemberType::ListOfSum(exp_arms), MemberType::Simple(actual_lazy)) => {
      let actual_type = match actual_lazy.resolve(db) {
        Some(t) => t,
        None => return false,
      };
      match actual_type.as_td_list_type() {
        Some(list) => match list.elem(db) {
          Some(elem) => {
            let elem_member = MemberType::Simple(elem);
            exp_arms
              .iter()
              .any(|exp_arm| member_types_compatible(db, &exp_arm.typ(db), &elem_member))
          }
          None => true,
        },
        None => false,
      }
    }

    // DictOfSum is compatible with a Simple if it's a dict type whose value matches some arm, or a product type whose every field's type matches some arm
    (MemberType::DictOfSum(exp_arms), MemberType::Simple(actual_lazy)) => {
      let actual_type = match actual_lazy.resolve(db) {
        Some(t) => t,
        None => return false,
      };
      if let Some(dict) = actual_type.as_td_dict_type() {
        return match dict.value(db).and_then(|l| l.resolve(db)) {
          Some(value) => {
            let value_member = MemberType::Simple(LazyType::eager(value));
            exp_arms
              .iter()
              .any(|exp_arm| member_types_compatible(db, &exp_arm.typ(db), &value_member))
          }
          None => true,
        };
      }
      if let Some(product) = actual_type.as_td_product_type() {
        return product.fields(db).values().all(|field_member| {
          let field_typ = field_member.typ(db);
          exp_arms
            .iter()
            .any(|exp_arm| member_types_compatible(db, &exp_arm.typ(db), &field_typ))
        });
      }
      false
    }

    // Sum assignable to simple if every arm is compatible with the simple
    (MemberType::Simple(_), MemberType::Sum(act_arms)) => act_arms
      .iter()
      .all(|act_arm| member_types_compatible(db, expected, &act_arm.typ(db))),

    // ListOfSum assignable to simple if the simple is a list and every arm is compatible with its elem
    (MemberType::Simple(expected_lazy), MemberType::ListOfSum(act_arms)) => {
      let exp_type = match expected_lazy.resolve(db) {
        Some(t) => t,
        None => return false,
      };
      match exp_type.as_td_list_type() {
        Some(list) => match list.elem(db) {
          Some(elem) => {
            let elem_member = MemberType::Simple(elem);
            act_arms
              .iter()
              .all(|act_arm| member_types_compatible(db, &elem_member, &act_arm.typ(db)))
          }
          None => true,
        },
        None => false,
      }
    }
    // DictOfSum assignable to simple if the simple is a dict/product and every arm is compatible
    (MemberType::Simple(expected_lazy), MemberType::DictOfSum(act_arms)) => {
      let exp_type = match expected_lazy.resolve(db) {
        Some(t) => t,
        None => return false,
      };
      if let Some(dict) = exp_type.as_td_dict_type() {
        return match dict.value(db).and_then(|l| l.resolve(db)) {
          Some(value) => {
            let value_member = MemberType::Simple(LazyType::eager(value));
            act_arms
              .iter()
              .all(|act_arm| member_types_compatible(db, &value_member, &act_arm.typ(db)))
          }
          None => true,
        };
      }

      if let Some(product) = exp_type.as_td_product_type() {
        // Every arm of the DictOfSum must be compatible with every field of the product
        return product.fields(db).values().all(|field_member| {
          let field_typ = field_member.typ(db);
          act_arms
            .iter()
            .all(|act_arm| member_types_compatible(db, &field_typ, &act_arm.typ(db)))
        });
      }
      false
    }

    // Structural vs Structural: every required expected field must exist and match in actual
    (MemberType::Structural(exp_fields), MemberType::Structural(act_fields)) => {
      exp_fields.iter().all(|(name, exp_member)| {
        if exp_member.typ(db).is_nullable(db) {
          return true;
        }
        match act_fields.get(name) {
          Some(actual_member) => {
            member_types_compatible(db, &exp_member.typ(db), &actual_member.typ(db))
          }
          None => false,
        }
      })
    }

    // Type expected accepts a structural actual if all required expected fields match
    (MemberType::Simple(expected_lazy), MemberType::Structural(act_fields)) => {
      let exp_type = match expected_lazy.resolve(db) {
        Some(t) => t,
        None => return false,
      };
      // Product type accepts structural if all required fields are present and
      // all present fields have compatible types
      if let Some(product) = exp_type.as_td_product_type() {
        return product.fields(db).iter().all(|(name, exp_member)| {
          let is_optional = exp_member.typ(db).is_nullable(db);
          match act_fields.get(name) {
            Some(actual_member) => {
              member_types_compatible(db, &exp_member.typ(db), &actual_member.typ(db))
            }
            // Missing required field
            None => is_optional,
          }
        });
      }
      // Dict type accepts structural if every field value matches the dict's value type
      if let Some(dict) = exp_type.as_td_dict_type() {
        return match dict.value(db).and_then(|l| l.resolve(db)) {
          Some(value_type) => {
            let value_member = MemberType::Simple(LazyType::eager(value_type));
            act_fields.values().all(|actual_member| {
              member_types_compatible(db, &value_member, &actual_member.typ(db))
            })
          }
          None => true,
        };
      }
      false
    }

    // Structural expected accepts a type if all required fields match
    (MemberType::Structural(exp_fields), MemberType::Simple(actual_lazy)) => {
      let actual_type = match actual_lazy.resolve(db) {
        Some(t) => t,
        None => return false,
      };
      exp_fields.iter().all(|(name, exp_member)| {
        if exp_member.typ(db).is_nullable(db) {
          return true;
        }
        match actual_type.get_owned_field_type_member(db, name) {
          Some(actual_member) => {
            member_types_compatible(db, &exp_member.typ(db), &actual_member.typ(db))
          }
          None => false,
        }
      })
    }

    // DictOfSum expected accepts a structural actual if every field value matches some arm
    (MemberType::DictOfSum(exp_arms), MemberType::Structural(act_fields)) => {
      act_fields.values().all(|field_member| {
        let field_typ = field_member.typ(db);
        exp_arms
          .iter()
          .any(|exp_arm| member_types_compatible(db, &exp_arm.typ(db), &field_typ))
      })
    }

    // A string is not assignable to "foo", so (Literal, Simple) correctly falls through to false
    (MemberType::Literal(exp_val), MemberType::Literal(act_val)) => exp_val == act_val,
    (MemberType::Never, _) | (_, MemberType::Never) => false,
    _ => false,
  }
}

/// Check if a value's actual type matches the expected member type
pub fn value_matches_member_type(
  db: &TypedownDatabase,
  expected: &MemberType,
  actual: &TdTypeEnum,
  value_hir: HirValue,
) -> bool {
  match expected {
    MemberType::Simple(expected_lazy) => {
      let exp_type = match expected_lazy.resolve(db) {
        Some(t) => t,
        None => return false,
      };
      exp_type.accepts(db, actual)
    }
    MemberType::Sum(members) => members
      .iter()
      .any(|member| value_matches_member_type(db, &member.typ(db), actual, value_hir)),
    MemberType::ListOfSum(members) => {
      // Actual must be a list type, and its elem must match some arm
      match actual.as_td_list_type() {
        Some(list) => match list.elem(db).and_then(|e| e.resolve(db)) {
          Some(elem) => members
            .iter()
            .any(|member| value_matches_member_type(db, &member.typ(db), &elem, value_hir)),
          None => true,
        },
        None => false,
      }
    }
    MemberType::DictOfSum(members) => {
      // Actual must be a dict or product type, and its values must match some arm
      if let Some(dict) = actual.as_td_dict_type() {
        return match dict.value(db).and_then(|l| l.resolve(db)) {
          Some(value) => members
            .iter()
            .any(|member| value_matches_member_type(db, &member.typ(db), &value, value_hir)),
          None => true,
        };
      }
      if let Some(product) = actual.as_td_product_type() {
        return product.fields(db).values().all(|field_member| {
          if let MemberType::Simple(lazy) = field_member.typ(db)
            && let Some(field_type) = lazy.resolve(db)
          {
            members
              .iter()
              .any(|member| value_matches_member_type(db, &member.typ(db), &field_type, value_hir))
          } else {
            false
          }
        });
      }
      false
    }
    MemberType::Literal(literal) => match (literal, value_hir.kind(db)) {
      (LiteralValue::Str(expected_val), HirValueKind::Str(actual_val)) => {
        *expected_val == actual_val
      }
      (LiteralValue::Num(expected_val), HirValueKind::Num(actual_val)) => {
        *expected_val == actual_val
      }
      (LiteralValue::Bool(expected_val), HirValueKind::Bool(actual_val)) => {
        *expected_val == actual_val
      }
      _ => false,
    },
    MemberType::Structural(_) => false,
    MemberType::Never => false,
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::db::derived::get_builtin_types::{get_bool_type, get_num_type, get_str_type};
  use crate::db::types::{TypeMember, TypeMemberDescriptors};
  use crate::db::{QueryStorage, TypedownDatabase};

  fn db() -> TypedownDatabase {
    TypedownDatabase {
      storage: QueryStorage::default(),
    }
  }

  fn simple(_db: &TypedownDatabase, typ: TdTypeEnum) -> MemberType {
    MemberType::Simple(LazyType::eager(typ))
  }

  fn literal_str(val: &str) -> MemberType {
    MemberType::Literal(LiteralValue::Str(val.to_string()))
  }

  fn literal_num(val: &str) -> MemberType {
    MemberType::Literal(LiteralValue::Num(val.to_string()))
  }

  fn arm(db: &TypedownDatabase, member_type: MemberType) -> TypeMember {
    TypeMember::new(db, member_type, TypeMemberDescriptors::empty())
  }

  // Simple vs Simple
  #[test]
  fn compatible_simple_same_type() {
    let db = db();
    let string = simple(&db, get_str_type(&db).into());
    assert!(member_types_compatible(&db, &string, &string));
  }

  #[test]
  fn incompatible_simple_different_type() {
    let db = db();
    let string = simple(&db, get_str_type(&db).into());
    let number = simple(&db, get_num_type(&db).into());
    assert!(!member_types_compatible(&db, &string, &number));
  }

  // Literal vs Simple
  #[test]
  fn literal_compatible_with_base_simple() {
    let db = db();
    let string = simple(&db, get_str_type(&db).into());
    let lit = literal_str("hello");
    assert!(member_types_compatible(&db, &string, &lit));
  }

  #[test]
  fn literal_incompatible_with_wrong_simple() {
    let db = db();
    let number = simple(&db, get_num_type(&db).into());
    let lit = literal_str("hello");
    assert!(!member_types_compatible(&db, &number, &lit));
  }

  // Literal vs Literal
  #[test]
  fn literal_compatible_same_value() {
    let db = db();
    let lit1 = literal_str("draft");
    let lit2 = literal_str("draft");
    assert!(member_types_compatible(&db, &lit1, &lit2));
  }

  #[test]
  fn literal_incompatible_different_value() {
    let db = db();
    let lit1 = literal_str("draft");
    let lit2 = literal_str("published");
    assert!(!member_types_compatible(&db, &lit1, &lit2));
  }

  // Sum compatibility
  #[test]
  fn literal_compatible_with_sum_containing_base() {
    let db = db();
    let sum = MemberType::Sum(vec![
      arm(&db, simple(&db, get_str_type(&db).into())),
      arm(&db, simple(&db, get_num_type(&db).into())),
    ]);
    let lit = literal_str("hello");
    assert!(member_types_compatible(&db, &sum, &lit));
  }

  #[test]
  fn simple_incompatible_with_sum_no_match() {
    let db = db();
    let sum = MemberType::Sum(vec![
      arm(&db, simple(&db, get_str_type(&db).into())),
      arm(&db, simple(&db, get_num_type(&db).into())),
    ]);
    let boolean = simple(&db, get_bool_type(&db).into());
    assert!(!member_types_compatible(&db, &sum, &boolean));
  }

  // ListOfSum compatibility
  #[test]
  fn list_of_sum_compatible_with_matching_arms() {
    let db = db();
    let expected = MemberType::ListOfSum(vec![
      arm(&db, simple(&db, get_str_type(&db).into())),
      arm(&db, simple(&db, get_num_type(&db).into())),
    ]);
    // Actual has literal arms that match base types
    let actual = MemberType::ListOfSum(vec![
      arm(&db, literal_str("hello")),
      arm(&db, literal_num("42")),
    ]);
    assert!(member_types_compatible(&db, &expected, &actual));
  }

  #[test]
  fn list_of_sum_incompatible_with_wrong_arm() {
    let db = db();
    let expected = MemberType::ListOfSum(vec![arm(&db, simple(&db, get_str_type(&db).into()))]);
    let actual = MemberType::ListOfSum(vec![arm(&db, simple(&db, get_num_type(&db).into()))]);
    assert!(!member_types_compatible(&db, &expected, &actual));
  }

  // Never
  #[test]
  fn never_incompatible_with_anything() {
    let db = db();
    let string = simple(&db, get_str_type(&db).into());
    assert!(!member_types_compatible(&db, &MemberType::Never, &string));
    assert!(!member_types_compatible(&db, &string, &MemberType::Never));
  }

  // lift_member_type
  #[test]
  fn lift_simple_returns_type() {
    let db = db();
    let typ: TdTypeEnum = get_str_type(&db).into();
    let member = simple(&db, typ.clone());
    assert!(lift_member_type(&db, &member) == Some(typ));
  }

  #[test]
  fn lift_literal_returns_base_type() {
    let db = db();
    let member = literal_str("hello");
    let expected: TdTypeEnum = get_str_type(&db).into();
    assert!(lift_member_type(&db, &member) == Some(expected));
  }

  #[test]
  fn lift_never_returns_none() {
    let db = db();
    assert!(lift_member_type(&db, &MemberType::Never).is_none());
  }

  // DictOfSum compatibility
  #[test]
  fn dict_of_sum_compatible_with_matching_arms() {
    let db = db();
    let expected = MemberType::DictOfSum(vec![
      arm(&db, simple(&db, get_str_type(&db).into())),
      arm(&db, simple(&db, get_num_type(&db).into())),
    ]);
    let actual = MemberType::DictOfSum(vec![
      arm(&db, literal_str("hello")),
      arm(&db, literal_num("42")),
    ]);
    assert!(member_types_compatible(&db, &expected, &actual));
  }

  #[test]
  fn dict_of_sum_incompatible_with_wrong_arm() {
    let db = db();
    let expected = MemberType::DictOfSum(vec![arm(&db, simple(&db, get_str_type(&db).into()))]);
    let actual = MemberType::DictOfSum(vec![arm(&db, simple(&db, get_num_type(&db).into()))]);
    assert!(!member_types_compatible(&db, &expected, &actual));
  }

  // Cross-variant: Sum vs Literal
  #[test]
  fn sum_compatible_with_literal_matching_arm() {
    let db = db();
    let sum = MemberType::Sum(vec![
      arm(&db, literal_str("draft")),
      arm(&db, literal_str("published")),
    ]);
    let lit = literal_str("draft");
    assert!(member_types_compatible(&db, &sum, &lit));
  }

  #[test]
  fn sum_incompatible_with_literal_no_match() {
    let db = db();
    let sum = MemberType::Sum(vec![
      arm(&db, literal_str("draft")),
      arm(&db, literal_str("published")),
    ]);
    let lit = literal_str("archived");
    assert!(!member_types_compatible(&db, &sum, &lit));
  }

  // Cross-variant: Simple vs Sum
  #[test]
  fn simple_compatible_with_sum_all_arms_match() {
    let db = db();
    let string = simple(&db, get_str_type(&db).into());
    let sum = MemberType::Sum(vec![arm(&db, literal_str("a")), arm(&db, literal_str("b"))]);
    // Sum assignable to Simple if every arm is compatible
    assert!(member_types_compatible(&db, &string, &sum));
  }

  #[test]
  fn simple_incompatible_with_sum_mixed_arms() {
    let db = db();
    let string = simple(&db, get_str_type(&db).into());
    let sum = MemberType::Sum(vec![
      arm(&db, literal_str("a")),
      arm(&db, simple(&db, get_num_type(&db).into())),
    ]);
    // Number arm not compatible with string
    assert!(!member_types_compatible(&db, &string, &sum));
  }

  // Literal num
  #[test]
  fn literal_num_compatible_with_number() {
    let db = db();
    let number = simple(&db, get_num_type(&db).into());
    let lit = literal_num("42");
    assert!(member_types_compatible(&db, &number, &lit));
  }

  #[test]
  fn literal_num_incompatible_with_string() {
    let db = db();
    let string = simple(&db, get_str_type(&db).into());
    let lit = literal_num("42");
    assert!(!member_types_compatible(&db, &string, &lit));
  }
}
