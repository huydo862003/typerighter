use typedown_macros::query_derived;

use super::base::{TdRuntimeObject, TdStaticType, TdTypeType};
use super::{TdObjectEnum, TdStrObj, TdTypeEnum};
use crate::db::TypedownDatabase;
use crate::db::derived::get_builtin_types::{get_blob_type, get_str_type};
use crate::db::types::{AssetKind, File};

#[query_derived]
pub struct TdBlobType<'db> {}

impl<'db> TdRuntimeObject<'db> for TdBlobType<'db> {
  fn get_type(&self, db: &'db TypedownDatabase) -> TdTypeEnum<'db> {
    TdTypeType::get(db).into()
  }
  fn get_owned_field(&self, _db: &'db TypedownDatabase, _key: &str) -> Option<TdObjectEnum<'db>> {
    None
  }
  fn get_builtin_field(&self, _db: &'db TypedownDatabase, _key: &str) -> Option<TdObjectEnum<'db>> {
    None
  }
  fn source_path(&self, _db: &'db TypedownDatabase) -> String {
    "@builtin::blob".to_string()
  }
}

impl<'db> TdStaticType<'db> for TdBlobType<'db> {
  fn display_name(&self, _db: &'db TypedownDatabase) -> String {
    "blob".to_string()
  }
  fn runtime_type(&self, _db: &'db TypedownDatabase) -> Option<TdTypeEnum<'db>> {
    Some((*self).into())
  }
  fn get_owned_field_type(&self, db: &'db TypedownDatabase, name: &str) -> Option<TdTypeEnum<'db>> {
    match name {
      "format" => Some(get_str_type(db).into()),
      _ => None,
    }
  }
}

impl<'db> TdBlobType<'db> {
  pub fn get(db: &'db TypedownDatabase) -> TdBlobType<'db> {
    get_blob_type(db)
  }
}

#[query_derived]
pub struct TdBlobObj<'db> {
  asset_kind: AssetKind,
  file: File,
}

impl<'db> TdRuntimeObject<'db> for TdBlobObj<'db> {
  fn get_type(&self, db: &'db TypedownDatabase) -> TdTypeEnum<'db> {
    TdBlobType::get(db).into()
  }
  fn get_owned_field(&self, db: &'db TypedownDatabase, key: &str) -> Option<TdObjectEnum<'db>> {
    match key {
      "format" => Some(TdStrObj::new(db, self.asset_kind(db).as_format_str().to_string()).into()),
      _ => None,
    }
  }
  fn get_builtin_field(&self, _db: &'db TypedownDatabase, _key: &str) -> Option<TdObjectEnum<'db>> {
    None
  }
  fn source_path(&self, db: &'db TypedownDatabase) -> String {
    self.get_type(db).source_path(db)
  }
}
