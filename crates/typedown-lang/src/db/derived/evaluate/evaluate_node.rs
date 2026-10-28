//! Evaluate an AST node into a typed object

use typedown_macros::query_derived;

use crate::db::TypedownDatabase;
use crate::db::derived::evaluate::utils::construct_from_node;
use crate::db::derived::typechecker::typecheck::{typecheck, typecheck_with_expected};
use crate::db::types::{ProjectRedNode, ResourceResult, RuntimeScope, TdTypEnum};
use crate::syntax::diagnostic::TdDiagnostic;
use typedown_incremental::QueryDatabase;

// Evaluate with schema-resolved expected type
#[query_derived(no_hash)]
pub fn evaluate_node<'db>(
  db: &'db TypedownDatabase,
  project_node: ProjectRedNode,
  runtime_scope: RuntimeScope<'db>,
) -> ResourceResult<'db> {
  let typecheck_result = typecheck(db, project_node.clone());
  let diagnostics = typecheck_result.diagnostics(db).to_vec();
  evaluate_body(db, project_node, runtime_scope, diagnostics)
}

// Evaluate with an externally provided expected type
pub fn evaluate_node_with_expected<'db>(
  db: &'db TypedownDatabase,
  project_node: ProjectRedNode,
  expected_typ: &TdTypEnum<'db>,
  runtime_scope: RuntimeScope<'db>,
) -> ResourceResult<'db> {
  let typecheck_result = typecheck_with_expected(db, project_node.clone(), expected_typ);
  let diagnostics = typecheck_result.diagnostics(db).to_vec();
  evaluate_body(db, project_node, runtime_scope, diagnostics)
}

fn evaluate_body<'db>(
  db: &'db TypedownDatabase,
  project_node: ProjectRedNode,
  runtime_scope: RuntimeScope<'db>,
  mut diagnostics: Vec<TdDiagnostic>,
) -> ResourceResult<'db> {
  let obj = construct_from_node(db, project_node, runtime_scope, &mut diagnostics);
  ResourceResult::new(db, obj, diagnostics)
}
