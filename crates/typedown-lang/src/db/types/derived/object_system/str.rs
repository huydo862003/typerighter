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

impl<'db> TdRuntimeObject for TdStrType<'db> {
  fn get_type(&self, db: &TypedownDatabase) -> TdTypeEnum {
    TdTypeType::get(db).into()
  }
  fn get_owned_field(&self, _db: &TypedownDatabase, _key: &str) -> Option<TdObjectEnum> {
    None
  }
  fn source_path(&self, _db: &TypedownDatabase) -> String {
    "@builtin::string".to_string()
  }
}

impl<'db> TdStaticType<'db> for TdStrType<'db> {
  fn display_name(&self, _db: &TypedownDatabase) -> String {
    "string".to_string()
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
    arg.as_td_str_obj()?;
    Some(arg)
  }
  fn index_type(&self, db: &TypedownDatabase, _key_type: &TdTypeEnum) -> Option<FuncSignature> {
    let key_type: TdTypeEnum = get_num_type(db).into();
    Some(FuncSignature::new(db, vec![key_type], (*self).into()))
  }
}

impl<'db> TdStrType<'db> {
  pub fn get(db: &TypedownDatabase) -> TdStrType {
    get_str_type(db)
  }
}

#[query_derived]
pub struct TdStrObj<'db> {
  pub value: String,
}

impl<'db> TdRuntimeObject for TdStrObj<'db> {
  fn get_type(&self, db: &TypedownDatabase) -> TdTypeEnum {
    TdStrType::get(db).into()
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
  fn index(&self, db: &TypedownDatabase, key: &TdObjectEnum) -> Option<TdObjectEnum> {
    let num = key.as_td_num_obj()?;
    let idx = num.value(db) as usize;
    let ch = self.value(db).chars().nth(idx)?;
    Some(TdStrObj::new(db, ch.to_string()).into())
  }
  fn len(&self, db: &TypedownDatabase) -> Option<usize> {
    Some(self.value(db).chars().count())
  }
  fn eq(&self, db: &TypedownDatabase, other: &TdObjectEnum) -> bool {
    if let TdObjectEnum::TdStrObj(other) = other {
      self.value(db) == other.value(db)
    } else {
      self.as_id() == other.as_id()
    }
  }
  fn lt(&self, db: &TypedownDatabase, other: &TdObjectEnum) -> bool {
    if let TdObjectEnum::TdStrObj(other) = other {
      self.value(db) < other.value(db)
    } else {
      self.as_id() < other.as_id()
    }
  }
  fn gt(&self, db: &TypedownDatabase, other: &TdObjectEnum) -> bool {
    if let TdObjectEnum::TdStrObj(other) = other {
      self.value(db) > other.value(db)
    } else {
      self.as_id() > other.as_id()
    }
  }
  fn le(&self, db: &TypedownDatabase, other: &TdObjectEnum) -> bool {
    if let TdObjectEnum::TdStrObj(other) = other {
      self.value(db) <= other.value(db)
    } else {
      self.as_id() <= other.as_id()
    }
  }
  fn ge(&self, db: &TypedownDatabase, other: &TdObjectEnum) -> bool {
    if let TdObjectEnum::TdStrObj(other) = other {
      self.value(db) >= other.value(db)
    } else {
      self.as_id() >= other.as_id()
    }
  }
}
