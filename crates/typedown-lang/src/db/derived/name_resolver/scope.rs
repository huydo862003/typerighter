use typedown_macros::query_derived;

use crate::db::TypedownDatabase;
use crate::db::types::{HirValue, Scope, ScopeKind};
use typedown_incremental::QueryDatabase;

#[query_derived]
pub struct MaybeScope {
  pub value: Option<Scope>,
}

#[query_derived]
pub fn scope(db: &TypedownDatabase, hir: HirValue) -> Scope {
  Scope::file_scope(db, hir.project(db), hir.file(db))
}

#[query_derived]
pub fn parent_scope(db: &TypedownDatabase, scope: Scope) -> MaybeScope {
  match scope.kind(db) {
    ScopeKind::Builtin => MaybeScope::new(db, None),
    ScopeKind::Project(_) => MaybeScope::new(db, Some(Scope::builtin_scope(db))),
    ScopeKind::File(project, _) => MaybeScope::new(db, Some(Scope::project_scope(db, project))),
    ScopeKind::Fn(project, file, value) => todo!(),
  }
}
