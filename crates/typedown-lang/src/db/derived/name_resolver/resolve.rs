use typedown_macros::query_derived;

use crate::syntax::diagnostic::Diagnostic;

use crate::db::TypedownDatabase;
use crate::db::derived::get_vault_config::get_vault_config;
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
      for (key, value) in entries {
        if key == "_imports" {
          check_imports(db, hir, value, diagnostics);
          continue;
        }
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
    HirValueKind::Prefix { operand, .. } | HirValueKind::Postfix { operand, .. } => {
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
    HirValueKind::Closure { body, .. } => {
      collect_unresolved(db, *body, diagnostics);
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

// Check that each _imports path resolves to an existing file
fn check_imports(
  db: &TypedownDatabase,
  hir: HirValue,
  imports_hir: HirValue,
  diagnostics: &mut Vec<Diagnostic>,
) {
  let HirValueKind::Mapping(entries) = imports_hir.kind(db) else {
    return;
  };
  let project = hir.project(db);
  let root_dir = get_vault_config(db, project).root_dir(db);

  for (_, path_hir) in entries {
    let HirValueKind::Str(path) = path_hir.kind(db) else {
      continue;
    };
    let target_path = root_dir.join(&path);
    if !project.files(db).contains_key(&target_path) {
      let node = path_hir.node(db);
      let (start, len) = node.trimmed_range();
      diagnostics.push(Diagnostic::UnresolvedImport {
        path,
        start_offset: start,
        end_offset: start + len,
      });
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::db::derived::hir::lower_node;
  use crate::db::fixtures::load_vault_fixture;
  use crate::syntax::parse::tests::helpers::parse;
  use crate::syntax::red::RedNode;

  #[test]
  fn resolve_closure_params_are_not_unresolved() {
    let (db, project, file) = load_vault_fixture("evaluate/my_vault", "valid_person.td");
    let (root, _) = parse(
      r#"---
fn: (a, b) -> a + b
---"#,
    );
    let red_root = RedNode::new_root(root.as_node().unwrap().clone());
    let hir = lower_node(&db, project, file, red_root);

    let res = resolve(&db, hir);
    let diags = res.diagnostics(&db);
    assert!(
      diags.is_empty(),
      "closure parameters 'a' and 'b' should resolve cleanly, got: {:?}",
      diags
    );
  }

  #[test]
  fn resolve_closure_unresolved_ident_emits_diagnostic() {
    let (db, project, file) = load_vault_fixture("evaluate/my_vault", "valid_person.td");
    let (root, _) = parse(
      r#"---
fn: (a) -> a + missing_variable
---"#,
    );
    let red_root = RedNode::new_root(root.as_node().unwrap().clone());
    let hir = lower_node(&db, project, file, red_root);

    let res = resolve(&db, hir);
    let diags = res.diagnostics(&db);
    assert_eq!(diags.len(), 1);
    match &diags[0] {
      Diagnostic::UnresolvedIdentifier { name, .. } => {
        assert_eq!(name.trim(), "missing_variable");
      }
      d => panic!("expected UnresolvedIdentifier diagnostic, got: {:?}", d),
    }
  }

  #[test]
  fn resolve_unresolved_import_emits_diagnostic() {
    let (db, project, file) = load_vault_fixture("evaluate/my_vault", "valid_person.td");
    let (root, _) = parse(
      r#"---
_imports:
  missing: "_partials/nonexistent.td"
name: missing.foo
---"#,
    );
    let red_root = RedNode::new_root(root.as_node().unwrap().clone());
    let hir = lower_node(&db, project, file, red_root);

    let res = resolve(&db, hir);
    let diags = res.diagnostics(&db);
    assert!(
      diags
        .iter()
        .any(|d| matches!(d, Diagnostic::UnresolvedImport { .. })),
      "should have UnresolvedImport diagnostic, got: {:?}",
      diags
    );
  }
}
