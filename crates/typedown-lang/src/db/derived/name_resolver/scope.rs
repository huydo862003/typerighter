use typedown_macros::query_derived;

use crate::db::TypedownDatabase;
use crate::db::derived::hir::lower_node;
use crate::db::types::{HirValue, Scope, ScopeKind};
use crate::syntax::syntax_kind::SyntaxKind;
use typedown_incremental::QueryDatabase;

#[query_derived]
pub struct MaybeScope {
  pub value: Option<Scope>,
}

#[query_derived]
pub fn scope(db: &TypedownDatabase, hir: HirValue) -> Scope {
  let project = hir.project(db);
  let file = hir.file(db);
  let node = hir.node(db);

  // Walk successively up to find the closest closure and return file symbol once reaching root
  let mut curr = node.parent();
  while let Some(p) = curr {
    if p.kind() == SyntaxKind::ClosureExpr {
      let closure_hir = lower_node(db, project, file, p);
      return Scope::fn_scope(db, project, file, closure_hir);
    }
    curr = p.parent();
  }

  Scope::file_scope(db, project, file)
}

#[query_derived]
pub fn parent_scope(db: &TypedownDatabase, scope: Scope) -> MaybeScope {
  match scope.kind(db) {
    ScopeKind::Builtin => MaybeScope::new(db, None),
    ScopeKind::Project(_) => MaybeScope::new(db, Some(Scope::builtin_scope(db))),
    ScopeKind::File(project, _) => MaybeScope::new(db, Some(Scope::project_scope(db, project))),
    ScopeKind::Fn(_project, _file, value) => MaybeScope::new(db, Some(self::scope(db, value))),
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::db::fixtures::load_vault_fixture;
  use crate::db::types::{HirValueKind, InterpolatedPart};
  use crate::db::utils::lower_file;
  use crate::syntax::ast::AstNode;
  use crate::syntax::parse::tests::helpers::parse;
  use crate::syntax::red::RedNode;

  #[test]
  fn scope_for_file_hir_returns_file_scope() {
    let (db, project, file) = load_vault_fixture("evaluate/my_vault", "content/valid_person.td");
    let (hir, _) = lower_file(&db, project, file);
    let hir = hir.expect("should have HIR");

    let file_scope = scope(&db, hir);
    assert!(matches!(file_scope.kind(&db), ScopeKind::File(..)));

    let parent = parent_scope(&db, file_scope).value(&db).unwrap();
    assert!(matches!(parent.kind(&db), ScopeKind::Project(..)));
  }

  #[test]
  fn scope_inside_closure_returns_fn_scope() {
    let (db, project, file) = load_vault_fixture("evaluate/my_vault", "content/valid_person.td");
    let (root, _) = parse(
      r#"---
fn: (a, b) -> a + b
---"#,
    );
    let red_root = RedNode::new_root(root.as_node().unwrap().clone());
    let hir = lower_node(&db, project, file, red_root);

    let entries = match hir.kind(&db) {
      HirValueKind::Mapping(e) => e,
      _ => panic!("expected mapping"),
    };
    let fn_hir = entries.iter().find(|(k, _)| k == "fn").unwrap().1;
    let closure_body = match fn_hir.kind(&db) {
      HirValueKind::Closure { body, .. } => body,
      _ => panic!("expected closure"),
    };

    let inner_scope = scope(&db, *closure_body);
    assert!(matches!(inner_scope.kind(&db), ScopeKind::Fn(..)));

    let parent = parent_scope(&db, inner_scope).value(&db).unwrap();
    assert!(matches!(parent.kind(&db), ScopeKind::File(..)));
  }

  #[test]
  fn scope_for_yaml_frontmatter_node() {
    let (db, project, file) = load_vault_fixture("evaluate/my_vault", "content/valid_person.td");
    let (root, _) = parse(
      r#"---
title: "Hello"
age: 30
---"#,
    );
    let red_root = RedNode::new_root(root.as_node().unwrap().clone());
    let hir = lower_node(&db, project, file, red_root);

    let entries = match hir.kind(&db) {
      HirValueKind::Mapping(e) => e,
      _ => panic!("expected mapping"),
    };
    let title_hir = entries.iter().find(|(k, _)| k == "title").unwrap().1;

    let yaml_scope = scope(&db, title_hir);
    assert!(matches!(
      yaml_scope.kind(&db),
      ScopeKind::File(p, f) if p == project && f == file
    ));
  }

  #[test]
  fn scope_for_markdown_body_and_interpolation() {
    let (db, project, file) = load_vault_fixture("evaluate/my_vault", "content/valid_person.td");
    let (root, _) = parse(
      r#"---
title: "Hello"
---
Hello world ${(a, b) -> a + b}
"#,
    );
    let red_root = RedNode::new_root(root.as_node().unwrap().clone());
    let hir = lower_node(&db, project, file, red_root);

    let entries = match hir.kind(&db) {
      HirValueKind::Mapping(e) => e,
      _ => panic!("expected mapping"),
    };
    let content_hir = entries.iter().find(|(k, _)| k == "_content").unwrap().1;

    // Top-level markdown body HIR has File scope
    let md_scope = scope(&db, content_hir);
    assert!(matches!(
      md_scope.kind(&db),
      ScopeKind::File(p, f) if p == project && f == file
    ));

    // Markdown interpolation with closure body inside markdown body has Fn scope
    let parts = match content_hir.kind(&db) {
      HirValueKind::Markdown(p) => p,
      _ => panic!("expected markdown parts"),
    };
    let closure_expr = parts
      .iter()
      .find_map(|part| match part {
        InterpolatedPart::Expr(e) => Some(e),
        _ => None,
      })
      .expect("expected interpolated closure expr");

    let closure_body = match closure_expr.kind(&db) {
      HirValueKind::Closure { body, .. } => body,
      _ => panic!("expected closure in markdown interpolation"),
    };

    let interp_fn_scope = scope(&db, *closure_body);
    assert!(matches!(interp_fn_scope.kind(&db), ScopeKind::Fn(..)));
  }

  #[test]
  fn scope_nested_closure_in_yaml_frontmatter() {
    let (db, project, file) = load_vault_fixture("evaluate/my_vault", "content/valid_person.td");
    let (root, _) = parse(
      r#"---
outer: (a) -> (b) -> a + b
---"#,
    );
    let red_root = RedNode::new_root(root.as_node().unwrap().clone());
    let hir = lower_node(&db, project, file, red_root);

    let entries = match hir.kind(&db) {
      HirValueKind::Mapping(e) => e,
      _ => panic!("expected mapping"),
    };
    let outer_closure = entries.iter().find(|(k, _)| k == "outer").unwrap().1;
    let inner_closure = match outer_closure.kind(&db) {
      HirValueKind::Closure { body, .. } => body,
      _ => panic!("expected outer closure"),
    };
    let innermost_body = match inner_closure.kind(&db) {
      HirValueKind::Closure { body, .. } => body,
      _ => panic!("expected inner closure"),
    };

    // Innermost scope is Fn scope for inner_closure
    let inner_scope = scope(&db, *innermost_body);
    let inner_scope_val = match inner_scope.kind(&db) {
      ScopeKind::Fn(_, _, val) => val,
      _ => panic!("expected Fn scope for inner closure"),
    };
    assert_eq!(inner_scope_val, *inner_closure);

    // Parent of inner scope is Fn scope for outer_closure
    let outer_scope = parent_scope(&db, inner_scope).value(&db).unwrap();
    let outer_scope_val = match outer_scope.kind(&db) {
      ScopeKind::Fn(_, _, val) => val,
      _ => panic!("expected Fn scope for outer closure"),
    };
    assert_eq!(outer_scope_val, outer_closure);

    // Parent of outer scope is File scope
    let file_scope = parent_scope(&db, outer_scope).value(&db).unwrap();
    assert!(matches!(
      file_scope.kind(&db),
      ScopeKind::File(p, f) if p == project && f == file
    ));
  }

  #[test]
  fn scope_nested_closure_in_markdown_body() {
    let (db, project, file) = load_vault_fixture("evaluate/my_vault", "content/valid_person.td");
    let (root, _) = parse(
      r#"---
title: "Nested"
---
${(a) -> (b) -> a + b}
"#,
    );
    let red_root = RedNode::new_root(root.as_node().unwrap().clone());
    let hir = lower_node(&db, project, file, red_root);

    let entries = match hir.kind(&db) {
      HirValueKind::Mapping(e) => e,
      _ => panic!("expected mapping"),
    };
    let content_hir = entries.iter().find(|(k, _)| k == "_content").unwrap().1;
    let parts = match content_hir.kind(&db) {
      HirValueKind::Markdown(p) => p,
      _ => panic!("expected markdown parts"),
    };
    let outer_closure = parts
      .iter()
      .find_map(|part| match part {
        InterpolatedPart::Expr(e) => Some(*e),
        _ => None,
      })
      .expect("expected interpolated closure expr");

    let inner_closure = match outer_closure.kind(&db) {
      HirValueKind::Closure { body, .. } => body,
      _ => panic!("expected outer closure"),
    };
    let innermost_body = match inner_closure.kind(&db) {
      HirValueKind::Closure { body, .. } => body,
      _ => panic!("expected inner closure"),
    };

    // Innermost scope is Fn scope for inner_closure
    let inner_scope = scope(&db, *innermost_body);
    let inner_scope_val = match inner_scope.kind(&db) {
      ScopeKind::Fn(_, _, val) => val,
      _ => panic!("expected Fn scope for inner closure"),
    };
    assert_eq!(inner_scope_val, *inner_closure);

    // Parent of inner scope is Fn scope for outer_closure
    let outer_scope = parent_scope(&db, inner_scope).value(&db).unwrap();
    let outer_scope_val = match outer_scope.kind(&db) {
      ScopeKind::Fn(_, _, val) => val,
      _ => panic!("expected Fn scope for outer closure"),
    };
    assert_eq!(outer_scope_val, outer_closure);

    // Parent of outer scope is File scope
    let file_scope = parent_scope(&db, outer_scope).value(&db).unwrap();
    assert!(matches!(
      file_scope.kind(&db),
      ScopeKind::File(p, f) if p == project && f == file
    ));
  }
}
