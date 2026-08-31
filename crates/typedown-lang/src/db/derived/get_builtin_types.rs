//! Derived queries for constructing builtin type singletons

use typedown_macros::query_derived;

use crate::db::TypedownDatabase;
use std::collections::HashSet;

use crate::db::types::{
  BuiltinSchemaKind, FuncSignature, LazyType, LiteralValue, Symbol, SymbolKind, TdBlobType,
  TdBoolObj, TdBoolType, TdDateTimeType, TdDateType, TdDictType, TdFuncType, TdIconType,
  TdListType, TdLiteralType, TdMathType, TdNeverType, TdNullObj, TdNullType, TdNumType,
  TdObjectType, TdSchemaMetaType, TdStrType, TdSumType, TdTimeType, TdTypeEnum, TdTypeType,
};
use typedown_incremental::{QueryDatabase, StableCompare};

#[query_derived]
pub fn get_type_type<'db>(db: &'db TypedownDatabase) -> TdTypeType<'db> {
  TdTypeType::new(db)
}

#[query_derived]
pub fn get_object_type<'db>(db: &'db TypedownDatabase) -> TdObjectType<'db> {
  TdObjectType::new(db)
}

#[query_derived]
pub fn get_bool_type<'db>(db: &'db TypedownDatabase) -> TdBoolType<'db> {
  TdBoolType::new(db)
}

#[query_derived]
pub fn get_str_type<'db>(db: &'db TypedownDatabase) -> TdStrType<'db> {
  TdStrType::new(db)
}

#[query_derived]
pub fn get_num_type<'db>(db: &'db TypedownDatabase) -> TdNumType<'db> {
  TdNumType::new(db)
}

#[query_derived]
pub fn get_list_type<'db>(db: &'db TypedownDatabase) -> TdListType<'db> {
  TdListType::new(db, None)
}

#[query_derived]
pub fn get_dict_type<'db>(db: &'db TypedownDatabase) -> TdDictType<'db> {
  TdDictType::new(db, None, None)
}

#[query_derived]
pub fn get_math_type<'db>(db: &'db TypedownDatabase) -> TdMathType<'db> {
  TdMathType::new(db)
}

#[query_derived]
pub fn get_datetime_type<'db>(db: &'db TypedownDatabase) -> TdDateTimeType<'db> {
  TdDateTimeType::new(db)
}

#[query_derived]
pub fn get_date_type<'db>(db: &'db TypedownDatabase) -> TdDateType<'db> {
  TdDateType::new(db)
}

#[query_derived]
pub fn get_time_type<'db>(db: &'db TypedownDatabase) -> TdTimeType<'db> {
  TdTimeType::new(db)
}

#[query_derived]
pub fn get_true<'db>(db: &'db TypedownDatabase) -> TdBoolObj<'db> {
  TdBoolObj::new(db, true)
}

#[query_derived]
pub fn get_false<'db>(db: &'db TypedownDatabase) -> TdBoolObj<'db> {
  TdBoolObj::new(db, false)
}

// Schema metatype: the type of all schema types
#[query_derived]
pub fn get_schema_meta_type<'db>(db: &'db TypedownDatabase) -> TdSchemaMetaType<'db> {
  TdSchemaMetaType::new(db)
}

pub fn get_type_type_symbol<'db>(db: &'db TypedownDatabase) -> Symbol<'db> {
  Symbol::new(
    db,
    SymbolKind::BuiltinSchema(BuiltinSchemaKind::TypeType),
    "type".to_string(),
    "@builtin::type".to_string(),
  )
}

pub fn get_object_symbol<'db>(db: &'db TypedownDatabase) -> Symbol<'db> {
  Symbol::new(
    db,
    SymbolKind::BuiltinSchema(BuiltinSchemaKind::TypeType),
    "Object".to_string(),
    "@builtin::Object".to_string(),
  )
}

pub fn get_schema_symbol<'db>(db: &'db TypedownDatabase) -> Symbol<'db> {
  Symbol::new(
    db,
    SymbolKind::BuiltinSchema(BuiltinSchemaKind::Schema),
    "schema".to_string(),
    "@builtin::schema".to_string(),
  )
}

pub fn get_str_symbol<'db>(db: &'db TypedownDatabase) -> Symbol<'db> {
  Symbol::new(
    db,
    SymbolKind::BuiltinSchema(BuiltinSchemaKind::Str),
    "string".to_string(),
    "@builtin::string".to_string(),
  )
}

pub fn get_num_symbol<'db>(db: &'db TypedownDatabase) -> Symbol<'db> {
  Symbol::new(
    db,
    SymbolKind::BuiltinSchema(BuiltinSchemaKind::Num),
    "number".to_string(),
    "@builtin::number".to_string(),
  )
}

pub fn get_bool_symbol<'db>(db: &'db TypedownDatabase) -> Symbol<'db> {
  Symbol::new(
    db,
    SymbolKind::BuiltinSchema(BuiltinSchemaKind::Bool),
    "boolean".to_string(),
    "@builtin::boolean".to_string(),
  )
}

pub fn get_date_symbol<'db>(db: &'db TypedownDatabase) -> Symbol<'db> {
  Symbol::new(
    db,
    SymbolKind::BuiltinSchema(BuiltinSchemaKind::Date),
    "date".to_string(),
    "@builtin::date".to_string(),
  )
}

pub fn get_datetime_symbol<'db>(db: &'db TypedownDatabase) -> Symbol<'db> {
  Symbol::new(
    db,
    SymbolKind::BuiltinSchema(BuiltinSchemaKind::DateTime),
    "datetime".to_string(),
    "@builtin::datetime".to_string(),
  )
}

pub fn get_time_symbol<'db>(db: &'db TypedownDatabase) -> Symbol<'db> {
  Symbol::new(
    db,
    SymbolKind::BuiltinSchema(BuiltinSchemaKind::Time),
    "time".to_string(),
    "@builtin::time".to_string(),
  )
}

pub fn get_math_symbol<'db>(db: &'db TypedownDatabase) -> Symbol<'db> {
  Symbol::new(
    db,
    SymbolKind::BuiltinSchema(BuiltinSchemaKind::Math),
    "math".to_string(),
    "@builtin::math".to_string(),
  )
}

pub fn get_list_symbol<'db>(db: &'db TypedownDatabase) -> Symbol<'db> {
  Symbol::new(
    db,
    SymbolKind::BuiltinSchema(BuiltinSchemaKind::List),
    "list".to_string(),
    "@builtin::list".to_string(),
  )
}

pub fn get_dict_symbol<'db>(db: &'db TypedownDatabase) -> Symbol<'db> {
  Symbol::new(
    db,
    SymbolKind::BuiltinSchema(BuiltinSchemaKind::Dict),
    "dict".to_string(),
    "@builtin::dict".to_string(),
  )
}

#[query_derived]
pub fn get_icon_type<'db>(db: &'db TypedownDatabase) -> TdIconType<'db> {
  TdIconType::new(db)
}

#[query_derived]
pub fn get_blob_type<'db>(db: &'db TypedownDatabase) -> TdBlobType<'db> {
  TdBlobType::new(db)
}

#[query_derived]
pub fn get_null_type<'db>(db: &'db TypedownDatabase) -> TdNullType<'db> {
  TdNullType::new(db)
}

#[query_derived]
pub fn get_never_type<'db>(db: &'db TypedownDatabase) -> TdNeverType<'db> {
  TdNeverType::new(db)
}

#[query_derived]
pub fn get_literal_type<'db>(db: &'db TypedownDatabase, value: LiteralValue) -> TdLiteralType<'db> {
  TdLiteralType::new(db, value)
}

#[query_derived]
pub fn get_null_obj<'db>(db: &'db TypedownDatabase) -> TdNullObj<'db> {
  TdNullObj::new(db)
}

#[query_derived]
pub fn get_func_type<'db>(
  db: &'db TypedownDatabase,
  signature: FuncSignature<'db>,
) -> TdFuncType<'db> {
  TdFuncType::new(db, signature)
}

#[query_derived]
pub fn get_sum_type<'db>(db: &'db TypedownDatabase, members: Vec<LazyType<'db>>) -> TdSumType<'db> {
  fn flatten_sum_members<'db>(
    db: &'db TypedownDatabase,
    members: &[LazyType<'db>],
  ) -> Vec<LazyType<'db>> {
    fn recurse<'db>(
      db: &'db TypedownDatabase,
      members: &[LazyType<'db>],
      visited: &mut HashSet<TdSumType<'db>>,
      out: &mut Vec<LazyType<'db>>,
    ) {
      for member in members {
        if let Some(TdTypeEnum::TdSumType(sum)) = member.as_eager() {
          if visited.insert(sum) {
            let sum_members: Vec<_> = sum.members(db).into_iter().collect();
            recurse(db, &sum_members, visited, out);
          }
        } else {
          out.push(member.clone());
        }
      }
    }

    let mut out = Vec::new();
    let mut visited = HashSet::new();
    recurse(db, members, &mut visited, &mut out);
    out
  }

  let flat_members = flatten_sum_members(db, &members);
  let mut sorted = flat_members;
  sorted.sort_by(|a, b| a.stable_cmp(db, b));
  sorted.dedup();

  if sorted != members {
    get_sum_type(db, sorted)
  } else {
    let members_set: HashSet<LazyType> = sorted.into_iter().collect();
    TdSumType::new(db, members_set)
  }
}

#[cfg(test)]
mod tests {
  use super::{get_bool_type, get_sum_type};
  use crate::db::typecheck::utils::validate_type_params;
  use crate::db::types::derived::object_system::TdStaticType;
  use crate::db::types::{LazyType, TdTypeEnum, TypeParams, TypeVariable};
  use crate::syntax::diagnostic::Diagnostic;

  use crate::db::{
    QueryStorage, TypedownDatabase,
    derived::get_builtin_types::{get_dict_type, get_list_type, get_num_type, get_str_type},
  };

  fn make_db() -> TypedownDatabase {
    TypedownDatabase {
      storage: QueryStorage::default(),
    }
  }

  #[test]
  fn instantiate_list_with_correct_arity() {
    let db = make_db();
    let list = TdTypeEnum::from(get_list_type(&db));
    let str_type = TdTypeEnum::from(get_str_type(&db));

    let result = list.instantiate(&db, vec![LazyType::eager(str_type.clone())]);

    assert!(
      result.diagnostics(&db).is_empty(),
      "expected no diagnostics"
    );
    let _expected = TdTypeEnum::from(get_str_type(&db));
    let instantiated = result.typ(&db);
    // The result should be a TdListType with elem = str
    assert!(
      instantiated.arity(&db) == 0,
      "instantiated list should have arity 0"
    );
  }

  #[test]
  fn instantiate_record_with_correct_arity() {
    let db = make_db();
    let record = TdTypeEnum::from(get_dict_type(&db));
    let str_type = TdTypeEnum::from(get_str_type(&db));
    let num_type = TdTypeEnum::from(get_num_type(&db));

    let result = record.instantiate(
      &db,
      vec![LazyType::eager(str_type), LazyType::eager(num_type)],
    );

    assert!(
      result.diagnostics(&db).is_empty(),
      "expected no diagnostics"
    );
    assert!(
      result.typ(&db).arity(&db) == 0,
      "instantiated record should have arity 0"
    );
  }

  #[test]
  fn instantiate_list_wrong_arity_produces_diagnostic() {
    let db = make_db();
    let list = TdTypeEnum::from(get_list_type(&db));

    let result = list.instantiate(&db, vec![]);

    let diagnostics = result.diagnostics(&db);
    assert_eq!(diagnostics.len(), 1);
    assert!(
      matches!(
        diagnostics[0],
        Diagnostic::WrongTypeArgCount {
          expected: 1,
          got: 0
        }
      ),
      "expected WrongTypeArgCount diagnostic"
    );
  }

  #[test]
  fn instantiate_record_wrong_arity_produces_diagnostic() {
    let db = make_db();
    let record = TdTypeEnum::from(get_dict_type(&db));
    let str_type = TdTypeEnum::from(get_str_type(&db));

    // Only 1 arg, record needs 2
    let result = record.instantiate(&db, vec![LazyType::eager(str_type)]);

    let diagnostics = result.diagnostics(&db);
    assert_eq!(diagnostics.len(), 1);
    assert!(
      matches!(
        diagnostics[0],
        Diagnostic::WrongTypeArgCount {
          expected: 2,
          got: 1
        }
      ),
      "expected WrongTypeArgCount diagnostic"
    );
  }

  #[test]
  fn instantiate_arity0_type_with_no_args() {
    let db = make_db();
    let str_type = TdTypeEnum::from(get_str_type(&db));
    let expected = str_type.clone();

    let result = str_type.instantiate(&db, vec![]);

    assert!(
      result.diagnostics(&db).is_empty(),
      "expected no diagnostics"
    );
    assert!(
      result.typ(&db) == expected,
      "arity-0 type instantiated with no args should return itself"
    );
  }

  #[test]
  fn instantiate_arity0_type_with_extra_args_produces_diagnostic() {
    let db = make_db();
    let str_type = TdTypeEnum::from(get_str_type(&db));
    let num_type = TdTypeEnum::from(get_num_type(&db));

    let result = str_type.instantiate(&db, vec![LazyType::eager(num_type)]);

    let diagnostics = result.diagnostics(&db);
    assert_eq!(diagnostics.len(), 1);
    assert!(
      matches!(
        diagnostics[0],
        Diagnostic::WrongTypeArgCount {
          expected: 0,
          got: 1
        }
      ),
      "expected WrongTypeArgCount diagnostic"
    );
  }

  #[test]
  fn instantiate_bounded_type_violating_bound_produces_diagnostic() {
    let db = make_db();
    let num_type = TdTypeEnum::from(get_num_type(&db));
    let str_type = TdTypeEnum::from(get_str_type(&db));

    let params = TypeParams::new(
      &db,
      vec![TypeVariable::get(&db, Some(LazyType::eager(num_type)))],
      vec![],
    );
    let diagnostics = validate_type_params(&db, Some(&params), &[LazyType::eager(str_type)]);
    assert_eq!(diagnostics.len(), 1);
    assert!(
      matches!(
        diagnostics[0],
        Diagnostic::TypeArgBoundViolation { index: 0, .. }
      ),
      "expected TypeArgBoundViolation diagnostic"
    );
  }

  #[test]
  fn sum_type_flattening() {
    let db = make_db();
    let str_t = LazyType::eager(get_str_type(&db).into());
    let num_t = LazyType::eager(get_num_type(&db).into());
    let bool_t = LazyType::eager(get_bool_type(&db).into());

    let inner_sum = get_sum_type(&db, vec![num_t.clone(), bool_t.clone()]);
    let outer_sum = get_sum_type(&db, vec![str_t.clone(), LazyType::eager(inner_sum.into())]);

    let members = outer_sum.members(&db);
    assert_eq!(members.len(), 3);
    assert!(members.contains(&str_t));
    assert!(members.contains(&num_t));
    assert!(members.contains(&bool_t));
  }
}
