use typedown_macros::query_derived;

use super::base::{TdRuntimeObject, TdStaticType, TdTypeType};
use super::{TdObjectEnum, TdTypeEnum};
use crate::db::TypedownDatabase;
use crate::db::derived::get_builtin_types::{get_bool_type, get_false, get_true};
use crate::db::types::Project;
use typedown_incremental::Id;

#[query_derived]
pub struct TdBoolType<'db> {}

impl TdRuntimeObject for TdBoolType<'_> {
  fn get_type(&self, db: &TypedownDatabase) -> TdTypeEnum {
    TdTypeType::get(db).into()
  }
  fn get_owned_field(&self, _db: &TypedownDatabase, _key: &str) -> Option<TdObjectEnum> {
    None
  }
  fn source_path(&self, _db: &TypedownDatabase) -> String {
    "@builtin::boolean".to_string()
  }
}

impl TdStaticType for TdBoolType<'_> {
  fn display_name(&self, _db: &TypedownDatabase) -> String {
    "boolean".to_string()
  }
  fn runtime_type(&self, _db: &TypedownDatabase) -> Option<TdTypeEnum> {
    Some((*self).into())
  }
  fn construct(
    &self,
    _db: &TypedownDatabase,
    _project: Project,
    args: Vec<TdObjectEnum>,
  ) -> Option<TdObjectEnum> {
    let arg = args.into_iter().next()?;
    arg.as_td_bool_obj()?;
    Some(arg)
  }
}

impl TdBoolType<'_> {
  pub fn get(db: &TypedownDatabase) -> TdBoolType {
    get_bool_type(db)
  }
}

#[query_derived]
pub struct TdBoolObj<'db> {
  pub value: bool,
}

impl TdRuntimeObject for TdBoolObj<'_> {
  fn get_type(&self, db: &TypedownDatabase) -> TdTypeEnum {
    TdBoolType::get(db).into()
  }
  fn get_owned_field(&self, _db: &TypedownDatabase, _key: &str) -> Option<TdObjectEnum> {
    None
  }
  fn source_path(&self, db: &TypedownDatabase) -> String {
    self.get_type(db).source_path(db)
  }
  fn to_display_string(&self, db: &TypedownDatabase) -> String {
    self.value(db).to_string()
  }
  fn eq(&self, db: &TypedownDatabase, other: &TdObjectEnum) -> bool {
    if let TdObjectEnum::TdBoolObj(other) = other {
      self.value(db) == other.value(db)
    } else {
      self.as_id() == other.as_id()
    }
  }
  fn lt(&self, db: &TypedownDatabase, other: &TdObjectEnum) -> bool {
    if let TdObjectEnum::TdBoolObj(other) = other {
      !self.value(db) && other.value(db)
    } else {
      self.as_id() < other.as_id()
    }
  }
  fn gt(&self, db: &TypedownDatabase, other: &TdObjectEnum) -> bool {
    if let TdObjectEnum::TdBoolObj(other) = other {
      self.value(db) && !other.value(db)
    } else {
      self.as_id() > other.as_id()
    }
  }
  fn le(&self, db: &TypedownDatabase, other: &TdObjectEnum) -> bool {
    if let TdObjectEnum::TdBoolObj(other) = other {
      !self.value(db) || other.value(db)
    } else {
      self.as_id() <= other.as_id()
    }
  }
  fn ge(&self, db: &TypedownDatabase, other: &TdObjectEnum) -> bool {
    if let TdObjectEnum::TdBoolObj(other) = other {
      self.value(db) || !other.value(db)
    } else {
      self.as_id() >= other.as_id()
    }
  }
}

impl TdBoolObj<'_> {
  pub fn get_true(db: &TypedownDatabase) -> TdBoolObj {
    get_true(db)
  }

  pub fn get_false(db: &TypedownDatabase) -> TdBoolObj {
    get_false(db)
  }
}
