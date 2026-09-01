use typedown_macros::query_derived;

use super::base::{TdRuntimeObject, TdStaticType, TdTypeType};
use super::{TdObjectEnum, TdTypeEnum};
use crate::db::TypedownDatabase;
use crate::db::derived::get_builtin_types::{get_num_type, get_str_type};
use crate::db::types::FuncSignature;
use crate::db::types::Project;
use typedown_incremental::Id;

#[query_derived]
pub struct TdStrType<'db> {}

impl<'db> TdRuntimeObject<'db> for TdStrType<'db> {
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
    "@builtin::string".to_string()
  }
}

impl<'db> TdStaticType<'db> for TdStrType<'db> {
  fn display_name(&self, _db: &'db TypedownDatabase) -> String {
    "string".to_string()
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
    arg.as_td_str_obj()?;
    Some(arg)
  }
  fn index_type(
    &self,
    db: &'db TypedownDatabase,
    _key_type: &TdTypeEnum<'db>,
  ) -> Option<FuncSignature<'db>> {
    let key_type: TdTypeEnum = get_num_type(db).into();
    Some(FuncSignature::new(db, vec![key_type], (*self).into()))
  }
}

impl<'db> TdStrType<'db> {
  pub fn get(db: &'db TypedownDatabase) -> TdStrType<'db> {
    get_str_type(db)
  }
}

#[query_derived]
pub struct TdStrObj<'db> {
  pub value: String,
}

impl<'db> TdRuntimeObject<'db> for TdStrObj<'db> {
  fn get_type(&self, db: &'db TypedownDatabase) -> TdTypeEnum<'db> {
    TdStrType::get(db).into()
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
    self.value(db)
  }
  fn index(&self, db: &'db TypedownDatabase, key: &TdObjectEnum<'db>) -> Option<TdObjectEnum<'db>> {
    let num = key.as_td_num_obj()?;
    let idx = num.value(db) as usize;
    let ch = self.value(db).chars().nth(idx)?;
    Some(TdStrObj::new(db, ch.to_string()).into())
  }
  fn len(&self, db: &'db TypedownDatabase) -> Option<usize> {
    Some(self.value(db).chars().count())
  }
  fn eq(&self, db: &'db TypedownDatabase, other: &TdObjectEnum<'db>) -> bool {
    if let TdObjectEnum::TdStrObj(other) = other {
      self.value(db) == other.value(db)
    } else {
      self.as_id() == other.as_id()
    }
  }
  fn lt(&self, db: &'db TypedownDatabase, other: &TdObjectEnum<'db>) -> bool {
    if let TdObjectEnum::TdStrObj(other) = other {
      self.value(db) < other.value(db)
    } else {
      self.as_id() < other.as_id()
    }
  }
  fn gt(&self, db: &'db TypedownDatabase, other: &TdObjectEnum<'db>) -> bool {
    if let TdObjectEnum::TdStrObj(other) = other {
      self.value(db) > other.value(db)
    } else {
      self.as_id() > other.as_id()
    }
  }
  fn le(&self, db: &'db TypedownDatabase, other: &TdObjectEnum<'db>) -> bool {
    if let TdObjectEnum::TdStrObj(other) = other {
      self.value(db) <= other.value(db)
    } else {
      self.as_id() <= other.as_id()
    }
  }
  fn ge(&self, db: &'db TypedownDatabase, other: &TdObjectEnum<'db>) -> bool {
    if let TdObjectEnum::TdStrObj(other) = other {
      self.value(db) >= other.value(db)
    } else {
      self.as_id() >= other.as_id()
    }
  }
}
