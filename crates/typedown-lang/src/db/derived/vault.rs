use std::collections::BTreeMap;

use typedown_incremental::QueryDatabase;
use typedown_macros::query_derived;

use crate::db::TypedownDatabase;
use crate::db::types::TdSchemaTyp;

#[query_derived]
pub fn get_vault_typ<'db>(db: &'db TypedownDatabase) -> TdSchemaTyp<'db> {
  TdSchemaTyp::new(
    db,
    "vault".to_string(),
    BTreeMap::new(),
    BTreeMap::new(),
    BTreeMap::new(),
    None,
  )
}
