use typedown_macros::query_derived;

use crate::syntax::red::RedNode;
use crate::syntax::syntax_kind::SyntaxKind;

use crate::db::TypedownDatabase;
use crate::db::derived::get_vault_config::get_vault_config;
use crate::db::derived::name_resolver::file_symbol::{MaybeSymbol, file_symbol};
use crate::db::derived::name_resolver::members::{members, schema_members};
use crate::db::derived::name_resolver::scope::{parent_scope, scope};
use crate::db::types::{HirValue, HirValueKind};
use typedown_incremental::QueryDatabase;

#[query_derived]
pub fn referee<'db>(db: &'db TypedownDatabase, hir: HirValue<'db>) -> MaybeSymbol<'db> {
  match hir.kind(db) {
    HirValueKind::Ident(name) => resolve_ident(db, hir, name),
    HirValueKind::Call { callee, args } => resolve_call(db, hir, *callee, args),
    _ => MaybeSymbol::new(db, None),
  }
}

fn resolve_ident<'db>(
  db: &'db TypedownDatabase,
  hir: HirValue<'db>,
  name: String,
) -> MaybeSymbol<'db> {
  if is_dot_rhs(&hir.node(db)) {
    return MaybeSymbol::new(db, None);
  }

  // Fast path: check schema members before walking the full scope chain
  let schema_result = schema_members(db, hir.project(db));
  if let Some(sym) = schema_result.members(db).get(&name) {
    return MaybeSymbol::new(db, Some(*sym));
  }

  let mut current_scope = scope(db, hir);
  loop {
    let result = members(db, current_scope);
    if let Some(sym) = result.members(db).get(&name) {
      return MaybeSymbol::new(db, Some(*sym));
    }
    let parent = parent_scope(db, current_scope);
    match parent.value(db) {
      Some(parent) => current_scope = parent,
      None => return MaybeSymbol::new(db, None),
    }
  }
}

fn resolve_call<'db>(
  db: &'db TypedownDatabase,
  hir: HirValue<'db>,
  callee: HirValue<'db>,
  args: Vec<HirValue<'db>>,
) -> MaybeSymbol<'db> {
  if let HirValueKind::Ident(name) = callee.kind(db)
    && name == "fref"
    && let Some(first_arg) = args.first()
    && let HirValueKind::Str(path) = first_arg.kind(db)
  {
    let project = hir.project(db);
    let root_dir = get_vault_config(db, project).root_dir(db);
    let target_path = root_dir.join(&path);
    if let Some(&target_file) = project.files(db).get(&target_path) {
      return file_symbol(db, project, target_file);
    }
  }
  MaybeSymbol::new(db, None)
}

// Returns true if `node` is the right-hand operand of a dot binary expression.
fn is_dot_rhs(node: &RedNode) -> bool {
  let parent = match node.parent() {
    Some(parent) => parent,
    None => return false,
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
  use crate::db::TypedownDatabase;
  use crate::db::derived::hir::lower_node;
  use crate::db::derived::parse_file::parse_file;
  use crate::db::fixtures::load_vault_fixture;
  use crate::db::types::{HirValue, HirValueKind, SymbolKind};
  use crate::db::utils::lower_file;
  use crate::syntax::parse::tests::helpers::parse;
  use crate::syntax::red::RedNode;

  use super::referee;

  // fref("path.td") resolves to the target file's resource symbol
  #[test]
  fn fref_resolves_to_target_resource_symbol() {
    let (db, project, file) = load_vault_fixture("evaluate/my_vault", "with_fref.td");
    let (hir, _) = lower_file(&db, project, file);
    let hir = hir.expect("should lower file");

    let friend_hir = match hir.kind(&db) {
      HirValueKind::Mapping(entries) => entries.into_iter().find(|(k, _)| k == "friend").unwrap().1,
      _ => panic!("expected mapping"),
    };

    let resolved = referee(&db, friend_hir);
    let symbol = resolved
      .value(&db)
      .expect("fref should resolve to a symbol");
    assert!(
      matches!(symbol.kind(&db), SymbolKind::UserDefinedResource(..)),
      "fref target should be a resource"
    );
    assert_eq!(symbol.name(&db), "valid_person");
  }

  // fref("nonexistent.td") resolves to None when the target file does not exist
  #[test]
  fn fref_with_nonexistent_path_resolves_to_none<'db>() {
    let (db, project, file) = load_vault_fixture("evaluate/my_vault", "with_fref.td");

    // Construct a fref("nonexistent.td") HIR node manually
    let node = parse_file(&db, project, file).ast(&db);
    let callee = HirValue::new(
      &db,
      project,
      file,
      node.clone(),
      HirValueKind::Ident("fref".to_string()),
      vec![],
    );
    let arg = HirValue::new(
      &db,
      project,
      file,
      node.clone(),
      HirValueKind::Str("nonexistent.td".to_string()),
      vec![],
    );
    let call_hir = HirValue::new(
      &db,
      project,
      file,
      node,
      HirValueKind::Call {
        callee: callee.into(),
        args: vec![arg],
      },
      vec![],
    );

    let resolved = referee(&db, call_hir);
    assert!(
      resolved.value(&db).is_none(),
      "nonexistent path should not resolve"
    );
  }

  /// Recursively searches the HIR value tree for the first `Ident` node matching `target_name`
  fn find_ident<'db>(
    db: &'db TypedownDatabase,
    root: HirValue<'db>,
    target_name: &str,
  ) -> Option<HirValue<'db>> {
    if let HirValueKind::Ident(name) = root.kind(db)
      && name == target_name
    {
      return Some(root);
    }
    match root.kind(db) {
      HirValueKind::Mapping(entries) => {
        for (_, v) in entries {
          if let Some(found) = find_ident(db, v, target_name) {
            return Some(found);
          }
        }
      }
      HirValueKind::Sequence(items) => {
        for item in items {
          if let Some(found) = find_ident(db, item, target_name) {
            return Some(found);
          }
        }
      }
      HirValueKind::Binary { left, right, .. } => {
        if let Some(found) = find_ident(db, *left, target_name) {
          return Some(found);
        }
        if let Some(found) = find_ident(db, *right, target_name) {
          return Some(found);
        }
      }
      HirValueKind::Closure { body, .. } => {
        if let Some(found) = find_ident(db, *body, target_name) {
          return Some(found);
        }
      }
      _ => {}
    }
    None
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
    let hir = lower_node(&db, project, file, red_root);

    let a_hir = find_ident(&db, hir, "a").expect("should find Ident('a')");
    let resolved_a = referee(&db, a_hir).value(&db).expect("a should resolve");
    assert_eq!(resolved_a.name(&db), "a");
    assert!(matches!(resolved_a.kind(&db), SymbolKind::FnParam(..)));

    let b_hir = find_ident(&db, hir, "b").expect("should find Ident('b')");
    let resolved_b = referee(&db, b_hir).value(&db).expect("b should resolve");
    assert_eq!(resolved_b.name(&db), "b");
    assert!(matches!(resolved_b.kind(&db), SymbolKind::FnParam(..)));
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
    let hir = lower_node(&db, project, file, red_root);

    let a_hir = find_ident(&db, hir, "a").expect("should find Ident('a')");
    let resolved_a = referee(&db, a_hir).value(&db).expect("a should resolve");
    assert_eq!(resolved_a.name(&db), "a");
    assert!(matches!(resolved_a.kind(&db), SymbolKind::FnParam(..)));

    let b_hir = find_ident(&db, hir, "b").expect("should find Ident('b')");
    let resolved_b = referee(&db, b_hir).value(&db).expect("b should resolve");
    assert_eq!(resolved_b.name(&db), "b");
    assert!(matches!(resolved_b.kind(&db), SymbolKind::FnParam(..)));
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
    let hir = lower_node(&db, project, file, red_root);

    let unknown_hir =
      find_ident(&db, hir, "unknown_var").expect("should find Ident('unknown_var')");
    let resolved = referee(&db, unknown_hir);
    assert!(
      resolved.value(&db).is_none(),
      "unknown_var in closure should resolve to None"
    );
  }
}
