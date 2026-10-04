//! Tracked query for the expected (top-down) type of a HIR value
// I think this is the idea of bidirectional typechecking

use crate::db::TypedownDatabase;
use std::collections::{BTreeMap, HashSet};

use crate::db::derived::evaluate::evaluate_typ::evaluate_typ;
use crate::db::derived::get_builtin_typs::{
  get_bool_typ, get_func_typ, get_number_typ, get_sum_typ,
};
use crate::db::derived::hir::lower_node;
use crate::db::derived::name_resolver::referee::referee;
use crate::db::derived::typechecker::actual_node_typ::actual_node_typ;
use crate::db::types::TdSchemaTyp;
use crate::db::types::derived::obj_system::TdStaticTyp;
use crate::db::types::typecheck::is_subtype_of;
use crate::db::types::{
  File, FileRedNode, FuncSignature, HirValue, LazyTyp, Project, StaticAccessPath, Symbol,
  TdProductTyp, TdTypEnum, TypResult,
};
use crate::db::utils::is_schemaless_file;
use crate::syntax::ast::{
  AstNode, BinaryExpr, CallExpr, ClosureExpr, Expr, ParenExpr, PrefixExpr, YamlOpKind,
};
use crate::syntax::red::RedNode;
use crate::syntax::syntax_kind::SyntaxKind;
use typedown_incremental::QueryDatabase;
use typedown_macros::query_derived;

use crate::db::types::PathStep;

/// Result of walking up from a node to the nearest _typ anchor
struct AnchorResult<'db> {
  symbol: Symbol<'db>,
  typ: TdTypEnum<'db>,
  path: Vec<(PathStep, RedNode)>,
}

#[query_derived(no_hash)]
pub fn expected_node_typ<'db>(db: &'db TypedownDatabase, hir: HirValue<'db>) -> TypResult<'db> {
  let project = hir.project(db);
  let file = hir.node(db).owner_file;
  let node = hir.node(db).node.clone();

  // Expression nodes propagate expected types through expression structure
  if !is_top_level(&node) {
    return get_expected_expr_typ(db, hir);
  }

  let anchor = match collect_path_to_anchor(db, project, file, &node) {
    Some(result) => result,
    None => {
      // If the file has no _typ, return schemaless type
      if is_schemaless_file(db, project, file) {
        return simple_schemaless_result(db);
      }
      return TypResult::new(db, None, vec![]);
    }
  };

  // Traverse down the type structure following the path
  let mut current_typ = LazyTyp::eager(anchor.typ);

  for (step, step_node) in &anchor.path {
    let step_hir = lower_node(db, project, FileRedNode::new(file, step_node.clone()));

    // Resolve Sum ambiguity using actual_node_typ
    let resolved_typ = resolve_lazy_typ(db, &current_typ, step_hir);

    current_typ = match step {
      PathStep::Field(name) => match traverse_field(db, &resolved_typ, name) {
        Some(lazy) => lazy,
        None => return TypResult::new(db, None, vec![]),
      },
      PathStep::Index => match traverse_index(db, &resolved_typ) {
        Some(lazy) => lazy,
        None => return TypResult::new(db, None, vec![]),
      },
    };
  }

  TypResult::new(db, current_typ.resolve(db), vec![])
}

// Propagate expected type through expression structure
fn get_expected_expr_typ<'db>(db: &'db TypedownDatabase, hir: HirValue<'db>) -> TypResult<'db> {
  let node = hir.node(db).node.clone();
  let project = hir.project(db);
  let file = hir.node(db).owner_file;

  let parent = match node.parent() {
    Some(parent) => parent,
    None => return TypResult::new(db, None, vec![]),
  };

  // CallExpr: first Expr child is callee, rest are args
  if let Some(call) = CallExpr::cast(parent.clone()) {
    let expr_children: Vec<Expr> = parent.children().filter_map(Expr::cast).collect();

    // Is it the callee (first Expr child)?
    if expr_children.first().is_some_and(|c| *c.syntax() == node) {
      return get_expected_call_callee_typ(db, project, file, &call);
    }

    // Is it an arg? Find its position (0-indexed, skipping callee)
    let arg_index = expr_children
      .iter()
      .skip(1)
      .position(|c| *c.syntax() == node);
    if let Some(idx) = arg_index {
      return get_expected_call_arg_typ(db, project, file, &call, idx);
    }
  }

  // ClosureExpr: body gets the return type from expected(closure)
  if let Some(closure) = ClosureExpr::cast(parent.clone())
    && closure.body().is_some_and(|b| *b.syntax() == node)
  {
    return get_expected_closure_body_typ(db, project, file, &closure);
  }

  // ParenExpr: transparent, forward expected type from parent
  if ParenExpr::cast(parent.clone()).is_some() {
    let paren_hir = lower_node(db, project, FileRedNode::new(file, parent));
    return expected_node_typ(db, paren_hir);
  }

  // BinaryExpr: propagate expected types to operands based on operator
  if let Some(binary) = BinaryExpr::cast(parent.clone())
    && let Some(op) = binary.op().and_then(|o| o.kind())
  {
    return get_expected_binary_operand_typ(db, &op);
  }

  // PrefixExpr: propagate expected type to operand based on operator
  if let Some(prefix) = PrefixExpr::cast(parent.clone())
    && let Some(op) = prefix.op().and_then(|o| o.kind())
  {
    return get_expected_prefix_operand_typ(db, &op);
  }

  // No expression propagation rule matched, fall back to actual type
  actual_node_typ(db, hir)
}

// expected(arg_i) = param_i from actual(callee)
// Macros have no TdFuncType so this returns None for macro args (they validate internally)
fn get_expected_call_arg_typ<'db>(
  db: &'db TypedownDatabase,
  project: Project,
  file: File,
  call: &CallExpr,
  arg_index: usize,
) -> TypResult<'db> {
  let callee_node = match call.callee() {
    Some(callee) => callee,
    None => return TypResult::new(db, None, vec![]),
  };
  let callee_hir = lower_node(
    db,
    project,
    FileRedNode::new(file, callee_node.syntax().clone()),
  );
  let callee_typ = actual_node_typ(db, callee_hir).typ(db);

  if let Some(TdTypEnum::TdFuncTyp(func)) = callee_typ {
    let params = func.signature(db).params(db);
    if let Some(param_typ) = params.get(arg_index) {
      return TypResult::new(db, Some(param_typ.clone()), vec![]);
    }
  }

  TypResult::new(db, None, vec![])
}

// expected(callee) = fn(actual(arg_i)...) -> expected(call)
fn get_expected_call_callee_typ<'db>(
  db: &'db TypedownDatabase,
  project: Project,
  file: File,
  call: &CallExpr,
) -> TypResult<'db> {
  let call_hir = lower_node(db, project, FileRedNode::new(file, call.syntax().clone()));
  let ret = match expected_node_typ(db, call_hir).typ(db) {
    Some(typ) => typ,
    None => return TypResult::new(db, None, vec![]),
  };

  let mut param_typs = vec![];
  for arg in call.args() {
    let arg_hir = lower_node(db, project, FileRedNode::new(file, arg.syntax().clone()));
    match actual_node_typ(db, arg_hir).typ(db) {
      Some(typ) => param_typs.push(typ),
      None => return TypResult::new(db, None, vec![]),
    }
  }

  let signature = FuncSignature::new(db, param_typs, ret);
  let func_typ = get_func_typ(db, signature);
  TypResult::new(db, Some(func_typ.into()), vec![])
}

// expected(body) = return type from expected(closure)
fn get_expected_closure_body_typ<'db>(
  db: &'db TypedownDatabase,
  project: Project,
  file: File,
  closure: &ClosureExpr,
) -> TypResult<'db> {
  let closure_hir = lower_node(
    db,
    project,
    FileRedNode::new(file, closure.syntax().clone()),
  );
  let expected = expected_node_typ(db, closure_hir).typ(db);
  match expected {
    Some(TdTypEnum::TdFuncTyp(func)) => {
      let ret = func.signature(db).ret(db);
      TypResult::new(db, Some(ret), vec![])
    }
    _ => TypResult::new(db, None, vec![]),
  }
}

fn get_expected_binary_operand_typ<'db>(
  db: &'db TypedownDatabase,
  op: &YamlOpKind,
) -> TypResult<'db> {
  match op {
    // Arithmetic and power expect number
    YamlOpKind::Plus
    | YamlOpKind::Minus
    | YamlOpKind::Mul
    | YamlOpKind::Div
    | YamlOpKind::Mod
    | YamlOpKind::Pow => TypResult::new(db, Some(get_number_typ(db).into()), vec![]),
    // Logical expects boolean
    YamlOpKind::And | YamlOpKind::Or => TypResult::new(db, Some(get_bool_typ(db).into()), vec![]),
    // Comparison and dot access have no operand constraint
    _ => TypResult::new(db, None, vec![]),
  }
}

fn get_expected_prefix_operand_typ<'db>(
  db: &'db TypedownDatabase,
  op: &YamlOpKind,
) -> TypResult<'db> {
  match op {
    YamlOpKind::Plus | YamlOpKind::Minus => {
      TypResult::new(db, Some(get_number_typ(db).into()), vec![])
    }
    YamlOpKind::Tilde => TypResult::new(db, Some(get_bool_typ(db).into()), vec![]),
    _ => TypResult::new(db, None, vec![]),
  }
}

/// Check if a node is top-level in the YAML structure
fn is_top_level(node: &RedNode) -> bool {
  let parent = match node.parent() {
    Some(parent) => parent,
    None => return true,
  };
  match parent.kind() {
    SyntaxKind::YamlFrontmatter | SyntaxKind::SourceFile => true,
    SyntaxKind::YamlMapping
    | SyntaxKind::YamlMappingEntry
    | SyntaxKind::YamlMappingEntryValue
    | SyntaxKind::YamlMappingEntryKey
    | SyntaxKind::YamlSequence
    | SyntaxKind::YamlSequenceItem
    | SyntaxKind::ListLit
    | SyntaxKind::ListItem
    | SyntaxKind::DictLit
    | SyntaxKind::DictEntry
    | SyntaxKind::DictEntryKey
    | SyntaxKind::DictEntryValue => is_top_level(&parent),
    _ => false,
  }
}

/// Get the static access path from the nearest _typ anchor to the target node
pub fn static_access_path<'db>(
  db: &'db TypedownDatabase,
  project: Project,
  file: File,
  node: &RedNode,
) -> Option<StaticAccessPath<'db>> {
  let anchor = collect_path_to_anchor(db, project, file, node)?;
  let steps = anchor.path.into_iter().map(|(step, _)| step).collect();
  Some(StaticAccessPath {
    owner: anchor.symbol,
    steps,
  })
}

/// Walk up from target to the nearest _typ anchor, collecting path steps
fn collect_path_to_anchor<'db>(
  db: &'db TypedownDatabase,
  project: Project,
  file: File,
  target: &RedNode,
) -> Option<AnchorResult<'db>> {
  let mut path = vec![];
  let mut current = target.clone();

  loop {
    let parent = match current.parent() {
      Some(parent) => parent,
      None => {
        return None;
      }
    };

    match parent.kind() {
      SyntaxKind::YamlMappingEntryValue => {
        let entry = parent.parent()?;
        if entry.kind() != SyntaxKind::YamlMappingEntry {
          return None;
        }
        let key_name = entry
          .children()
          .find(|child| child.kind() == SyntaxKind::YamlMappingEntryKey)?
          .text()
          .trim()
          .to_string();

        if key_name == "_type" {
          return None;
        }

        path.push((PathStep::Field(key_name.clone()), current.clone()));

        let mapping = entry.parent()?;
        if mapping.kind() != SyntaxKind::YamlMapping {
          return None;
        }

        // Anchor found
        if let Some((symbol, schema_typ)) = resolve_typ_anchor(db, project, file, &mapping) {
          path.reverse();
          return Some(AnchorResult {
            symbol,
            typ: schema_typ,
            path,
          });
        }

        current = mapping;
      }
      SyntaxKind::YamlSequenceItem => {
        let sequence = parent.parent()?;
        if sequence.kind() != SyntaxKind::YamlSequence {
          return None;
        }
        path.push((PathStep::Index, current.clone()));
        current = sequence;
      }
      SyntaxKind::ListItem => {
        let list = parent.parent()?;
        if list.kind() != SyntaxKind::ListLit {
          return None;
        }
        path.push((PathStep::Index, current.clone()));
        current = list;
      }
      SyntaxKind::DictEntryValue => {
        let entry = parent.parent()?;
        if entry.kind() != SyntaxKind::DictEntry {
          return None;
        }
        let key_name = entry
          .children()
          .find(|child| child.kind() == SyntaxKind::DictEntryKey)?
          .text()
          .trim()
          .to_string();

        if key_name == "_type" {
          return None;
        }

        path.push((PathStep::Field(key_name), current.clone()));

        let dict = entry.parent()?;
        if dict.kind() != SyntaxKind::DictLit {
          return None;
        }

        // Anchor found
        if let Some((symbol, schema_typ)) = resolve_typ_anchor(db, project, file, &dict) {
          path.reverse();
          return Some(AnchorResult {
            symbol,
            typ: schema_typ,
            path,
          });
        }

        current = dict;
      }
      SyntaxKind::YamlFrontmatter | SyntaxKind::SourceFile => {
        return None;
      }
      _ => {
        current = parent;
      }
    }
  }
}

/// Resolve the _typ field in a mapping to its symbol and type
fn resolve_typ_anchor<'db>(
  db: &'db TypedownDatabase,
  project: Project,
  file: File,
  mapping: &RedNode,
) -> Option<(Symbol<'db>, TdTypEnum<'db>)> {
  for entry in mapping.children() {
    let entry_kind = entry.kind();
    if entry_kind != SyntaxKind::YamlMappingEntry && entry_kind != SyntaxKind::DictEntry {
      continue;
    }
    let key_kind = if entry_kind == SyntaxKind::YamlMappingEntry {
      SyntaxKind::YamlMappingEntryKey
    } else {
      SyntaxKind::DictEntryKey
    };
    let value_kind = if entry_kind == SyntaxKind::YamlMappingEntry {
      SyntaxKind::YamlMappingEntryValue
    } else {
      SyntaxKind::DictEntryValue
    };

    let key = entry.children().find(|child| child.kind() == key_kind)?;
    if key.text().trim() != "_type" {
      continue;
    }
    let entry_value = entry.children().find(|child| child.kind() == value_kind)?;
    let value_expr = entry_value.children().find_map(Expr::cast)?;
    let value_hir = lower_node(
      db,
      project,
      FileRedNode::new(file, value_expr.syntax().clone()),
    );
    let symbol = referee(db, value_hir).value(db)?;
    let typ = evaluate_typ(db, symbol).typ(db)?;
    return Some((symbol, typ));
  }
  None
}

/// Resolve a Sum type by picking the most specific matching arm
fn resolve_lazy_typ<'db>(
  db: &'db TypedownDatabase,
  lazy: &LazyTyp<'db>,
  hir: HirValue<'db>,
) -> LazyTyp<'db> {
  let Some(typ) = lazy.resolve(db) else {
    return lazy.clone();
  };
  if let TdTypEnum::TdSumTyp(sum) = &typ {
    let members = sum.members(db);
    // For nullable types, strip null and return the non-null arm without calling
    // actual_node_typ to avoid re-entering the type query for the same node
    let non_null: Vec<_> = members
      .iter()
      .filter(|member| !member.resolve(db).is_some_and(|t| t.is_td_null_typ()))
      .cloned()
      .collect();
    if non_null.len() == 1 {
      return non_null.into_iter().next().unwrap();
    }
    if let Some(picked) = pick_most_specific_arm(db, &members, hir) {
      return picked;
    }
  }
  lazy.clone()
}

/// Pick the matching arms for the actual value
fn pick_most_specific_arm<'db>(
  db: &'db TypedownDatabase,
  arms: &HashSet<LazyTyp<'db>>,
  hir: HirValue<'db>,
) -> Option<LazyTyp<'db>> {
  let actual_typ = actual_node_typ(db, hir).typ(db)?;

  let matching: Vec<_> = arms
    .iter()
    .filter(|arm| {
      arm
        .resolve(db)
        .is_some_and(|t| is_subtype_of(db, &actual_typ, &t))
    })
    .cloned()
    .collect();

  if matching.is_empty() {
    return None;
  }
  if matching.len() == 1 {
    return Some(matching.into_iter().next().unwrap());
  }

  Some(LazyTyp::eager(get_sum_typ(db, matching).into()))
}

/// Look up a field in the resolved type
fn traverse_field<'db>(
  db: &'db TypedownDatabase,
  lazy: &LazyTyp<'db>,
  field_name: &str,
) -> Option<LazyTyp<'db>> {
  if let Some(builtin_typ) = TdSchemaTyp::builtin_field_typ(db, field_name) {
    return Some(LazyTyp::eager(builtin_typ));
  }
  let typ = lazy.resolve(db)?;
  if let Some(field_typ) = typ.get_owned_field_typ(db, field_name) {
    return Some(LazyTyp::eager(field_typ));
  }
  // Dict: any key maps to the value type
  if let Some(dict) = typ.as_td_dict_typ().copied()
    && let Some(value_typ) = dict.value(db).and_then(|l| l.resolve(db))
  {
    return Some(LazyTyp::eager(value_typ));
  }
  None
}

/// Get the element type from a list
fn traverse_index<'db>(db: &'db TypedownDatabase, lazy: &LazyTyp<'db>) -> Option<LazyTyp<'db>> {
  let typ = lazy.resolve(db)?;
  let list = *typ.as_td_list_typ()?;
  list.element(db)
}

fn simple_schemaless_result<'db>(db: &'db TypedownDatabase) -> TypResult<'db> {
  TypResult::new(
    db,
    Some(TdProductTyp::new(db, None, BTreeMap::new()).into()),
    vec![],
  )
}

#[cfg(test)]
mod tests {
  use crate::db::TypedownDatabase;
  use crate::db::types::derived::obj_system::TdStaticTyp;

  use crate::db::{
    derived::typechecker::expected_node_typ::expected_node_typ,
    fixtures::load_vault_fixture,
    types::{File, HirValue, HirValueKind, Project},
    utils::lower_file,
  };

  fn get_field_hir<'a>(
    db: &'a TypedownDatabase,
    project: Project,
    file: File,
    field: &str,
  ) -> Option<HirValue<'a>> {
    let (hir, _) = lower_file(db, project, file);
    let hir = hir?;
    if let HirValueKind::Mapping(entries) = hir.kind(db) {
      entries
        .into_iter()
        .find(|(key, _)| key == field)
        .map(|(_, value)| value)
    } else {
      None
    }
  }

  #[test]
  fn expected_node_typ_known_field_returns_member() {
    let (db, project, file) = load_vault_fixture("typecheck/my_vault", "valid_person.td");
    let name_hir = get_field_hir(&db, project, file, "name")
      .expect("valid_person.td should have a 'name' field");

    let result = expected_node_typ(&db, name_hir);

    assert!(
      result.diagnostics(&db).is_empty(),
      "expected no diagnostics, got: {:?}",
      result.diagnostics(&db)
    );
    let typ = result
      .typ(&db)
      .expect("'name' field should have a declared type");
    assert_eq!(
      typ.display_name(&db),
      "string",
      "expected declared type 'string', got '{}'",
      typ.display_name(&db)
    );
  }

  #[test]
  fn expected_node_typ_untyped_mapping_returns_none() {
    let (db, project, file) = load_vault_fixture("typecheck/my_vault", "literal_value.td");
    let (hir, _) = lower_file(&db, project, file);
    let hir = hir.expect("literal_value.td should have parseable frontmatter");

    let result = expected_node_typ(&db, hir);

    assert!(
      result.typ(&db).is_none(),
      "untyped mapping root should have no declared member"
    );
  }

  #[test]
  fn expected_node_typ_no_frontmatter_returns_product() {
    let (db, project, file) = load_vault_fixture("typecheck/my_vault", "no_frontmatter.td");
    let (hir, _) = lower_file(&db, project, file);
    let hir = hir.expect("no_frontmatter.td should produce HIR");

    let result = expected_node_typ(&db, hir);
    let typ = result
      .typ(&db)
      .expect("schemaless file should return a type");
    assert!(
      typ.is_td_product_typ(),
      "schemaless file should return a product type"
    );
  }

  /// Get the HIR for a nested field value: top[field1][field2]
  fn get_nested_field_hir<'a>(
    db: &'a TypedownDatabase,
    project: Project,
    file: File,
    fields: &[&str],
  ) -> Option<HirValue<'a>> {
    let (hir, _) = lower_file(db, project, file);
    let mut current = hir?;
    for field in fields {
      if let HirValueKind::Mapping(entries) = current.kind(db) {
        current = entries
          .into_iter()
          .find(|(key, _)| key == field)
          .map(|(_, value)| value)?;
      } else {
        return None;
      }
    }
    Some(current)
  }

  // Nested field inside a schema property descriptor
  #[test]
  fn expected_node_typ_nested_schema_property_field() {
    let (db, project, file) = load_vault_fixture("typecheck/my_vault", "_types/WithUnion.td");
    let type_hir = get_nested_field_hir(&db, project, file, &["properties", "status", "type"]);
    let type_hir = type_hir.expect("should find nested type field");
    let result = expected_node_typ(&db, type_hir);

    assert!(
      result.typ(&db).is_some(),
      "nested 'type' field should have an expected type from SchemaProperty"
    );
  }

  // Schema with union: the 'status' field value should have expected type
  #[test]
  fn expected_node_typ_union_field_value() {
    let (db, project, file) = load_vault_fixture("typecheck/my_vault", "valid_status.td");
    let state_hir = get_field_hir(&db, project, file, "state").expect("should have 'state' field");
    let result = expected_node_typ(&db, state_hir);

    let typ = result.typ(&db).expect("state should have expected type");
    assert!(
      typ.as_td_lit_typ().is_some(),
      "expected literal type for state field"
    );
  }

  // Sequence item inside a list field should have expected type
  #[test]
  fn expected_node_typ_sequence_item() {
    let (db, project, file) = load_vault_fixture("typecheck/my_vault", "valid_event.td");
    let (hir, _) = lower_file(&db, project, file);
    let hir = hir.expect("should parse");

    if let HirValueKind::Mapping(entries) = hir.kind(&db) {
      for (_key, value) in entries {
        if let HirValueKind::Sequence(items) = value.kind(&db)
          && let Some(first_item) = items.first()
        {
          let result = expected_node_typ(&db, *first_item);
          let _ = result.typ(&db);
          return;
        }
      }
    }
  }
}
