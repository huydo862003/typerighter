use typedown_macros::query_derived;

use super::base::{TdRuntimeObj, TdStaticTyp, TdTypTyp};
use super::{TdObjEnum, TdTypEnum};
use crate::db::TypedownDatabase;
use crate::db::derived::get_builtin_typs::{get_bool_typ, get_false, get_true};
use crate::db::types::Project;
use typedown_incremental::Id;

#[query_derived]
pub struct TdBoolTyp<'db> {}

impl<'db> TdRuntimeObj<'db> for TdBoolTyp<'db> {
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
    "@builtin::boolean".to_string()
  }
}

impl<'db> TdStaticTyp<'db> for TdBoolTyp<'db> {
  fn display_name(&self, _db: &'db TypedownDatabase) -> String {
    "boolean".to_string()
  }
  fn runtime_typ(&self, _db: &'db TypedownDatabase) -> Option<TdTypEnum<'db>> {
    Some((*self).into())
  }
  fn construct(
    &self,
    _db: &'db TypedownDatabase,
    _project: Project,
    args: Vec<TdObjEnum<'db>>,
  ) -> Option<TdObjEnum<'db>> {
    let arg = args.into_iter().next()?;
    arg.as_td_bool_obj()?;
    Some(arg)
  }
}

impl<'db> TdBoolTyp<'db> {
  pub fn get(db: &'db TypedownDatabase) -> TdBoolTyp<'db> {
    get_bool_typ(db)
  }
}

#[query_derived]
pub struct TdBoolObj<'db> {
  pub value: bool,
}

impl<'db> TdRuntimeObj<'db> for TdBoolObj<'db> {
  fn get_typ(&self, db: &'db TypedownDatabase) -> TdTypEnum<'db> {
    TdBoolTyp::get(db).into()
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
    self.value(db).to_string()
  }
  fn eq(&self, db: &'db TypedownDatabase, other: &TdObjEnum<'db>) -> bool {
    if let TdObjEnum::TdBoolObj(other) = other {
      self.value(db) == other.value(db)
    } else {
      self.as_id() == other.as_id()
    }
  }
  fn lt(&self, db: &'db TypedownDatabase, other: &TdObjEnum<'db>) -> bool {
    if let TdObjEnum::TdBoolObj(other) = other {
      !self.value(db) && other.value(db)
    } else {
      self.as_id() < other.as_id()
    }
  }
  fn gt(&self, db: &'db TypedownDatabase, other: &TdObjEnum<'db>) -> bool {
    if let TdObjEnum::TdBoolObj(other) = other {
      self.value(db) && !other.value(db)
    } else {
      self.as_id() > other.as_id()
    }
  }
  fn le(&self, db: &'db TypedownDatabase, other: &TdObjEnum<'db>) -> bool {
    if let TdObjEnum::TdBoolObj(other) = other {
      !self.value(db) || other.value(db)
    } else {
      self.as_id() <= other.as_id()
    }
  }
  fn ge(&self, db: &'db TypedownDatabase, other: &TdObjEnum<'db>) -> bool {
    if let TdObjEnum::TdBoolObj(other) = other {
      self.value(db) || !other.value(db)
    } else {
      self.as_id() >= other.as_id()
    }
  }
}

impl<'db> TdBoolObj<'db> {
  pub fn get_true(db: &'db TypedownDatabase) -> TdBoolObj<'db> {
    get_true(db)
  }

  pub fn get_false(db: &'db TypedownDatabase) -> TdBoolObj<'db> {
    get_false(db)
  }
}
