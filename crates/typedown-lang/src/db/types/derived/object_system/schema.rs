use std::collections::HashMap;
use typedown_incremental::Id;
use typedown_macros::query_derived;

use super::base::{TdObjectLike, TdTypeLike, TdTypeType};
use super::dict::TdDictType;
use super::func::TdFuncObj;
use super::{TdObjectEnum, TdProductType, TdTypeEnum};
use crate::db::TypedownDatabase;
use crate::db::derived::evaluate::evaluate_node::evaluate_node;
use crate::db::derived::get_builtin_types::{get_schema_type, get_str_type};
use crate::db::derived::schema_property::get_schema_property_type;
use crate::db::types::{InstResult, LazyType, MemberType, TypeMember, TypeMemberDescriptors};
use typedown_types::either::Either;

// Schema type is actually a kind
// and its a subtype of the "type" kind
#[query_derived]
pub struct TdSchemaType {}

impl TdObjectLike for TdSchemaType {
  fn get_type(&self, db: &TypedownDatabase) -> TdTypeEnum {
    TdTypeType::get(db).into()
  }
  fn get_owned_field(&self, _db: &TypedownDatabase, _key: &str) -> Option<TdObjectEnum> {
    None
  }
  fn source_path(&self, _db: &TypedownDatabase) -> String {
    "@builtin::schema".to_string()
  }
}

impl TdTypeLike for TdSchemaType {
  fn arity(&self, _db: &TypedownDatabase) -> usize {
    0
  }
  fn get_supertype(&self, db: &TypedownDatabase) -> TdTypeEnum {
    TdTypeType::get(db).into()
  }
  fn get_vtable(&self, _db: &TypedownDatabase) -> HashMap<String, TdFuncObj> {
    HashMap::new()
  }
  fn get_owned_field_type_member(&self, db: &TypedownDatabase, name: &str) -> Option<TypeMember> {
    match name {
      "properties" => {
        let properties_type = TdDictType::new(
          db,
          Some(get_str_type(db).into()),
          Some(get_schema_property_type(db).into()),
        );
        Some(TypeMember::new(
          db,
          MemberType::Simple(LazyType::eager(properties_type.into())),
          TypeMemberDescriptors::empty(),
        ))
      }
      _ => None,
    }
  }
  fn instantiate(&self, db: &TypedownDatabase, _args: Vec<TdTypeEnum>) -> InstResult {
    InstResult::new(db, (*self).into(), vec![])
  }
  fn get_type_args(&self, _db: &TypedownDatabase) -> Vec<TdTypeEnum> {
    vec![]
  }
  fn is_compatible_with(&self, _db: &TypedownDatabase, actual: &TdTypeEnum) -> bool {
    self.as_id() == actual.as_id()
  }
  fn construct(&self, db: &TypedownDatabase, args: Vec<TdObjectEnum>) -> Option<TdObjectEnum> {
    let arg = args.into_iter().next()?;
    let dict = arg.as_td_dict_obj()?;
    let mut fields = HashMap::new();
    for (name, entry) in dict.entries(db) {
      let obj = match entry {
        Either::Left(hir) => evaluate_node(db, hir).value(db)?,
        Either::Right(obj) => obj,
      };
      let typ = obj.as_type()?;
      fields.insert(
        name,
        TypeMember::new(
          db,
          MemberType::Simple(LazyType::eager(typ)),
          TypeMemberDescriptors::empty(),
        ),
      );
    }
    Some(
      TdProductType::new(
        db,
        None,
        TdSchemaType::get(db).into(),
        fields,
        HashMap::new(),
      )
      .into(),
    )
  }
  fn display_name(&self, _db: &TypedownDatabase) -> String {
    "schema".to_string()
  }
}

impl TdSchemaType {
  pub fn get(db: &TypedownDatabase) -> TdSchemaType {
    get_schema_type(db)
  }
}
