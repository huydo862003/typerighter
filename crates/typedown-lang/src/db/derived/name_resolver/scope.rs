use typedown_macros::{query_derived, query_interned};

use crate::db::TypedownDatabase;
use crate::db::types::{File, Project, ProjectRedNode, RuntimeScope, Scope, ScopeKind};
use crate::syntax::syntax_kind::SyntaxKind;
use typedown_incremental::QueryDatabase;

#[query_interned]
pub struct MaybeScope {
  pub value: Option<Scope>,
}

#[query_derived]
pub fn scope<'db>(db: &'db TypedownDatabase, project_node: ProjectRedNode) -> Scope {
  let project = project_node.project;
  let file = project_node.file;

  // Walk successively up to find the closest closure and return file symbol once reaching root
  let mut curr = project_node.node.parent();
  while let Some(p) = curr {
    if p.kind() == SyntaxKind::ClosureExpr {
      return Scope::new(
        db,
        ScopeKind::Func(project, file, ProjectRedNode::new(project, file, p)),
      );
    }
    curr = p.parent();
  }

  Scope::new(db, ScopeKind::File(project, file))
}

#[query_derived]
pub fn parent_scope<'db>(db: &'db TypedownDatabase, scope: Scope) -> MaybeScope {
  match scope.kind(db) {
    ScopeKind::Builtin(_) => MaybeScope::new(db, None),
    ScopeKind::Project(project) => {
      MaybeScope::new(db, Some(Scope::new(db, ScopeKind::Builtin(project))))
    }
    ScopeKind::File(project, _) => {
      MaybeScope::new(db, Some(Scope::new(db, ScopeKind::Project(project))))
    }
    ScopeKind::Func(_project, _file, value) => MaybeScope::new(db, Some(self::scope(db, value))),
  }
}

#[query_derived]
pub fn get_builtin_runtime_scope<'db>(
  db: &'db TypedownDatabase,
  project: Project,
) -> RuntimeScope<'db> {
  RuntimeScope::new(
    db,
    Scope::new(db, ScopeKind::Builtin(project)),
    vec![],
    None,
  )
}

#[query_derived]
pub fn get_project_runtime_scope<'db>(
  db: &'db TypedownDatabase,
  project: Project,
) -> RuntimeScope<'db> {
  let parent = get_builtin_runtime_scope(db, project);
  RuntimeScope::new(
    db,
    Scope::new(db, ScopeKind::Project(project)),
    vec![],
    Some(Box::new(parent)),
  )
}

#[query_derived]
pub fn get_file_runtime_scope<'db>(
  db: &'db TypedownDatabase,
  project: Project,
  file: File,
) -> RuntimeScope<'db> {
  let parent = get_project_runtime_scope(db, project);
  RuntimeScope::new(
    db,
    Scope::new(db, ScopeKind::File(project, file)),
    vec![],
    Some(Box::new(parent)),
  )
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::db::derived::parse_file::parse_file;
  use crate::db::fixtures::load_vault_fixture;
  use crate::syntax::ast::{AstNode, ClosureExpr, SourceFile};
  use crate::syntax::parse::tests::helpers::parse;
  use crate::syntax::red::RedNode;
  use crate::syntax::syntax_kind::SyntaxKind;

  fn make_root_node(
    _db: &crate::db::TypedownDatabase,
    project: Project,
    file: File,
    src: &str,
  ) -> ProjectRedNode {
    let (root, _) = parse(src);
    let red_root = RedNode::new_root(root.as_node().unwrap().clone());
    ProjectRedNode::new(project, file, red_root)
  }

  // Walk a node tree recursively to find the first ClosureExpr's body
  fn find_first_closure_body(
    project: Project,
    file: File,
    node: &RedNode,
  ) -> Option<ProjectRedNode> {
    if let Some(closure) = ClosureExpr::cast(node.clone())
      && let Some(body) = closure.body()
    {
      return Some(ProjectRedNode::new(project, file, body.syntax().clone()));
    }
    for child in node.children() {
      if let Some(result) = find_first_closure_body(project, file, &child) {
        return Some(result);
      }
    }
    None
  }

  // Walk to find the Nth closure body (0-indexed)
  fn find_closure_bodies(
    project: Project,
    file: File,
    node: &RedNode,
    result: &mut Vec<ProjectRedNode>,
  ) {
    if node.kind() == SyntaxKind::ClosureExpr
      && let Some(closure) = ClosureExpr::cast(node.clone())
      && let Some(body) = closure.body()
    {
      result.push(ProjectRedNode::new(project, file, body.syntax().clone()));
    }
    for child in node.children() {
      find_closure_bodies(project, file, &child, result);
    }
  }

  #[test]
  fn scope_for_file_hir_returns_file_scope() {
    let (db, project, file) = load_vault_fixture("evaluate/my_vault", "valid_person.td");
    let root_node = parse_file(&db, project, file).ast(&db);

    let file_scope = scope(&db, root_node);
    assert!(matches!(file_scope.kind(&db), ScopeKind::File(..)));

    let parent = parent_scope(&db, file_scope).value(&db).unwrap();
    assert!(matches!(parent.kind(&db), ScopeKind::Project(..)));
  }

  #[test]
  fn scope_inside_closure_returns_fn_scope() {
    let (db, project, file) = load_vault_fixture("evaluate/my_vault", "valid_person.td");
    let root_node = make_root_node(
      &db,
      project,
      file,
      r#"---
fn: (a, b) -> a + b
---"#,
    );

    let entries = root_node.mapping_entries().expect("expected mapping");
    let fn_node = entries
      .iter()
      .find(|(k, _)| k == "fn")
      .map(|(_, v)| v.clone())
      .expect("expected fn field");
    let closure = ClosureExpr::cast(fn_node.node.clone()).expect("expected closure");
    let body_expr = closure.body().expect("expected closure body");
    let body_node = ProjectRedNode::new(project, file, body_expr.syntax().clone());

    let inner_scope = scope(&db, body_node);
    assert!(matches!(inner_scope.kind(&db), ScopeKind::Func(..)));

    let parent = parent_scope(&db, inner_scope).value(&db).unwrap();
    assert!(matches!(parent.kind(&db), ScopeKind::File(..)));
  }

  #[test]
  #[should_panic(expected = "Cannot construct static RuntimeScope<'db> for a nested closure scope")]
  fn nested_closure_runtime_scope_panics() {
    let (db, project, file) = load_vault_fixture("evaluate/my_vault", "valid_person.td");
    let root_node = make_root_node(
      &db,
      project,
      file,
      r#"---
fn: (x) -> (y) -> x + y
---"#,
    );

    // Find innermost closure body (depth 2)
    let mut bodies = vec![];
    find_closure_bodies(project, file, &root_node.node, &mut bodies);
    // bodies[0] = outer closure body (which is the inner closure expr)
    // bodies[1] = inner closure body (x + y)
    let inner_body = bodies
      .into_iter()
      .last()
      .expect("expected inner closure body");

    let inner_scope = scope(&db, inner_body);
    let _ = inner_scope.runtime_scope(&db);
  }

  #[test]
  fn scope_for_yaml_frontmatter_node() {
    let (db, project, file) = load_vault_fixture("evaluate/my_vault", "valid_person.td");
    let root_node = make_root_node(
      &db,
      project,
      file,
      r#"---
title: "Hello"
age: 30
---"#,
    );

    let entries = root_node.mapping_entries().expect("expected mapping");
    let title_node = entries
      .iter()
      .find(|(k, _)| k == "title")
      .map(|(_, v)| v.clone())
      .expect("expected title field");

    let yaml_scope = scope(&db, title_node);
    assert!(matches!(
      yaml_scope.kind(&db),
      ScopeKind::File(p, f) if p == project && f == file
    ));
  }

  #[test]
  fn scope_for_markdown_body_and_interpolation() {
    let (db, project, file) = load_vault_fixture("evaluate/my_vault", "valid_person.td");
    let root_node = make_root_node(
      &db,
      project,
      file,
      r#"---
title: "Hello"
---
Hello world ${(a, b) -> a + b}
"#,
    );

    // Find the closure body in the markdown body via tree walk
    let source_file = SourceFile::cast(root_node.node.clone()).expect("expected SourceFile");
    let md_body = source_file.body().expect("expected MdBody");
    let md_node = ProjectRedNode::new(project, file, md_body.syntax().clone());

    // The MdBody node itself has File scope
    let md_scope = scope(&db, md_node);
    assert!(matches!(
      md_scope.kind(&db),
      ScopeKind::File(p, f) if p == project && f == file
    ));

    // Find closure body inside markdown body
    let body_node = find_first_closure_body(project, file, &md_body.syntax().clone())
      .expect("expected closure in markdown body");
    let interp_fn_scope = scope(&db, body_node);
    assert!(matches!(interp_fn_scope.kind(&db), ScopeKind::Func(..)));
  }

  #[test]
  fn scope_nested_closure_in_yaml_frontmatter() {
    let (db, project, file) = load_vault_fixture("evaluate/my_vault", "valid_person.td");
    let root_node = make_root_node(
      &db,
      project,
      file,
      r#"---
outer: (a) -> (b) -> a + b
---"#,
    );

    let entries = root_node.mapping_entries().expect("expected mapping");
    let outer_node = entries
      .iter()
      .find(|(key, _)| key == "outer")
      .map(|(_, value)| value.clone())
      .expect("expected outer field");

    // outer_node is the outer closure expr
    let outer_closure = ClosureExpr::cast(outer_node.node.clone()).expect("expected outer closure");
    let outer_body_expr = outer_closure.body().expect("expected outer body");
    let outer_closure_node = outer_node.clone();

    // outer_body is the inner closure expr
    let inner_closure =
      ClosureExpr::cast(outer_body_expr.syntax().clone()).expect("expected inner closure expr");
    let inner_body_expr = inner_closure.body().expect("expected inner body");
    let inner_closure_node = ProjectRedNode::new(project, file, outer_body_expr.syntax().clone());
    let innermost_body = ProjectRedNode::new(project, file, inner_body_expr.syntax().clone());

    // Innermost scope is Fn scope for inner_closure
    let inner_scope = scope(&db, innermost_body);
    let inner_scope_value = match inner_scope.kind(&db) {
      ScopeKind::Func(_, _, value) => value,
      _ => panic!("expected Fn scope for inner closure"),
    };
    assert_eq!(inner_scope_value, inner_closure_node);

    // Parent of inner scope is Func scope for outer_closure
    let outer_scope = parent_scope(&db, inner_scope).value(&db).unwrap();
    let outer_scope_value = match outer_scope.kind(&db) {
      ScopeKind::Func(_, _, value) => value,
      _ => panic!("expected Fn scope for outer closure"),
    };
    assert_eq!(outer_scope_value, outer_closure_node);

    // Parent of outer scope is File scope
    let file_scope = parent_scope(&db, outer_scope).value(&db).unwrap();
    assert!(matches!(
      file_scope.kind(&db),
      ScopeKind::File(p, f) if p == project && f == file
    ));
  }

  #[test]
  fn scope_nested_closure_in_markdown_body() {
    let (db, project, file) = load_vault_fixture("evaluate/my_vault", "valid_person.td");
    let root_node = make_root_node(
      &db,
      project,
      file,
      r#"---
title: "Nested"
---
${(a) -> (b) -> a + b}
"#,
    );

    let source_file = SourceFile::cast(root_node.node.clone()).expect("expected SourceFile");
    let md_body = source_file.body().expect("expected MdBody");

    // Find all closure bodies in the markdown body tree
    // bodies[0] = outer closure body = inner ClosureExpr
    // bodies[1] = inner closure body = BinaryExpr (a + b)
    let mut bodies = vec![];
    find_closure_bodies(project, file, &md_body.syntax().clone(), &mut bodies);
    assert!(bodies.len() >= 2, "expected at least 2 closure bodies");

    // inner_closure_node = the inner ClosureExpr (body of outer closure)
    let inner_closure_node = bodies[0].clone();
    // outer_closure_node = the outer ClosureExpr (parent of inner_closure_node in the red tree)
    let outer_closure_node = inner_closure_node
      .node
      .parent()
      .map(|parent| ProjectRedNode::new(project, file, parent))
      .expect("inner closure should have outer closure as parent");
    let innermost_body = bodies[1].clone();

    // Innermost scope is Fn scope for inner_closure
    let inner_scope = scope(&db, innermost_body);
    let inner_scope_value = match inner_scope.kind(&db) {
      ScopeKind::Func(_, _, value) => value,
      _ => panic!("expected Fn scope for inner closure"),
    };
    assert_eq!(inner_scope_value, inner_closure_node);

    // Parent of inner scope is Fn scope for outer_closure
    let outer_scope = parent_scope(&db, inner_scope).value(&db).unwrap();
    let outer_scope_value = match outer_scope.kind(&db) {
      ScopeKind::Func(_, _, value) => value,
      _ => panic!("expected Fn scope for outer closure"),
    };
    assert_eq!(outer_scope_value, outer_closure_node);

    // Parent of outer scope is File scope
    let file_scope = parent_scope(&db, outer_scope).value(&db).unwrap();
    assert!(matches!(
      file_scope.kind(&db),
      ScopeKind::File(p, f) if p == project && f == file
    ));
  }
}
