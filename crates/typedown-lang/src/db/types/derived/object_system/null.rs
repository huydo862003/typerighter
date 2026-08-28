use typedown_macros::query_derived;

use super::base::{TdRuntimeObject, TdStaticType, TdTypeType};
use super::{TdObjectEnum, TdTypeEnum};
use crate::db::TypedownDatabase;
use crate::db::derived::get_builtin_types::{get_null_obj, get_null_type};
use crate::db::types::Project;

#[query_derived]
pub struct TdNullType<'db> {}

impl<'db> TdRuntimeObject<'db> for TdNullType<'db> {
  fn get_type(&self, db: &'db TypedownDatabase) -> TdTypeEnum<'db> {
    TdTypeType::get(db).into()
  }
  fn get_owned_field(&self, _db: &'db TypedownDatabase, _key: &str) -> Option<TdObjectEnum<'db>> {
    None
  }
  fn source_path(&self, _db: &'db TypedownDatabase) -> String {
    "@builtin::null".to_string()
  }
}

impl<'db> TdStaticType<'db> for TdNullType<'db> {
  fn display_name(&self, _db: &'db TypedownDatabase) -> String {
    "null".to_string()
  }
  fn runtime_type(&self, _db: &'db TypedownDatabase) -> Option<TdTypeEnum<'db>> {
    Some((*self).into())
  }
  fn construct(
    &self,
    db: &'db TypedownDatabase,
    _project: Project,
    _args: Vec<TdObjectEnum<'db>>,
  ) -> Option<TdObjectEnum<'db>> {
    Some(TdNullObj::get(db).into())
  }
}

impl<'db> TdNullType<'db> {
  pub fn get(db: &'db TypedownDatabase) -> TdNullType<'db> {
    get_null_type(db)
  }
}

#[query_derived]
pub struct TdNullObj<'db> {}

impl<'db> TdRuntimeObject<'db> for TdNullObj<'db> {
  fn get_type(&self, db: &'db TypedownDatabase) -> TdTypeEnum<'db> {
    TdNullType::get(db).into()
  }
  fn get_owned_field(&self, _db: &'db TypedownDatabase, _key: &str) -> Option<TdObjectEnum<'db>> {
    None
  }
  fn source_path(&self, db: &'db TypedownDatabase) -> String {
    self.get_type(db).source_path(db)
  }
  fn eq(&self, _db: &'db TypedownDatabase, other: &TdObjectEnum<'db>) -> bool {
    other.as_td_null_obj().is_some()
  }
}

impl<'db> TdNullObj<'db> {
  pub fn get(db: &'db TypedownDatabase) -> TdNullObj<'db> {
    get_null_obj(db)
  }
}
