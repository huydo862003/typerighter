mod utils;

use typedown_macros::query_derived;

use super::base::{TdRuntimeObj, TdStaticTyp, TdTypTyp};
use super::{TdObjEnum, TdTypEnum};
use crate::db::TypedownDatabase;
use crate::db::derived::get_builtin_typs::{get_date_typ, get_datetime_typ, get_time_typ};
use crate::db::types::Project;
use typedown_incremental::Id;
pub(crate) use utils::{is_valid_iso_date, is_valid_iso_datetime, is_valid_iso_time};

// DateTime

#[query_derived]
pub struct TdDateTimeTyp<'db> {}

impl<'db> TdRuntimeObj<'db> for TdDateTimeTyp<'db> {
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
    "@builtin::datetime".to_string()
  }
}

impl<'db> TdStaticTyp<'db> for TdDateTimeTyp<'db> {
  fn display_name(&self, _db: &'db TypedownDatabase) -> String {
    "datetime".to_string()
  }
  fn runtime_typ(&self, _db: &'db TypedownDatabase) -> Option<TdTypEnum<'db>> {
    Some((*self).into())
  }
  fn construct(
    &self,
    db: &'db TypedownDatabase,
    _project: Project,
    args: Vec<TdObjEnum<'db>>,
  ) -> Option<TdObjEnum<'db>> {
    let arg = args.into_iter().next()?;
    let string_obj = arg.as_td_string_obj()?;
    let val = string_obj.value(db);
    if is_valid_iso_datetime(&val) {
      return Some(TdDateTimeObj::new(db, val).into());
    }
    None
  }
}

impl<'db> TdDateTimeTyp<'db> {
  pub fn get(db: &'db TypedownDatabase) -> TdDateTimeTyp<'db> {
    get_datetime_typ(db)
  }
}

#[query_derived]
pub struct TdDateTimeObj<'db> {
  pub value: String,
}

impl<'db> TdRuntimeObj<'db> for TdDateTimeObj<'db> {
  fn get_typ(&self, db: &'db TypedownDatabase) -> TdTypEnum<'db> {
    TdDateTimeTyp::get(db).into()
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
    self.value(db)
  }
  fn eq(&self, db: &'db TypedownDatabase, other: &TdObjEnum<'db>) -> bool {
    if let TdObjEnum::TdDateTimeObj(other) = other {
      self.value(db) == other.value(db)
    } else {
      self.as_id() == other.as_id()
    }
  }
  fn lt(&self, db: &'db TypedownDatabase, other: &TdObjEnum<'db>) -> bool {
    if let TdObjEnum::TdDateTimeObj(other) = other {
      self.value(db) < other.value(db)
    } else {
      self.as_id() < other.as_id()
    }
  }
  fn gt(&self, db: &'db TypedownDatabase, other: &TdObjEnum<'db>) -> bool {
    if let TdObjEnum::TdDateTimeObj(other) = other {
      self.value(db) > other.value(db)
    } else {
      self.as_id() > other.as_id()
    }
  }
  fn le(&self, db: &'db TypedownDatabase, other: &TdObjEnum<'db>) -> bool {
    if let TdObjEnum::TdDateTimeObj(other) = other {
      self.value(db) <= other.value(db)
    } else {
      self.as_id() <= other.as_id()
    }
  }
  fn ge(&self, db: &'db TypedownDatabase, other: &TdObjEnum<'db>) -> bool {
    if let TdObjEnum::TdDateTimeObj(other) = other {
      self.value(db) >= other.value(db)
    } else {
      self.as_id() >= other.as_id()
    }
  }
}

// Date

#[query_derived]
pub struct TdDateTyp<'db> {}

impl<'db> TdRuntimeObj<'db> for TdDateTyp<'db> {
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
    "@builtin::date".to_string()
  }
}

impl<'db> TdStaticTyp<'db> for TdDateTyp<'db> {
  fn display_name(&self, _db: &'db TypedownDatabase) -> String {
    "date".to_string()
  }
  fn runtime_typ(&self, _db: &'db TypedownDatabase) -> Option<TdTypEnum<'db>> {
    Some((*self).into())
  }
  fn construct(
    &self,
    db: &'db TypedownDatabase,
    _project: Project,
    args: Vec<TdObjEnum<'db>>,
  ) -> Option<TdObjEnum<'db>> {
    let arg = args.into_iter().next()?;
    let string_obj = arg.as_td_string_obj()?;
    let val = string_obj.value(db);
    if is_valid_iso_date(&val) {
      return Some(TdDateObj::new(db, val).into());
    }
    None
  }
}

impl<'db> TdDateTyp<'db> {
  pub fn get(db: &'db TypedownDatabase) -> TdDateTyp<'db> {
    get_date_typ(db)
  }
}

#[query_derived]
pub struct TdDateObj<'db> {
  pub value: String,
}

impl<'db> TdRuntimeObj<'db> for TdDateObj<'db> {
  fn get_typ(&self, db: &'db TypedownDatabase) -> TdTypEnum<'db> {
    TdDateTyp::get(db).into()
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
    self.value(db)
  }
  fn eq(&self, db: &'db TypedownDatabase, other: &TdObjEnum<'db>) -> bool {
    if let TdObjEnum::TdDateObj(other) = other {
      self.value(db) == other.value(db)
    } else {
      self.as_id() == other.as_id()
    }
  }
  fn lt(&self, db: &'db TypedownDatabase, other: &TdObjEnum<'db>) -> bool {
    if let TdObjEnum::TdDateObj(other) = other {
      self.value(db) < other.value(db)
    } else {
      self.as_id() < other.as_id()
    }
  }
  fn gt(&self, db: &'db TypedownDatabase, other: &TdObjEnum<'db>) -> bool {
    if let TdObjEnum::TdDateObj(other) = other {
      self.value(db) > other.value(db)
    } else {
      self.as_id() > other.as_id()
    }
  }
  fn le(&self, db: &'db TypedownDatabase, other: &TdObjEnum<'db>) -> bool {
    if let TdObjEnum::TdDateObj(other) = other {
      self.value(db) <= other.value(db)
    } else {
      self.as_id() <= other.as_id()
    }
  }
  fn ge(&self, db: &'db TypedownDatabase, other: &TdObjEnum<'db>) -> bool {
    if let TdObjEnum::TdDateObj(other) = other {
      self.value(db) >= other.value(db)
    } else {
      self.as_id() >= other.as_id()
    }
  }
}

// Time

#[query_derived]
pub struct TdTimeTyp<'db> {}

impl<'db> TdRuntimeObj<'db> for TdTimeTyp<'db> {
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
    "@builtin::time".to_string()
  }
}

impl<'db> TdStaticTyp<'db> for TdTimeTyp<'db> {
  fn display_name(&self, _db: &'db TypedownDatabase) -> String {
    "time".to_string()
  }
  fn runtime_typ(&self, _db: &'db TypedownDatabase) -> Option<TdTypEnum<'db>> {
    Some((*self).into())
  }
  fn construct(
    &self,
    db: &'db TypedownDatabase,
    _project: Project,
    args: Vec<TdObjEnum<'db>>,
  ) -> Option<TdObjEnum<'db>> {
    let arg = args.into_iter().next()?;
    let string_obj = arg.as_td_string_obj()?;
    let val = string_obj.value(db);
    if is_valid_iso_time(&val) {
      return Some(TdTimeObj::new(db, val).into());
    }
    None
  }
}

impl<'db> TdTimeTyp<'db> {
  pub fn get(db: &'db TypedownDatabase) -> TdTimeTyp<'db> {
    get_time_typ(db)
  }
}

#[query_derived]
pub struct TdTimeObj<'db> {
  pub value: String,
}

impl<'db> TdRuntimeObj<'db> for TdTimeObj<'db> {
  fn get_typ(&self, db: &'db TypedownDatabase) -> TdTypEnum<'db> {
    TdTimeTyp::get(db).into()
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
    self.value(db)
  }
  fn eq(&self, db: &'db TypedownDatabase, other: &TdObjEnum<'db>) -> bool {
    if let TdObjEnum::TdTimeObj(other) = other {
      self.value(db) == other.value(db)
    } else {
      self.as_id() == other.as_id()
    }
  }
  fn lt(&self, db: &'db TypedownDatabase, other: &TdObjEnum<'db>) -> bool {
    if let TdObjEnum::TdTimeObj(other) = other {
      self.value(db) < other.value(db)
    } else {
      self.as_id() < other.as_id()
    }
  }
  fn gt(&self, db: &'db TypedownDatabase, other: &TdObjEnum<'db>) -> bool {
    if let TdObjEnum::TdTimeObj(other) = other {
      self.value(db) > other.value(db)
    } else {
      self.as_id() > other.as_id()
    }
  }
  fn le(&self, db: &'db TypedownDatabase, other: &TdObjEnum<'db>) -> bool {
    if let TdObjEnum::TdTimeObj(other) = other {
      self.value(db) <= other.value(db)
    } else {
      self.as_id() <= other.as_id()
    }
  }
  fn ge(&self, db: &'db TypedownDatabase, other: &TdObjEnum<'db>) -> bool {
    if let TdObjEnum::TdTimeObj(other) = other {
      self.value(db) >= other.value(db)
    } else {
      self.as_id() >= other.as_id()
    }
  }
}
