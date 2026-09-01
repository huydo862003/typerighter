use typedown_macros::query_derived;

use super::base::{TdRuntimeObject, TdStaticType, TdTypeType};
use super::{TdObjectEnum, TdTypeEnum};
use crate::db::TypedownDatabase;
use crate::db::derived::get_builtin_types::{get_bool_type, get_false, get_true};
use crate::db::types::Project;
use typedown_incremental::Id;

#[query_derived]
pub struct TdBoolType<'db> {}

impl<'db> TdRuntimeObject<'db> for TdBoolType<'db> {
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
    "@builtin::boolean".to_string()
  }
}

impl<'db> TdStaticType<'db> for TdBoolType<'db> {
  fn display_name(&self, _db: &'db TypedownDatabase) -> String {
    "boolean".to_string()
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
    arg.as_td_bool_obj()?;
    Some(arg)
  }
}

impl<'db> TdBoolType<'db> {
  pub fn get(db: &'db TypedownDatabase) -> TdBoolType<'db> {
    get_bool_type(db)
  }
}

#[query_derived]
pub struct TdBoolObj<'db> {
  pub value: bool,
}

impl<'db> TdRuntimeObject<'db> for TdBoolObj<'db> {
  fn get_type(&self, db: &'db TypedownDatabase) -> TdTypeEnum<'db> {
    TdBoolType::get(db).into()
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
    if let TdObjectEnum::TdBoolObj(other) = other {
      self.value(db) == other.value(db)
    } else {
      self.as_id() == other.as_id()
    }
  }
  fn lt(&self, db: &'db TypedownDatabase, other: &TdObjectEnum<'db>) -> bool {
    if let TdObjectEnum::TdBoolObj(other) = other {
      !self.value(db) && other.value(db)
    } else {
      self.as_id() < other.as_id()
    }
  }
  fn gt(&self, db: &'db TypedownDatabase, other: &TdObjectEnum<'db>) -> bool {
    if let TdObjectEnum::TdBoolObj(other) = other {
      self.value(db) && !other.value(db)
    } else {
      self.as_id() > other.as_id()
    }
  }
  fn le(&self, db: &'db TypedownDatabase, other: &TdObjectEnum<'db>) -> bool {
    if let TdObjectEnum::TdBoolObj(other) = other {
      !self.value(db) || other.value(db)
    } else {
      self.as_id() <= other.as_id()
    }
  }
  fn ge(&self, db: &'db TypedownDatabase, other: &TdObjectEnum<'db>) -> bool {
    if let TdObjectEnum::TdBoolObj(other) = other {
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
