//! Tracked query to get the type of a symbol

use typedown_macros::query_derived;

use crate::db::TypedownDatabase;
use crate::db::derived::get_builtin_types::{get_schema_meta_type, get_type_type};
use crate::db::derived::typechecker::actual_node_type::actual_node_type;
use crate::db::derived::typechecker::expected_node_type::expected_node_type;
use crate::db::derived::vault::get_vault_type;
use crate::db::types::{
  BuiltinGlobalKind, HirValue, HirValueKind, Symbol, SymbolKind, TdBlobType, TdTypeEnum, TypeResult,
};
use crate::db::utils::lower_file;
use typedown_incremental::QueryDatabase;

#[query_derived]
pub fn get_symbol_type<'db>(db: &'db TypedownDatabase, symbol: Symbol<'db>) -> TypeResult<'db> {
  match symbol.kind(db) {
    SymbolKind::BuiltinSchema(_) => TypeResult::new(db, Some(get_type_type(db).into()), vec![]),
    SymbolKind::UserDefinedSchema(_, _) => {
      TypeResult::new(db, Some(get_schema_meta_type(db).into()), vec![])
    }
    SymbolKind::UserDefinedResource(project, file) => {
      let (hir, _) = lower_file(db, project, file);
      match hir {
        Some(hir) => actual_node_type(db, hir),
        None => TypeResult::new(db, None, vec![]),
      }
    }
    SymbolKind::Asset(_, _, _) => TypeResult::new(db, Some(TdBlobType::get(db).into()), vec![]),
    // Builtin macro call return types are evaluated for the call expression as a whole in actual_node_type, not from the macro symbol itself
    SymbolKind::BuiltinMacro(_) => TypeResult::new(db, None, vec![]),
    SymbolKind::FnParam(_, _, closure) => get_fn_param_type(db, symbol, closure),
    SymbolKind::BuiltinGlobal(kind) => {
      let typ = match kind {
        BuiltinGlobalKind::Vault => get_vault_type(db).into(),
      };
      TypeResult::new(db, Some(typ), vec![])
    }
  }
}

// Get param type from expected(closure) by position
fn get_fn_param_type(db: &'db TypedownDatabase, symbol: Symbol, closure: HirValue) -> TypeResult {
  let expected = expected_node_type(db, closure).typ(db);
  let func_type = match expected {
    Some(TdTypeEnum::TdFuncType(f)) => f,
    _ => return TypeResult::new(db, None, vec![]),
  };

  let params = func_type.signature(db).params(db);
  let param_name = symbol.name(db);

  // Find param position in the closure's param list
  if let HirValueKind::Closure {
    params: param_names,
    ..
  } = closure.kind(db)
    && let Some(idx) = param_names.iter().position(|n| *n == param_name)
    && let Some(typ) = params.get(idx)
  {
    return TypeResult::new(db, Some(typ.clone()), vec![]);
  }

  TypeResult::new(db, None, vec![])
}
