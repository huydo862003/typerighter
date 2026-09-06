use std::collections::BTreeMap;

use typedown_incremental::QueryDatabase;
use typedown_macros::query_derived;

use crate::db::TypedownDatabase;
use crate::db::types::TdSchemaType;

#[query_derived]
pub fn get_vault_type<'db>(db: &'db TypedownDatabase) -> TdSchemaType<'db> {
  TdSchemaType::new(
    db,
    "vault".to_string(),
    BTreeMap::new(),
    BTreeMap::new(),
    BTreeMap::new(),
    None,
  )
}
