//! Tracked query to get the type of a symbol

use std::collections::VecDeque;

use typedown_macros::query_derived;

use crate::db::TypedownDatabase;
use crate::db::derived::get_builtin_typs::{get_schema_meta_typ, get_typ_typ};
use crate::db::derived::icon::get_icon_module_typ;
use crate::db::derived::name_resolver::referee::referee;
use crate::db::derived::parse_file::parse_file;
use crate::db::derived::typechecker::actual_node_typ::actual_node_typ;
use crate::db::derived::typechecker::expected_node_typ::expected_node_typ;
use crate::db::derived::vault::get_vault_typ;
use crate::db::types::{
  BuiltinGlobalKind, File, Project, ProjectRedNode, Symbol, SymbolKind, TdBlobTyp, TdTypEnum,
  TdVariableTyp, TypResult, TypVariable,
};
use crate::syntax::ast::{AstNode, ClosureExpr, IdentLit};
use crate::syntax::red::RedNode;
use typedown_incremental::QueryDatabase;
use typedown_types::either::Either;

#[query_derived(no_hash)]
pub fn get_symbol_typ<'db>(db: &'db TypedownDatabase, symbol: Symbol) -> TypResult<'db> {
  match symbol.kind(db) {
    SymbolKind::BuiltinSchema(_) => TypResult::new(db, Some(get_typ_typ(db).into()), vec![]),
    SymbolKind::UserDefinedSchema(_, _) => {
      TypResult::new(db, Some(get_schema_meta_typ(db).into()), vec![])
    }
    SymbolKind::UserDefinedResource(project, file) => {
      let project_node = parse_file(db, project, file).ast(db);
      actual_node_typ(db, project_node)
    }
    SymbolKind::Asset(_, _, _) => TypResult::new(db, Some(TdBlobTyp::get(db).into()), vec![]),
    // Builtin macro call return types are evaluated for the call expression as a whole in actual_node_type, not from the macro symbol itself
    SymbolKind::BuiltinMacro(_) => TypResult::new(db, None, vec![]),
    SymbolKind::FuncParam(_, _, closure_node, _) => get_func_param_typ(db, symbol, closure_node),
    SymbolKind::BuiltinGlobal(kind) => {
      let typ = match kind {
        BuiltinGlobalKind::Vault => get_vault_typ(db).into(),
        BuiltinGlobalKind::Icon => get_icon_module_typ(db).into(),
      };
      TypResult::new(db, Some(typ), vec![])
    }
  }
}

// Get param type from expected(closure) by position, falling back to body inference
fn get_func_param_typ<'db>(
  db: &'db TypedownDatabase,
  symbol: Symbol,
  closure_node: ProjectRedNode,
) -> TypResult<'db> {
  let expected = expected_node_typ(db, closure_node.clone()).typ(db);
  let param_name = symbol.name(db);

  let Some(closure) = ClosureExpr::cast(closure_node.node.clone()) else {
    return TypResult::new(db, None, vec![]);
  };
  let Some(closure_params) = closure.params() else {
    return TypResult::new(db, None, vec![]);
  };
  let param_names: Vec<_> = match closure_params {
    Either::Left(param_list) => param_list.params().collect(),
    Either::Right(ident) => vec![ident],
  };

  if let Some(TdTypEnum::TdFuncTyp(func_typ)) = expected {
    let params = func_typ.signature(db).params(db);
    if let Some(index) = param_names
      .iter()
      .filter_map(|param| param.value())
      .position(|name| name == param_name)
      && let Some(typ) = params.get(index)
    {
      return TypResult::new(db, Some(typ.clone()), vec![]);
    }
    return TypResult::new(db, None, vec![]);
  }

  // No expected type: infer param type from body usage
  // Fall back to a generic type variable bound to this param symbol if unresolvable
  let project = closure_node.project;
  let file = closure_node.file;
  let Some(body) = closure.body() else {
    return TypResult::new(db, None, vec![]);
  };
  let typ = infer_param_typ_from_body(db, project, file, body.syntax(), symbol)
    .unwrap_or_else(|| TdVariableTyp::new(db, TypVariable::get(db, None), symbol).into());
  TypResult::new(db, Some(typ), vec![])
}

/// Find the expected type at the first usage site of a param that resolves to `target_symbol`
pub(super) fn infer_param_typ_from_body<'db>(
  db: &'db TypedownDatabase,
  project: Project,
  file: File,
  body: &RedNode,
  target_symbol: Symbol,
) -> Option<TdTypEnum<'db>> {
  let param_name = target_symbol.name(db);

  let mut queue = VecDeque::from([body.clone()]);
  while let Some(node) = queue.pop_front() {
    if let Some(ident) = IdentLit::cast(node.clone())
      && ident.value().as_deref() == Some(param_name.as_str())
    {
      let project_node = ProjectRedNode::new(project, file, node.clone());
      // Verify this identifier resolves to exactly this param symbol (not a shadowed one)
      let is_our_param = referee(db, project_node.clone())
        .value(db)
        .is_some_and(|symbol| symbol == target_symbol);
      if is_our_param && let Some(typ) = expected_node_typ(db, project_node).typ(db) {
        return Some(typ);
      }
    }
    queue.extend(node.children());
  }
  None
}
