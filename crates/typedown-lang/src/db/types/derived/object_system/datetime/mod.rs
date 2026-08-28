mod utils;

use typedown_macros::query_derived;

use super::base::{TdRuntimeObject, TdStaticType, TdTypeType};
use super::{TdObjectEnum, TdTypeEnum};
use crate::db::TypedownDatabase;
use crate::db::derived::get_builtin_types::{get_date_type, get_datetime_type, get_time_type};
use crate::db::types::Project;
use typedown_incremental::Id;
pub(crate) use utils::{is_valid_iso_date, is_valid_iso_datetime, is_valid_iso_time};

// DateTime

#[query_derived]
pub struct TdDateTimeType<'db> {}

impl<'db> TdRuntimeObject<'db> for TdDateTimeType<'db> {
  fn get_type(&self, db: &'db TypedownDatabase) -> TdTypeEnum<'db> {
    TdTypeType::get(db).into()
  }
  fn get_owned_field(&self, _db: &'db TypedownDatabase, _key: &str) -> Option<TdObjectEnum<'db>> {
    None
  }
  fn source_path(&self, _db: &'db TypedownDatabase) -> String {
    "@builtin::datetime".to_string()
  }
}

impl<'db> TdStaticType<'db> for TdDateTimeType<'db> {
  fn display_name(&self, _db: &'db TypedownDatabase) -> String {
    "datetime".to_string()
  }
  fn runtime_type(&self, _db: &'db TypedownDatabase) -> Option<TdTypeEnum<'db>> {
    Some((*self).into())
  }
  fn construct(
    &self,
    db: &'db TypedownDatabase,
    _project: Project,
    args: Vec<TdObjectEnum<'db>>,
  ) -> Option<TdObjectEnum<'db>> {
    let arg = args.into_iter().next()?;
    let str_obj = arg.as_td_str_obj()?;
    let val = str_obj.value(db);
    if is_valid_iso_datetime(&val) {
      return Some(TdDateTimeObj::new(db, val).into());
    }
    None
  }
}

impl<'db> TdDateTimeType<'db> {
  pub fn get(db: &'db TypedownDatabase) -> TdDateTimeType<'db> {
    get_datetime_type(db)
  }
}

#[query_derived]
pub struct TdDateTimeObj<'db> {
  pub value: String,
}

impl<'db> TdRuntimeObject<'db> for TdDateTimeObj<'db> {
  fn get_type(&self, db: &'db TypedownDatabase) -> TdTypeEnum<'db> {
    TdDateTimeType::get(db).into()
  }
  fn get_owned_field(&self, _db: &'db TypedownDatabase, _key: &str) -> Option<TdObjectEnum<'db>> {
    None
  }
  fn source_path(&self, db: &'db TypedownDatabase) -> String {
    self.get_type(db).source_path(db)
  }
  fn to_display_string(&self, db: &'db TypedownDatabase) -> String {
    self.value(db)
  }
  fn eq(&self, db: &'db TypedownDatabase, other: &TdObjectEnum<'db>) -> bool {
    if let TdObjectEnum::TdDateTimeObj(other) = other {
      self.value(db) == other.value(db)
    } else {
      self.as_id() == other.as_id()
    }
  }
  fn lt(&self, db: &'db TypedownDatabase, other: &TdObjectEnum<'db>) -> bool {
    if let TdObjectEnum::TdDateTimeObj(other) = other {
      self.value(db) < other.value(db)
    } else {
      self.as_id() < other.as_id()
    }
  }
  fn gt(&self, db: &'db TypedownDatabase, other: &TdObjectEnum<'db>) -> bool {
    if let TdObjectEnum::TdDateTimeObj(other) = other {
      self.value(db) > other.value(db)
    } else {
      self.as_id() > other.as_id()
    }
  }
  fn le(&self, db: &'db TypedownDatabase, other: &TdObjectEnum<'db>) -> bool {
    if let TdObjectEnum::TdDateTimeObj(other) = other {
      self.value(db) <= other.value(db)
    } else {
      self.as_id() <= other.as_id()
    }
  }
  fn ge(&self, db: &'db TypedownDatabase, other: &TdObjectEnum<'db>) -> bool {
    if let TdObjectEnum::TdDateTimeObj(other) = other {
      self.value(db) >= other.value(db)
    } else {
      self.as_id() >= other.as_id()
    }
  }
}

// Date

#[query_derived]
pub struct TdDateType<'db> {}

impl<'db> TdRuntimeObject<'db> for TdDateType<'db> {
  fn get_type(&self, db: &'db TypedownDatabase) -> TdTypeEnum<'db> {
    TdTypeType::get(db).into()
  }
  fn get_owned_field(&self, _db: &'db TypedownDatabase, _key: &str) -> Option<TdObjectEnum<'db>> {
    None
  }
  fn source_path(&self, _db: &'db TypedownDatabase) -> String {
    "@builtin::date".to_string()
  }
}

impl<'db> TdStaticType<'db> for TdDateType<'db> {
  fn display_name(&self, _db: &'db TypedownDatabase) -> String {
    "date".to_string()
  }
  fn runtime_type(&self, _db: &'db TypedownDatabase) -> Option<TdTypeEnum<'db>> {
    Some((*self).into())
  }
  fn construct(
    &self,
    db: &'db TypedownDatabase,
    _project: Project,
    args: Vec<TdObjectEnum<'db>>,
  ) -> Option<TdObjectEnum<'db>> {
    let arg = args.into_iter().next()?;
    let str_obj = arg.as_td_str_obj()?;
    let val = str_obj.value(db);
    if is_valid_iso_date(&val) {
      return Some(TdDateObj::new(db, val).into());
    }
    None
  }
}

impl<'db> TdDateType<'db> {
  pub fn get(db: &'db TypedownDatabase) -> TdDateType<'db> {
    get_date_type(db)
  }
}

#[query_derived]
pub struct TdDateObj<'db> {
  pub value: String,
}

impl<'db> TdRuntimeObject<'db> for TdDateObj<'db> {
  fn get_type(&self, db: &'db TypedownDatabase) -> TdTypeEnum<'db> {
    TdDateType::get(db).into()
  }
  fn get_owned_field(&self, _db: &'db TypedownDatabase, _key: &str) -> Option<TdObjectEnum<'db>> {
    None
  }
  fn source_path(&self, db: &'db TypedownDatabase) -> String {
    self.get_type(db).source_path(db)
  }
  fn to_display_string(&self, db: &'db TypedownDatabase) -> String {
    self.value(db)
  }
  fn eq(&self, db: &'db TypedownDatabase, other: &TdObjectEnum<'db>) -> bool {
    if let TdObjectEnum::TdDateObj(other) = other {
      self.value(db) == other.value(db)
    } else {
      self.as_id() == other.as_id()
    }
  }
  fn lt(&self, db: &'db TypedownDatabase, other: &TdObjectEnum<'db>) -> bool {
    if let TdObjectEnum::TdDateObj(other) = other {
      self.value(db) < other.value(db)
    } else {
      self.as_id() < other.as_id()
    }
  }
  fn gt(&self, db: &'db TypedownDatabase, other: &TdObjectEnum<'db>) -> bool {
    if let TdObjectEnum::TdDateObj(other) = other {
      self.value(db) > other.value(db)
    } else {
      self.as_id() > other.as_id()
    }
  }
  fn le(&self, db: &'db TypedownDatabase, other: &TdObjectEnum<'db>) -> bool {
    if let TdObjectEnum::TdDateObj(other) = other {
      self.value(db) <= other.value(db)
    } else {
      self.as_id() <= other.as_id()
    }
  }
  fn ge(&self, db: &'db TypedownDatabase, other: &TdObjectEnum<'db>) -> bool {
    if let TdObjectEnum::TdDateObj(other) = other {
      self.value(db) >= other.value(db)
    } else {
      self.as_id() >= other.as_id()
    }
  }
}

// Time

#[query_derived]
pub struct TdTimeType<'db> {}

impl<'db> TdRuntimeObject<'db> for TdTimeType<'db> {
  fn get_type(&self, db: &'db TypedownDatabase) -> TdTypeEnum<'db> {
    TdTypeType::get(db).into()
  }
  fn get_owned_field(&self, _db: &'db TypedownDatabase, _key: &str) -> Option<TdObjectEnum<'db>> {
    None
  }
  fn source_path(&self, _db: &'db TypedownDatabase) -> String {
    "@builtin::time".to_string()
  }
}

impl<'db> TdStaticType<'db> for TdTimeType<'db> {
  fn display_name(&self, _db: &'db TypedownDatabase) -> String {
    "time".to_string()
  }
  fn runtime_type(&self, _db: &'db TypedownDatabase) -> Option<TdTypeEnum<'db>> {
    Some((*self).into())
  }
  fn construct(
    &self,
    db: &'db TypedownDatabase,
    _project: Project,
    args: Vec<TdObjectEnum<'db>>,
  ) -> Option<TdObjectEnum<'db>> {
    let arg = args.into_iter().next()?;
    let str_obj = arg.as_td_str_obj()?;
    let val = str_obj.value(db);
    if is_valid_iso_time(&val) {
      return Some(TdTimeObj::new(db, val).into());
    }
    None
  }
}

impl<'db> TdTimeType<'db> {
  pub fn get(db: &'db TypedownDatabase) -> TdTimeType<'db> {
    get_time_type(db)
  }
}

#[query_derived]
pub struct TdTimeObj<'db> {
  pub value: String,
}

impl<'db> TdRuntimeObject<'db> for TdTimeObj<'db> {
  fn get_type(&self, db: &'db TypedownDatabase) -> TdTypeEnum<'db> {
    TdTimeType::get(db).into()
  }
  fn get_owned_field(&self, _db: &'db TypedownDatabase, _key: &str) -> Option<TdObjectEnum<'db>> {
    None
  }
  fn source_path(&self, db: &'db TypedownDatabase) -> String {
    self.get_type(db).source_path(db)
  }
  fn to_display_string(&self, db: &'db TypedownDatabase) -> String {
    self.value(db)
  }
  fn eq(&self, db: &'db TypedownDatabase, other: &TdObjectEnum<'db>) -> bool {
    if let TdObjectEnum::TdTimeObj(other) = other {
      self.value(db) == other.value(db)
    } else {
      self.as_id() == other.as_id()
    }
  }
  fn lt(&self, db: &'db TypedownDatabase, other: &TdObjectEnum<'db>) -> bool {
    if let TdObjectEnum::TdTimeObj(other) = other {
      self.value(db) < other.value(db)
    } else {
      self.as_id() < other.as_id()
    }
  }
  fn gt(&self, db: &'db TypedownDatabase, other: &TdObjectEnum<'db>) -> bool {
    if let TdObjectEnum::TdTimeObj(other) = other {
      self.value(db) > other.value(db)
    } else {
      self.as_id() > other.as_id()
    }
  }
  fn le(&self, db: &'db TypedownDatabase, other: &TdObjectEnum<'db>) -> bool {
    if let TdObjectEnum::TdTimeObj(other) = other {
      self.value(db) <= other.value(db)
    } else {
      self.as_id() <= other.as_id()
    }
  }
  fn ge(&self, db: &'db TypedownDatabase, other: &TdObjectEnum<'db>) -> bool {
    if let TdObjectEnum::TdTimeObj(other) = other {
      self.value(db) >= other.value(db)
    } else {
      self.as_id() >= other.as_id()
    }
  }
}
