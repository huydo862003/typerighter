// Schema property type definition
// A property descriptor inside a schema's `properties` field
// Has a required `type` field and an optional `optional` field

use std::collections::HashMap;

use typedown_incremental::QueryDatabase;
use typedown_macros::query_derived;

use crate::db::TypedownDatabase;
use crate::db::derived::get_builtin_types::{
  get_bool_type, get_num_type, get_str_type, get_type_type,
};
use crate::db::types::{
  BuiltinSchemaKind, LazyType, MemberType, Symbol, SymbolKind, TdProductType, TdTypeEnum,
  TypeMember, TypeMemberDescriptors,
};

fn get_schema_property_symbol(db: &TypedownDatabase) -> Symbol {
  Symbol::new(
    db,
    SymbolKind::BuiltinSchema(BuiltinSchemaKind::SchemaProperty),
    "SchemaProperty".to_string(),
    "@builtin::schema_property".to_string(),
  )
}

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
      MemberType::Simple(LazyType::eager(type_type)),
      TypeMemberDescriptors::empty(),
    ),
    TypeMember::new(
      db,
      MemberType::Simple(LazyType::eager(str_type)),
      TypeMemberDescriptors::empty(),
    ),
    TypeMember::new(
      db,
      MemberType::Simple(LazyType::eager(bool_type.clone())),
      TypeMemberDescriptors::empty(),
    ),
    TypeMember::new(
      db,
      MemberType::Simple(LazyType::eager(num_type)),
      TypeMemberDescriptors::empty(),
    ),
  ];

  // Lazy self-reference to avoid recursive query
  let self_symbol = get_schema_property_symbol(db);
  let self_member = TypeMember::new(
    db,
    MemberType::Simple(LazyType::lazy(self_symbol)),
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
                  MemberType::Simple(LazyType::lazy(self_symbol)),
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
    MemberType::Simple(LazyType::eager(get_bool_type(db).into())),
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
