use typedown_macros::query_derived;

use super::base::{TdRuntimeObject, TdStaticType, TdTypeType};
use super::{TdObjectEnum, TdTypeEnum};
use crate::db::TypedownDatabase;
use crate::db::types::TypeVariable;

/// A type variable reference within a type expression
#[query_derived]
pub struct TdVariableType {
  #[id]
  pub index: usize,
  #[id]
  pub variable: TypeVariable,
}

impl TdStaticType for TdVariableType {
  fn display_name(&self, db: &TypedownDatabase) -> String {
    let var = self.variable(db);
    if let Some(b) = var.upper_bound(db).resolve(db) {
      if b.as_td_object_type().is_some() {
        format!("T{}", self.index(db))
      } else {
        format!("T{} <: {}", self.index(db), b.display_name(db))
      }
    } else {
      format!("T{}", self.index(db))
    }
  }
}

impl TdRuntimeObject for TdVariableType {
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
