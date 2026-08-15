use typedown_macros::query_derived;

use super::base::TdObjectLike;
use super::{TdObjectEnum, TdTypeEnum};
use crate::db::TypedownDatabase;
use crate::db::derived::vault::get_vault_type;
use crate::db::types::Project;

#[query_derived]
pub struct TdVaultObj {
  pub project: Project,
}

impl TdObjectLike for TdVaultObj {
  fn get_type(&self, db: &TypedownDatabase) -> TdTypeEnum {
    get_vault_type(db).into()
  }
  fn get_owned_field(&self, _db: &TypedownDatabase, _key: &str) -> Option<TdObjectEnum> {
    None
  }
  fn source_path(&self, _db: &TypedownDatabase) -> String {
    "@builtin::vault".to_string()
  }
}
