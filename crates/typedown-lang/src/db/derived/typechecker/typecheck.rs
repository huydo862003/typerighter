//! Tracked query for typechecking
// I think this is the idea of bidirectional typechecking

use std::collections::HashSet;

use crate::db::derived::typechecker::expected_node_typ::expected_node_typ;
use crate::syntax::diagnostic::*;
use typedown_macros::query_derived;

use crate::db::TypedownDatabase;
use crate::db::derived::check_custom_components::check_custom_components;
use crate::db::derived::evaluate::evaluate_typ::evaluate_typ;
use crate::db::derived::get_builtin_typs::{get_bool_typ, get_number_typ};
use crate::db::derived::name_resolver::referee::referee;
use crate::db::derived::typechecker::actual_node_typ::actual_node_typ;

use crate::db::types::derived::obj_system::TdStaticTyp;
use crate::db::types::typecheck::{is_nullable, is_subtype_of};
use crate::db::types::{File, Project, ProjectRedNode, TdSchemaTyp, TdTypEnum, TypecheckResult};
use crate::syntax::ast::{
  AstNode, Expr, IdentLit, InterpFragment, SourceFile, YamlFrontmatter, YamlMapping, YamlOpKind,
};
use crate::syntax::red::RedNode;
use typedown_incremental::QueryDatabase;
use typedown_types::either::Either;

#[query_derived(no_hash)]
pub fn typecheck<'db>(
  db: &'db TypedownDatabase,
  project_node: ProjectRedNode,
) -> TypecheckResult<'db> {
  // SourceFile/YamlFrontmatter: delegate to the frontmatter mapping for type resolution
  let node = project_node.node.clone();
  if let Some(source_file) = SourceFile::cast(node.clone()) {
    let mut diags = check_custom_components(db, project_node.project, project_node.file)
      .diagnostics(db)
      .clone();
    if let Some(frontmatter) = source_file.frontmatter()
      && let Some(mapping) = frontmatter.mapping()
    {
      let mapping_node = ProjectRedNode::new(
        project_node.project,
        project_node.file,
        mapping.syntax().clone(),
      );
      diags.extend(typecheck(db, mapping_node).diagnostics(db).iter().cloned());
    }
    if let Some(body) = source_file.body() {
      typecheck_md_body(
        db,
        project_node.project,
        project_node.file,
        body.syntax().clone(),
        &mut diags,
      );
    }
    return TypecheckResult::new(db, diags);
  }
  if let Some(frontmatter) = YamlFrontmatter::cast(node.clone()) {
    if let Some(mapping) = frontmatter.mapping() {
      let mapping_node = ProjectRedNode::new(
        project_node.project,
        project_node.file,
        mapping.syntax().clone(),
      );
      return typecheck(db, mapping_node);
    }
    return TypecheckResult::new(db, vec![]);
  }
  let typ_result = actual_node_typ(db, project_node.clone());
  let mut diagnostics = typ_result.diagnostics(db).clone();

  // Use expected type from schema if available, otherwise fall back to inferred type
  let declared_typ = match expected_node_typ(db, project_node.clone()).typ(db) {
    Some(typ) => typ,
    None => match typ_result.typ(db) {
      Some(typ) => typ,
      None => return TypecheckResult::new(db, diagnostics),
    },
  };

  diagnostics.extend(typecheck_body(db, project_node, &declared_typ));
  TypecheckResult::new(db, diagnostics)
}

// Typecheck a node against an externally provided expected type
pub fn typecheck_with_expected<'db>(
  db: &'db TypedownDatabase,
  project_node: ProjectRedNode,
  expected_typ: &TdTypEnum<'db>,
) -> TypecheckResult<'db> {
  let typ_result = actual_node_typ(db, project_node.clone());
  let mut diagnostics = typ_result.diagnostics(db).clone();

  diagnostics.extend(typecheck_body(db, project_node, expected_typ));
  TypecheckResult::new(db, diagnostics)
}

// Validate structure based on the node kind
fn typecheck_body<'db>(
  db: &'db TypedownDatabase,
  project_node: ProjectRedNode,
  declared_typ: &TdTypEnum<'db>,
) -> Vec<TdDiagnostic> {
  let project = project_node.project;
  let file = project_node.file;
  let node = project_node.node.clone();
  let mut diagnostics = vec![];

  // Check mapping nodes (YamlMapping or DictLit)
  if let Some(entries) = project_node.mapping_entries() {
    return check_mapping_fields(db, project_node, &entries, declared_typ);
  }

  // Check sequence nodes (YamlSequence or ListLit)
  if let Some(items) = project_node.sequence_items() {
    return check_sequence(db, declared_typ, items);
  }

  // Dispatch on Expr variants
  let Some(expr) = Expr::cast(node.clone()) else {
    return diagnostics;
  };

  match expr {
    Expr::BinaryExpr(binary) => {
      let op_kind = binary.op().and_then(|op| op.kind());
      match op_kind {
        // Check tag inner matches the tag's schema
        Some(YamlOpKind::Tag(_)) => {
          if let Some(inner_expr) = binary.right() {
            let inner_node = ProjectRedNode::new(project, file, inner_expr.syntax().clone());
            diagnostics.extend(check_tag(db, declared_typ, inner_node));
          }
        }
        // Check binary operand types
        _ => {
          let op_text = binary.op().map(|op| op.syntax().text()).unwrap_or_default();
          if let (Some(left), Some(right)) = (binary.left(), binary.right()) {
            let left_node = ProjectRedNode::new(project, file, left.syntax().clone());
            let right_node = ProjectRedNode::new(project, file, right.syntax().clone());
            diagnostics.extend(check_binary(db, &op_text, left_node, right_node));
          }
        }
      }
    }
    // Check call arity and arg types against function signature
    Expr::CallExpr(call) => {
      if let Some(callee_expr) = call.callee() {
        let callee_node = ProjectRedNode::new(project, file, callee_expr.syntax().clone());
        let arg_nodes: Vec<_> = call
          .args()
          .into_iter()
          .map(|arg| ProjectRedNode::new(project, file, arg.syntax().clone()))
          .collect();
        diagnostics.extend(check_call(db, callee_node, arg_nodes));
      }
    }
    // Check prefix operand type
    Expr::PrefixExpr(prefix) => {
      let op_text = prefix.op().map(|op| op.syntax().text()).unwrap_or_default();
      if let Some(operand_expr) = prefix.expr() {
        let operand_node = ProjectRedNode::new(project, file, operand_expr.syntax().clone());
        diagnostics.extend(check_prefix(db, &op_text, operand_node));
      }
    }
    // Check postfix operand type
    Expr::PostfixExpr(postfix) => {
      let op_text = postfix
        .op()
        .map(|op| op.syntax().text())
        .unwrap_or_default();
      if let Some(operand_expr) = postfix.expr() {
        let operand_node = ProjectRedNode::new(project, file, operand_expr.syntax().clone());
        diagnostics.extend(check_postfix(db, &op_text, operand_node));
      }
    }
    // Check index types against container key types
    Expr::IndexExpr(index) => {
      if let Some(expr_part) = index.expr() {
        let expr_node = ProjectRedNode::new(project, file, expr_part.syntax().clone());
        let index_nodes: Vec<_> = index
          .indices()
          .into_iter()
          .map(|idx| ProjectRedNode::new(project, file, idx.syntax().clone()))
          .collect();
        diagnostics.extend(check_index(db, expr_node, index_nodes));
      }
    }
    // Recurse into closure body
    Expr::ClosureExpr(closure) => {
      if let Some(body_expr) = closure.body() {
        let body_node = ProjectRedNode::new(project, file, body_expr.syntax().clone());
        let typecheck_result = typecheck(db, body_node);
        diagnostics.extend(typecheck_result.diagnostics(db).iter().cloned());
      }
    }
    // Recurse into parenthesized expression
    Expr::ParenExpr(paren) => {
      if let Some(inner_expr) = paren.expr() {
        let inner_node = ProjectRedNode::new(project, file, inner_expr.syntax().clone());
        diagnostics.extend(typecheck_body(db, inner_node, declared_typ));
      }
    }
    // Typecheck each embedded expression in an interpolated string
    Expr::StrLit(str_lit) => {
      for fragment in str_lit.fragments() {
        if let Either::Right(interp) = fragment
          && let Some(expr) = interp.expr()
        {
          let expr_node = ProjectRedNode::new(project, file, expr.syntax().clone());
          let typecheck_result = typecheck(db, expr_node);
          diagnostics.extend(typecheck_result.diagnostics(db).iter().cloned());
        }
      }
    }
    _ => {}
  }

  diagnostics
}

// Walk an MdBody subtree and typecheck each interpolation expression
fn typecheck_md_body(
  db: &TypedownDatabase,
  project: Project,
  file: File,
  node: RedNode,
  diagnostics: &mut Vec<TdDiagnostic>,
) {
  if let Some(interp) = InterpFragment::cast(node.clone()) {
    if let Some(expr) = interp.expr() {
      let expr_node = ProjectRedNode::new(project, file, expr.syntax().clone());
      diagnostics.extend(typecheck(db, expr_node).diagnostics(db).iter().cloned());
    }
    return;
  }
  for child in node.children() {
    typecheck_md_body(db, project, file, child, diagnostics);
  }
}

fn check_mapping_fields<'db>(
  db: &'db TypedownDatabase,
  mapping_node: ProjectRedNode,
  entries: &[(String, ProjectRedNode)],
  expected_typ: &TdTypEnum<'db>,
) -> Vec<TdDiagnostic> {
  let mut diagnostics = vec![];
  let declared_fields = expected_typ.get_fields(db);

  for (key, value_node) in entries {
    // _type requires the value to resolve to a schema symbol
    if key == "_type" {
      let resolved = referee(db, value_node.clone());
      if let Some(symbol) = resolved.value(db)
        && !symbol.kind(db).is_schema()
      {
        let (trimmed_offset, trimmed_len) = value_node.node.trimmed_range();
        diagnostics.push(
          TdDiagnosticKind::FieldTypMismatch {
            field: "_type".to_string(),
            expected: "schema".to_string(),
          }
          .into_error(trimmed_offset, trimmed_offset + trimmed_len),
        );
      }
      continue;
    }
    // Built-in fields (_label, _icon) have fixed types
    if let Some(builtin_typ) = TdSchemaTyp::builtin_field_typ(db, key) {
      let value_result = actual_node_typ(db, value_node.clone());
      if let Some(actual_typ) = value_result.typ(db)
        && !is_subtype_of(db, &actual_typ, &builtin_typ)
      {
        let (trimmed_offset, trimmed_len) = value_node.node.trimmed_range();
        diagnostics.push(
          TdDiagnosticKind::FieldTypMismatch {
            field: key.clone(),
            expected: builtin_typ.display_name(db),
          }
          .into_error(trimmed_offset, trimmed_offset + trimmed_len),
        );
      }
      continue;
    }
    if let Some(field_lazy) = declared_fields.get(key) {
      if let Some(field_typ) = field_lazy.resolve(db) {
        // Recursively typecheck the field value
        let typecheck_result = typecheck(db, value_node.clone());
        diagnostics.extend(typecheck_result.diagnostics(db).iter().cloned());

        // Check synthesized type against expected field type
        let value_result = actual_node_typ(db, value_node.clone());
        let is_optional = is_nullable(db, &field_typ);
        // Skip structural subtype check for schema declaration fields that hold mappings:
        // schema metatype fields like `properties` are validated structurally during
        // schema member resolution, not via subtype comparison here
        let is_schema_meta_mapping =
          expected_typ.as_td_schema_meta_typ().is_some() && value_node.mapping_entries().is_some();
        match value_result.typ(db) {
          Some(actual_typ)
            if !is_subtype_of(db, &actual_typ, &field_typ) && !is_schema_meta_mapping =>
          {
            let (trimmed_offset, trimmed_len) = value_node.node.trimmed_range();
            diagnostics.push(
              TdDiagnosticKind::FieldTypMismatch {
                field: key.clone(),
                expected: field_typ.display_name(db),
              }
              .into_error(trimmed_offset, trimmed_offset + trimmed_len),
            );
          }
          // Unresolved identifier used as a field value
          None if IdentLit::cast(value_node.node.clone()).is_some() => {
            let (trimmed_offset, trimmed_len) = value_node.node.trimmed_range();
            diagnostics.push(
              TdDiagnosticKind::UnresolvedSchema {
                name: value_node.node.text(),
              }
              .into_error(trimmed_offset, trimmed_offset + trimmed_len),
            );
          }
          // Unknown value type on a required field: treat as type mismatch
          None if !is_optional => {
            let (trimmed_offset, trimmed_len) = value_node.node.trimmed_range();
            diagnostics.push(
              TdDiagnosticKind::FieldTypMismatch {
                field: key.clone(),
                expected: field_typ.display_name(db),
              }
              .into_error(trimmed_offset, trimmed_offset + trimmed_len),
            );
          }
          Some(_) | None => {}
        }
      }
    } else if expected_typ.as_td_schema_typ().is_some() && !key.starts_with('_') {
      // Excess property: key not declared on the schema
      let (start_offset, end_offset) = YamlMapping::cast(mapping_node.node.clone())
        .and_then(|mapping| mapping.find_entry(key))
        .and_then(|entry| entry.key_node())
        .map(|node| node.trimmed_range())
        .unwrap_or_else(|| value_node.node.trimmed_range());
      diagnostics.push(
        TdDiagnosticKind::UnknownField {
          field: key.clone(),
          on_typ: expected_typ.display_name(db),
        }
        .into_error(start_offset, end_offset),
      );
    }
  }

  // For schema metatype, validate that each property descriptor mapping has a required `type` field
  // Full subtype checking against SchemaProperty is skipped because descriptor values use
  // type-expression syntax (e.g. `string | null`, `list[string]?`) that actual_node_typ
  // cannot evaluate as plain values
  if expected_typ.as_td_schema_meta_typ().is_some() {
    for (key, value_node) in entries {
      if key == "properties"
        && let Some(descriptors) = value_node.mapping_entries()
      {
        for (_, descriptor_node) in &descriptors {
          if let Some(descriptor_entries) = descriptor_node.mapping_entries() {
            let has_type_field = descriptor_entries.iter().any(|(k, _)| k == "type");
            if !has_type_field {
              let (offset, len) = descriptor_node.node.trimmed_range();
              diagnostics.push(
                TdDiagnosticKind::MissingRequiredField {
                  field: "type".to_string(),
                }
                .into_error(offset, offset + len),
              );
            }
          }
        }
      }
    }
    return diagnostics;
  }

  // Check required fields are present (null values are checked above)
  let (trimmed_offset, trimmed_len) = mapping_node.node.trimmed_range();
  let present_keys: HashSet<&str> = entries.iter().map(|(key, _)| key.as_str()).collect();

  let default_fields: HashSet<String> = expected_typ
    .as_td_schema_typ()
    .map(|path| {
      path
        .fields(db)
        .iter()
        .filter_map(|(key, descriptor)| {
          if descriptor.default_value.is_some() {
            Some(key.clone())
          } else {
            None
          }
        })
        .collect()
    })
    .unwrap_or_default();

  for (field_name, field_lazy) in declared_fields {
    let is_optional = field_lazy
      .resolve(db)
      .is_some_and(|typ| is_nullable(db, &typ))
      || default_fields.contains(&field_name);
    if !is_optional && !present_keys.contains(field_name.as_str()) {
      diagnostics.push(
        TdDiagnosticKind::MissingRequiredField { field: field_name }
          .into_error(trimmed_offset, trimmed_offset + trimmed_len),
      );
    }
  }

  diagnostics
}

fn check_tag<'db>(
  db: &'db TypedownDatabase,
  expected_typ: &TdTypEnum<'db>,
  inner: ProjectRedNode,
) -> Vec<TdDiagnostic> {
  let mut diagnostics = vec![];
  let inner_result = actual_node_typ(db, inner.clone());
  diagnostics.extend(inner_result.diagnostics(db).iter().cloned());
  if let Some(actual_typ) = inner_result.typ(db)
    && !is_subtype_of(db, &actual_typ, expected_typ)
  {
    let (trimmed_offset, trimmed_len) = inner.node.trimmed_range();
    diagnostics.push(
      TdDiagnosticKind::TagTypMismatch {
        expected: expected_typ.display_name(db),
      }
      .into_error(trimmed_offset, trimmed_offset + trimmed_len),
    );
  }
  diagnostics
}

fn check_call(
  db: &TypedownDatabase,
  callee: ProjectRedNode,
  args: Vec<ProjectRedNode>,
) -> Vec<TdDiagnostic> {
  let mut diagnostics = vec![];

  let callee_result = actual_node_typ(db, callee.clone());
  diagnostics.extend(callee_result.diagnostics(db).iter().cloned());

  let callee_typ = match callee_result.typ(db) {
    Some(typ) => typ,
    None => return diagnostics,
  };

  let arg_typs: Vec<TdTypEnum> = args
    .iter()
    .filter_map(|arg| actual_node_typ(db, arg.clone()).typ(db))
    .collect();

  let Some(signature) = callee_typ.call_typ(db, arg_typs) else {
    let (trimmed_offset, trimmed_len) = callee.node.trimmed_range();
    diagnostics
      .push(TdDiagnosticKind::NotCallable.into_error(trimmed_offset, trimmed_offset + trimmed_len));
    return diagnostics;
  };

  let params = signature.params(db);

  if params.len() != args.len() {
    let (trimmed_offset, trimmed_len) = callee.node.trimmed_range();
    diagnostics.push(
      TdDiagnosticKind::WrongArgCount {
        expected: params.len(),
        got: args.len(),
      }
      .into_error(trimmed_offset, trimmed_offset + trimmed_len),
    );
    return diagnostics;
  }

  for (param, arg_node) in params.iter().zip(args.iter()) {
    let arg_result = actual_node_typ(db, arg_node.clone());
    diagnostics.extend(arg_result.diagnostics(db).iter().cloned());
    if let Some(arg_type) = arg_result.typ(db)
      && !is_subtype_of(db, &arg_type, param)
    {
      let (trimmed_offset, trimmed_len) = arg_node.node.trimmed_range();
      diagnostics.push(
        TdDiagnosticKind::ArgTypMismatch {
          expected: param.display_name(db),
        }
        .into_error(trimmed_offset, trimmed_offset + trimmed_len),
      );
    }
  }

  diagnostics
}

fn check_index(
  db: &TypedownDatabase,
  expr: ProjectRedNode,
  indices: Vec<ProjectRedNode>,
) -> Vec<TdDiagnostic> {
  let mut diagnostics = vec![];

  let expr_result = actual_node_typ(db, expr.clone());
  diagnostics.extend(expr_result.diagnostics(db).iter().cloned());

  let expr_typ = match expr_result.typ(db) {
    Some(typ) => typ,
    None => return diagnostics,
  };

  // Unwrap literal types to their underlying type
  let expr_type = match &expr_typ {
    TdTypEnum::TdLitTyp(lit) => lit.underlying_typ(db),
    _ => expr_typ,
  };

  // If the base expr resolves to a type constructor (e.g. `list` in `list[string]`),
  // use the generic type's arity instead of the metatype (TdTypTyp has arity 0)
  let expr_typ = if expr_type.arity(db) == 0
    && IdentLit::cast(expr.node.clone()).is_some()
    && let Some(symbol) = referee(db, expr.clone()).value(db)
    && let Some(generic_typ) = evaluate_typ(db, symbol).typ(db)
    && generic_typ.arity(db) > 0
  {
    generic_typ
  } else {
    expr_type
  };

  // Type instantiation: no checking is needed because we do not support type bound, only check arity
  if expr_typ.arity(db) > 0 {
    return diagnostics;
  }

  // List element access: index must be a number
  if expr_typ.is_td_list_typ() {
    for index_node in &indices {
      let index_result = actual_node_typ(db, index_node.clone());
      diagnostics.extend(index_result.diagnostics(db).iter().cloned());
      if let Some(index_typ) = index_result.typ(db) {
        let number_typ = get_number_typ(db);
        if !is_subtype_of(db, &index_typ, &number_typ.into()) {
          let (trimmed_offset, trimmed_len) = index_node.node.trimmed_range();
          diagnostics.push(
            TdDiagnosticKind::IndexTypMismatch {
              expected: "number".to_string(),
            }
            .into_error(trimmed_offset, trimmed_offset + trimmed_len),
          );
        }
      }
    }
    return diagnostics;
  }

  // Dict element access: index must match key type
  if let TdTypEnum::TdDictTyp(dict) = &expr_typ {
    if let Some(key_typ) = dict.key(db).and_then(|lazy_typ| lazy_typ.resolve(db)) {
      for index_node in &indices {
        let index_result = actual_node_typ(db, index_node.clone());
        diagnostics.extend(index_result.diagnostics(db).iter().cloned());
        if let Some(index_typ) = index_result.typ(db)
          && !is_subtype_of(db, &index_typ, &key_typ)
        {
          let (trimmed_offset, trimmed_len) = index_node.node.trimmed_range();
          diagnostics.push(
            TdDiagnosticKind::IndexTypMismatch {
              expected: key_typ.display_name(db),
            }
            .into_error(trimmed_offset, trimmed_offset + trimmed_len),
          );
        }
      }
    }
    return diagnostics;
  }

  // String indexing is valid: index must be a number
  if expr_typ.is_td_string_typ() {
    for index_node in &indices {
      let index_result = actual_node_typ(db, index_node.clone());
      diagnostics.extend(index_result.diagnostics(db).iter().cloned());
      if let Some(index_typ) = index_result.typ(db) {
        let number_typ = get_number_typ(db);
        if !is_subtype_of(db, &index_typ, &number_typ.into()) {
          let (trimmed_offset, trimmed_len) = index_node.node.trimmed_range();
          diagnostics.push(
            TdDiagnosticKind::IndexTypMismatch {
              expected: "number".to_string(),
            }
            .into_error(trimmed_offset, trimmed_offset + trimmed_len),
          );
        }
      }
    }
    return diagnostics;
  }

  // Not indexable
  let (trimmed_offset, trimmed_len) = expr.node.trimmed_range();
  diagnostics
    .push(TdDiagnosticKind::NotIndexable.into_error(trimmed_offset, trimmed_offset + trimmed_len));

  diagnostics
}

fn check_prefix(db: &TypedownDatabase, op: &str, operand: ProjectRedNode) -> Vec<TdDiagnostic> {
  let mut diagnostics = vec![];

  let typecheck_result = typecheck(db, operand.clone());
  diagnostics.extend(typecheck_result.diagnostics(db).iter().cloned());

  let operand_result = actual_node_typ(db, operand.clone());
  let operand_typ = match operand_result.typ(db) {
    Some(typ) => typ,
    None => return diagnostics,
  };

  let expected_typ: TdTypEnum = match op {
    "-" | "+" => get_number_typ(db).into(),
    // ~ is logical not: accepts any type (only null and false are falsy)
    "~" => return diagnostics,
    _ => return diagnostics,
  };

  if !is_subtype_of(db, &operand_typ, &expected_typ) {
    let (trimmed_offset, trimmed_len) = operand.node.trimmed_range();
    diagnostics.push(
      TdDiagnosticKind::OperandTypMismatch {
        op: op.to_string(),
        expected: expected_typ.display_name(db),
      }
      .into_error(trimmed_offset, trimmed_offset + trimmed_len),
    );
  }

  diagnostics
}

fn check_postfix(db: &TypedownDatabase, op: &str, operand: ProjectRedNode) -> Vec<TdDiagnostic> {
  let mut diagnostics = vec![];
  let typecheck_result = typecheck(db, operand.clone());
  diagnostics.extend(typecheck_result.diagnostics(db).iter().cloned());

  if op == "?" {
    let operand_result = actual_node_typ(db, operand.clone());
    if let Some(operand_type) = operand_result.typ(db)
      && !operand_type.is_typ(db)
    {
      let (trimmed_offset, trimmed_len) = operand.node.trimmed_range();
      diagnostics.push(
        TdDiagnosticKind::OperandTypMismatch {
          op: "?".to_string(),
          expected: "type".to_string(),
        }
        .into_error(trimmed_offset, trimmed_offset + trimmed_len),
      );
    }
  }

  diagnostics
}

fn check_binary(
  db: &TypedownDatabase,
  op: &str,
  left: ProjectRedNode,
  right: ProjectRedNode,
) -> Vec<TdDiagnostic> {
  let mut diagnostics = vec![];

  let typecheck_left = typecheck(db, left.clone());
  diagnostics.extend(typecheck_left.diagnostics(db).iter().cloned());
  let typecheck_right = typecheck(db, right.clone());
  diagnostics.extend(typecheck_right.diagnostics(db).iter().cloned());

  let left_typ_result = actual_node_typ(db, left.clone()).typ(db);
  let right_typ_result = actual_node_typ(db, right.clone()).typ(db);

  match op {
    // Arithmetic: both operands must be number
    "+" | "-" | "*" | "/" | "%" | "**" => {
      let number_typ: TdTypEnum = get_number_typ(db).into();
      if let Some(left_typ) = &left_typ_result
        && !is_subtype_of(db, left_typ, &number_typ)
      {
        let (trimmed_offset, trimmed_len) = left.node.trimmed_range();
        diagnostics.push(
          TdDiagnosticKind::OperandTypMismatch {
            op: op.to_string(),
            expected: "number".to_string(),
          }
          .into_error(trimmed_offset, trimmed_offset + trimmed_len),
        );
      }
      if let Some(right_typ) = &right_typ_result
        && !is_subtype_of(db, right_typ, &number_typ)
      {
        let (trimmed_offset, trimmed_len) = right.node.trimmed_range();
        diagnostics.push(
          TdDiagnosticKind::OperandTypMismatch {
            op: op.to_string(),
            expected: "number".to_string(),
          }
          .into_error(trimmed_offset, trimmed_offset + trimmed_len),
        );
      }
    }
    // Logical: both operands must be boolean
    "&&" | "||" => {
      let bool_typ: TdTypEnum = get_bool_typ(db).into();
      if let Some(left_typ) = &left_typ_result
        && !is_subtype_of(db, left_typ, &bool_typ)
      {
        let (trimmed_offset, trimmed_len) = left.node.trimmed_range();
        diagnostics.push(
          TdDiagnosticKind::OperandTypMismatch {
            op: op.to_string(),
            expected: "boolean".to_string(),
          }
          .into_error(trimmed_offset, trimmed_offset + trimmed_len),
        );
      }
      if let Some(right_type) = &right_typ_result
        && !is_subtype_of(db, right_type, &bool_typ)
      {
        let (trimmed_offset, trimmed_len) = right.node.trimmed_range();
        diagnostics.push(
          TdDiagnosticKind::OperandTypMismatch {
            op: op.to_string(),
            expected: "boolean".to_string(),
          }
          .into_error(trimmed_offset, trimmed_offset + trimmed_len),
        );
      }
    }
    // Comparison: any type can be compared
    "==" | "!=" | "<" | ">" | "<=" | ">=" => {}
    _ => {}
  }

  diagnostics
}

fn check_sequence<'db>(
  db: &'db TypedownDatabase,
  declared_typ: &TdTypEnum<'db>,
  items: Vec<ProjectRedNode>,
) -> Vec<TdDiagnostic> {
  let mut diagnostics = vec![];

  // Get the element type from the list type
  let Some(list) = declared_typ.as_td_list_typ() else {
    return diagnostics;
  };

  let element_typ = match list.element(db).and_then(|element| element.resolve(db)) {
    Some(typ) => typ,
    // Uninstantiated list: no element type constraint
    None => return diagnostics,
  };

  for item in items {
    // Recursively typecheck each item
    let typcheck_result = typecheck(db, item.clone());
    diagnostics.extend(typcheck_result.diagnostics(db).iter().cloned());

    // Check item type against element type
    let item_result = actual_node_typ(db, item.clone());
    if let Some(item_type) = item_result.typ(db)
      && !is_subtype_of(db, &item_type, &element_typ)
    {
      let (trimmed_offset, trimmed_len) = item.node.trimmed_range();
      diagnostics.push(
        TdDiagnosticKind::ElementTypMismatch {
          expected: element_typ.display_name(db),
        }
        .into_error(trimmed_offset, trimmed_offset + trimmed_len),
      );
    }
  }

  diagnostics
}

#[cfg(test)]
mod tests {
  use crate::db::{
    derived::parse_file::parse_file, derived::typechecker::typecheck::typecheck,
    fixtures::load_vault_fixture,
  };
  use crate::syntax::diagnostic::*;

  // Mapping without _type: infers product type, no validation errors
  #[test]
  fn typecheck_mapping_without_typ_infers_product_no_errors() {
    let (db, project, file) = load_vault_fixture("typecheck/my_vault", "literal_value.td");
    let root_node = parse_file(&db, project, file).ast(&db);
    let result = typecheck(&db, root_node);
    assert!(
      result.diagnostics(&db).is_empty(),
      "mapping without _type infers product type, no errors expected: {:?}",
      result.diagnostics(&db)
    );
  }

  // _type references a non-existent schema
  #[test]
  fn typecheck_unresolved_typ_has_diagnostics() {
    let (db, project, file) = load_vault_fixture("typecheck/my_vault", "unresolved_type.td");
    let root_node = parse_file(&db, project, file).ast(&db);
    let result = typecheck(&db, root_node);
    assert!(
      !result.diagnostics(&db).is_empty(),
      "expected diagnostics for unresolved schema"
    );
  }

  #[test]
  fn typecheck_mapping_with_ident_value() {
    let (db, project, file) = load_vault_fixture("typecheck/my_vault", "ident_value.td");
    let root_node = parse_file(&db, project, file).ast(&db);
    let result = typecheck(&db, root_node);
    assert!(
      result.diagnostics(&db).is_empty(),
      "expected no diagnostics, got: {:?}",
      result.diagnostics(&db)
    );
  }

  #[test]
  fn typecheck_excess_field_has_diagnostics() {
    let (db, project, file) = load_vault_fixture("typecheck/my_vault", "excess_field.td");
    let root_node = parse_file(&db, project, file).ast(&db);
    let result = typecheck(&db, root_node);
    let diagnostics = result.diagnostics(&db);
    assert!(
      diagnostics
        .iter()
        .any(|diagnostic| matches!(diagnostic, TdDiagnostic { kind: TdDiagnosticKind::UnknownField { field, on_typ, ..}, .. } if field == "favorite_color" && on_typ == "Person")),
      "expected UnknownField for 'favorite_color', got: {:?}",
      diagnostics
    );
  }

  #[test]
  fn typecheck_schema_missing_properties_has_diagnostics() {
    let (db, project, file) =
      load_vault_fixture("typecheck/my_vault", "schema_missing_properties.td");
    let root_node = parse_file(&db, project, file).ast(&db);
    let result = typecheck(&db, root_node);
    let diagnostics = result.diagnostics(&db);
    // properties is optional: a schema without it is valid
    assert!(
      diagnostics.is_empty(),
      "schema without properties should have no errors, got: {:?}",
      diagnostics
    );
  }

  #[test]
  fn typecheck_valid_person_no_errors() {
    let (db, project, file) = load_vault_fixture("typecheck/my_vault", "valid_person.td");
    let root_node = parse_file(&db, project, file).ast(&db);
    let result = typecheck(&db, root_node);
    assert!(
      result.diagnostics(&db).is_empty(),
      "valid Person should have no errors: {:?}",
      result.diagnostics(&db)
    );
  }

  #[test]
  fn typecheck_wrong_field_typ_has_diagnostics() {
    let (db, project, file) = load_vault_fixture("typecheck/my_vault", "wrong_field_type.td");
    let root_node = parse_file(&db, project, file).ast(&db);
    let result = typecheck(&db, root_node);
    let diagnostics = result.diagnostics(&db);
    assert!(
      diagnostics.iter().any(|diagnostic| matches!(diagnostic, TdDiagnostic { kind: TdDiagnosticKind::FieldTypMismatch { field, expected, ..}, .. } if field == "name" && expected == "string")),
      "expected FieldTypeMismatch for 'name' with expected 'string', got: {:?}",
      diagnostics
    );
  }

  #[test]
  fn typecheck_nested_valid_no_errors() {
    let (db, project, file) = load_vault_fixture("typecheck/my_vault", "nested_valid.td");
    let root_node = parse_file(&db, project, file).ast(&db);
    let result = typecheck(&db, root_node);
    assert!(
      result.diagnostics(&db).is_empty(),
      "valid nested PersonWithAddress should have no errors: {:?}",
      result.diagnostics(&db)
    );
  }

  #[test]
  fn typecheck_nested_wrong_type_has_diagnostics() {
    let (db, project, file) = load_vault_fixture("typecheck/my_vault", "nested_wrong_type.td");
    let root_node = parse_file(&db, project, file).ast(&db);
    let result = typecheck(&db, root_node);
    let diags = result.diagnostics(&db);
    assert!(
      diags
        .iter()
        .any(|diagnostic| matches!(diagnostic, TdDiagnostic { kind: TdDiagnosticKind::FieldTypMismatch { field, ..}, .. } if field == "address")),
      "expected FieldTypeMismatch for 'address', got: {:?}",
      diags
    );
  }

  #[test]
  fn typecheck_prefix_valid() {
    let (db, project, file) = load_vault_fixture("typecheck/my_vault", "unary_valid.td");
    let root_node = parse_file(&db, project, file).ast(&db);
    let result = typecheck(&db, root_node);
    assert!(
      result.diagnostics(&db).is_empty(),
      "unary minus on number should have no errors: {:?}",
      result.diagnostics(&db)
    );
  }

  #[test]
  fn typecheck_prefix_wrong_typ() {
    let (db, project, file) = load_vault_fixture("typecheck/my_vault", "unary_wrong_type.td");
    let root_node = parse_file(&db, project, file).ast(&db);
    let result = typecheck(&db, root_node);
    assert!(
      result.diagnostics(&db).iter().any(|diagnostic| matches!(
        diagnostic,
        TdDiagnostic {
          kind: TdDiagnosticKind::OperandTypMismatch { .. },
          ..
        }
      )),
      "expected OperandTypeMismatch"
    );
  }

  #[test]
  fn typecheck_binary_valid() {
    let (db, project, file) = load_vault_fixture("typecheck/my_vault", "binary_valid.td");
    let root_node = parse_file(&db, project, file).ast(&db);
    let result = typecheck(&db, root_node);
    assert!(
      result.diagnostics(&db).is_empty(),
      "binary addition should have no errors: {:?}",
      result.diagnostics(&db)
    );
  }

  #[test]
  fn typecheck_binary_wrong_typ() {
    let (db, project, file) = load_vault_fixture("typecheck/my_vault", "binary_wrong_type.td");
    let root_node = parse_file(&db, project, file).ast(&db);
    let result = typecheck(&db, root_node);
    assert!(
      result.diagnostics(&db).iter().any(|diagnostic| matches!(
        diagnostic,
        TdDiagnostic {
          kind: TdDiagnosticKind::OperandTypMismatch { .. },
          ..
        }
      )),
      "expected OperandTypeMismatch"
    );
  }

  #[test]
  fn typecheck_math_field_valid() {
    let (db, project, file) = load_vault_fixture("typecheck/my_vault", "valid_math.td");
    let root_node = parse_file(&db, project, file).ast(&db);
    let result = typecheck(&db, root_node);
    assert!(
      result.diagnostics(&db).is_empty(),
      "math field should typecheck: {:?}",
      result.diagnostics(&db)
    );
  }

  #[test]
  fn typecheck_markdown_body_with_interpolation() {
    let (db, project, file) = load_vault_fixture("typecheck/my_vault", "valid_markdown.td");
    let root_node = parse_file(&db, project, file).ast(&db);
    let result = typecheck(&db, root_node);
    assert!(
      result.diagnostics(&db).is_empty(),
      "markdown body should typecheck: {:?}",
      result.diagnostics(&db)
    );
  }

  #[test]
  fn typecheck_literal_typ_valid() {
    let (db, project, file) = load_vault_fixture("typecheck/my_vault", "valid_status.td");
    let root_node = parse_file(&db, project, file).ast(&db);
    let result = typecheck(&db, root_node);
    assert!(
      result.diagnostics(&db).is_empty(),
      r#"state: "draft" should match: {:?}"#,
      result.diagnostics(&db)
    );
  }

  #[test]
  fn typecheck_literal_typ_mismatch() {
    let (db, project, file) = load_vault_fixture("typecheck/my_vault", "invalid_status.td");
    let root_node = parse_file(&db, project, file).ast(&db);
    let result = typecheck(&db, root_node);
    assert!(
      result
        .diagnostics(&db)
        .iter()
        .any(|diagnostic| matches!(diagnostic, TdDiagnostic { kind: TdDiagnosticKind::FieldTypMismatch { field, ..}, .. } if field == "state")),
      "state mismatch expected"
    );
  }

  #[test]
  fn typecheck_date_time_fields_accept_quoted_strings() {
    let (db, project, file) = load_vault_fixture("typecheck/my_vault", "valid_event.td");
    let root_node = parse_file(&db, project, file).ast(&db);
    let result = typecheck(&db, root_node);
    assert!(
      result.diagnostics(&db).is_empty(),
      "date/time fields should accept strings: {:?}",
      result.diagnostics(&db)
    );
  }

  #[test]
  fn typecheck_string_with_inline_math_no_errors() {
    let (db, project, file) = load_vault_fixture("typecheck/my_vault", "math_in_string.td");
    let root_node = parse_file(&db, project, file).ast(&db);
    let result = typecheck(&db, root_node);
    assert!(
      result.diagnostics(&db).is_empty(),
      "string with inline math: {:?}",
      result.diagnostics(&db)
    );
  }

  #[test]
  fn typecheck_string_with_multiple_inline_math_no_errors() {
    let (db, project, file) = load_vault_fixture("typecheck/my_vault", "math_mixed_string.td");
    let root_node = parse_file(&db, project, file).ast(&db);
    let result = typecheck(&db, root_node);
    assert!(
      result.diagnostics(&db).is_empty(),
      "multiple inline math: {:?}",
      result.diagnostics(&db)
    );
  }

  #[test]
  fn typecheck_math_only_string_as_math_field() {
    let (db, project, file) = load_vault_fixture("typecheck/my_vault", "math_only_string.td");
    let root_node = parse_file(&db, project, file).ast(&db);
    let result = typecheck(&db, root_node);
    assert!(
      result.diagnostics(&db).is_empty(),
      "math-only string: {:?}",
      result.diagnostics(&db)
    );
  }

  #[test]
  fn typecheck_schema_with_list_typ_no_errors() {
    let (db, project, file) = load_vault_fixture("typecheck/my_vault", "_types/WithListType.td");
    let root_node = parse_file(&db, project, file).ast(&db);
    let result = typecheck(&db, root_node);
    assert!(
      result.diagnostics(&db).is_empty(),
      "schema with list[string]: {:?}",
      result.diagnostics(&db)
    );
  }

  #[test]
  fn typecheck_circular_schema_no_errors() {
    let (db, project, file) = load_vault_fixture("typecheck/my_vault", "_types/CircularA.td");
    let root_node = parse_file(&db, project, file).ast(&db);
    let result = typecheck(&db, root_node);
    assert!(
      result.diagnostics(&db).is_empty(),
      "circular schema: {:?}",
      result.diagnostics(&db)
    );
  }

  #[test]
  fn typecheck_schema_with_bare_user_typ_ref() {
    let (db, project, file) = load_vault_fixture("typecheck/my_vault", "_types/WithBareRef.td");
    let root_node = parse_file(&db, project, file).ast(&db);
    let result = typecheck(&db, root_node);
    assert!(
      result.diagnostics(&db).is_empty(),
      "bare ref: {:?}",
      result.diagnostics(&db)
    );
  }

  #[test]
  fn typecheck_schema_with_self_ref() {
    let (db, project, file) = load_vault_fixture("typecheck/my_vault", "_types/SelfRef.td");
    let root_node = parse_file(&db, project, file).ast(&db);
    let result = typecheck(&db, root_node);
    assert!(
      result.diagnostics(&db).is_empty(),
      "self ref: {:?}",
      result.diagnostics(&db)
    );
  }

  #[test]
  fn typecheck_schema_with_list_of_user_typ_no_errors() {
    let (db, project, file) = load_vault_fixture("typecheck/my_vault", "_types/WithRefList.td");
    let root_node = parse_file(&db, project, file).ast(&db);
    let result = typecheck(&db, root_node);
    assert!(
      result.diagnostics(&db).is_empty(),
      "list[Person]: {:?}",
      result.diagnostics(&db)
    );
  }

  #[test]
  fn typecheck_circular_content_no_errors() {
    let (db, project, file) = load_vault_fixture("typecheck/my_vault", "circular_ref.td");
    let root_node = parse_file(&db, project, file).ast(&db);
    let result = typecheck(&db, root_node);
    assert!(
      result.diagnostics(&db).is_empty(),
      "circular ref: {:?}",
      result.diagnostics(&db)
    );
  }

  #[test]
  fn typecheck_schema_simple_props_no_errors() {
    let (db, project, file) = load_vault_fixture("typecheck/my_vault", "_types/SimpleProps.td");
    let root_node = parse_file(&db, project, file).ast(&db);
    let result = typecheck(&db, root_node);
    assert!(
      result.diagnostics(&db).is_empty(),
      "simple props: {:?}",
      result.diagnostics(&db)
    );
  }

  #[test]
  fn typecheck_schema_with_optional_no_errors() {
    let (db, project, file) = load_vault_fixture("typecheck/my_vault", "_types/WithOptional.td");
    let root_node = parse_file(&db, project, file).ast(&db);
    let root_node = root_node;
    let result = typecheck(&db, root_node);
    assert!(
      result.diagnostics(&db).is_empty(),
      "optional: {:?}",
      result.diagnostics(&db)
    );
  }

  // Schema using ? postfix (e.g. string?, date?) should parse as T | null
  #[test]
  fn typecheck_schema_with_postfix_optional_no_errors() {
    let (db, project, file) =
      load_vault_fixture("typecheck/my_vault", "_types/WithPostfixOptional.td");
    let root_node = parse_file(&db, project, file).ast(&db);
    let result = typecheck(&db, root_node);
    assert!(
      result.diagnostics(&db).is_empty(),
      "postfix optional schema: {:?}",
      result.diagnostics(&db)
    );
  }

  // Content file using a schema with ? postfix fields should typecheck without errors
  #[test]
  fn typecheck_content_with_postfix_optional_no_errors() {
    let (db, project, file) = load_vault_fixture("typecheck/my_vault", "with_postfix_optional.td");
    let root_node = parse_file(&db, project, file).ast(&db);
    let result = typecheck(&db, root_node);
    assert!(
      result.diagnostics(&db).is_empty(),
      "content with postfix optional fields should have no errors: {:?}",
      result.diagnostics(&db)
    );
  }

  #[test]
  fn typecheck_schema_with_pipe_union_no_errors() {
    let (db, project, file) = load_vault_fixture("typecheck/my_vault", "_types/WithPipeUnion.td");
    let root_node = parse_file(&db, project, file).ast(&db);
    let result = typecheck(&db, root_node);
    assert!(
      result.diagnostics(&db).is_empty(),
      "pipe union schema: {:?}",
      result.diagnostics(&db)
    );
  }

  #[test]
  fn typecheck_content_with_pipe_union_valid() {
    let (db, project, file) = load_vault_fixture("typecheck/my_vault", "valid_pipe_union.td");
    let root_node = parse_file(&db, project, file).ast(&db);
    let result = typecheck(&db, root_node);
    assert!(
      result.diagnostics(&db).is_empty(),
      "valid pipe union content: {:?}",
      result.diagnostics(&db)
    );
  }

  #[test]
  fn typecheck_content_with_pipe_union_invalid() {
    let (db, project, file) = load_vault_fixture("typecheck/my_vault", "invalid_pipe_union.td");
    let root_node = parse_file(&db, project, file).ast(&db);
    let result = typecheck(&db, root_node);
    assert!(
      !result.diagnostics(&db).is_empty(),
      "invalid pipe union content should have errors"
    );
  }

  #[test]
  fn typecheck_schema_with_explicit_typ_tag_no_errors() {
    let (db, project, file) =
      load_vault_fixture("typecheck/my_vault", "_types/WithExplicitTypeTag.td");
    let root_node = parse_file(&db, project, file).ast(&db);
    let result = typecheck(&db, root_node);
    assert!(
      result.diagnostics(&db).is_empty(),
      "explicit type tag: {:?}",
      result.diagnostics(&db)
    );
  }

  #[test]
  fn typecheck_schema_with_union_typ_no_errors() {
    let (db, project, file) = load_vault_fixture("typecheck/my_vault", "_types/WithUnion.td");
    let root_node = parse_file(&db, project, file).ast(&db);
    let result = typecheck(&db, root_node);
    assert!(
      result.diagnostics(&db).is_empty(),
      "union: {:?}",
      result.diagnostics(&db)
    );
  }

  #[test]
  fn typecheck_schema_with_nested_typ_no_errors() {
    let (db, project, file) = load_vault_fixture("typecheck/my_vault", "_types/WithNestedType.td");
    let root_node = parse_file(&db, project, file).ast(&db);
    let result = typecheck(&db, root_node);
    assert!(
      result.diagnostics(&db).is_empty(),
      "nested type: {:?}",
      result.diagnostics(&db)
    );
  }

  #[test]
  fn typecheck_schema_with_lit_typ_no_errors() {
    let (db, project, file) = load_vault_fixture("typecheck/my_vault", "_types/WithLiteralType.td");
    let root_node = parse_file(&db, project, file).ast(&db);
    let result = typecheck(&db, root_node);
    assert!(
      result.diagnostics(&db).is_empty(),
      "literal type: {:?}",
      result.diagnostics(&db)
    );
  }

  #[test]
  fn typecheck_schema_properties_not_mapping_has_errors() {
    let (db, project, file) =
      load_vault_fixture("typecheck/my_vault", "_types/PropertiesNotMapping.td");
    let root_node = parse_file(&db, project, file).ast(&db);
    let result = typecheck(&db, root_node);
    assert!(
      !result.diagnostics(&db).is_empty(),
      "non-mapping properties should error"
    );
  }

  #[test]
  fn typecheck_schema_missing_properties_field() {
    let (db, project, file) =
      load_vault_fixture("typecheck/my_vault", "_types/MissingProperties.td");
    let root_node = parse_file(&db, project, file).ast(&db);
    let result = typecheck(&db, root_node);
    // properties is optional: a schema without it is valid
    assert!(
      result.diagnostics(&db).is_empty(),
      "schema without properties should have no errors: {:?}",
      result.diagnostics(&db)
    );
  }

  #[test]
  fn typecheck_schema_prop_descriptor_extra_field_no_typecheck_errors() {
    let (db, project, file) =
      load_vault_fixture("typecheck/my_vault", "_types/PropDescriptorExtraField.td");
    let root_node = parse_file(&db, project, file).ast(&db);
    let result = typecheck(&db, root_node);
    assert!(
      result.diagnostics(&db).is_empty(),
      "extra fields: {:?}",
      result.diagnostics(&db)
    );
  }

  #[test]
  fn typecheck_schema_prop_descriptor_missing_typ_has_errors() {
    let (db, project, file) =
      load_vault_fixture("typecheck/my_vault", "_types/PropDescriptorMissingType.td");
    let root_node = parse_file(&db, project, file).ast(&db);
    let result = typecheck(&db, root_node);
    assert!(
      !result.diagnostics(&db).is_empty(),
      "missing type in prop descriptor should error"
    );
  }

  #[test]
  fn typecheck_valid_list_sum_no_errors() {
    let (db, project, file) = load_vault_fixture("typecheck/narrow_vault", "valid_list_sum.td");
    let root_node = parse_file(&db, project, file).ast(&db);
    let result = typecheck(&db, root_node);
    assert!(
      result.diagnostics(&db).is_empty(),
      r#"string | number should accept "hello": {:?}"#,
      result.diagnostics(&db)
    );
  }

  #[test]
  fn typecheck_invalid_list_sum_has_errors() {
    let (db, project, file) = load_vault_fixture("typecheck/narrow_vault", "invalid_list_sum.td");
    let root_node = parse_file(&db, project, file).ast(&db);
    let result = typecheck(&db, root_node);
    assert!(!result.diagnostics(&db).is_empty(), "should reject true");
  }

  #[test]
  fn typecheck_valid_dict_sum_no_errors() {
    let (db, project, file) = load_vault_fixture("typecheck/narrow_vault", "valid_dict_sum.td");
    let root_node = parse_file(&db, project, file).ast(&db);
    let result = typecheck(&db, root_node);
    assert!(
      result.diagnostics(&db).is_empty(),
      "dict sum: {:?}",
      result.diagnostics(&db)
    );
  }

  #[test]
  fn typecheck_invalid_dict_sum_has_errors() {
    let (db, project, file) = load_vault_fixture("typecheck/narrow_vault", "invalid_dict_sum.td");
    let root_node = parse_file(&db, project, file).ast(&db);
    let result = typecheck(&db, root_node);
    assert!(!result.diagnostics(&db).is_empty(), "should reject boolean");
  }

  #[test]
  fn typecheck_mixed_union_accepts_string() {
    let (db, project, file) = load_vault_fixture("typecheck/narrow_vault", "mixed_union_string.td");
    let root_node = parse_file(&db, project, file).ast(&db);
    let result = typecheck(&db, root_node);
    assert!(
      result.diagnostics(&db).is_empty(),
      "mixed union string: {:?}",
      result.diagnostics(&db)
    );
  }

  #[test]
  fn typecheck_mixed_union_accepts_literal() {
    let (db, project, file) =
      load_vault_fixture("typecheck/narrow_vault", "mixed_union_literal.td");
    let root_node = parse_file(&db, project, file).ast(&db);
    let result = typecheck(&db, root_node);
    assert!(
      result.diagnostics(&db).is_empty(),
      "mixed union literal: {:?}",
      result.diagnostics(&db)
    );
  }

  #[test]
  fn typecheck_mixed_union_accepts_number() {
    let (db, project, file) = load_vault_fixture("typecheck/narrow_vault", "mixed_union_number.td");
    let root_node = parse_file(&db, project, file).ast(&db);
    let result = typecheck(&db, root_node);
    assert!(
      result.diagnostics(&db).is_empty(),
      "mixed union number: {:?}",
      result.diagnostics(&db)
    );
  }

  #[test]
  fn typecheck_mixed_union_rejects_bool() {
    let (db, project, file) =
      load_vault_fixture("typecheck/narrow_vault", "mixed_union_invalid.td");
    let root_node = parse_file(&db, project, file).ast(&db);
    let result = typecheck(&db, root_node);
    assert!(!result.diagnostics(&db).is_empty(), "should reject true");
  }

  #[test]
  fn typecheck_nested_product_valid() {
    let (db, project, file) = load_vault_fixture("typecheck/narrow_vault", "nested_valid.td");
    let root_node = parse_file(&db, project, file).ast(&db);
    let result = typecheck(&db, root_node);
    assert!(
      result.diagnostics(&db).is_empty(),
      "nested valid: {:?}",
      result.diagnostics(&db)
    );
  }

  #[test]
  fn typecheck_nested_product_wrong_field_typ() {
    let (db, project, file) = load_vault_fixture("typecheck/narrow_vault", "nested_wrong_type.td");
    let root_node = parse_file(&db, project, file).ast(&db);
    let result = typecheck(&db, root_node);
    assert!(!result.diagnostics(&db).is_empty(), "should fail");
  }

  #[test]
  fn typecheck_contrived_valid() {
    let (db, project, file) = load_vault_fixture("typecheck/narrow_vault", "contrived_valid.td");
    let root_node = parse_file(&db, project, file).ast(&db);
    let result = typecheck(&db, root_node);
    assert!(
      result.diagnostics(&db).is_empty(),
      "contrived valid: {:?}",
      result.diagnostics(&db)
    );
  }

  #[test]
  fn typecheck_contrived_wrong_lit_number() {
    let (db, project, file) =
      load_vault_fixture("typecheck/narrow_vault", "contrived_wrong_literal.td");
    let root_node = parse_file(&db, project, file).ast(&db);
    let result = typecheck(&db, root_node);
    assert!(
      !result.diagnostics(&db).is_empty(),
      "literal_num: 99 should fail"
    );
  }

  #[test]
  fn typecheck_contrived_missing_required_nested() {
    let (db, project, file) =
      load_vault_fixture("typecheck/narrow_vault", "contrived_missing_required.td");
    let root_node = parse_file(&db, project, file).ast(&db);
    let result = typecheck(&db, root_node);
    assert!(
      !result.diagnostics(&db).is_empty(),
      "missing required should fail"
    );
  }

  #[test]
  fn typecheck_contrived_mixed_accepts_lit_number() {
    let (db, project, file) =
      load_vault_fixture("typecheck/narrow_vault", "contrived_mixed_num.td");
    let root_node = parse_file(&db, project, file).ast(&db);
    let result = typecheck(&db, root_node);
    assert!(
      result.diagnostics(&db).is_empty(),
      "mixed num: {:?}",
      result.diagnostics(&db)
    );
  }

  #[test]
  fn typecheck_contrived_mixed_accepts_lit_bool() {
    let (db, project, file) =
      load_vault_fixture("typecheck/narrow_vault", "contrived_mixed_bool.td");
    let root_node = parse_file(&db, project, file).ast(&db);
    let result = typecheck(&db, root_node);
    assert!(
      result.diagnostics(&db).is_empty(),
      "mixed bool: {:?}",
      result.diagnostics(&db)
    );
  }

  #[test]
  fn typecheck_fref_narrower_union_field() {
    let (db, project, file) =
      load_vault_fixture("typecheck/narrow_vault", "article_fref_status.td");
    let root_node = parse_file(&db, project, file).ast(&db);
    let result = typecheck(&db, root_node);
    assert!(
      result.diagnostics(&db).is_empty(),
      "fref narrower: {:?}",
      result.diagnostics(&db)
    );
  }

  // Binary expression in a schema number field should pass
  #[test]
  fn typecheck_binary_expr_in_schema_number_field() {
    let (db, project, file) = load_vault_fixture("typecheck/my_vault", "binary_schema_valid.td");
    let root_node = parse_file(&db, project, file).ast(&db);
    let result = typecheck(&db, root_node);
    assert!(
      result.diagnostics(&db).is_empty(),
      "age: 10 + 20 should pass: {:?}",
      result.diagnostics(&db)
    );
  }

  // Binary expression with wrong operand type should fail
  #[test]
  fn typecheck_binary_expr_wrong_operand_typ() {
    let (db, project, file) =
      load_vault_fixture("typecheck/my_vault", "binary_schema_wrong_operand.td");
    let root_node = parse_file(&db, project, file).ast(&db);
    let result = typecheck(&db, root_node);
    assert!(
      result.diagnostics(&db).iter().any(|diagnostic| matches!(
        diagnostic,
        TdDiagnostic {
          kind: TdDiagnosticKind::OperandTypMismatch { .. },
          ..
        }
      )),
      "1 + true should report operand mismatch"
    );
  }

  // Binary expression result assigned to wrong field type should fail
  #[test]
  fn typecheck_binary_expr_wrong_field_typ() {
    let (db, project, file) =
      load_vault_fixture("typecheck/my_vault", "binary_schema_wrong_field_type.td");
    let root_node = parse_file(&db, project, file).ast(&db);
    let result = typecheck(&db, root_node);
    assert!(
      result
        .diagnostics(&db)
        .iter()
        .any(|diagnostic| matches!(diagnostic, TdDiagnostic { kind: TdDiagnosticKind::FieldTypMismatch { field, ..}, .. } if field == "name")),
      "name: 1 + 2 should report field type mismatch"
    );
  }

  // Parenthesized expression in schema field should pass
  #[test]
  fn typecheck_paren_expr_in_schema_field() {
    let (db, project, file) = load_vault_fixture("typecheck/my_vault", "paren_schema_valid.td");
    let root_node = parse_file(&db, project, file).ast(&db);
    let result = typecheck(&db, root_node);
    assert!(
      result.diagnostics(&db).is_empty(),
      "age: (10 + 20) * 3 should pass: {:?}",
      result.diagnostics(&db)
    );
  }

  // Unary expression in schema number field should pass
  #[test]
  fn typecheck_unary_expr_in_schema_field() {
    let (db, project, file) = load_vault_fixture("typecheck/my_vault", "unary_schema_valid.td");
    let root_node = parse_file(&db, project, file).ast(&db);
    let result = typecheck(&db, root_node);
    assert!(
      result.diagnostics(&db).is_empty(),
      "age: -42 should pass: {:?}",
      result.diagnostics(&db)
    );
  }

  // Nested parenthesized binary: (1 + 2) * (3 + 4) in number field
  #[test]
  fn typecheck_nested_paren_binary_valid() {
    let (db, project, file) = load_vault_fixture("typecheck/my_vault", "nested_binary_valid.td");
    let root_node = parse_file(&db, project, file).ast(&db);
    let result = typecheck(&db, root_node);
    assert!(
      result.diagnostics(&db).is_empty(),
      "(1 + 2) * (3 + 4) should pass: {:?}",
      result.diagnostics(&db)
    );
  }

  // Nested binary with wrong operand deep inside parens
  #[test]
  fn typecheck_nested_paren_binary_wrong() {
    let (db, project, file) = load_vault_fixture("typecheck/my_vault", "nested_binary_wrong.td");
    let root_node = parse_file(&db, project, file).ast(&db);
    let result = typecheck(&db, root_node);
    assert!(
      result.diagnostics(&db).iter().any(|diagnostic| matches!(
        diagnostic,
        TdDiagnostic {
          kind: TdDiagnosticKind::OperandTypMismatch { .. },
          ..
        }
      )),
      "(1 + true) * 3 should report operand mismatch"
    );
  }

  // Comparison expression is valid schemaless
  #[test]
  fn typecheck_comparison_valid() {
    let (db, project, file) = load_vault_fixture("typecheck/my_vault", "comparison_valid.td");
    let root_node = parse_file(&db, project, file).ast(&db);
    let result = typecheck(&db, root_node);
    assert!(
      result.diagnostics(&db).is_empty(),
      "1 == 2 should pass: {:?}",
      result.diagnostics(&db)
    );
  }

  // Logical operators with comparison operands
  #[test]
  fn typecheck_logical_with_comparisons_valid() {
    let (db, project, file) = load_vault_fixture("typecheck/my_vault", "logical_valid.td");
    let root_node = parse_file(&db, project, file).ast(&db);
    let result = typecheck(&db, root_node);
    assert!(
      result.diagnostics(&db).is_empty(),
      "(1 == 2) && (3 == 4) should pass: {:?}",
      result.diagnostics(&db)
    );
  }

  // Comparison with null should be valid
  #[test]
  fn typecheck_comparison_null_valid() {
    let (db, project, file) = load_vault_fixture("typecheck/my_vault", "comparison_null.td");
    let root_node = parse_file(&db, project, file).ast(&db);
    let result = typecheck(&db, root_node);
    assert!(
      result.diagnostics(&db).is_empty(),
      "1 == null should pass: {:?}",
      result.diagnostics(&db)
    );
  }

  // null == null should be valid
  #[test]
  fn typecheck_comparison_null_null_valid() {
    let (db, project, file) = load_vault_fixture("typecheck/my_vault", "comparison_null_null.td");
    let root_node = parse_file(&db, project, file).ast(&db);
    let result = typecheck(&db, root_node);
    assert!(
      result.diagnostics(&db).is_empty(),
      "null == null should pass: {:?}",
      result.diagnostics(&db)
    );
  }

  // Logical operators with non-boolean operands
  #[test]
  fn typecheck_logical_wrong_operand() {
    let (db, project, file) = load_vault_fixture("typecheck/my_vault", "logical_wrong_operand.td");
    let root_node = parse_file(&db, project, file).ast(&db);
    let result = typecheck(&db, root_node);
    assert!(
      result.diagnostics(&db).iter().any(|diagnostic| matches!(
        diagnostic,
        TdDiagnostic {
          kind: TdDiagnosticKind::OperandTypMismatch { .. },
          ..
        }
      )),
      "1 && 2 should report operand mismatch"
    );
  }

  // Closure referencing self should typecheck
  #[test]
  fn typecheck_closure_self_ref() {
    let (db, project, file) = load_vault_fixture("typecheck/my_vault", "closure_self_ref.td");
    let root_node = parse_file(&db, project, file).ast(&db);
    let result = typecheck(&db, root_node);
    assert!(
      result.diagnostics(&db).is_empty(),
      "closure referencing self.a should pass: {:?}",
      result.diagnostics(&db)
    );
  }

  // Internal file (_partials/colors.td) infers as product type with no errors
  #[test]
  fn typecheck_internal_file_infers_product() {
    let (db, project, file) = load_vault_fixture("typecheck/my_vault", "_partials/colors.td");
    let root_node = parse_file(&db, project, file).ast(&db);
    let result = typecheck(&db, root_node);
    assert!(
      result.diagnostics(&db).is_empty(),
      "internal file should typecheck cleanly: {:?}",
      result.diagnostics(&db)
    );
  }

  // File with _imports typechecks with no errors when types are compatible
  #[test]
  fn typecheck_with_imports_no_errors() {
    let (db, project, file) = load_vault_fixture("typecheck/my_vault", "with_imports.td");
    let root_node = parse_file(&db, project, file).ast(&db);
    let result = typecheck(&db, root_node);
    assert!(
      result.diagnostics(&db).is_empty(),
      "file with valid imports should typecheck cleanly: {:?}",
      result.diagnostics(&db)
    );
  }

  // File with _imports has type error when imported field type mismatches schema
  #[test]
  fn typecheck_with_imports_wrong_typ_has_diagnostics() {
    let (db, project, file) =
      load_vault_fixture("typecheck/my_vault", "with_imports_wrong_type.td");
    let root_node = parse_file(&db, project, file).ast(&db);
    let result = typecheck(&db, root_node);
    assert!(
      result.diagnostics(&db).iter().any(|diagnostic| matches!(
        diagnostic,
        TdDiagnostic {
          kind: TdDiagnosticKind::FieldTypMismatch { .. },
          ..
        }
      )),
      "should have FieldTypeMismatch for string assigned to number field: {:?}",
      result.diagnostics(&db)
    );
  }

  // String literal indexing should not produce NotIndexable
  #[test]
  fn typecheck_string_lit_index_no_errors() {
    let (db, project, file) = load_vault_fixture("typecheck/my_vault", "str_literal_index.td");
    let root_node = parse_file(&db, project, file).ast(&db);
    let result = typecheck(&db, root_node);
    assert!(
      !result.diagnostics(&db).iter().any(|diagnostic| matches!(
        diagnostic,
        TdDiagnostic {
          kind: TdDiagnosticKind::NotIndexable,
          ..
        }
      )),
      "string literal indexing should not be flagged as not indexable: {:?}",
      result.diagnostics(&db)
    );
  }

  // ${fref("file.td").validField} in markdown body should typecheck cleanly
  #[test]
  fn typecheck_md_body_fref_valid_field_no_errors() {
    let (db, project, file) = load_vault_fixture("typecheck/my_vault", "md_body_fref_valid.td");
    let root_node = parse_file(&db, project, file).ast(&db);
    let result = typecheck(&db, root_node);
    assert!(
      result.diagnostics(&db).is_empty(),
      "fref().validField in markdown body should typecheck cleanly: {:?}",
      result.diagnostics(&db)
    );
  }

  // ${fref("file.td").nonexistentField} in markdown body should emit UnknownField
  #[test]
  fn typecheck_md_body_fref_unknown_field_has_diagnostic() {
    let (db, project, file) =
      load_vault_fixture("typecheck/my_vault", "md_body_fref_unknown_field.td");
    let root_node = parse_file(&db, project, file).ast(&db);
    let result = typecheck(&db, root_node);
    assert!(
      result.diagnostics(&db).iter().any(|diagnostic| matches!(
        diagnostic,
        TdDiagnostic {
          kind: TdDiagnosticKind::UnknownField { .. },
          ..
        }
      )),
      "fref().nonexistentField in markdown body should emit UnknownField: {:?}",
      result.diagnostics(&db)
    );
  }

  // ${1 + 2} in markdown body should typecheck cleanly
  #[test]
  fn typecheck_md_body_valid_arithmetic_no_errors() {
    let (db, project, file) =
      load_vault_fixture("typecheck/my_vault", "md_body_valid_arithmetic.td");
    let root_node = parse_file(&db, project, file).ast(&db);
    let result = typecheck(&db, root_node);
    assert!(
      result.diagnostics(&db).is_empty(),
      "valid arithmetic in markdown body should have no errors: {:?}",
      result.diagnostics(&db)
    );
  }

  // ${1 + true} in markdown body should emit OperandTypMismatch
  #[test]
  fn typecheck_md_body_binary_wrong_operand_has_diagnostic() {
    let (db, project, file) =
      load_vault_fixture("typecheck/my_vault", "md_body_binary_wrong_operand.td");
    let root_node = parse_file(&db, project, file).ast(&db);
    let result = typecheck(&db, root_node);
    assert!(
      result.diagnostics(&db).iter().any(|diagnostic| matches!(
        diagnostic,
        TdDiagnostic {
          kind: TdDiagnosticKind::OperandTypMismatch { .. },
          ..
        }
      )),
      "1 + true in markdown body should emit OperandTypMismatch: {:?}",
      result.diagnostics(&db)
    );
  }

  // ${-"hello"} in markdown body should emit OperandTypMismatch
  #[test]
  fn typecheck_md_body_prefix_wrong_operand_has_diagnostic() {
    let (db, project, file) =
      load_vault_fixture("typecheck/my_vault", "md_body_prefix_wrong_operand.td");
    let root_node = parse_file(&db, project, file).ast(&db);
    let result = typecheck(&db, root_node);
    assert!(
      result.diagnostics(&db).iter().any(|diagnostic| matches!(
        diagnostic,
        TdDiagnostic {
          kind: TdDiagnosticKind::OperandTypMismatch { .. },
          ..
        }
      )),
      "prefix minus on string in markdown body should emit OperandTypMismatch: {:?}",
      result.diagnostics(&db)
    );
  }
}
