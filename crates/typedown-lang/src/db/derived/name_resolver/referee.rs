use typedown_macros::query_derived;

use crate::syntax::red::RedNode;
use crate::syntax::syntax_kind::SyntaxKind;

use crate::db::TypedownDatabase;
use crate::db::derived::get_vault_config::get_vault_config;
use crate::db::derived::name_resolver::file_symbol::{MaybeSymbol, file_symbol};
use crate::db::derived::name_resolver::members::members;
use crate::db::derived::name_resolver::scope::{parent_scope, scope};
use crate::db::types::{HirValue, HirValueKind};
use typedown_incremental::QueryDatabase;

#[query_derived]
pub fn referee(db: &TypedownDatabase, hir: HirValue) -> MaybeSymbol {
  match hir.kind(db) {
    HirValueKind::Ident(name) => resolve_ident(db, hir, name),
    HirValueKind::Call { callee, args } => resolve_call(db, hir, *callee, args),
    _ => MaybeSymbol::new(db, None),
  }
}

fn resolve_ident(db: &TypedownDatabase, hir: HirValue, name: String) -> MaybeSymbol {
  if is_dot_rhs(&hir.node(db)) {
    return MaybeSymbol::new(db, None);
  }

  let mut current_scope = scope(db, hir);
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

fn resolve_call(
  db: &TypedownDatabase,
  hir: HirValue,
  callee: HirValue,
  args: Vec<HirValue>,
) -> MaybeSymbol {
  if let HirValueKind::Ident(name) = callee.kind(db)
    && name == "fref"
    && let Some(first_arg) = args.first()
    && let HirValueKind::Str(path) = first_arg.kind(db)
  {
    let project = hir.project(db);
    let content_dir = get_vault_config(db, project).content_dir(db);
    let target_path = content_dir.join(&path);
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
  use crate::db::derived::parse_file::parse_file;
  use crate::db::fixtures::load_vault_fixture;
  use crate::db::types::{HirValue, HirValueKind, SymbolKind};
  use crate::db::utils::lower_file;

  use super::referee;

  // fref("path.td") resolves to the target file's resource symbol
  #[test]
  fn fref_resolves_to_target_resource_symbol() {
    let (db, project, file) = load_vault_fixture("evaluate/my_vault", "content/with_fref.td");
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
  fn fref_with_nonexistent_path_resolves_to_none() {
    let (db, project, file) = load_vault_fixture("evaluate/my_vault", "content/with_fref.td");

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
}
