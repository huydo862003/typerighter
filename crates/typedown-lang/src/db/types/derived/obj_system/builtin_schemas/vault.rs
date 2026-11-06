use typedown_macros::query_derived;

use crate::db::TypedownDatabase;
use crate::db::derived::builtin_schemas::vault::get_vault_typ;
use crate::db::types::Project;
use crate::db::types::{TdObjEnum, TdRuntimeObj, TdTypEnum};

#[query_derived]
pub struct TdVaultObj<'db> {
  pub project: Project,
}

impl<'db> TdRuntimeObj<'db> for TdVaultObj<'db> {
  fn get_typ(&self, db: &'db TypedownDatabase) -> TdTypEnum<'db> {
    get_vault_typ(db).into()
  }
  fn get_owned_field(&self, _db: &'db TypedownDatabase, _key: &str) -> Option<TdObjEnum<'db>> {
    None
  }
  fn get_builtin_field(&self, _db: &'db TypedownDatabase, _key: &str) -> Option<TdObjEnum<'db>> {
    None
  }
  fn source_path(&self, _db: &'db TypedownDatabase) -> String {
    "@builtin::vault".to_string()
  }
}
