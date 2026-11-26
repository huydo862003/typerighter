//! Expected type at a position in a `.td` file
//!
//! Tolerates incomplete input (empty values, `{key: |}`, trailing whitespace after a half-typed entry)
//! Each helper handles one enclosing construct and returns `None` when the node is not inside it

use typedown_lang::db::TypedownDatabase;
use typedown_lang::db::derived::evaluate::evaluate_typ::evaluate_typ;
use typedown_lang::db::derived::name_resolver::members::members;
use typedown_lang::db::derived::typechecker::expected_node_typ::expected_node_typ;
use typedown_lang::db::types::{
  File, Project, ProjectRedNode, Scope, ScopeKind, TdStaticTyp, TdTypEnum,
};
use typedown_lang::db::utils::get_mapping_schema_name;
use typedown_lang::syntax::ast::{AstNode, Expr, YamlMappingEntry};
use typedown_lang::syntax::red::RedNode;
use typedown_lang::syntax::syntax_kind::SyntaxKind;

use crate::core::utils::ast::{
  find_ancestor, get_last_child_ending_before, get_nearest_expr_ancestor,
  is_in_mapping_value_position,
};

/// Expected type of the expression at `node`, trying the innermost enclosing construct first
pub fn typ_at_node<'db>(
  db: &'db TypedownDatabase,
  project: Project,
  file: File,
  node: &RedNode,
) -> Option<TdTypEnum<'db>> {
  dict_entry_typ(db, project, file, node)
    .or_else(|| block_sequence_item_typ(db, project, file, node))
    .or_else(|| flow_list_item_typ(db, project, file, node))
    .or_else(|| mapping_entry_typ(db, project, file, node))
}

/// Like `typ_at_node` but only for a mapping value, resolving by the entry key when the value is empty
pub fn typ_at_value_position<'db>(
  db: &'db TypedownDatabase,
  project: Project,
  file: File,
  node: &RedNode,
) -> Option<TdTypEnum<'db>> {
  if !is_in_mapping_value_position(node) {
    return None;
  }
  typ_at_node(db, project, file, node).or_else(|| {
    let entry = find_ancestor(node, SyntaxKind::YamlMappingEntry).or_else(|| {
      let mapping = find_ancestor(node, SyntaxKind::YamlMapping)?;
      get_last_child_ending_before(&mapping, SyntaxKind::YamlMappingEntry, node.start_offset())
    })?;
    typ_by_entry_key(db, project, file, &entry)
  })
}

/// `typ` itself or, for a nullable `T?`, one of its members satisfies `predicate`
pub fn is_or_nullable<'db>(
  db: &'db TypedownDatabase,
  typ: &TdTypEnum<'db>,
  predicate: fn(&TdTypEnum<'db>) -> bool,
) -> bool {
  predicate(typ) || has_nullable_member(db, typ, predicate)
}

/// A member of the sum type `typ` (e.g. the `T` in `T?`) satisfies `predicate`
pub fn has_nullable_member<'db>(
  db: &'db TypedownDatabase,
  typ: &TdTypEnum<'db>,
  predicate: fn(&TdTypEnum<'db>) -> bool,
) -> bool {
  typ.as_td_sum_typ().is_some_and(|sum| {
    sum
      .members(db)
      .iter()
      .any(|member| member.resolve(db).is_some_and(|typ| predicate(&typ)))
  })
}

/// Type of field `key` in `mapping`, from its `_type` schema or else the mapping's own expected type
pub fn resolve_field_typ_from_schema<'db>(
  db: &'db TypedownDatabase,
  project: Project,
  file: File,
  mapping: &RedNode,
  key: &str,
) -> Option<TdTypEnum<'db>> {
  if let Some(schema_name) = get_mapping_schema_name(mapping) {
    let scope = Scope::new(db, ScopeKind::Project(project));
    let symbol = *members(db, scope).members(db).get(&schema_name)?;
    let typ = evaluate_typ(db, symbol).typ(db)?;
    return lookup_member_field_typ(db, &typ, key);
  }
  let mapping_typ = expected_typ_of(db, project, file, mapping)?;
  lookup_member_field_typ(db, &mapping_typ, key)
}

fn expected_typ_of<'db>(
  db: &'db TypedownDatabase,
  project: Project,
  file: File,
  node: &RedNode,
) -> Option<TdTypEnum<'db>> {
  expected_node_typ(db, ProjectRedNode::new(project, file, node.clone())).typ(db)
}

/// Expected type of the expression around the cursor, if `accept` allows that expression
fn innermost_expr_typ<'db>(
  db: &'db TypedownDatabase,
  project: Project,
  file: File,
  node: &RedNode,
  accept: impl Fn(&RedNode) -> bool,
) -> Option<TdTypEnum<'db>> {
  let expr = get_nearest_expr_ancestor(node)?;
  if !accept(expr.syntax()) {
    return None;
  }
  expected_typ_of(db, project, file, expr.syntax())
}

/// `{key: |}`
fn dict_entry_typ<'db>(
  db: &'db TypedownDatabase,
  project: Project,
  file: File,
  node: &RedNode,
) -> Option<TdTypEnum<'db>> {
  let dict = find_ancestor(node, SyntaxKind::DictLit)?;
  // With an empty value the cursor parses outside the entry, so use the entry just before it
  let entry = find_ancestor(node, SyntaxKind::DictEntry)
    .or_else(|| get_last_child_ending_before(&dict, SyntaxKind::DictEntry, node.start_offset()))?;
  let colon_end = entry
    .children()
    .find(|child| child.kind() == SyntaxKind::Colon)
    .map(|colon| colon.end_offset());
  let in_value = find_ancestor(node, SyntaxKind::DictEntryValue).is_some()
    || colon_end.is_some_and(|end| node.start_offset() >= end);
  if !in_value {
    return None;
  }

  innermost_expr_typ(db, project, file, node, |expr| {
    expr.start_offset() > entry.start_offset()
  })
  .or_else(|| {
    let key = entry
      .children()
      .find(|child| child.kind() == SyntaxKind::DictEntryKey)?
      .text()
      .trim()
      .to_string();
    let dict_typ = expected_typ_of(db, project, file, &dict)?;
    lookup_member_field_typ(db, &dict_typ, &key)
  })
}

/// `- |`
fn block_sequence_item_typ<'db>(
  db: &'db TypedownDatabase,
  project: Project,
  file: File,
  node: &RedNode,
) -> Option<TdTypEnum<'db>> {
  let item = find_ancestor(node, SyntaxKind::YamlSequenceItem)?;
  let sequence = item
    .parent()
    .filter(|parent| parent.kind() == SyntaxKind::YamlSequence)?;

  innermost_expr_typ(db, project, file, node, |expr| {
    expr.start_offset() >= item.start_offset() && expr.kind() != SyntaxKind::YamlSequence
  })
  .or_else(|| unwrap_list_element_typ(db, &expected_typ_of(db, project, file, &sequence)?))
}

/// `[|]`
fn flow_list_item_typ<'db>(
  db: &'db TypedownDatabase,
  project: Project,
  file: File,
  node: &RedNode,
) -> Option<TdTypEnum<'db>> {
  let list = find_ancestor(node, SyntaxKind::ListLit)?;
  // After the closing `]` the cursor is no longer an item
  let closing = list
    .children()
    .find(|child| child.kind() == SyntaxKind::RBracket)
    .map(|bracket| bracket.start_offset());
  if closing.is_some_and(|end| node.start_offset() >= end) {
    return None;
  }

  innermost_expr_typ(db, project, file, node, |expr| {
    expr.start_offset() > list.start_offset()
  })
  .or_else(|| unwrap_list_element_typ(db, &expected_typ_of(db, project, file, &list)?))
}

/// `key: |`, including nested mappings
fn mapping_entry_typ<'db>(
  db: &'db TypedownDatabase,
  project: Project,
  file: File,
  node: &RedNode,
) -> Option<TdTypEnum<'db>> {
  let entry_value = find_ancestor(node, SyntaxKind::YamlMappingEntryValue)?;
  // An empty inner entry (`outer:\n  inner: |`) has no value node, so go by its key
  if let Some(entry) = find_ancestor(node, SyntaxKind::YamlMappingEntry)
    && entry_value.start_offset() < entry.start_offset()
  {
    return typ_by_entry_key(db, project, file, &entry);
  }

  let is_scalar = |expr: &RedNode| {
    !matches!(
      expr.kind(),
      SyntaxKind::YamlMapping | SyntaxKind::YamlSequence
    )
  };
  innermost_expr_typ(db, project, file, node, |expr| {
    expr.start_offset() >= entry_value.start_offset() && is_scalar(expr)
  })
  .or_else(|| {
    let value_expr = entry_value.children().find_map(Expr::cast)?;
    if !is_scalar(value_expr.syntax()) {
      return None;
    }
    expected_typ_of(db, project, file, value_expr.syntax())
  })
  .or_else(|| typ_by_entry_key(db, project, file, &entry_value.parent()?))
}

fn typ_by_entry_key<'db>(
  db: &'db TypedownDatabase,
  project: Project,
  file: File,
  entry: &RedNode,
) -> Option<TdTypEnum<'db>> {
  let key = YamlMappingEntry::cast(entry.clone()).and_then(|entry| entry.key())?;
  let mapping = entry
    .parent()
    .filter(|parent| parent.kind() == SyntaxKind::YamlMapping)?;
  resolve_field_typ_from_schema(db, project, file, &mapping, &key)
}

/// Element type of a list, or of a nullable list
fn unwrap_list_element_typ<'db>(
  db: &'db TypedownDatabase,
  typ: &TdTypEnum<'db>,
) -> Option<TdTypEnum<'db>> {
  if let Some(list) = typ.as_td_list_typ() {
    return list.element(db).and_then(|element| element.resolve(db));
  }
  let sum = typ.as_td_sum_typ()?;
  sum.members(db).into_iter().find_map(|member| {
    let list = *member.resolve(db)?.as_td_list_typ()?;
    list.element(db).and_then(|element| element.resolve(db))
  })
}

/// Type of field `key` on a schema, product, dict or nullable wrapper of those
fn lookup_member_field_typ<'db>(
  db: &'db TypedownDatabase,
  container_typ: &TdTypEnum<'db>,
  key: &str,
) -> Option<TdTypEnum<'db>> {
  if let Some(schema) = container_typ.as_td_schema_typ()
    && let Some(prop) = schema.fields(db).get(key)
  {
    return prop.field_typ.resolve(db);
  }
  if let Some(field_typ) = container_typ.get_owned_field_typ(db, key) {
    return Some(field_typ);
  }
  if let Some(dict) = container_typ.as_td_dict_typ() {
    return dict.value(db).and_then(|value| value.resolve(db));
  }
  let sum = container_typ.as_td_sum_typ()?;
  sum
    .members(db)
    .into_iter()
    .find_map(|member| lookup_member_field_typ(db, &member.resolve(db)?, key))
}
