use typedown_macros::query_derived;

use super::base::{TdRuntimeObj, TdStaticTyp, TdTypTyp};
use super::{TdObjEnum, TdTypEnum};
use crate::db::TypedownDatabase;
use crate::db::derived::get_builtin_typs::{get_number_typ, get_string_typ};
use crate::db::types::FuncSignature;
use crate::db::types::Project;
use typedown_incremental::Id;

#[query_derived]
pub struct TdStringTyp<'db> {}

impl<'db> TdRuntimeObj<'db> for TdStringTyp<'db> {
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
    "@builtin::string".to_string()
  }
}

impl<'db> TdStaticTyp<'db> for TdStringTyp<'db> {
  fn display_name(&self, _db: &'db TypedownDatabase) -> String {
    "string".to_string()
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
    arg.as_td_string_obj()?;
    Some(arg)
  }
  fn index_typ(
    &self,
    db: &'db TypedownDatabase,
    _key_typ: &TdTypEnum<'db>,
  ) -> Option<FuncSignature<'db>> {
    let key_typ: TdTypEnum = get_number_typ(db).into();
    Some(FuncSignature::new(db, vec![key_typ], (*self).into()))
  }
}

impl<'db> TdStringTyp<'db> {
  pub fn get(db: &'db TypedownDatabase) -> TdStringTyp<'db> {
    get_string_typ(db)
  }
}

#[query_derived]
pub struct TdStringObj<'db> {
  pub value: String,
}

impl<'db> TdRuntimeObj<'db> for TdStringObj<'db> {
  fn get_typ(&self, db: &'db TypedownDatabase) -> TdTypEnum<'db> {
    TdStringTyp::get(db).into()
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
  fn index(&self, db: &'db TypedownDatabase, key: &TdObjEnum<'db>) -> Option<TdObjEnum<'db>> {
    let number = key.as_td_number_obj()?;
    let index = number.value(db) as usize;
    let char = self.value(db).chars().nth(index)?;
    Some(TdStringObj::new(db, char.to_string()).into())
  }
  fn len(&self, db: &'db TypedownDatabase) -> Option<usize> {
    Some(self.value(db).chars().count())
  }
  fn eq(&self, db: &'db TypedownDatabase, other: &TdObjEnum<'db>) -> bool {
    if let TdObjEnum::TdStringObj(other) = other {
      self.value(db) == other.value(db)
    } else {
      self.as_id() == other.as_id()
    }
  }
  fn lt(&self, db: &'db TypedownDatabase, other: &TdObjEnum<'db>) -> bool {
    if let TdObjEnum::TdStringObj(other) = other {
      self.value(db) < other.value(db)
    } else {
      self.as_id() < other.as_id()
    }
  }
  fn gt(&self, db: &'db TypedownDatabase, other: &TdObjEnum<'db>) -> bool {
    if let TdObjEnum::TdStringObj(other) = other {
      self.value(db) > other.value(db)
    } else {
      self.as_id() > other.as_id()
    }
  }
  fn le(&self, db: &'db TypedownDatabase, other: &TdObjEnum<'db>) -> bool {
    if let TdObjEnum::TdStringObj(other) = other {
      self.value(db) <= other.value(db)
    } else {
      self.as_id() <= other.as_id()
    }
  }
  fn ge(&self, db: &'db TypedownDatabase, other: &TdObjEnum<'db>) -> bool {
    if let TdObjEnum::TdStringObj(other) = other {
      self.value(db) >= other.value(db)
    } else {
      self.as_id() >= other.as_id()
    }
  }
}
