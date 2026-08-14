//! Derived queries for constructing builtin type singletons

use typedown_macros::query_derived;

use crate::syntax::diagnostic::Diagnostic;

use std::collections::HashMap;

use crate::db::TypedownDatabase;
use crate::db::types::{
  BuiltinSchemaKind, FuncSignature, InstResult, MemberType, Symbol, SymbolKind, TdBlobType,
  TdBoolObj, TdBoolType, TdDateTimeType, TdDateType, TdDictObj, TdDictType, TdFuncType, TdListType,
  TdMathType, TdNumType, TdObjectType, TdProductType, TdSchemaType, TdStrType, TdTimeType,
  TdTypeEnum, TdTypeLike, TdTypeType, TypeMember, TypeMemberDescriptors,
};
use typedown_incremental::QueryDatabase;
use typedown_types::either::Either;

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

// A property descriptor inside a schema's `properties` field
// Has a required `type` field and an optional `optional` field
#[query_derived]
pub fn get_schema_property_type(db: &TypedownDatabase) -> TdProductType {
  let type_type: TdTypeEnum = get_type_type(db).into();
  let str_type: TdTypeEnum = get_str_type(db).into();
  let bool_type: TdTypeEnum = get_bool_type(db).into();
  let num_type: TdTypeEnum = get_num_type(db).into();

  // The base scalar types that the `type` field accepts
  let base_type_members = vec![
    TypeMember::new(
      db,
      MemberType::eager_simple(type_type),
      TypeMemberDescriptors::empty(),
    ),
    TypeMember::new(
      db,
      MemberType::eager_simple(str_type),
      TypeMemberDescriptors::empty(),
    ),
    TypeMember::new(
      db,
      MemberType::eager_simple(bool_type.clone()),
      TypeMemberDescriptors::empty(),
    ),
    TypeMember::new(
      db,
      MemberType::eager_simple(num_type),
      TypeMemberDescriptors::empty(),
    ),
  ];

  // Lazy self-reference to avoid recursive query
  let self_symbol = get_schema_property_symbol(db);
  let self_member = TypeMember::new(
    db,
    MemberType::Simple(Either::Right(self_symbol)),
    TypeMemberDescriptors::empty(),
  );

  let type_field = TypeMember::new(
    db,
    MemberType::Sum(
      [
        base_type_members.clone(),
        vec![
          TypeMember::new(
            db,
            MemberType::ListOfSum([base_type_members.clone(), vec![self_member]].concat()),
            TypeMemberDescriptors::empty(),
          ),
          TypeMember::new(
            db,
            MemberType::DictOfSum(
              [
                base_type_members,
                vec![TypeMember::new(
                  db,
                  MemberType::Simple(Either::Right(self_symbol)),
                  TypeMemberDescriptors::empty(),
                )],
              ]
              .concat(),
            ),
            TypeMemberDescriptors::empty(),
          ),
        ],
      ]
      .concat(),
    ),
    TypeMemberDescriptors::empty(),
  );

  let optional_field = TypeMember::new(
    db,
    MemberType::eager_simple(get_bool_type(db).into()),
    TypeMemberDescriptors::OPTIONAL,
  );

  let fields = HashMap::from([
    ("type".to_string(), type_field),
    ("optional".to_string(), optional_field),
  ]);

  TdProductType::new(
    db,
    Some("SchemaProperty".to_string()),
    get_type_type(db).into(),
    fields,
    HashMap::new(),
  )
}

// Schema type is actually a kind
// and its a subtype of the "type" kind
#[query_derived]
pub fn get_schema_type(db: &TypedownDatabase) -> TdSchemaType {
  TdSchemaType::new(db)
}

// A schema with no declared fields, used for typeless resources
#[query_derived]
pub fn get_schemaless_type(db: &TypedownDatabase) -> TdProductType {
  let schema_type = get_schema_type(db);
  let empty_dict = TdDictObj::new(db, std::collections::HashMap::new());
  schema_type
    .construct(db, vec![empty_dict.into()])
    .and_then(|obj| obj.as_td_product_type().copied())
    .expect("TdSchemaType::construct with empty dict must produce a TdProductType")
}

pub fn get_type_type_symbol(db: &TypedownDatabase) -> Symbol {
  Symbol::new(
    db,
    SymbolKind::BuiltinSchema(BuiltinSchemaKind::TypeType),
    "type".to_string(),
    "@builtin::type".to_string(),
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

pub fn get_schema_property_symbol(db: &TypedownDatabase) -> Symbol {
  Symbol::new(
    db,
    SymbolKind::BuiltinSchema(BuiltinSchemaKind::SchemaProperty),
    "SchemaProperty".to_string(),
    "@builtin::schema_property".to_string(),
  )
}

#[query_derived]
pub fn get_blob_type(db: &TypedownDatabase) -> TdBlobType {
  TdBlobType::new(db)
}

#[query_derived]
pub fn get_func_type(db: &TypedownDatabase, signature: FuncSignature) -> TdFuncType {
  TdFuncType::new(db, signature)
}

#[query_derived]
pub fn instantiate_type(
  db: &TypedownDatabase,
  constructor: TdTypeEnum,
  args: Vec<TdTypeEnum>,
) -> InstResult {
  let arity = constructor.arity(db);
  if arity != args.len() {
    return InstResult::new(
      db,
      constructor.clone(),
      vec![Diagnostic::WrongTypeArgCount {
        expected: arity,
        got: args.len(),
      }],
    );
  }
  constructor.instantiate(db, args)
}

#[cfg(test)]
mod tests {
  use crate::db::types::TdTypeEnum;
  use crate::syntax::diagnostic::Diagnostic;

  use crate::db::{
    QueryStorage, TypedownDatabase,
    derived::get_builtin_types::{
      get_dict_type, get_list_type, get_num_type, get_str_type, instantiate_type,
    },
    types::TdTypeLike,
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

    let result = instantiate_type(&db, list, vec![str_type.clone()]);

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

    let result = instantiate_type(&db, record, vec![str_type, num_type]);

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

    let result = instantiate_type(&db, list, vec![]);

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
    let result = instantiate_type(&db, record, vec![str_type]);

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

    let result = instantiate_type(&db, str_type, vec![]);

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

    let result = instantiate_type(&db, str_type, vec![num_type]);

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
}
