use std::collections::HashMap;

use typedown_incremental::QueryDatabase;
use typedown_macros::query_derived;

use crate::db::TypedownDatabase;
use crate::db::types::TdSchemaType;

#[query_derived]
pub fn get_vault_type(db: &TypedownDatabase) -> TdSchemaType {
  TdSchemaType::new(
    db,
    "vault".to_string(),
    HashMap::new(),
    HashMap::new(),
    None,
  )
}
