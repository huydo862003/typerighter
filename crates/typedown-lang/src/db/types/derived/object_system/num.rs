use typedown_macros::query_derived;

use super::base::{TdRuntimeObject, TdStaticType, TdTypeType};
use super::{TdObjectEnum, TdTypeEnum};
use crate::db::TypedownDatabase;
use crate::db::derived::get_builtin_types::get_num_type;
use crate::db::types::Project;
use typedown_incremental::Id;

#[query_derived]
pub struct TdNumType<'db> {}

impl<'db> TdRuntimeObject<'db> for TdNumType<'db> {
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
    "@builtin::number".to_string()
  }
}

impl<'db> TdStaticType<'db> for TdNumType<'db> {
  fn display_name(&self, _db: &'db TypedownDatabase) -> String {
    "number".to_string()
  }
  fn runtime_type(&self, _db: &'db TypedownDatabase) -> Option<TdTypeEnum<'db>> {
    Some((*self).into())
  }
  fn construct(
    &self,
    _db: &'db TypedownDatabase,
    _project: Project,
    args: Vec<TdObjectEnum<'db>>,
  ) -> Option<TdObjectEnum<'db>> {
    let arg = args.into_iter().next()?;
    arg.as_td_num_obj()?;
    Some(arg)
  }
}

impl<'db> TdNumType<'db> {
  pub fn get(db: &'db TypedownDatabase) -> TdNumType<'db> {
    get_num_type(db)
  }
}

#[query_derived]
pub struct TdNumObj<'db> {
  pub value: f64,
}

impl<'db> TdRuntimeObject<'db> for TdNumObj<'db> {
  fn get_type(&self, db: &'db TypedownDatabase) -> TdTypeEnum<'db> {
    TdNumType::get(db).into()
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
    self.value(db).to_string()
  }
  fn eq(&self, db: &'db TypedownDatabase, other: &TdObjectEnum<'db>) -> bool {
    if let TdObjectEnum::TdNumObj(other) = other {
      self.value(db) == other.value(db)
    } else {
      self.as_id() == other.as_id()
    }
  }
  fn lt(&self, db: &'db TypedownDatabase, other: &TdObjectEnum<'db>) -> bool {
    if let TdObjectEnum::TdNumObj(other) = other {
      self.value(db) < other.value(db)
    } else {
      self.as_id() < other.as_id()
    }
  }
  fn gt(&self, db: &'db TypedownDatabase, other: &TdObjectEnum<'db>) -> bool {
    if let TdObjectEnum::TdNumObj(other) = other {
      self.value(db) > other.value(db)
    } else {
      self.as_id() > other.as_id()
    }
  }
  fn le(&self, db: &'db TypedownDatabase, other: &TdObjectEnum<'db>) -> bool {
    if let TdObjectEnum::TdNumObj(other) = other {
      self.value(db) <= other.value(db)
    } else {
      self.as_id() <= other.as_id()
    }
  }
  fn ge(&self, db: &'db TypedownDatabase, other: &TdObjectEnum<'db>) -> bool {
    if let TdObjectEnum::TdNumObj(other) = other {
      self.value(db) >= other.value(db)
    } else {
      self.as_id() >= other.as_id()
    }
  }
}
