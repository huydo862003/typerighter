use typedown_macros::query_derived;

use super::base::{TdRuntimeObject, TdStaticType, TdTypeType};
use super::{TdObjectEnum, TdTypeEnum};
use crate::db::TypedownDatabase;
use crate::db::derived::get_builtin_types::{get_null_obj, get_null_type};
use crate::db::types::Project;

#[query_derived]
pub struct TdNullType {}

impl TdRuntimeObject for TdNullType {
  fn get_type(&self, db: &TypedownDatabase) -> TdTypeEnum {
    TdTypeType::get(db).into()
  }
  fn get_owned_field(&self, _db: &TypedownDatabase, _key: &str) -> Option<TdObjectEnum> {
    None
  }
  fn source_path(&self, _db: &TypedownDatabase) -> String {
    "@builtin::null".to_string()
  }
}

impl TdStaticType for TdNullType {
  fn display_name(&self, _db: &TypedownDatabase) -> String {
    "null".to_string()
  }
  fn runtime_type(&self, _db: &TypedownDatabase) -> Option<TdTypeEnum> {
    Some((*self).into())
  }
  fn construct(
    &self,
    db: &TypedownDatabase,
    _project: Project,
    _args: Vec<TdObjectEnum>,
  ) -> Option<TdObjectEnum> {
    Some(TdNullObj::get(db).into())
  }
}

impl TdNullType {
  pub fn get(db: &TypedownDatabase) -> TdNullType {
    get_null_type(db)
  }
}

#[query_derived]
pub struct TdNullObj {}

impl TdRuntimeObject for TdNullObj {
  fn get_type(&self, db: &TypedownDatabase) -> TdTypeEnum {
    TdNullType::get(db).into()
  }
  fn get_owned_field(&self, _db: &TypedownDatabase, _key: &str) -> Option<TdObjectEnum> {
    None
  }
  fn source_path(&self, db: &TypedownDatabase) -> String {
    self.get_type(db).source_path(db)
  }
  fn eq(&self, _db: &TypedownDatabase, other: &TdObjectEnum) -> bool {
    other.as_td_null_obj().is_some()
  }
}

impl TdNullObj {
  pub fn get(db: &TypedownDatabase) -> TdNullObj {
    get_null_obj(db)
  }
}
