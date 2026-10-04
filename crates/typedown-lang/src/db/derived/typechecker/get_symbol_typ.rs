//! Tracked query to get the type of a symbol

use typedown_macros::query_derived;

use crate::db::TypedownDatabase;
use crate::db::derived::get_builtin_typs::{get_schema_meta_typ, get_typ_typ};
use crate::db::derived::icon::get_icon_module_typ;
use crate::db::derived::typechecker::actual_node_typ::actual_node_typ;
use crate::db::derived::typechecker::expected_node_typ::expected_node_typ;
use crate::db::derived::vault::get_vault_typ;
use crate::db::types::{
  BuiltinGlobalKind, HirValue, HirValueKind, Symbol, SymbolKind, TdBlobTyp, TdTypEnum, TypResult,
};
use crate::db::utils::lower_file;
use typedown_incremental::QueryDatabase;

#[query_derived(no_hash)]
pub fn get_symbol_typ<'db>(db: &'db TypedownDatabase, symbol: Symbol<'db>) -> TypResult<'db> {
  match symbol.kind(db) {
    SymbolKind::BuiltinSchema(_) => TypResult::new(db, Some(get_typ_typ(db).into()), vec![]),
    SymbolKind::UserDefinedSchema(_, _) => {
      TypResult::new(db, Some(get_schema_meta_typ(db).into()), vec![])
    }
    SymbolKind::UserDefinedResource(project, file) => {
      let (hir, _) = lower_file(db, project, file);
      match hir {
        Some(hir) => actual_node_typ(db, hir),
        None => TypResult::new(db, None, vec![]),
      }
    }
    SymbolKind::Asset(_, _, _) => TypResult::new(db, Some(TdBlobTyp::get(db).into()), vec![]),
    // Builtin macro call return types are evaluated for the call expression as a whole in actual_node_type, not from the macro symbol itself
    SymbolKind::BuiltinMacro(_) => TypResult::new(db, None, vec![]),
    SymbolKind::FuncParam(_, _, closure) => get_func_param_typ(db, symbol, closure),
    SymbolKind::BuiltinGlobal(kind) => {
      let typ = match kind {
        BuiltinGlobalKind::Vault => get_vault_typ(db).into(),
        BuiltinGlobalKind::Icon => get_icon_module_typ(db).into(),
      };
      TypResult::new(db, Some(typ), vec![])
    }
  }
}

// Get param type from expected(closure) by position
fn get_func_param_typ<'db>(
  db: &'db TypedownDatabase,
  symbol: Symbol<'db>,
  closure: HirValue<'db>,
) -> TypResult<'db> {
  let expected = expected_node_typ(db, closure).typ(db);
  let func_typ = match expected {
    Some(TdTypEnum::TdFuncTyp(f)) => f,
    _ => return TypResult::new(db, None, vec![]),
  };

  let params = func_typ.signature(db).params(db);
  let param_name = symbol.name(db);

  // Find param position in the closure's param list
  if let HirValueKind::Closure {
    params: param_names,
    ..
  } = closure.kind(db)
    && let Some(idx) = param_names.iter().position(|n| *n == param_name)
    && let Some(typ) = params.get(idx)
  {
    return TypResult::new(db, Some(typ.clone()), vec![]);
  }

  TypResult::new(db, None, vec![])
}
