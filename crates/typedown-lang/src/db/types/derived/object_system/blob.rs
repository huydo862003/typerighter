use std::collections::HashMap;

use typedown_macros::query_derived;

use super::base::{TdObjectLike, TdObjectType, TdTypeLike, TdTypeType};
use super::func::TdFuncObj;
use super::str::TdStrType;
use super::{TdObjectEnum, TdStrObj, TdTypeEnum};
use crate::db::TypedownDatabase;
use crate::db::derived::get_builtin_types::get_blob_type;
use crate::db::types::{
  AssetKind, File, InstResult, LazyType, MemberType, TypeMember, TypeMemberDescriptors,
};
use typedown_incremental::Id;

#[query_derived]
pub struct TdBlobType {}

impl TdObjectLike for TdBlobType {
  fn get_type(&self, db: &TypedownDatabase) -> TdTypeEnum {
    TdTypeType::get(db).into()
  }
  fn get_owned_field(&self, _db: &TypedownDatabase, _key: &str) -> Option<TdObjectEnum> {
    None
  }
  fn source_path(&self, _db: &TypedownDatabase) -> String {
    "@builtin::blob".to_string()
  }
}

impl TdTypeLike for TdBlobType {
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
    let str_type: TdTypeEnum = TdStrType::get(db).into();
    match name {
      "format" => Some(TypeMember::new(
        db,
        MemberType::Simple(LazyType::eager(str_type)),
        TypeMemberDescriptors::empty(),
      )),
      _ => None,
    }
  }
  fn instantiate(&self, db: &TypedownDatabase, args: Vec<LazyType>) -> InstResult {
    assert_eq!(args.len(), self.arity(db), "arity mismatch");
    InstResult::new(db, (*self).into(), vec![])
  }
  fn get_type_args(&self, _db: &TypedownDatabase) -> Vec<TdTypeEnum> {
    vec![]
  }
  fn accepts(&self, _db: &TypedownDatabase, actual: &TdTypeEnum) -> bool {
    self.as_id() == actual.as_id()
  }
  fn construct(&self, _db: &TypedownDatabase, _args: Vec<TdObjectEnum>) -> Option<TdObjectEnum> {
    None
  }
  fn display_name(&self, _db: &TypedownDatabase) -> String {
    "blob".to_string()
  }
}

impl TdBlobType {
  pub fn get(db: &TypedownDatabase) -> TdBlobType {
    get_blob_type(db)
  }
}

#[query_derived]
pub struct TdBlobObj {
  asset_kind: AssetKind,
  file: File,
}

impl TdObjectLike for TdBlobObj {
  fn get_type(&self, db: &TypedownDatabase) -> TdTypeEnum {
    TdBlobType::get(db).into()
  }
  fn get_owned_field(&self, db: &TypedownDatabase, key: &str) -> Option<TdObjectEnum> {
    match key {
      "format" => Some(TdStrObj::new(db, self.asset_kind(db).as_format_str().to_string()).into()),
      _ => None,
    }
  }
  fn source_path(&self, db: &TypedownDatabase) -> String {
    self.get_type(db).source_path(db)
  }
}
