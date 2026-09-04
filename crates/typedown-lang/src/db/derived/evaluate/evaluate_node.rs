//! Evaluate a HIR node into a typed object

use typedown_macros::query_derived;

use crate::db::TypedownDatabase;
use crate::db::derived::evaluate::utils::construct_from_hir;
use crate::db::derived::typechecker::typecheck::{typecheck, typecheck_with_expected};
use crate::db::types::{HirValue, ResourceResult, RuntimeScope, TdTypeEnum};
use crate::syntax::diagnostic::Diagnostic;
use typedown_incremental::QueryDatabase;

// Evaluate with schema-resolved expected type
#[query_derived]
pub fn evaluate_node<'db>(
  db: &'db TypedownDatabase,
  hir: HirValue<'db>,
  runtime_scope: RuntimeScope<'db>,
) -> ResourceResult<'db> {
  let typecheck_result = typecheck(db, hir);
  let diagnostics = typecheck_result.diagnostics(db).to_vec();
  evaluate_body(db, hir, runtime_scope, diagnostics)
}

// Evaluate with an externally provided expected type
pub fn evaluate_node_with_expected<'db>(
  db: &'db TypedownDatabase,
  hir: HirValue<'db>,
  expected_type: &TdTypeEnum<'db>,
  runtime_scope: RuntimeScope<'db>,
) -> ResourceResult<'db> {
  let typecheck_result = typecheck_with_expected(db, hir, expected_type);
  let diagnostics = typecheck_result.diagnostics(db).to_vec();
  evaluate_body(db, hir, runtime_scope, diagnostics)
}

fn evaluate_body<'db>(
  db: &'db TypedownDatabase,
  hir: HirValue<'db>,
  runtime_scope: RuntimeScope<'db>,
  mut diagnostics: Vec<Diagnostic>,
) -> ResourceResult<'db> {
  let obj = construct_from_hir(db, hir, runtime_scope, &mut diagnostics);
  ResourceResult::new(db, obj, diagnostics)
}
