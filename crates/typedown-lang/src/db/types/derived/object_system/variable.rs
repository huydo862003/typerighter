use typedown_macros::query_derived;

use super::base::{TdRuntimeObject, TdStaticType, TdTypeType};
use super::{TdObjectEnum, TdTypeEnum};
use crate::db::TypedownDatabase;
use crate::db::types::TypeVariable;

/// A type variable reference within a type expression
#[query_derived]
pub struct TdVariableType {
  pub index: usize,
  pub variable: TypeVariable,
}

impl TdStaticType for TdVariableType {
  fn display_name(&self, db: &TypedownDatabase) -> String {
    let var = self.variable(db);
    if let Some(val) = var.value(db).as_ref().and_then(|l| l.resolve(db)) {
      val.display_name(db)
    } else if let Some(b) = var.bound(db).as_ref().and_then(|l| l.resolve(db)) {
      format!("T{} <: {}", self.index(db), b.display_name(db))
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
