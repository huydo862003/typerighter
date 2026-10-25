use typedown_macros::query_derived;

use crate::syntax::diagnostic::*;

use crate::db::TypedownDatabase;
use crate::db::derived::get_vault_config::get_vault_config;
use crate::db::derived::name_resolver::referee::referee;
use crate::db::types::{File, Project, ProjectRedNode, ResolveResult};
use crate::syntax::ast::{AstNode, Expr, IdentLit, InterpFragment, SourceFile, StrLit, YamlOpKind};
use crate::syntax::red::RedNode;
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
  diagnostics: &mut Vec<TdDiagnostic>,
) {
  let project = project_node.project;
  let file = project_node.file;
  let node = project_node.node.clone();

  // SourceFile: process frontmatter via mapping entries, then also walk the markdown body
  if let Some(source_file) = SourceFile::cast(node.clone()) {
    if let Some(frontmatter) = source_file.frontmatter() {
      collect_unresolved(
        db,
        ProjectRedNode::new(project, file, frontmatter.syntax().clone()),
        diagnostics,
      );
    }
    if let Some(body) = source_file.body() {
      collect_unresolved_in_md_body(db, project, file, body.syntax().clone(), diagnostics);
    }
    return;
  }

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
      diagnostics.push(
        TdDiagnosticKind::UnresolvedIdentifier {
          name: project_node.node.text(),
        }
        .into_error(trimmed_offset, trimmed_offset + trimmed_len),
      );
    }
    return;
  }

  // Interpolated string: recurse into embedded expressions
  if let Some(string_lit) = StrLit::cast(node.clone()) {
    for fragment in string_lit.fragments() {
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
      // fref() is a builtin that references another file - check it resolves
      let is_fref = call
        .callee()
        .and_then(|callee| IdentLit::cast(callee.syntax().clone()))
        .and_then(|id| id.value())
        .map(|value| value == "fref")
        .unwrap_or(false);
      if is_fref {
        let resolved = referee(db, project_node.clone());
        if resolved.value(db).is_none() {
          // Only emit when the path is a plain non-empty string (interpolated/missing args are already implicitly invalid and caught by other diagnostics)
          if let Some(first_arg) = call.args().first()
            && let Some(string_lit) = StrLit::cast(first_arg.syntax().clone())
            && !string_lit.is_interpolated()
            && let Some(path) = string_lit.plain_value()
            && !path.is_empty()
          {
            let (trimmed_offset, trimmed_len) = project_node.node.trimmed_range();
            diagnostics.push(
              TdDiagnosticKind::UnresolvedFileRef { path }
                .into_error(trimmed_offset, trimmed_offset + trimmed_len),
            );
          }
        }
        return;
      }

      // Normal call expression
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
      if let Some(expression_part) = index.expr() {
        collect_unresolved(
          db,
          ProjectRedNode::new(project, file, expression_part.syntax().clone()),
          diagnostics,
        );
      }
      for index in index.indices() {
        collect_unresolved(
          db,
          ProjectRedNode::new(project, file, index.syntax().clone()),
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

// Walk an MdBody subtree and collect unresolved references inside interpolations
fn collect_unresolved_in_md_body(
  db: &TypedownDatabase,
  project: Project,
  file: File,
  node: RedNode,
  diagnostics: &mut Vec<TdDiagnostic>,
) {
  if let Some(interp) = InterpFragment::cast(node.clone()) {
    if let Some(expr) = interp.expr() {
      collect_unresolved(
        db,
        ProjectRedNode::new(project, file, expr.syntax().clone()),
        diagnostics,
      );
    }
    return;
  }
  for child in node.children() {
    collect_unresolved_in_md_body(db, project, file, child, diagnostics);
  }
}

// Check that each _imports path resolves to an existing file
fn check_imports(
  db: &TypedownDatabase,
  mapping_node: ProjectRedNode,
  imports_node: ProjectRedNode,
  diagnostics: &mut Vec<TdDiagnostic>,
) {
  let Some(entries) = imports_node.mapping_entries() else {
    return;
  };
  let project = mapping_node.project;
  let root_dir = get_vault_config(db, project).root_dir(db);

  for (_, path_node) in entries {
    let Some(path) = StrLit::cast(path_node.node.clone()).and_then(|string| string.plain_value())
    else {
      continue;
    };
    let target_path = root_dir.join(&path);
    if !project.files(db).contains_key(&target_path) {
      let (start, len) = path_node.node.trimmed_range();
      diagnostics.push(TdDiagnosticKind::UnresolvedImport { path }.into_error(start, start + len));
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::db::derived::parse_file::parse_file;
  use crate::db::fixtures::load_vault_fixture;
  use crate::db::types::File;
  use crate::db::types::Project;
  use crate::syntax::parse::tests::helpers::parse;
  use crate::syntax::red::RedNode;

  fn make_root_node(
    _db: &TypedownDatabase,
    project: Project,
    file: File,
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

    let result = resolve(&db, project_node);
    let diagnostics = result.diagnostics(&db);
    assert!(
      diagnostics.is_empty(),
      "closure parameters 'a' and 'b' should resolve cleanly, got: {:?}",
      diagnostics
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

    let result = resolve(&db, project_node);
    let diagnostics = result.diagnostics(&db);
    assert_eq!(diagnostics.len(), 1);
    match &diagnostics[0] {
      TdDiagnostic {
        kind: TdDiagnosticKind::UnresolvedIdentifier { name, .. },
        ..
      } => {
        assert_eq!(name.trim(), "missing_variable");
      }
      diagnostic => panic!(
        "expected UnresolvedIdentifier diagnostic, got: {:?}",
        diagnostic
      ),
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

    let result = resolve(&db, project_node);
    let diagnostics = result.diagnostics(&db);
    assert!(
      diagnostics.iter().any(|diagnostic| matches!(
        diagnostic,
        TdDiagnostic {
          kind: TdDiagnosticKind::UnresolvedImport { .. },
          ..
        }
      )),
      "should have UnresolvedImport diagnostic, got: {:?}",
      diagnostics
    );
  }

  // fref("nonexistent.td") in YAML frontmatter should emit UnresolvedFileRef
  #[test]
  fn resolve_fref_nonexistent_in_yaml_emits_diagnostic() {
    let (db, project, file) = load_vault_fixture("evaluate/my_vault", "valid_person.td");
    let project_node = make_root_node(
      &db,
      project,
      file,
      r#"---
value: fref("nonexistent_file_that_does_not_exist.td")
---"#,
    );

    let result = resolve(&db, project_node);
    let diagnostics = result.diagnostics(&db);
    assert!(
      diagnostics.iter().any(|diagnostic| matches!(
        diagnostic,
        TdDiagnostic {
          kind: TdDiagnosticKind::UnresolvedFileRef { .. },
          ..
        }
      )),
      "fref() in YAML with nonexistent path should emit UnresolvedFileRef, got: {:?}",
      diagnostics
    );
  }

  // fref("valid_person.td") in YAML frontmatter should not emit any diagnostic
  #[test]
  fn resolve_fref_existing_in_yaml_is_clean() {
    let (db, project, file) = load_vault_fixture("evaluate/my_vault", "with_fref.td");
    let root = parse_file(&db, project, file).ast(&db);
    let result = resolve(&db, root);
    let diagnostics = result.diagnostics(&db);
    assert!(
      !diagnostics.iter().any(|diagnostic| matches!(
        diagnostic,
        TdDiagnostic {
          kind: TdDiagnosticKind::UnresolvedFileRef { .. },
          ..
        }
      )),
      "fref() in YAML with existing path should not emit UnresolvedFileRef, got: {:?}",
      diagnostics
    );
  }

  // fref("nonexistent.td") in a markdown body interpolation should emit UnresolvedFileRef
  #[test]
  fn resolve_fref_nonexistent_in_markdown_body_emits_diagnostic() {
    let (db, project, file) = load_vault_fixture("evaluate/my_vault", "valid_person.td");
    let project_node = make_root_node(
      &db,
      project,
      file,
      r#"---
---
${fref("nonexistent_file_that_does_not_exist.td")}"#,
    );

    let result = resolve(&db, project_node);
    let diagnostics = result.diagnostics(&db);
    assert!(
      diagnostics.iter().any(|diagnostic| matches!(
        diagnostic,
        TdDiagnostic {
          kind: TdDiagnosticKind::UnresolvedFileRef { .. },
          ..
        }
      )),
      "fref() in markdown body with nonexistent path should emit UnresolvedFileRef, got: {:?}",
      diagnostics
    );
  }
}
