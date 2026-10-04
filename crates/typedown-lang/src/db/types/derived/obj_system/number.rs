use typedown_macros::query_derived;

use super::base::{TdRuntimeObj, TdStaticTyp, TdTypTyp};
use super::{TdObjEnum, TdTypEnum};
use crate::db::TypedownDatabase;
use crate::db::derived::get_builtin_typs::get_number_typ;
use crate::db::types::Project;
use typedown_incremental::Id;

#[query_derived]
pub struct TdNumberTyp<'db> {}

impl<'db> TdRuntimeObj<'db> for TdNumberTyp<'db> {
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
    "@builtin::number".to_string()
  }
}

impl<'db> TdStaticTyp<'db> for TdNumberTyp<'db> {
  fn display_name(&self, _db: &'db TypedownDatabase) -> String {
    "number".to_string()
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
    arg.as_td_number_obj()?;
    Some(arg)
  }
}

impl<'db> TdNumberTyp<'db> {
  pub fn get(db: &'db TypedownDatabase) -> TdNumberTyp<'db> {
    get_number_typ(db)
  }
}

#[query_derived]
pub struct TdNumberObj<'db> {
  pub value: f64,
}

impl<'db> TdRuntimeObj<'db> for TdNumberObj<'db> {
  fn get_typ(&self, db: &'db TypedownDatabase) -> TdTypEnum<'db> {
    TdNumberTyp::get(db).into()
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
    if let TdObjEnum::TdNumberObj(other) = other {
      self.value(db) == other.value(db)
    } else {
      self.as_id() == other.as_id()
    }
  }
  fn lt(&self, db: &'db TypedownDatabase, other: &TdObjEnum<'db>) -> bool {
    if let TdObjEnum::TdNumberObj(other) = other {
      self.value(db) < other.value(db)
    } else {
      self.as_id() < other.as_id()
    }
  }
  fn gt(&self, db: &'db TypedownDatabase, other: &TdObjEnum<'db>) -> bool {
    if let TdObjEnum::TdNumberObj(other) = other {
      self.value(db) > other.value(db)
    } else {
      self.as_id() > other.as_id()
    }
  }
  fn le(&self, db: &'db TypedownDatabase, other: &TdObjEnum<'db>) -> bool {
    if let TdObjEnum::TdNumberObj(other) = other {
      self.value(db) <= other.value(db)
    } else {
      self.as_id() <= other.as_id()
    }
  }
  fn ge(&self, db: &'db TypedownDatabase, other: &TdObjEnum<'db>) -> bool {
    if let TdObjEnum::TdNumberObj(other) = other {
      self.value(db) >= other.value(db)
    } else {
      self.as_id() >= other.as_id()
    }
  }
}
