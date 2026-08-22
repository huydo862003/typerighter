//! Derived queries for constructing builtin type singletons

use typedown_macros::query_derived;

use crate::db::TypedownDatabase;
use crate::db::derived::schema_property::get_schema_property_type;
use std::collections::{HashMap, HashSet};

use crate::db::types::{
  BuiltinSchemaKind, FuncSignature, LazyType, LiteralValue, Symbol, SymbolKind, TdBlobType,
  TdBoolObj, TdBoolType, TdDateTimeType, TdDateType, TdDictType, TdFuncType, TdListType,
  TdLiteralType, TdMathType, TdNeverType, TdNullObj, TdNullType, TdNumType, TdObjectType,
  TdProductType, TdStaticType, TdStrType, TdSumType, TdTimeType, TdTypeEnum, TdTypeType,
  make_property_descriptors,
};
use typedown_incremental::{QueryDatabase, StableCompare};

#[query_derived]
pub fn get_type_type(db: &TypedownDatabase) -> TdTypeType {
  TdTypeType::new(db)
}

#[query_derived]
pub fn get_object_type(db: &TypedownDatabase) -> TdObjectType {
  TdObjectType::new(db)
}

#[query_derived]
pub fn get_bool_type(db: &TypedownDatabase) -> TdBoolType {
  TdBoolType::new(db)
}

#[query_derived]
pub fn get_str_type(db: &TypedownDatabase) -> TdStrType {
  TdStrType::new(db)
}

#[query_derived]
pub fn get_num_type(db: &TypedownDatabase) -> TdNumType {
  TdNumType::new(db)
}

#[query_derived]
pub fn get_list_type(db: &TypedownDatabase) -> TdListType {
  TdListType::new(db, None)
}

#[query_derived]
pub fn get_dict_type(db: &TypedownDatabase) -> TdDictType {
  TdDictType::new(db, None, None)
}

#[query_derived]
pub fn get_math_type(db: &TypedownDatabase) -> TdMathType {
  TdMathType::new(db)
}

#[query_derived]
pub fn get_datetime_type(db: &TypedownDatabase) -> TdDateTimeType {
  TdDateTimeType::new(db)
}

#[query_derived]
pub fn get_date_type(db: &TypedownDatabase) -> TdDateType {
  TdDateType::new(db)
}

#[query_derived]
pub fn get_time_type(db: &TypedownDatabase) -> TdTimeType {
  TdTimeType::new(db)
}

#[query_derived]
pub fn get_true(db: &TypedownDatabase) -> TdBoolObj {
  TdBoolObj::new(db, true)
}

#[query_derived]
pub fn get_false(db: &TypedownDatabase) -> TdBoolObj {
  TdBoolObj::new(db, false)
}

// Schema is a metatype: its instances are user-defined types (product types)
// Has a single field `properties` of type dict[string, SchemaProperty]
#[query_derived]
pub fn get_schema_type(db: &TypedownDatabase) -> TdProductType {
  let properties_type = get_dict_type(db)
    .instantiate(
      db,
      vec![
        LazyType::eager(get_str_type(db).into()),
        LazyType::eager(get_schema_property_type(db).into()),
      ],
    )
    .typ(db);

  let fields = HashMap::from([("properties".to_string(), LazyType::eager(properties_type))]);

  TdProductType::new(
    db,
    Some("schema".to_string()),
    get_type_type(db).into(),
    make_property_descriptors(db, fields),
    HashMap::new(),
  )
}

// A schema with no declared fields, used for typeless resources
#[query_derived]
pub fn get_schemaless_type(db: &TypedownDatabase) -> TdProductType {
  TdProductType::new(
    db,
    None,
    get_schema_type(db).into(),
    HashMap::new(),
    HashMap::new(),
  )
}

pub fn get_type_type_symbol(db: &TypedownDatabase) -> Symbol {
  Symbol::new(
    db,
    SymbolKind::BuiltinSchema(BuiltinSchemaKind::TypeType),
    "type".to_string(),
    "@builtin::type".to_string(),
  )
}

pub fn get_object_symbol(db: &TypedownDatabase) -> Symbol {
  Symbol::new(
    db,
    SymbolKind::BuiltinSchema(BuiltinSchemaKind::TypeType),
    "Object".to_string(),
    "@builtin::Object".to_string(),
  )
}

pub fn get_schema_symbol(db: &TypedownDatabase) -> Symbol {
  Symbol::new(
    db,
    SymbolKind::BuiltinSchema(BuiltinSchemaKind::Schema),
    "schema".to_string(),
    "@builtin::schema".to_string(),
  )
}

pub fn get_str_symbol(db: &TypedownDatabase) -> Symbol {
  Symbol::new(
    db,
    SymbolKind::BuiltinSchema(BuiltinSchemaKind::Str),
    "string".to_string(),
    "@builtin::string".to_string(),
  )
}

pub fn get_num_symbol(db: &TypedownDatabase) -> Symbol {
  Symbol::new(
    db,
    SymbolKind::BuiltinSchema(BuiltinSchemaKind::Num),
    "number".to_string(),
    "@builtin::number".to_string(),
  )
}

pub fn get_bool_symbol(db: &TypedownDatabase) -> Symbol {
  Symbol::new(
    db,
    SymbolKind::BuiltinSchema(BuiltinSchemaKind::Bool),
    "boolean".to_string(),
    "@builtin::boolean".to_string(),
  )
}

pub fn get_date_symbol(db: &TypedownDatabase) -> Symbol {
  Symbol::new(
    db,
    SymbolKind::BuiltinSchema(BuiltinSchemaKind::Date),
    "date".to_string(),
    "@builtin::date".to_string(),
  )
}

pub fn get_datetime_symbol(db: &TypedownDatabase) -> Symbol {
  Symbol::new(
    db,
    SymbolKind::BuiltinSchema(BuiltinSchemaKind::DateTime),
    "datetime".to_string(),
    "@builtin::datetime".to_string(),
  )
}

pub fn get_time_symbol(db: &TypedownDatabase) -> Symbol {
  Symbol::new(
    db,
    SymbolKind::BuiltinSchema(BuiltinSchemaKind::Time),
    "time".to_string(),
    "@builtin::time".to_string(),
  )
}

pub fn get_math_symbol(db: &TypedownDatabase) -> Symbol {
  Symbol::new(
    db,
    SymbolKind::BuiltinSchema(BuiltinSchemaKind::Math),
    "math".to_string(),
    "@builtin::math".to_string(),
  )
}

pub fn get_list_symbol(db: &TypedownDatabase) -> Symbol {
  Symbol::new(
    db,
    SymbolKind::BuiltinSchema(BuiltinSchemaKind::List),
    "list".to_string(),
    "@builtin::list".to_string(),
  )
}

pub fn get_dict_symbol(db: &TypedownDatabase) -> Symbol {
  Symbol::new(
    db,
    SymbolKind::BuiltinSchema(BuiltinSchemaKind::Dict),
    "dict".to_string(),
    "@builtin::dict".to_string(),
  )
}

#[query_derived]
pub fn get_blob_type(db: &TypedownDatabase) -> TdBlobType {
  TdBlobType::new(db)
}

#[query_derived]
pub fn get_null_type(db: &TypedownDatabase) -> TdNullType {
  TdNullType::new(db)
}

#[query_derived]
pub fn get_never_type(db: &TypedownDatabase) -> TdNeverType {
  TdNeverType::new(db)
}

#[query_derived]
pub fn get_literal_type(db: &TypedownDatabase, value: LiteralValue) -> TdLiteralType {
  TdLiteralType::new(db, value)
}

#[query_derived]
pub fn get_null_obj(db: &TypedownDatabase) -> TdNullObj {
  TdNullObj::new(db)
}

#[query_derived]
pub fn get_func_type(db: &TypedownDatabase, signature: FuncSignature) -> TdFuncType {
  TdFuncType::new(db, signature)
}

#[query_derived]
pub fn get_sum_type(db: &TypedownDatabase, members: Vec<LazyType>) -> TdSumType {
  fn flatten_sum_members(db: &TypedownDatabase, members: &[LazyType]) -> Vec<LazyType> {
    fn recurse(
      db: &TypedownDatabase,
      members: &[LazyType],
      visited: &mut HashSet<TdSumType>,
      out: &mut Vec<LazyType>,
    ) {
      for member in members {
        if let Some(TdTypeEnum::TdSumType(sum)) = member.as_eager() {
          if visited.insert(*sum) {
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
