use typedown_macros::query_derived;

use super::base::{TdRuntimeObject, TdStaticType, TdTypeType};
use super::{TdObjectEnum, TdTypeEnum};
use crate::db::TypedownDatabase;
use crate::db::derived::get_builtin_types::get_never_type;

#[query_derived]
pub struct TdNeverType {}

impl TdRuntimeObject for TdNeverType {
  fn get_type(&self, db: &TypedownDatabase) -> TdTypeEnum {
    TdTypeType::get(db).into()
  }
  fn get_owned_field(&self, _db: &TypedownDatabase, _key: &str) -> Option<TdObjectEnum> {
    None
  }
  fn source_path(&self, _db: &TypedownDatabase) -> String {
    "@builtin::never".to_string()
  }
}

impl TdStaticType for TdNeverType {
  fn display_name(&self, _db: &TypedownDatabase) -> String {
    "never".to_string()
  }
}

impl TdNeverType {
  pub fn get(db: &TypedownDatabase) -> TdNeverType {
    get_never_type(db)
  }
}
