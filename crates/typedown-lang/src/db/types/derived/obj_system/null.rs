use typedown_macros::query_derived;

use super::base::{TdRuntimeObj, TdStaticTyp, TdTypTyp};
use super::{TdObjEnum, TdTypEnum};
use crate::db::TypedownDatabase;
use crate::db::derived::get_builtin_typs::{get_null_obj, get_null_typ};
use crate::db::types::Project;

#[query_derived]
pub struct TdNullTyp<'db> {}

impl<'db> TdRuntimeObj<'db> for TdNullTyp<'db> {
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
    "@builtin::null".to_string()
  }
}

impl<'db> TdStaticTyp<'db> for TdNullTyp<'db> {
  fn display_name(&self, _db: &'db TypedownDatabase) -> String {
    "null".to_string()
  }
  fn runtime_typ(&self, _db: &'db TypedownDatabase) -> Option<TdTypEnum<'db>> {
    Some((*self).into())
  }
  fn construct(
    &self,
    db: &'db TypedownDatabase,
    _project: Project,
    _args: Vec<TdObjEnum<'db>>,
  ) -> Option<TdObjEnum<'db>> {
    Some(TdNullObj::get(db).into())
  }
}

impl<'db> TdNullTyp<'db> {
  pub fn get(db: &'db TypedownDatabase) -> TdNullTyp<'db> {
    get_null_typ(db)
  }
}

#[query_derived]
pub struct TdNullObj<'db> {}

impl<'db> TdRuntimeObj<'db> for TdNullObj<'db> {
  fn get_typ(&self, db: &'db TypedownDatabase) -> TdTypEnum<'db> {
    TdNullTyp::get(db).into()
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
  fn eq(&self, _db: &'db TypedownDatabase, other: &TdObjEnum<'db>) -> bool {
    other.as_td_null_obj().is_some()
  }
}

impl<'db> TdNullObj<'db> {
  pub fn get(db: &'db TypedownDatabase) -> TdNullObj<'db> {
    get_null_obj(db)
  }
}
