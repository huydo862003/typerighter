//! Evaluate a HIR node into a typed object

use typedown_macros::query_derived;

use crate::db::TypedownDatabase;
use crate::db::derived::evaluate::utils::construct_from_hir;
use crate::db::derived::typechecker::typecheck::typecheck;
use crate::db::types::{HirValue, ResourceResult, RuntimeScope};
use typedown_incremental::QueryDatabase;

#[query_derived]
pub fn evaluate_node(
  db: &TypedownDatabase,
  hir: HirValue,
  runtime_scope: RuntimeScope,
) -> ResourceResult {
  let mut diagnostics = vec![];

  let typecheck_result = typecheck(db, hir);
  diagnostics.extend(typecheck_result.diagnostics(db).iter().cloned());

  let obj = construct_from_hir(db, hir, runtime_scope, &mut diagnostics);

  ResourceResult::new(db, obj, diagnostics)
}
