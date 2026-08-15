//! Tracked query to get the type of a symbol

use typedown_macros::query_derived;

use crate::db::TypedownDatabase;
use crate::db::derived::get_builtin_types::{get_schema_type, get_type_type};
use crate::db::derived::typechecker::actual_node_type::actual_node_type;
use crate::db::derived::vault::get_vault_type;
use crate::db::types::{BuiltinGlobalKind, Symbol, SymbolKind, TdBlobType, TypeResult};
use crate::db::utils::lower_file;
use typedown_incremental::QueryDatabase;

#[query_derived]
pub fn get_symbol_type(db: &TypedownDatabase, symbol: Symbol) -> TypeResult {
  match symbol.kind(db) {
    SymbolKind::BuiltinSchema(_) => TypeResult::new(db, Some(get_type_type(db).into()), vec![]),
    SymbolKind::UserDefinedSchema(_, _) => {
      TypeResult::new(db, Some(get_schema_type(db).into()), vec![])
    }
    SymbolKind::UserDefinedResource(project, file) => {
      let (hir, _) = lower_file(db, project, file);
      match hir {
        Some(hir) => actual_node_type(db, hir),
        None => TypeResult::new(db, None, vec![]),
      }
    }
    SymbolKind::Asset(_, _, _) => TypeResult::new(db, Some(TdBlobType::get(db).into()), vec![]),
    SymbolKind::BuiltinMacro(_) => TypeResult::new(db, None, vec![]),
    SymbolKind::BuiltinGlobal(kind) => {
      let typ = match kind {
        BuiltinGlobalKind::Vault => get_vault_type(db).into(),
      };
      TypeResult::new(db, Some(typ), vec![])
    }
  }
}
