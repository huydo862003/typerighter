use typedown_macros::query_derived;

use super::base::{TdRuntimeObject, TdStaticType, TdTypeType};
use super::{TdObjectEnum, TdTypeEnum};
use crate::db::TypedownDatabase;
use crate::db::derived::get_builtin_types::{get_bool_type, get_num_type, get_str_type};
use crate::db::types::LiteralValue;

#[query_derived]
pub struct TdLiteralType {
  pub value: LiteralValue,
}

impl TdRuntimeObject for TdLiteralType {
  fn get_type(&self, db: &TypedownDatabase) -> TdTypeEnum {
    TdTypeType::get(db).into()
  }
  fn get_owned_field(&self, _db: &TypedownDatabase, _key: &str) -> Option<TdObjectEnum> {
    None
  }
  fn source_path(&self, db: &TypedownDatabase) -> String {
    self.display_name(db)
  }
}

impl TdStaticType for TdLiteralType {
  fn display_name(&self, db: &TypedownDatabase) -> String {
    match self.value(db) {
      LiteralValue::Str(s) => format!("\"{}\"", s),
      LiteralValue::Num(n) => n,
      LiteralValue::Bool(b) => b.to_string(),
    }
  }

  fn runtime_type(&self, db: &TypedownDatabase) -> Option<TdTypeEnum> {
    Some(self.underlying_type(db))
  }
}

impl TdLiteralType {
  pub fn underlying_type(&self, db: &TypedownDatabase) -> TdTypeEnum {
    match self.value(db) {
      LiteralValue::Str(_) => get_str_type(db).into(),
      LiteralValue::Num(_) => get_num_type(db).into(),
      LiteralValue::Bool(_) => get_bool_type(db).into(),
    }
  }
}
