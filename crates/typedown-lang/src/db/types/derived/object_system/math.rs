use typedown_macros::query_derived;

use super::base::{TdRuntimeObject, TdStaticType, TdTypeType};
use super::{TdObjectEnum, TdTypeEnum};
use crate::db::TypedownDatabase;
use crate::db::derived::get_builtin_types::get_math_type;
use crate::db::types::Project;

#[query_derived]
pub struct TdMathType<'db> {}

impl<'db> TdRuntimeObject<'db> for TdMathType<'db> {
  fn get_type(&self, db: &'db TypedownDatabase) -> TdTypeEnum<'db> {
    TdTypeType::get(db).into()
  }
  fn get_owned_field(&self, _db: &'db TypedownDatabase, _key: &str) -> Option<TdObjectEnum<'db>> {
    None
  }
  fn source_path(&self, _db: &'db TypedownDatabase) -> String {
    "@builtin::math".to_string()
  }
}

impl<'db> TdStaticType<'db> for TdMathType<'db> {
  fn display_name(&self, _db: &'db TypedownDatabase) -> String {
    "math".to_string()
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
    arg.as_td_math_obj()?;
    Some(arg)
  }
}

impl<'db> TdMathType<'db> {
  pub fn get(db: &'db TypedownDatabase) -> TdMathType<'db> {
    get_math_type(db)
  }
}

#[query_derived]
pub struct TdMathObj<'db> {
  pub value: String,
}

impl<'db> TdRuntimeObject<'db> for TdMathObj<'db> {
  fn get_type(&self, db: &'db TypedownDatabase) -> TdTypeEnum<'db> {
    TdMathType::get(db).into()
  }
  fn get_owned_field(&self, _db: &'db TypedownDatabase, _key: &str) -> Option<TdObjectEnum<'db>> {
    None
  }
  fn source_path(&self, db: &'db TypedownDatabase) -> String {
    self.get_type(db).source_path(db)
  }
  fn to_display_string(&self, db: &'db TypedownDatabase) -> String {
    format!("${}$", self.value(db))
  }
}
