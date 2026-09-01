use typedown_macros::query_derived;

use super::base::{TdRuntimeObject, TdStaticType, TdTypeType};
use super::{TdObjectEnum, TdTypeEnum};
use crate::db::TypedownDatabase;
use crate::db::derived::get_builtin_types::get_icon_type;
use typedown_incremental::Id;

#[query_derived]
pub struct TdIconType<'db> {}

impl<'db> TdRuntimeObject<'db> for TdIconType<'db> {
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
    "@builtin::icon".to_string()
  }
}

impl<'db> TdStaticType<'db> for TdIconType<'db> {
  fn display_name(&self, _db: &'db TypedownDatabase) -> String {
    "icon".to_string()
  }
  fn runtime_type(&self, _db: &'db TypedownDatabase) -> Option<TdTypeEnum<'db>> {
    Some((*self).into())
  }
}

impl<'db> TdIconType<'db> {
  pub fn get(db: &'db TypedownDatabase) -> TdIconType<'db> {
    get_icon_type(db)
  }
}

#[query_derived]
pub struct TdIconObj<'db> {
  pub name: String,
  pub lucide_name: String,
}

impl<'db> TdRuntimeObject<'db> for TdIconObj<'db> {
  fn get_type(&self, db: &'db TypedownDatabase) -> TdTypeEnum<'db> {
    TdIconType::get(db).into()
  }
  fn get_owned_field(&self, _db: &'db TypedownDatabase, _key: &str) -> Option<TdObjectEnum<'db>> {
    None
  }
  fn get_builtin_field(&self, _db: &'db TypedownDatabase, _key: &str) -> Option<TdObjectEnum<'db>> {
    None
  }
  fn source_path(&self, db: &'db TypedownDatabase) -> String {
    self.get_type(db).source_path(db)
  }
  fn to_display_string(&self, db: &'db TypedownDatabase) -> String {
    self.name(db)
  }
  fn eq(&self, db: &'db TypedownDatabase, other: &TdObjectEnum<'db>) -> bool {
    if let TdObjectEnum::TdIconObj(other) = other {
      self.name(db) == other.name(db)
    } else {
      self.as_id() == other.as_id()
    }
  }
}
