use std::collections::HashMap;

use typedown_incremental::QueryDatabase;
use typedown_macros::query_derived;

use crate::db::TypedownDatabase;
use crate::db::derived::get_builtin_types::get_type_type;
use crate::db::types::TdProductType;

#[query_derived]
pub fn get_vault_type(db: &TypedownDatabase) -> TdProductType {
  TdProductType::new(
    db,
    Some("vault".to_string()),
    get_type_type(db).into(),
    HashMap::new(),
    HashMap::new(),
    None,
  )
}
