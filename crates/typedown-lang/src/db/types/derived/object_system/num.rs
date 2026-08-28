use typedown_macros::query_derived;

use super::base::{TdRuntimeObject, TdStaticType, TdTypeType};
use super::{TdObjectEnum, TdTypeEnum};
use crate::db::TypedownDatabase;
use crate::db::derived::get_builtin_types::get_num_type;
use crate::db::types::Project;
use typedown_incremental::Id;

#[query_derived]
pub struct TdNumType<'db> {}

impl<'db> TdRuntimeObject for TdNumType<'db> {
  fn get_type(&self, db: &TypedownDatabase) -> TdTypeEnum {
    TdTypeType::get(db).into()
  }
  fn get_owned_field(&self, _db: &TypedownDatabase, _key: &str) -> Option<TdObjectEnum> {
    None
  }
  fn source_path(&self, _db: &TypedownDatabase) -> String {
    "@builtin::number".to_string()
  }
}

impl<'db> TdStaticType<'db> for TdNumType<'db> {
  fn display_name(&self, _db: &TypedownDatabase) -> String {
    "number".to_string()
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
    arg.as_td_num_obj()?;
    Some(arg)
  }
}

impl<'db> TdNumType<'db> {
  pub fn get(db: &TypedownDatabase) -> TdNumType {
    get_num_type(db)
  }
}

#[query_derived]
pub struct TdNumObj<'db> {
  pub value: f64,
}

impl<'db> TdRuntimeObject for TdNumObj<'db> {
  fn get_type(&self, db: &TypedownDatabase) -> TdTypeEnum {
    TdNumType::get(db).into()
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
    if let TdObjectEnum::TdNumObj(other) = other {
      self.value(db) == other.value(db)
    } else {
      self.as_id() == other.as_id()
    }
  }
  fn lt(&self, db: &TypedownDatabase, other: &TdObjectEnum) -> bool {
    if let TdObjectEnum::TdNumObj(other) = other {
      self.value(db) < other.value(db)
    } else {
      self.as_id() < other.as_id()
    }
  }
  fn gt(&self, db: &TypedownDatabase, other: &TdObjectEnum) -> bool {
    if let TdObjectEnum::TdNumObj(other) = other {
      self.value(db) > other.value(db)
    } else {
      self.as_id() > other.as_id()
    }
  }
  fn le(&self, db: &TypedownDatabase, other: &TdObjectEnum) -> bool {
    if let TdObjectEnum::TdNumObj(other) = other {
      self.value(db) <= other.value(db)
    } else {
      self.as_id() <= other.as_id()
    }
  }
  fn ge(&self, db: &TypedownDatabase, other: &TdObjectEnum) -> bool {
    if let TdObjectEnum::TdNumObj(other) = other {
      self.value(db) >= other.value(db)
    } else {
      self.as_id() >= other.as_id()
    }
  }
}
