use std::collections::HashMap;
use typedown_incremental::Id;
use typedown_macros::query_derived;

use super::base::{TdObjectLike, TdObjectType, TdTypeLike, TdTypeType};
use super::func::TdFuncObj;
use super::{TdObjectEnum, TdTypeEnum};
use crate::db::TypedownDatabase;
use crate::db::derived::get_builtin_types::{
  get_bool_type, get_num_type, get_schema_property_type, get_str_type, get_type_type,
};
use crate::db::types::{InstResult, MemberType, TypeMember, TypeMemberDescriptors};
use crate::db::utils::typecheck::member_types_compatible;

/// The type of a single property descriptor inside a schema's `properties` field.
/// Each property descriptor has:
///   - `type`: a type value (required)
///   - `optional`: a boolean (optional, defaults to false)
#[query_derived]
pub struct TdSchemaPropertyType {}

impl TdObjectLike for TdSchemaPropertyType {
  fn get_type(&self, db: &TypedownDatabase) -> TdTypeEnum {
    TdTypeType::get(db).into()
  }
  fn get_owned_field(&self, _db: &TypedownDatabase, _key: &str) -> Option<TdObjectEnum> {
    None
  }
  fn source_path(&self, _db: &TypedownDatabase) -> String {
    "@builtin::schema_property".to_string()
  }
}

impl TdTypeLike for TdSchemaPropertyType {
  fn arity(&self, _db: &TypedownDatabase) -> usize {
    0
  }
  fn get_supertype(&self, db: &TypedownDatabase) -> TdTypeEnum {
    TdObjectType::get(db).into()
  }
  fn get_vtable(&self, _db: &TypedownDatabase) -> HashMap<String, TdFuncObj> {
    HashMap::new()
  }
  fn get_owned_field_type_member(&self, db: &TypedownDatabase, name: &str) -> Option<TypeMember> {
    let base_type_members = vec![
      // type: string
      TypeMember::new(
        db,
        MemberType::simple(get_type_type(db).into()),
        TypeMemberDescriptors::empty(),
      ),
      // type: 'literal'
      TypeMember::new(
        db,
        MemberType::simple(get_str_type(db).into()),
        TypeMemberDescriptors::empty(),
      ),
      // type: false
      TypeMember::new(
        db,
        MemberType::simple(get_bool_type(db).into()),
        TypeMemberDescriptors::empty(),
      ),
      // type: 0
      TypeMember::new(
        db,
        MemberType::simple(get_num_type(db).into()),
        TypeMemberDescriptors::empty(),
      ),
    ];
    match name {
      "type" => Some(TypeMember::new(
        db,
        MemberType::Sum(
          [
            base_type_members.clone(),
            vec![
              // type: [string, 0, 'literal']
              TypeMember::new(
                db,
                MemberType::ListOfSum(
                  [
                    base_type_members.clone(),
                    vec![TypeMember::new(
                      db,
                      MemberType::simple((*self).into()),
                      TypeMemberDescriptors::empty(),
                    )],
                  ]
                  .concat(),
                ),
                TypeMemberDescriptors::empty(),
              ),
              // type: {}
              TypeMember::new(
                db,
                MemberType::DictOfSum(
                  [
                    base_type_members.clone(),
                    vec![TypeMember::new(
                      db,
                      MemberType::simple((*self).into()),
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
      )),
      "optional" => Some(TypeMember::new(
        db,
        MemberType::simple(get_bool_type(db).into()),
        TypeMemberDescriptors::OPTIONAL,
      )),
      _ => None,
    }
  }
  fn instantiate(&self, db: &TypedownDatabase, args: Vec<TdTypeEnum>) -> InstResult {
    assert_eq!(args.len(), self.arity(db), "arity mismatch");
    InstResult::new(db, (*self).into(), vec![])
  }
  fn get_type_args(&self, _db: &TypedownDatabase) -> Vec<TdTypeEnum> {
    vec![]
  }
  fn is_compatible_with(&self, db: &TypedownDatabase, actual: &TdTypeEnum) -> bool {
    if self.as_id() == actual.as_id() {
      return true;
    }
    let actual_product = match actual.as_td_product_type() {
      Some(p) => p,
      None => return false,
    };
    // FIXME: Migrate the structural check like TdProductType
    let actual_fields = actual_product.fields(db);
    let field_names = ["type", "optional"];
    for name in field_names {
      let expected = match self.get_owned_field_type_member(db, name) {
        Some(m) => m,
        None => continue,
      };
      if expected
        .descriptors(db)
        .contains(TypeMemberDescriptors::OPTIONAL)
      {
        continue;
      }
      let actual_member = match actual_fields.get(name) {
        Some(m) => m,
        None => return false,
      };
      let exp_typ = expected.typ(db);
      let act_typ = actual_member.typ(db);
      if !member_types_compatible(db, &exp_typ, &act_typ) {
        return false;
      }
    }
    true
  }
  fn construct(&self, _db: &TypedownDatabase, _args: Vec<TdObjectEnum>) -> Option<TdObjectEnum> {
    None
  }
  fn display_name(&self, _db: &TypedownDatabase) -> String {
    "SchemaProperty".to_string()
  }
}

impl TdSchemaPropertyType {
  pub fn get(db: &TypedownDatabase) -> TdSchemaPropertyType {
    get_schema_property_type(db)
  }
}
