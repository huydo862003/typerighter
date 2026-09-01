use typedown_macros::query_derived;

use super::base::TdRuntimeObject;
use super::{TdObjectEnum, TdTypeEnum};
use crate::db::TypedownDatabase;
use crate::db::derived::vault::get_vault_type;
use crate::db::types::Project;

#[query_derived]
pub struct TdVaultObj<'db> {
  pub project: Project,
}

impl<'db> TdRuntimeObject<'db> for TdVaultObj<'db> {
  fn get_type(&self, db: &'db TypedownDatabase) -> TdTypeEnum<'db> {
    get_vault_type(db).into()
  }
  fn get_owned_field(&self, _db: &'db TypedownDatabase, _key: &str) -> Option<TdObjectEnum<'db>> {
    None
  }
  fn get_builtin_field(&self, _db: &'db TypedownDatabase, _key: &str) -> Option<TdObjectEnum<'db>> {
    None
  }
  fn source_path(&self, _db: &'db TypedownDatabase) -> String {
    "@builtin::vault".to_string()
  }
}
