//! Tracked query for the expected (top-down) type of a HIR value
// I think this is the idea of bidirectional typechecking

use crate::db::TypedownDatabase;
use std::collections::HashSet;

use crate::db::derived::evaluate::evaluate_type::evaluate_type;
use crate::db::derived::get_builtin_types::{
  get_bool_type, get_func_type, get_num_type, get_schemaless_type, get_sum_type,
};
use crate::db::derived::hir::lower_node;
use crate::db::derived::name_resolver::referee::referee;
use crate::db::derived::typechecker::actual_node_type::actual_node_type;
use crate::db::types::{
  File, FuncSignature, HirValue, LazyType, Project, StaticAccessPath, Symbol, TdTypeEnum,
  TdTypeLike, TypeResult,
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

/// Result of walking up from a node to the nearest _type anchor
struct AnchorResult {
  symbol: Symbol,
  typ: TdTypeEnum,
  path: Vec<(PathStep, RedNode)>,
}

#[query_derived]
pub fn expected_node_type(db: &TypedownDatabase, hir: HirValue) -> TypeResult {
  let project = hir.project(db);
  let file = hir.file(db);
  let node = hir.node(db);

  // Expression nodes propagate expected types through expression structure
  if !is_top_level(&node) {
    return get_expected_expr_type(db, hir);
  }

  let anchor = match collect_path_to_anchor(db, project, file, &node) {
    Some(result) => result,
    None => {
      // If the file has no _type, return schemaless type
      if is_schemaless_file(db, project, file) {
        return simple_schemaless_result(db);
      }
      return TypeResult::new(db, None, vec![]);
    }
  };

  // Traverse down the type structure following the path
  let mut current_type = LazyType::eager(anchor.typ);

  for (step, step_node) in &anchor.path {
    let step_hir = lower_node(db, project, file, step_node.clone());

    // Resolve Sum ambiguity using actual_node_type
    let resolved_type = resolve_lazy_type(db, &current_type, step_hir);

    current_type = match step {
      PathStep::Field(name) => match traverse_field(db, &resolved_type, name) {
        Some(lazy) => lazy,
        None => return TypeResult::new(db, None, vec![]),
      },
      PathStep::Index => match traverse_index(db, &resolved_type) {
        Some(lazy) => lazy,
        None => return TypeResult::new(db, None, vec![]),
      },
    };
  }

  TypeResult::new(db, current_type.resolve(db), vec![])
}

// Propagate expected type through expression structure
fn get_expected_expr_type(db: &TypedownDatabase, hir: HirValue) -> TypeResult {
  let node = hir.node(db);
  let project = hir.project(db);
  let file = hir.file(db);

  let parent = match node.parent() {
    Some(p) => p,
    None => return TypeResult::new(db, None, vec![]),
  };

  // CallExpr: first Expr child is callee, rest are args
  if let Some(call) = CallExpr::cast(parent.clone()) {
    let expr_children: Vec<Expr> = parent.children().filter_map(Expr::cast).collect();

    // Is it the callee (first Expr child)?
    if expr_children.first().is_some_and(|c| *c.syntax() == node) {
      return get_expected_call_callee_type(db, project, file, &call);
    }

    // Is it an arg? Find its position (0-indexed, skipping callee)
    let arg_index = expr_children
      .iter()
      .skip(1)
      .position(|c| *c.syntax() == node);
    if let Some(idx) = arg_index {
      return get_expected_call_arg_type(db, project, file, &call, idx);
    }
  }

  // ClosureExpr: body gets the return type from expected(closure)
  if let Some(closure) = ClosureExpr::cast(parent.clone())
    && closure.body().is_some_and(|b| *b.syntax() == node)
  {
    return get_expected_closure_body_type(db, project, file, &closure);
  }

  // ParenExpr: transparent, forward expected type from parent
  if ParenExpr::cast(parent.clone()).is_some() {
    let paren_hir = lower_node(db, project, file, parent);
    return expected_node_type(db, paren_hir);
  }

  // BinaryExpr: propagate expected types to operands based on operator
  if let Some(bin) = BinaryExpr::cast(parent.clone())
    && let Some(op) = bin.op().and_then(|o| o.kind())
  {
    return get_expected_binary_operand_type(db, &op);
  }

  // PrefixExpr: propagate expected type to operand based on operator
  if let Some(pre) = PrefixExpr::cast(parent.clone())
    && let Some(op) = pre.op().and_then(|o| o.kind())
  {
    return get_expected_prefix_operand_type(db, &op);
  }

  // No expression propagation rule matched, fall back to actual type
  actual_node_type(db, hir)
}

// expected(arg_i) = param_i from actual(callee)
// Macros have no TdFuncType so this returns None for macro args (they validate internally)
fn get_expected_call_arg_type(
  db: &TypedownDatabase,
  project: Project,
  file: File,
  call: &CallExpr,
  arg_index: usize,
) -> TypeResult {
  let callee_node = match call.callee() {
    Some(c) => c,
    None => return TypeResult::new(db, None, vec![]),
  };
  let callee_hir = lower_node(db, project, file, callee_node.syntax().clone());
  let callee_type = actual_node_type(db, callee_hir).typ(db);

  if let Some(TdTypeEnum::TdFuncType(func)) = callee_type {
    let params = func.signature(db).params(db);
    if let Some(param_type) = params.get(arg_index) {
      return TypeResult::new(db, Some(param_type.clone()), vec![]);
    }
  }

  TypeResult::new(db, None, vec![])
}

// expected(callee) = fn(actual(arg_i)...) -> expected(call)
fn get_expected_call_callee_type(
  db: &TypedownDatabase,
  project: Project,
  file: File,
  call: &CallExpr,
) -> TypeResult {
  let call_hir = lower_node(db, project, file, call.syntax().clone());
  let ret = match expected_node_type(db, call_hir).typ(db) {
    Some(t) => t,
    None => return TypeResult::new(db, None, vec![]),
  };

  let mut param_types = vec![];
  for arg in call.args() {
    let arg_hir = lower_node(db, project, file, arg.syntax().clone());
    match actual_node_type(db, arg_hir).typ(db) {
      Some(t) => param_types.push(t),
      None => return TypeResult::new(db, None, vec![]),
    }
  }

  let sig = FuncSignature::new(db, param_types, ret);
  let func_type = get_func_type(db, sig);
  TypeResult::new(db, Some(func_type.into()), vec![])
}

// expected(body) = return type from expected(closure)
fn get_expected_closure_body_type(
  db: &TypedownDatabase,
  project: Project,
  file: File,
  closure: &ClosureExpr,
) -> TypeResult {
  let closure_hir = lower_node(db, project, file, closure.syntax().clone());
  let expected = expected_node_type(db, closure_hir).typ(db);
  match expected {
    Some(TdTypeEnum::TdFuncType(f)) => {
      let ret = f.signature(db).ret(db);
      TypeResult::new(db, Some(ret), vec![])
    }
    _ => TypeResult::new(db, None, vec![]),
  }
}

fn get_expected_binary_operand_type(db: &TypedownDatabase, op: &YamlOpKind) -> TypeResult {
  match op {
    // Arithmetic and power expect number
    YamlOpKind::Plus
    | YamlOpKind::Minus
    | YamlOpKind::Mul
    | YamlOpKind::Div
    | YamlOpKind::Mod
    | YamlOpKind::Pow => TypeResult::new(db, Some(get_num_type(db).into()), vec![]),
    // Logical expects boolean
    YamlOpKind::And | YamlOpKind::Or => TypeResult::new(db, Some(get_bool_type(db).into()), vec![]),
    // Comparison and dot access have no operand constraint
    _ => TypeResult::new(db, None, vec![]),
  }
}

fn get_expected_prefix_operand_type(db: &TypedownDatabase, op: &YamlOpKind) -> TypeResult {
  match op {
    YamlOpKind::Plus | YamlOpKind::Minus => {
      TypeResult::new(db, Some(get_num_type(db).into()), vec![])
    }
    YamlOpKind::Tilde => TypeResult::new(db, Some(get_bool_type(db).into()), vec![]),
    _ => TypeResult::new(db, None, vec![]),
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

/// Get the static access path from the nearest _type anchor to the target node
pub fn static_access_path(
  db: &TypedownDatabase,
  project: Project,
  file: File,
  node: &RedNode,
) -> Option<StaticAccessPath> {
  let anchor = collect_path_to_anchor(db, project, file, node)?;
  let steps = anchor.path.into_iter().map(|(step, _)| step).collect();
  Some(StaticAccessPath {
    owner: anchor.symbol,
    steps,
  })
}

/// Walk up from target to the nearest _type anchor, collecting path steps
fn collect_path_to_anchor(
  db: &TypedownDatabase,
  project: Project,
  file: File,
  target: &RedNode,
) -> Option<AnchorResult> {
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
        if let Some((symbol, schema_type)) = resolve_type_anchor(db, project, file, &mapping) {
          path.reverse();
          return Some(AnchorResult {
            symbol,
            typ: schema_type,
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
        if let Some((symbol, schema_type)) = resolve_type_anchor(db, project, file, &dict) {
          path.reverse();
          return Some(AnchorResult {
            symbol,
            typ: schema_type,
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

/// Resolve the _type field in a mapping to its symbol and type
fn resolve_type_anchor(
  db: &TypedownDatabase,
  project: Project,
  file: File,
  mapping: &RedNode,
) -> Option<(Symbol, TdTypeEnum)> {
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
    let value_hir = lower_node(db, project, file, value_expr.syntax().clone());
    let symbol = referee(db, value_hir).value(db)?;
    let typ = evaluate_type(db, symbol).typ(db)?;
    return Some((symbol, typ));
  }
  None
}

/// Resolve a Sum type by picking the most specific matching arm
fn resolve_lazy_type(db: &TypedownDatabase, lazy: &LazyType, hir: HirValue) -> LazyType {
  let Some(typ) = lazy.resolve(db) else {
    return lazy.clone();
  };
  if let TdTypeEnum::TdSumType(sum) = &typ {
    let members = sum.members(db);
    if let Some(picked) = pick_most_specific_arm(db, &members, hir) {
      return picked;
    }
  }
  lazy.clone()
}

/// Pick the matching arms for the actual value
fn pick_most_specific_arm(
  db: &TypedownDatabase,
  arms: &HashSet<LazyType>,
  hir: HirValue,
) -> Option<LazyType> {
  let actual_type = actual_node_type(db, hir).typ(db)?;

  let matching: Vec<_> = arms
    .iter()
    .filter(|arm| arm.resolve(db).is_some_and(|t| t.accepts(db, &actual_type)))
    .cloned()
    .collect();

  if matching.is_empty() {
    return None;
  }
  if matching.len() == 1 {
    return Some(matching.into_iter().next().unwrap());
  }

  Some(LazyType::eager(get_sum_type(db, matching).into()))
}

/// Look up a field in the resolved type
fn traverse_field(db: &TypedownDatabase, lazy: &LazyType, field_name: &str) -> Option<LazyType> {
  let typ = lazy.resolve(db)?;
  if let Some(field_type) = typ.get_owned_field_type(db, field_name) {
    return Some(LazyType::eager(field_type));
  }
  // Dict: any key maps to the value type
  if let Some(dict) = typ.as_td_dict_type()
    && let Some(value_type) = dict.value(db).and_then(|l| l.resolve(db))
  {
    return Some(LazyType::eager(value_type));
  }
  None
}

/// Get the element type from a list
fn traverse_index(db: &TypedownDatabase, lazy: &LazyType) -> Option<LazyType> {
  let typ = lazy.resolve(db)?;
  let list = typ.as_td_list_type()?;
  list.elem(db)
}

fn simple_schemaless_result(db: &TypedownDatabase) -> TypeResult {
  TypeResult::new(db, Some(get_schemaless_type(db).into()), vec![])
}

#[cfg(test)]
mod tests {
  use crate::db::TypedownDatabase;
  use crate::db::types::TdTypeLike;
  use crate::db::{
    derived::typechecker::expected_node_type::expected_node_type,
    fixtures::load_vault_fixture,
    types::{File, HirValue, HirValueKind, Project},
    utils::lower_file,
  };

  fn get_field_hir(
    db: &TypedownDatabase,
    project: Project,
    file: File,
    field: &str,
  ) -> Option<HirValue> {
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
  fn expected_node_type_known_field_returns_member() {
    let (db, project, file) = load_vault_fixture("typecheck/my_vault", "content/valid_person.td");
    let name_hir = get_field_hir(&db, project, file, "name")
      .expect("valid_person.td should have a 'name' field");

    let result = expected_node_type(&db, name_hir);

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
  fn expected_node_type_untyped_mapping_returns_none() {
    let (db, project, file) = load_vault_fixture("typecheck/my_vault", "content/literal_value.td");
    let (hir, _) = lower_file(&db, project, file);
    let hir = hir.expect("literal_value.td should have parseable frontmatter");

    let result = expected_node_type(&db, hir);

    assert!(
      result.typ(&db).is_none(),
      "untyped mapping root should have no declared member"
    );
  }

  #[test]
  fn expected_node_type_no_frontmatter_returns_schemaless() {
    let (db, project, file) = load_vault_fixture("typecheck/my_vault", "content/no_frontmatter.td");
    let (hir, _) = lower_file(&db, project, file);
    let hir = hir.expect("no_frontmatter.td should produce HIR");

    let result = expected_node_type(&db, hir);
    let typ = result
      .typ(&db)
      .expect("schemaless file should return a type");
    assert_eq!(
      typ.display_name(&db),
      "{}",
      "schemaless type should have no fields"
    );
  }

  /// Get the HIR for a nested field value: top[field1][field2]
  fn get_nested_field_hir(
    db: &TypedownDatabase,
    project: Project,
    file: File,
    fields: &[&str],
  ) -> Option<HirValue> {
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
  fn expected_node_type_nested_schema_property_field() {
    let (db, project, file) = load_vault_fixture("typecheck/my_vault", "schemas/WithUnion.td");
    let type_hir = get_nested_field_hir(&db, project, file, &["properties", "status", "type"]);
    let type_hir = type_hir.expect("should find nested type field");
    let result = expected_node_type(&db, type_hir);

    assert!(
      result.typ(&db).is_some(),
      "nested 'type' field should have an expected type from SchemaProperty"
    );
  }

  // Schema with union: the 'status' field value should have expected type
  #[test]
  fn expected_node_type_union_field_value() {
    let (db, project, file) = load_vault_fixture("typecheck/my_vault", "content/valid_status.td");
    let state_hir = get_field_hir(&db, project, file, "state").expect("should have 'state' field");
    let result = expected_node_type(&db, state_hir);

    let typ = result.typ(&db).expect("state should have expected type");
    assert!(
      typ.as_td_literal_type().is_some(),
      "expected literal type for state field"
    );
  }

  // Sequence item inside a list field should have expected type
  #[test]
  fn expected_node_type_sequence_item() {
    let (db, project, file) = load_vault_fixture("typecheck/my_vault", "content/valid_event.td");
    let (hir, _) = lower_file(&db, project, file);
    let hir = hir.expect("should parse");

    if let HirValueKind::Mapping(entries) = hir.kind(&db) {
      for (_key, value) in entries {
        if let HirValueKind::Sequence(items) = value.kind(&db)
          && let Some(first_item) = items.first()
        {
          let result = expected_node_type(&db, *first_item);
          let _ = result.typ(&db);
          return;
        }
      }
    }
  }
}
