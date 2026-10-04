use typedown_macros::query_derived;

use super::base::{TdRuntimeObj, TdStaticTyp, TdTypTyp};
use super::{TdObjEnum, TdTypEnum};
use crate::db::TypedownDatabase;
use crate::db::derived::get_builtin_typs::get_icon_typ;
use typedown_incremental::Id;

#[query_derived]
pub struct TdIconTyp<'db> {}

impl<'db> TdRuntimeObj<'db> for TdIconTyp<'db> {
  fn get_typ(&self, db: &'db TypedownDatabase) -> TdTypEnum<'db> {
    TdTypTyp::get(db).into()
  }
  fn get_owned_field(&self, _db: &'db TypedownDatabase, _key: &str) -> Option<TdObjEnum<'db>> {
    None
  }
  fn get_builtin_field(&self, _db: &'db TypedownDatabase, _key: &str) -> Option<TdObjEnum<'db>> {
    None
  }
  fn source_path(&self, _db: &'db TypedownDatabase) -> String {
    "@builtin::icon".to_string()
  }
}

impl<'db> TdStaticTyp<'db> for TdIconTyp<'db> {
  fn display_name(&self, _db: &'db TypedownDatabase) -> String {
    "icon".to_string()
  }
  fn runtime_typ(&self, _db: &'db TypedownDatabase) -> Option<TdTypEnum<'db>> {
    Some((*self).into())
  }
}

impl<'db> TdIconTyp<'db> {
  pub fn get(db: &'db TypedownDatabase) -> TdIconTyp<'db> {
    get_icon_typ(db)
  }
}

#[query_derived]
pub struct TdIconObj<'db> {
  pub name: String,
  pub lucide_name: String,
}

impl<'db> TdRuntimeObj<'db> for TdIconObj<'db> {
  fn get_typ(&self, db: &'db TypedownDatabase) -> TdTypEnum<'db> {
    TdIconTyp::get(db).into()
  }
  fn get_owned_field(&self, _db: &'db TypedownDatabase, _key: &str) -> Option<TdObjEnum<'db>> {
    None
  }
  fn get_builtin_field(&self, _db: &'db TypedownDatabase, _key: &str) -> Option<TdObjEnum<'db>> {
    None
  }
  fn source_path(&self, db: &'db TypedownDatabase) -> String {
    self.get_typ(db).source_path(db)
  }
  fn to_display_string(&self, db: &'db TypedownDatabase) -> String {
    self.name(db)
  }
  fn eq(&self, db: &'db TypedownDatabase, other: &TdObjEnum<'db>) -> bool {
    if let TdObjEnum::TdIconObj(other) = other {
      self.name(db) == other.name(db)
    } else {
      self.as_id() == other.as_id()
    }
  }
}
