use std::collections::BTreeMap;

use typedown_macros::query_derived;

use crate::db::TypedownDatabase;
use crate::db::derived::get_builtin_typs::{
  get_bool_symbol, get_date_symbol, get_datetime_symbol, get_dict_symbol, get_list_symbol,
  get_math_symbol, get_number_symbol, get_schema_symbol, get_string_symbol, get_time_symbol,
  get_typ_typ_symbol,
};
use crate::db::types::{BuiltinGlobalKind, BuiltinMacroKind, Symbol, SymbolKind};
use typedown_incremental::QueryDatabase;

#[query_derived]
pub struct BuiltinScopeMembers<'db> {
  pub members: BTreeMap<String, Symbol<'db>>,
}

#[query_derived]
pub fn builtin_scope<'db>(db: &'db TypedownDatabase) -> BuiltinScopeMembers<'db> {
  let members = BTreeMap::from([
    ("schema".to_string(), get_schema_symbol(db)),
    ("string".to_string(), get_string_symbol(db)),
    ("number".to_string(), get_number_symbol(db)),
    ("boolean".to_string(), get_bool_symbol(db)),
    ("date".to_string(), get_date_symbol(db)),
    ("datetime".to_string(), get_datetime_symbol(db)),
    ("time".to_string(), get_time_symbol(db)),
    ("list".to_string(), get_list_symbol(db)),
    ("dict".to_string(), get_dict_symbol(db)),
    ("math".to_string(), get_math_symbol(db)),
    ("type".to_string(), get_typ_typ_symbol(db)),
    (
      "fref".to_string(),
      Symbol::new(
        db,
        SymbolKind::BuiltinMacro(BuiltinMacroKind::Fref),
        "fref".to_string(),
        "@builtin::fref".to_string(),
      ),
    ),
    (
      "vault".to_string(),
      Symbol::new(
        db,
        SymbolKind::BuiltinGlobal(BuiltinGlobalKind::Vault),
        "vault".to_string(),
        "@builtin::vault".to_string(),
      ),
    ),
    (
      "icon".to_string(),
      Symbol::new(
        db,
        SymbolKind::BuiltinGlobal(BuiltinGlobalKind::Icon),
        "icon".to_string(),
        "@builtin::icon".to_string(),
      ),
    ),
  ]);
  BuiltinScopeMembers::new(db, members)
}
