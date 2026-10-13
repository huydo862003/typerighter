use typedown_macros::query_derived;

use crate::syntax::diagnostic::Diagnostic;

use crate::db::TypedownDatabase;
use crate::db::derived::get_vault_config::get_vault_config;
use crate::db::derived::name_resolver::referee::referee;
use crate::db::types::{ProjectRedNode, ResolveResult};
use crate::syntax::ast::{AstNode, Expr, IdentLit, StrLit, YamlOpKind};
use typedown_incremental::QueryDatabase;
use typedown_types::either::Either;

#[query_derived(no_hash)]
pub fn resolve<'db>(db: &'db TypedownDatabase, project_node: ProjectRedNode) -> ResolveResult<'db> {
  let mut diagnostics = vec![];
  collect_unresolved(db, project_node, &mut diagnostics);
  ResolveResult::new(db, diagnostics)
}

fn collect_unresolved(
  db: &TypedownDatabase,
  project_node: ProjectRedNode,
  diagnostics: &mut Vec<Diagnostic>,
) {
  let project = project_node.project;
  let file = project_node.file;
  let node = project_node.node.clone();

  // Mapping: recurse into each value
  if let Some(entries) = project_node.mapping_entries() {
    for (key, value_node) in entries {
      if key == "_imports" {
        check_imports(db, project_node.clone(), value_node, diagnostics);
        continue;
      }
      collect_unresolved(db, value_node, diagnostics);
    }
    return;
  }

  // Sequence: recurse into each item
  if let Some(items) = project_node.sequence_items() {
    for item in items {
      collect_unresolved(db, item, diagnostics);
    }
    return;
  }

  // Identifier: check if it resolves
  if let Some(ident) = IdentLit::cast(node.clone()) {
    let name = ident.value().unwrap_or_default();
    // self, null, true, false are keywords, not free variables
    if matches!(name.as_str(), "self" | "null" | "true" | "false") {
      return;
    }
    let resolved = referee(db, project_node.clone());
    if resolved.value(db).is_none() {
      let (trimmed_offset, trimmed_len) = project_node.node.trimmed_range();
      diagnostics.push(Diagnostic::UnresolvedIdentifier {
        name: project_node.node.text(),
        start_offset: trimmed_offset,
        end_offset: trimmed_offset + trimmed_len,
      });
    }
    return;
  }

  // Interpolated string: recurse into embedded expressions
  if let Some(str_lit) = StrLit::cast(node.clone()) {
    for fragment in str_lit.fragments() {
      if let Either::Right(interp) = fragment
        && let Some(expr) = interp.expr()
      {
        collect_unresolved(
          db,
          ProjectRedNode::new(project, file, expr.syntax().clone()),
          diagnostics,
        );
      }
    }
    return;
  }

  // Expr variants
  let Some(expr) = Expr::cast(node.clone()) else {
    return;
  };

  match expr {
    Expr::BinaryExpr(binary) => {
      let op_kind = binary.op().and_then(|op| op.kind());
      if let Some(left) = binary.left() {
        collect_unresolved(
          db,
          ProjectRedNode::new(project, file, left.syntax().clone()),
          diagnostics,
        );
      }
      // The right side of a dot expression is a field name, not a free variable
      if !matches!(op_kind, Some(YamlOpKind::Dot))
        && let Some(right) = binary.right()
      {
        collect_unresolved(
          db,
          ProjectRedNode::new(project, file, right.syntax().clone()),
          diagnostics,
        );
      }
    }
    Expr::PrefixExpr(prefix) => {
      if let Some(operand) = prefix.expr() {
        collect_unresolved(
          db,
          ProjectRedNode::new(project, file, operand.syntax().clone()),
          diagnostics,
        );
      }
    }
    Expr::PostfixExpr(postfix) => {
      if let Some(operand) = postfix.expr() {
        collect_unresolved(
          db,
          ProjectRedNode::new(project, file, operand.syntax().clone()),
          diagnostics,
        );
      }
    }
    Expr::CallExpr(call) => {
      if let Some(callee) = call.callee() {
        collect_unresolved(
          db,
          ProjectRedNode::new(project, file, callee.syntax().clone()),
          diagnostics,
        );
      }
      for arg in call.args() {
        collect_unresolved(
          db,
          ProjectRedNode::new(project, file, arg.syntax().clone()),
          diagnostics,
        );
      }
    }
    Expr::IndexExpr(index) => {
      if let Some(expr_part) = index.expr() {
        collect_unresolved(
          db,
          ProjectRedNode::new(project, file, expr_part.syntax().clone()),
          diagnostics,
        );
      }
      for idx in index.indices() {
        collect_unresolved(
          db,
          ProjectRedNode::new(project, file, idx.syntax().clone()),
          diagnostics,
        );
      }
    }
    Expr::ClosureExpr(closure) => {
      if let Some(body) = closure.body() {
        collect_unresolved(
          db,
          ProjectRedNode::new(project, file, body.syntax().clone()),
          diagnostics,
        );
      }
    }
    Expr::ParenExpr(paren) => {
      if let Some(inner) = paren.expr() {
        collect_unresolved(
          db,
          ProjectRedNode::new(project, file, inner.syntax().clone()),
          diagnostics,
        );
      }
    }
    // Literals, math, code: nothing to resolve
    _ => {}
  }
}

// Check that each _imports path resolves to an existing file
fn check_imports(
  db: &TypedownDatabase,
  mapping_node: ProjectRedNode,
  imports_node: ProjectRedNode,
  diagnostics: &mut Vec<Diagnostic>,
) {
  let Some(entries) = imports_node.mapping_entries() else {
    return;
  };
  let project = mapping_node.project;
  let root_dir = get_vault_config(db, project).root_dir(db);

  for (_, path_node) in entries {
    let Some(path) = StrLit::cast(path_node.node.clone()).and_then(|s| s.plain_value()) else {
      continue;
    };
    let target_path = root_dir.join(&path);
    if !project.files(db).contains_key(&target_path) {
      let (start, len) = path_node.node.trimmed_range();
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
  use crate::db::fixtures::load_vault_fixture;
  use crate::syntax::parse::tests::helpers::parse;
  use crate::syntax::red::RedNode;

  fn make_root_node(
    _db: &crate::db::TypedownDatabase,
    project: crate::db::types::Project,
    file: crate::db::types::File,
    src: &str,
  ) -> ProjectRedNode {
    let (root, _) = parse(src);
    let red_root = RedNode::new_root(root.as_node().unwrap().clone());
    ProjectRedNode::new(project, file, red_root)
  }

  #[test]
  fn resolve_closure_params_are_not_unresolved() {
    let (db, project, file) = load_vault_fixture("evaluate/my_vault", "valid_person.td");
    let project_node = make_root_node(
      &db,
      project,
      file,
      r#"---
fn: (a, b) -> a + b
---"#,
    );

    let res = resolve(&db, project_node);
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
    let project_node = make_root_node(
      &db,
      project,
      file,
      r#"---
fn: (a) -> a + missing_variable
---"#,
    );

    let res = resolve(&db, project_node);
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
    let project_node = make_root_node(
      &db,
      project,
      file,
      r#"---
_imports:
  missing: "_partials/nonexistent.td"
name: missing.foo
---"#,
    );

    let res = resolve(&db, project_node);
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
