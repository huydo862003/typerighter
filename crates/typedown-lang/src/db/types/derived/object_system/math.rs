use typedown_macros::query_derived;

use super::base::{TdRuntimeObject, TdStaticType, TdTypeType};
use super::{TdObjectEnum, TdTypeEnum};
use crate::db::TypedownDatabase;
use crate::db::derived::get_builtin_types::get_math_type;
use crate::db::types::Project;

#[query_derived]
pub struct TdMathType {}

impl TdRuntimeObject for TdMathType<'_> {
  fn get_type(&self, db: &TypedownDatabase) -> TdTypeEnum {
    TdTypeType::get(db).into()
  }
  fn get_owned_field(&self, _db: &TypedownDatabase, _key: &str) -> Option<TdObjectEnum> {
    None
  }
  fn source_path(&self, _db: &TypedownDatabase) -> String {
    "@builtin::math".to_string()
  }
}

impl TdStaticType for TdMathType<'_> {
  fn display_name(&self, _db: &TypedownDatabase) -> String {
    "math".to_string()
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
    arg.as_td_math_obj()?;
    Some(arg)
  }
}

impl TdMathType<'_> {
  pub fn get(db: &TypedownDatabase) -> TdMathType {
    get_math_type(db)
  }
}

#[query_derived]
pub struct TdMathObj {
  pub value: String,
}

impl TdRuntimeObject for TdMathObj<'_> {
  fn get_type(&self, db: &TypedownDatabase) -> TdTypeEnum {
    TdMathType::get(db).into()
  }
  fn get_owned_field(&self, _db: &TypedownDatabase, _key: &str) -> Option<TdObjectEnum> {
    None
  }
  fn source_path(&self, db: &TypedownDatabase) -> String {
    self.get_type(db).source_path(db)
  }
  fn to_display_string(&self, db: &TypedownDatabase) -> String {
    format!("${}$", self.value(db))
  }
}
