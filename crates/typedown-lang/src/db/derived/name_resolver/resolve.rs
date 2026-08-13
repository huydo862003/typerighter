use typedown_macros::query_derived;

use crate::syntax::diagnostic::Diagnostic;

use crate::db::TypedownDatabase;
use crate::db::derived::name_resolver::referee::referee;
use crate::db::types::{HirValue, HirValueKind, InterpolatedPart, ResolveResult};
use typedown_incremental::QueryDatabase;

#[query_derived]
pub fn resolve(db: &TypedownDatabase, hir: HirValue) -> ResolveResult {
  let mut diagnostics = vec![];
  collect_unresolved(db, hir, &mut diagnostics);
  ResolveResult::new(db, diagnostics)
}

fn collect_unresolved(db: &TypedownDatabase, hir: HirValue, diagnostics: &mut Vec<Diagnostic>) {
  match hir.kind(db) {
    HirValueKind::Ident(name) => {
      // self is a keyword, not a free variable
      if name == "self" {
        return;
      }
      let resolved = referee(db, hir);
      if resolved.value(db).is_none() {
        let node = hir.node(db);
        let (tr_offset, tr_len) = node.trimmed_range();
        diagnostics.push(Diagnostic::UnresolvedIdentifier {
          name: node.text(),
          start_offset: tr_offset,
          end_offset: tr_offset + tr_len,
        });
      }
    }
    HirValueKind::Mapping(entries) => {
      for (_, value) in entries {
        collect_unresolved(db, value, diagnostics);
      }
    }
    HirValueKind::Sequence(items) => {
      for item in items {
        collect_unresolved(db, item, diagnostics);
      }
    }
    HirValueKind::Interpolated(parts) => {
      for part in parts {
        if let InterpolatedPart::Expr(expr) = part {
          collect_unresolved(db, expr, diagnostics);
        }
      }
    }
    HirValueKind::Tag { tag, inner } => {
      collect_unresolved(db, *tag, diagnostics);
      collect_unresolved(db, *inner, diagnostics);
    }
    HirValueKind::Unary { operand, .. } => {
      collect_unresolved(db, *operand, diagnostics);
    }
    HirValueKind::Binary { op, left, right } => {
      collect_unresolved(db, *left, diagnostics);
      // The right side of a dot expression is a field name, not a free variable.
      if op != "." {
        collect_unresolved(db, *right, diagnostics);
      }
    }
    HirValueKind::Call { callee, args } => {
      collect_unresolved(db, *callee, diagnostics);
      for arg in args {
        collect_unresolved(db, arg, diagnostics);
      }
    }
    HirValueKind::Index { expr, indices } => {
      collect_unresolved(db, *expr, diagnostics);
      for idx in indices {
        collect_unresolved(db, idx, diagnostics);
      }
    }
    HirValueKind::Str(_)
    | HirValueKind::Num(_)
    | HirValueKind::Math(_)
    | HirValueKind::Bool(_)
    | HirValueKind::Null => {}
    HirValueKind::Markdown(parts) => {
      for part in parts {
        if let InterpolatedPart::Expr(expr) = part {
          collect_unresolved(db, expr, diagnostics);
        }
      }
    }
  }
}
