use typedown_macros::query_derived;
use typedown_types::either::Either;

use crate::syntax::ast::{AstNode, CallExpr, IdentLit, StrLit};
use crate::syntax::red::RedNode;
use crate::syntax::syntax_kind::SyntaxKind;

use crate::db::TypedownDatabase;
use crate::db::derived::get_vault_config::get_vault_config;
use crate::db::derived::name_resolver::file_symbol::{MaybeSymbol, file_symbol};
use crate::db::derived::name_resolver::members::members;
use crate::db::derived::name_resolver::scope::{parent_scope, scope};
use crate::db::types::ProjectRedNode;
use crate::db::utils::resolve_fref_path;
use typedown_incremental::QueryDatabase;

#[query_derived]
pub fn referee<'db>(db: &'db TypedownDatabase, project_node: ProjectRedNode) -> MaybeSymbol {
  let node = project_node.node.clone();
  if let Some(ident) = IdentLit::cast(node.clone()) {
    let name = ident.value().unwrap_or_default();
    // null/bool bare values are not symbol references
    if !matches!(name.as_str(), "null" | "true" | "false") {
      return resolve_ident(db, project_node, name);
    }
  }
  if CallExpr::cast(node).is_some() {
    return resolve_call(db, project_node);
  }
  MaybeSymbol::new(db, None)
}

fn resolve_ident(db: &TypedownDatabase, project_node: ProjectRedNode, name: String) -> MaybeSymbol {
  if is_dot_rhs(&project_node.node) {
    return MaybeSymbol::new(db, None);
  }
  let mut current_scope = scope(db, project_node);
  loop {
    let result = members(db, current_scope);
    if let Some(sym) = result.members(db).get(&name) {
      return MaybeSymbol::new(db, Some(*sym));
    }
    match parent_scope(db, current_scope).value(db) {
      Some(parent) => current_scope = parent,
      None => return MaybeSymbol::new(db, None),
    }
  }
}

fn resolve_call(db: &TypedownDatabase, project_node: ProjectRedNode) -> MaybeSymbol {
  let Some(call) = CallExpr::cast(project_node.node.clone()) else {
    return MaybeSymbol::new(db, None);
  };
  let Some(callee) = call.callee() else {
    return MaybeSymbol::new(db, None);
  };
  let Some(callee_ident) = IdentLit::cast(callee.syntax().clone()) else {
    return MaybeSymbol::new(db, None);
  };
  if callee_ident.value().as_deref() != Some("fref") {
    return MaybeSymbol::new(db, None);
  }
  let args = call.args();
  let Some(first_arg) = args.first() else {
    return MaybeSymbol::new(db, None);
  };
  let Some(string_lit) = StrLit::cast(first_arg.syntax().clone()) else {
    return MaybeSymbol::new(db, None);
  };
  if string_lit.is_interpolated() {
    return MaybeSymbol::new(db, None);
  }
  let path: String = string_lit
    .fragments()
    .filter_map(|fragment| match fragment {
      Either::Left(s) => Some(s),
      _ => None,
    })
    .collect();
  if path.is_empty() {
    return MaybeSymbol::new(db, None);
  }
  let project = project_node.project;
  let root_dir = get_vault_config(db, project).root_dir(db);
  let file_dir = project_node
    .file
    .handle(db)
    .path()
    .and_then(|p| p.parent().map(|d| d.to_path_buf()))
    .unwrap_or_else(|| root_dir.clone());
  let target_path = resolve_fref_path(&path, &file_dir, &root_dir);
  match project.files(db).get(&target_path) {
    Some(&target_file) => file_symbol(db, project, target_file),
    None => MaybeSymbol::new(db, None),
  }
}

// Returns true if `node` is the right-hand operand of a dot binary expression
fn is_dot_rhs(node: &RedNode) -> bool {
  let Some(parent) = node.parent() else {
    return false;
  };
  if parent.kind() != SyntaxKind::BinaryExpr {
    return false;
  }
  let dot_op = parent
    .children()
    .find(|child| child.kind() == SyntaxKind::YamlOp && child.text() == ".");
  match dot_op {
    Some(op) => node.offset() > op.offset(),
    None => false,
  }
}

#[cfg(test)]
mod tests {
  use crate::db::derived::parse_file::parse_file;
  use crate::db::fixtures::load_vault_fixture;
  use crate::db::types::{ProjectRedNode, SymbolKind};
  use crate::syntax::ast::{AstNode, CallExpr, IdentLit};
  use crate::syntax::parse::tests::helpers::parse;
  use crate::syntax::red::RedNode;

  use super::referee;

  // walk the tree depth-first to find the first CallExpr node
  fn find_call_node(node: &RedNode) -> Option<RedNode> {
    if CallExpr::cast(node.clone()).is_some() {
      return Some(node.clone());
    }
    for child in node.children() {
      if let Some(found) = find_call_node(&child) {
        return Some(found);
      }
    }
    None
  }

  // walk the tree depth-first to find the first IdentLit matching target_name
  fn find_ident_node(node: &RedNode, target_name: &str) -> Option<RedNode> {
    if let Some(ident) = IdentLit::cast(node.clone())
      && ident.value().as_deref() == Some(target_name)
    {
      return Some(node.clone());
    }
    for child in node.children() {
      if let Some(found) = find_ident_node(&child, target_name) {
        return Some(found);
      }
    }
    None
  }

  // fref("path.td") resolves to the target file's resource symbol
  #[test]
  fn fref_resolves_to_target_resource_symbol() {
    let (db, project, file) = load_vault_fixture("evaluate/my_vault", "with_fref.td");
    let root = parse_file(&db, project, file).ast(&db).node.clone();
    let call_node = find_call_node(&root).expect("should find fref call in AST");
    let project_node = ProjectRedNode::new(project, file, call_node);
    let resolved = referee(&db, project_node);
    let symbol = resolved
      .value(&db)
      .expect("fref should resolve to a symbol");
    assert!(
      matches!(symbol.kind(&db), SymbolKind::UserDefinedResource(..)),
      "fref target should be a resource"
    );
    assert_eq!(symbol.name(&db), "valid_person");
  }

  // fref("./peer.td") from subdir/ resolves relative to the source file, not vault root
  #[test]
  fn fref_relative_path_resolves_from_source_file_directory() {
    let (db, project, source_file) =
      load_vault_fixture("evaluate/my_vault", "subdir/relative_fref.td");
    let root = parse_file(&db, project, source_file).ast(&db).node.clone();
    let call_node = find_call_node(&root).expect("should find fref call in AST");
    let project_node = ProjectRedNode::new(project, source_file, call_node);
    let resolved = referee(&db, project_node);
    let symbol = resolved
      .value(&db)
      .expect("fref(\"./peer.td\") from subdir/ should resolve to subdir/peer.td");
    assert_eq!(
      symbol.name(&db),
      "peer",
      "resolved symbol should be subdir/peer.td, not vault-root peer.td"
    );
  }

  // fref("nonexistent.td") resolves to None when the target file does not exist
  #[test]
  fn fref_with_nonexistent_path_resolves_to_none() {
    let (db, project, file) = load_vault_fixture("evaluate/my_vault", "with_fref.td");
    let (root, _) = parse(
      r#"---
value: fref("nonexistent.td")
---"#,
    );
    let red_root = RedNode::new_root(root.as_node().unwrap().clone());
    let call_node = find_call_node(&red_root).expect("should find fref call");
    let project_node = ProjectRedNode::new(project, file, call_node);
    let resolved = referee(&db, project_node);
    assert!(
      resolved.value(&db).is_none(),
      "nonexistent path should not resolve"
    );
  }

  #[test]
  fn referee_resolves_closure_param() {
    let (db, project, file) = load_vault_fixture("evaluate/my_vault", "valid_person.td");
    let (root, _) = parse(
      r#"---
fn: (a, b) -> a + b
---"#,
    );
    let red_root = RedNode::new_root(root.as_node().unwrap().clone());

    let a_node = find_ident_node(&red_root, "a").expect("should find ident 'a'");
    let project_node_a = ProjectRedNode::new(project, file, a_node);
    let resolved_a = referee(&db, project_node_a)
      .value(&db)
      .expect("a should resolve");
    assert_eq!(resolved_a.name(&db), "a");
    assert!(matches!(resolved_a.kind(&db), SymbolKind::FuncParam(..)));

    let b_node = find_ident_node(&red_root, "b").expect("should find ident 'b'");
    let project_node_b = ProjectRedNode::new(project, file, b_node);
    let resolved_b = referee(&db, project_node_b)
      .value(&db)
      .expect("b should resolve");
    assert_eq!(resolved_b.name(&db), "b");
    assert!(matches!(resolved_b.kind(&db), SymbolKind::FuncParam(..)));
  }

  #[test]
  fn referee_resolves_nested_closure_outer_param() {
    let (db, project, file) = load_vault_fixture("evaluate/my_vault", "valid_person.td");
    let (root, _) = parse(
      r#"---
fn: (a) -> (b) -> a + b
---"#,
    );
    let red_root = RedNode::new_root(root.as_node().unwrap().clone());

    let a_node = find_ident_node(&red_root, "a").expect("should find ident 'a'");
    let project_node_a = ProjectRedNode::new(project, file, a_node);
    let resolved_a = referee(&db, project_node_a)
      .value(&db)
      .expect("a should resolve");
    assert_eq!(resolved_a.name(&db), "a");
    assert!(matches!(resolved_a.kind(&db), SymbolKind::FuncParam(..)));
  }

  #[test]
  fn referee_unresolved_ident_in_closure_returns_none() {
    let (db, project, file) = load_vault_fixture("evaluate/my_vault", "valid_person.td");
    let (root, _) = parse(
      r#"---
fn: (a) -> a + unknown_var
---"#,
    );
    let red_root = RedNode::new_root(root.as_node().unwrap().clone());
    let unknown_node =
      find_ident_node(&red_root, "unknown_var").expect("should find 'unknown_var'");
    let project_node = ProjectRedNode::new(project, file, unknown_node);
    let resolved = referee(&db, project_node);
    assert!(
      resolved.value(&db).is_none(),
      "unknown_var in closure should resolve to None"
    );
  }

  #[test]
  fn referee_on_file_root_node_returns_none() {
    let (db, project, file) = load_vault_fixture("evaluate/my_vault", "valid_person.td");
    let project_node = parse_file(&db, project, file).ast(&db);
    let resolved = referee(&db, project_node);
    assert!(resolved.value(&db).is_none());
  }
}
