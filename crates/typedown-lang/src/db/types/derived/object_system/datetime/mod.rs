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

impl<'db> TdRuntimeObject for TdDateTimeType<'db> {
  fn get_type(&self, db: &TypedownDatabase) -> TdTypeEnum {
    TdTypeType::get(db).into()
  }
  fn get_owned_field(&self, _db: &TypedownDatabase, _key: &str) -> Option<TdObjectEnum> {
    None
  }
  fn source_path(&self, _db: &TypedownDatabase) -> String {
    "@builtin::datetime".to_string()
  }
}

impl<'db> TdStaticType<'db> for TdDateTimeType {
  fn display_name(&self, _db: &TypedownDatabase) -> String {
    "datetime".to_string()
  }
  fn runtime_type(&self, _db: &TypedownDatabase) -> Option<TdTypeEnum> {
    Some((*self).into())
  }
  fn construct(
    &self,
    db: &TypedownDatabase,
    _project: Project,
    args: Vec<TdObjectEnum>,
  ) -> Option<TdObjectEnum> {
    let arg = args.into_iter().next()?;
    let str_obj = arg.as_td_str_obj()?;
    let val = str_obj.value(db);
    if is_valid_iso_datetime(&val) {
      return Some(TdDateTimeObj::new(db, val).into());
    }
    None
  }
}

impl TdDateTimeType {
  pub fn get(db: &TypedownDatabase) -> TdDateTimeType {
    get_datetime_type(db)
  }
}

#[query_derived]
pub struct TdDateTimeObj<'db> {
  pub value: String,
}

impl<'db> TdRuntimeObject for TdDateTimeObj<'db> {
  fn get_type(&self, db: &TypedownDatabase) -> TdTypeEnum {
    TdDateTimeType::get(db).into()
  }
  fn get_owned_field(&self, _db: &TypedownDatabase, _key: &str) -> Option<TdObjectEnum> {
    None
  }
  fn source_path(&self, db: &TypedownDatabase) -> String {
    self.get_type(db).source_path(db)
  }
  fn to_display_string(&self, db: &TypedownDatabase) -> String {
    self.value(db)
  }
  fn eq(&self, db: &TypedownDatabase, other: &TdObjectEnum) -> bool {
    if let TdObjectEnum::TdDateTimeObj(other) = other {
      self.value(db) == other.value(db)
    } else {
      self.as_id() == other.as_id()
    }
  }
  fn lt(&self, db: &TypedownDatabase, other: &TdObjectEnum) -> bool {
    if let TdObjectEnum::TdDateTimeObj(other) = other {
      self.value(db) < other.value(db)
    } else {
      self.as_id() < other.as_id()
    }
  }
  fn gt(&self, db: &TypedownDatabase, other: &TdObjectEnum) -> bool {
    if let TdObjectEnum::TdDateTimeObj(other) = other {
      self.value(db) > other.value(db)
    } else {
      self.as_id() > other.as_id()
    }
  }
  fn le(&self, db: &TypedownDatabase, other: &TdObjectEnum) -> bool {
    if let TdObjectEnum::TdDateTimeObj(other) = other {
      self.value(db) <= other.value(db)
    } else {
      self.as_id() <= other.as_id()
    }
  }
  fn ge(&self, db: &TypedownDatabase, other: &TdObjectEnum) -> bool {
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

impl<'db> TdRuntimeObject for TdDateType<'db> {
  fn get_type(&self, db: &TypedownDatabase) -> TdTypeEnum {
    TdTypeType::get(db).into()
  }
  fn get_owned_field(&self, _db: &TypedownDatabase, _key: &str) -> Option<TdObjectEnum> {
    None
  }
  fn source_path(&self, _db: &TypedownDatabase) -> String {
    "@builtin::date".to_string()
  }
}

impl<'db> TdStaticType<'db> for TdDateType {
  fn display_name(&self, _db: &TypedownDatabase) -> String {
    "date".to_string()
  }
  fn runtime_type(&self, _db: &TypedownDatabase) -> Option<TdTypeEnum> {
    Some((*self).into())
  }
  fn construct(
    &self,
    db: &TypedownDatabase,
    _project: Project,
    args: Vec<TdObjectEnum>,
  ) -> Option<TdObjectEnum> {
    let arg = args.into_iter().next()?;
    let str_obj = arg.as_td_str_obj()?;
    let val = str_obj.value(db);
    if is_valid_iso_date(&val) {
      return Some(TdDateObj::new(db, val).into());
    }
    None
  }
}

impl TdDateType {
  pub fn get(db: &TypedownDatabase) -> TdDateType {
    get_date_type(db)
  }
}

#[query_derived]
pub struct TdDateObj<'db> {
  pub value: String,
}

impl<'db> TdRuntimeObject for TdDateObj<'db> {
  fn get_type(&self, db: &TypedownDatabase) -> TdTypeEnum {
    TdDateType::get(db).into()
  }
  fn get_owned_field(&self, _db: &TypedownDatabase, _key: &str) -> Option<TdObjectEnum> {
    None
  }
  fn source_path(&self, db: &TypedownDatabase) -> String {
    self.get_type(db).source_path(db)
  }
  fn to_display_string(&self, db: &TypedownDatabase) -> String {
    self.value(db)
  }
  fn eq(&self, db: &TypedownDatabase, other: &TdObjectEnum) -> bool {
    if let TdObjectEnum::TdDateObj(other) = other {
      self.value(db) == other.value(db)
    } else {
      self.as_id() == other.as_id()
    }
  }
  fn lt(&self, db: &TypedownDatabase, other: &TdObjectEnum) -> bool {
    if let TdObjectEnum::TdDateObj(other) = other {
      self.value(db) < other.value(db)
    } else {
      self.as_id() < other.as_id()
    }
  }
  fn gt(&self, db: &TypedownDatabase, other: &TdObjectEnum) -> bool {
    if let TdObjectEnum::TdDateObj(other) = other {
      self.value(db) > other.value(db)
    } else {
      self.as_id() > other.as_id()
    }
  }
  fn le(&self, db: &TypedownDatabase, other: &TdObjectEnum) -> bool {
    if let TdObjectEnum::TdDateObj(other) = other {
      self.value(db) <= other.value(db)
    } else {
      self.as_id() <= other.as_id()
    }
  }
  fn ge(&self, db: &TypedownDatabase, other: &TdObjectEnum) -> bool {
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

impl<'db> TdRuntimeObject for TdTimeType<'db> {
  fn get_type(&self, db: &TypedownDatabase) -> TdTypeEnum {
    TdTypeType::get(db).into()
  }
  fn get_owned_field(&self, _db: &TypedownDatabase, _key: &str) -> Option<TdObjectEnum> {
    None
  }
  fn source_path(&self, _db: &TypedownDatabase) -> String {
    "@builtin::time".to_string()
  }
}

impl<'db> TdStaticType<'db> for TdTimeType {
  fn display_name(&self, _db: &TypedownDatabase) -> String {
    "time".to_string()
  }
  fn runtime_type(&self, _db: &TypedownDatabase) -> Option<TdTypeEnum> {
    Some((*self).into())
  }
  fn construct(
    &self,
    db: &TypedownDatabase,
    _project: Project,
    args: Vec<TdObjectEnum>,
  ) -> Option<TdObjectEnum> {
    let arg = args.into_iter().next()?;
    let str_obj = arg.as_td_str_obj()?;
    let val = str_obj.value(db);
    if is_valid_iso_time(&val) {
      return Some(TdTimeObj::new(db, val).into());
    }
    None
  }
}

impl TdTimeType {
  pub fn get(db: &TypedownDatabase) -> TdTimeType {
    get_time_type(db)
  }
}

#[query_derived]
pub struct TdTimeObj<'db> {
  pub value: String,
}

impl<'db> TdRuntimeObject for TdTimeObj<'db> {
  fn get_type(&self, db: &TypedownDatabase) -> TdTypeEnum {
    TdTimeType::get(db).into()
  }
  fn get_owned_field(&self, _db: &TypedownDatabase, _key: &str) -> Option<TdObjectEnum> {
    None
  }
  fn source_path(&self, db: &TypedownDatabase) -> String {
    self.get_type(db).source_path(db)
  }
  fn to_display_string(&self, db: &TypedownDatabase) -> String {
    self.value(db)
  }
  fn eq(&self, db: &TypedownDatabase, other: &TdObjectEnum) -> bool {
    if let TdObjectEnum::TdTimeObj(other) = other {
      self.value(db) == other.value(db)
    } else {
      self.as_id() == other.as_id()
    }
  }
  fn lt(&self, db: &TypedownDatabase, other: &TdObjectEnum) -> bool {
    if let TdObjectEnum::TdTimeObj(other) = other {
      self.value(db) < other.value(db)
    } else {
      self.as_id() < other.as_id()
    }
  }
  fn gt(&self, db: &TypedownDatabase, other: &TdObjectEnum) -> bool {
    if let TdObjectEnum::TdTimeObj(other) = other {
      self.value(db) > other.value(db)
    } else {
      self.as_id() > other.as_id()
    }
  }
  fn le(&self, db: &TypedownDatabase, other: &TdObjectEnum) -> bool {
    if let TdObjectEnum::TdTimeObj(other) = other {
      self.value(db) <= other.value(db)
    } else {
      self.as_id() <= other.as_id()
    }
  }
  fn ge(&self, db: &TypedownDatabase, other: &TdObjectEnum) -> bool {
    if let TdObjectEnum::TdTimeObj(other) = other {
      self.value(db) >= other.value(db)
    } else {
      self.as_id() >= other.as_id()
    }
  }
}
