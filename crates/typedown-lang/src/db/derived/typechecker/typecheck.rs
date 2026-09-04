//! Tracked query for typechecking
// I think this is the idea of bidirectional typechecking

use std::collections::HashSet;

use crate::syntax::diagnostic::Diagnostic;
use typedown_macros::query_derived;

use crate::db::TypedownDatabase;
use crate::db::derived::get_builtin_types::{get_bool_type, get_num_type};
use crate::db::derived::name_resolver::referee::referee;
use crate::db::derived::typechecker::actual_node_type::actual_node_type;
use crate::db::derived::typechecker::expected_node_type::expected_node_type;

use crate::db::typecheck::utils::{is_nullable, is_subtype_of};
use crate::db::types::derived::object_system::TdStaticType;
use crate::db::types::{
  HirValue, HirValueKind, InterpolatedPart, TdSchemaType, TdTypeEnum, TypecheckResult,
};
use crate::syntax::ast::{AstNode, YamlMapping};
use typedown_incremental::QueryDatabase;

#[query_derived]
pub fn typecheck<'db>(db: &'db TypedownDatabase, hir: HirValue<'db>) -> TypecheckResult<'db> {
  let type_result = actual_node_type(db, hir);
  let mut diagnostics = type_result.diagnostics(db).clone();

  // Use expected type from schema if available, otherwise fall back to inferred type
  let declared_type = match expected_node_type(db, hir).typ(db) {
    Some(typ) => typ,
    None => match type_result.typ(db) {
      Some(typ) => typ,
      None => return TypecheckResult::new(db, diagnostics),
    },
  };

  diagnostics.extend(typecheck_body(db, hir, &declared_type));
  TypecheckResult::new(db, diagnostics)
}

// Typecheck a HIR node against an externally provided expected type
pub fn typecheck_with_expected<'db>(
  db: &'db TypedownDatabase,
  hir: HirValue<'db>,
  expected_type: &TdTypeEnum<'db>,
) -> TypecheckResult<'db> {
  let type_result = actual_node_type(db, hir);
  let mut diagnostics = type_result.diagnostics(db).clone();

  diagnostics.extend(typecheck_body(db, hir, expected_type));
  TypecheckResult::new(db, diagnostics)
}

// Validate structure based on the node kind
fn typecheck_body<'db>(
  db: &'db TypedownDatabase,
  hir: HirValue<'db>,
  declared_type: &TdTypeEnum<'db>,
) -> Vec<Diagnostic> {
  let mut diagnostics = vec![];

  match hir.kind(db) {
    // Check mapping fields against declared schema type
    HirValueKind::Mapping(entries) => {
      diagnostics.extend(check_mapping_fields(db, hir, &entries, declared_type));
    }
    // Check tag inner matches the tag's schema
    HirValueKind::Tag { inner, .. } => {
      diagnostics.extend(check_tag(db, declared_type, *inner));
    }
    // Check call arity and arg types against function signature
    HirValueKind::Call { callee, args } => {
      diagnostics.extend(check_call(db, *callee, args));
    }
    // Check each item against the list's element type
    HirValueKind::Sequence(items) => {
      diagnostics.extend(check_sequence(db, declared_type, items));
    }
    // Typecheck each embedded expression in an interpolated string
    HirValueKind::Interpolated(parts) | HirValueKind::Markdown(parts) => {
      for part in parts {
        if let InterpolatedPart::Expr(expr) = part {
          let tc_result = typecheck(db, expr);
          diagnostics.extend(tc_result.diagnostics(db).iter().cloned());
        }
      }
    }
    // Check prefix operand type
    HirValueKind::Prefix { op, operand } => {
      diagnostics.extend(check_prefix(db, &op, *operand));
    }
    // Check postfix operand type
    HirValueKind::Postfix { op, operand } => {
      diagnostics.extend(check_postfix(db, &op, *operand));
    }
    // Check binary operand types
    HirValueKind::Binary { op, left, right } => {
      diagnostics.extend(check_binary(db, &op, *left, *right));
    }
    // Check index types against container key types
    HirValueKind::Index { expr, indices } => {
      diagnostics.extend(check_index(db, *expr, indices));
    }
    // Recurse into closure body
    HirValueKind::Closure { body, .. } => {
      let tc_result = typecheck(db, *body);
      diagnostics.extend(tc_result.diagnostics(db).iter().cloned());
    }
    _ => {}
  }

  diagnostics
}

fn check_mapping_fields<'db>(
  db: &'db TypedownDatabase,
  mapping_hir: HirValue<'db>,
  entries: &[(String, HirValue<'db>)],
  expected_type: &TdTypeEnum<'db>,
) -> Vec<Diagnostic> {
  let mut diagnostics = vec![];
  let declared_fields = expected_type.get_fields(db);

  for (key, value_hir) in entries {
    // _type requires the value to resolve to a schema symbol
    if key == "_type" {
      let resolved = referee(db, *value_hir);
      if let Some(symbol) = resolved.value(db)
        && !symbol.kind(db).is_schema()
      {
        let node = value_hir.node(db);
        let (tr_offset, tr_len) = node.trimmed_range();
        diagnostics.push(Diagnostic::FieldTypeMismatch {
          field: "_type".to_string(),
          expected: "schema".to_string(),
          start_offset: tr_offset,
          end_offset: tr_offset + tr_len,
        });
      }
      continue;
    }
    // Built-in fields (_label, _icon) have fixed types
    if let Some(builtin_type) = TdSchemaType::builtin_field_type(db, key) {
      let value_result = actual_node_type(db, *value_hir);
      if let Some(actual_type) = value_result.typ(db)
        && !is_subtype_of(db, &actual_type, &builtin_type)
      {
        let node = value_hir.node(db);
        let (tr_offset, tr_len) = node.trimmed_range();
        diagnostics.push(Diagnostic::FieldTypeMismatch {
          field: key.clone(),
          expected: builtin_type.display_name(db),
          start_offset: tr_offset,
          end_offset: tr_offset + tr_len,
        });
      }
      continue;
    }
    if let Some(field_lazy) = declared_fields.get(key) {
      if let Some(field_type) = field_lazy.resolve(db) {
        // Recursively typecheck the field value
        let tc_result = typecheck(db, *value_hir);
        diagnostics.extend(tc_result.diagnostics(db).iter().cloned());

        // Check synthesized type against expected field type
        let value_result = actual_node_type(db, *value_hir);
        let is_optional = is_nullable(db, &field_type);
        match value_result.typ(db) {
          Some(actual_type) if !is_subtype_of(db, &actual_type, &field_type) => {
            let node = value_hir.node(db);
            let (tr_offset, tr_len) = node.trimmed_range();
            diagnostics.push(Diagnostic::FieldTypeMismatch {
              field: key.clone(),
              expected: field_type.display_name(db),
              start_offset: tr_offset,
              end_offset: tr_offset + tr_len,
            });
          }
          // Unresolved identifier used as a field value
          None if matches!(value_hir.kind(db), HirValueKind::Ident(_)) => {
            let node = value_hir.node(db);
            let (tr_offset, tr_len) = node.trimmed_range();
            diagnostics.push(Diagnostic::UnresolvedSchema {
              name: node.text(),
              start_offset: tr_offset,
              end_offset: tr_offset + tr_len,
            });
          }
          // Null on a non-optional field is a type error
          None if !is_optional => {
            let node = value_hir.node(db);
            let (tr_offset, tr_len) = node.trimmed_range();
            diagnostics.push(Diagnostic::FieldTypeMismatch {
              field: key.clone(),
              expected: field_type.display_name(db),
              start_offset: tr_offset,
              end_offset: tr_offset + tr_len,
            });
          }
          Some(_) | None => {}
        }
      }
    } else if expected_type.as_td_schema_type().is_some() && !key.starts_with('_') {
      // Excess property: key not declared on the schema
      let (start_offset, end_offset) = YamlMapping::cast(mapping_hir.node(db))
        .and_then(|m| m.find_entry(key))
        .and_then(|e| e.key_node())
        .map(|n| n.trimmed_range())
        .unwrap_or_else(|| value_hir.node(db).trimmed_range());
      diagnostics.push(Diagnostic::UnknownField {
        field: key.clone(),
        on_type: expected_type.display_name(db),
        start_offset,
        end_offset,
      });
    }
  }

  // Check required fields are present (null values are checked above)
  let mapping_node = mapping_hir.node(db);
  let (tr_offset, tr_len) = mapping_node.trimmed_range();
  let present_keys: HashSet<&str> = entries.iter().map(|(key, _)| key.as_str()).collect();

  let default_fields: HashSet<String> = expected_type
    .as_td_schema_type()
    .map(|p| {
      p.fields(db)
        .iter()
        .filter_map(|(k, desc)| {
          if desc.default_value.is_some() {
            Some(k.clone())
          } else {
            None
          }
        })
        .collect()
    })
    .unwrap_or_default();

  for (field_name, field_lazy) in declared_fields {
    let is_optional = field_lazy.resolve(db).is_some_and(|t| is_nullable(db, &t))
      || default_fields.contains(&field_name);
    if !is_optional && !present_keys.contains(field_name.as_str()) {
      diagnostics.push(Diagnostic::MissingRequiredField {
        field: field_name,
        start_offset: tr_offset,
        end_offset: tr_offset + tr_len,
      });
    }
  }

  diagnostics
}

fn check_tag<'db>(
  db: &'db TypedownDatabase,
  expected_type: &TdTypeEnum<'db>,
  inner: HirValue<'db>,
) -> Vec<Diagnostic> {
  let mut diagnostics = vec![];
  let inner_result = actual_node_type(db, inner);
  diagnostics.extend(inner_result.diagnostics(db).iter().cloned());
  if let Some(actual_type) = inner_result.typ(db)
    && !is_subtype_of(db, &actual_type, expected_type)
  {
    let node = inner.node(db);
    let (tr_offset, tr_len) = node.trimmed_range();
    diagnostics.push(Diagnostic::TagTypeMismatch {
      expected: expected_type.display_name(db),
      start_offset: tr_offset,
      end_offset: tr_offset + tr_len,
    });
  }
  diagnostics
}

fn check_call<'db>(
  db: &'db TypedownDatabase,
  callee: HirValue<'db>,
  args: Vec<HirValue<'db>>,
) -> Vec<Diagnostic> {
  let mut diagnostics = vec![];

  let callee_result = actual_node_type(db, callee);
  diagnostics.extend(callee_result.diagnostics(db).iter().cloned());

  let callee_type = match callee_result.typ(db) {
    Some(typ) => typ,
    None => return diagnostics,
  };

  let arg_types: Vec<TdTypeEnum> = args
    .iter()
    .filter_map(|arg| actual_node_type(db, *arg).typ(db))
    .collect();

  let Some(sig) = callee_type.call_type(db, arg_types) else {
    let node = callee.node(db);
    let (tr_offset, tr_len) = node.trimmed_range();
    diagnostics.push(Diagnostic::NotCallable {
      start_offset: tr_offset,
      end_offset: tr_offset + tr_len,
    });
    return diagnostics;
  };

  let params = sig.params(db);

  if params.len() != args.len() {
    let node = callee.node(db);
    let (tr_offset, tr_len) = node.trimmed_range();
    diagnostics.push(Diagnostic::WrongArgCount {
      expected: params.len(),
      got: args.len(),
      start_offset: tr_offset,
      end_offset: tr_offset + tr_len,
    });
    return diagnostics;
  }

  for (param, arg_hir) in params.iter().zip(args.iter()) {
    let arg_result = actual_node_type(db, *arg_hir);
    diagnostics.extend(arg_result.diagnostics(db).iter().cloned());
    if let Some(arg_type) = arg_result.typ(db)
      && !is_subtype_of(db, &arg_type, param)
    {
      let node = arg_hir.node(db);
      let (tr_offset, tr_len) = node.trimmed_range();
      diagnostics.push(Diagnostic::ArgTypeMismatch {
        expected: param.display_name(db),
        start_offset: tr_offset,
        end_offset: tr_offset + tr_len,
      });
    }
  }

  diagnostics
}

fn check_index<'db>(
  db: &'db TypedownDatabase,
  expr: HirValue<'db>,
  indices: Vec<HirValue<'db>>,
) -> Vec<Diagnostic> {
  let mut diagnostics = vec![];

  let expr_result = actual_node_type(db, expr);
  diagnostics.extend(expr_result.diagnostics(db).iter().cloned());

  let expr_type = match expr_result.typ(db) {
    Some(typ) => typ,
    None => return diagnostics,
  };

  // Type instantiation: no checking is needed because we do not support type bound, only check arity
  if expr_type.arity(db) > 0 {
    return diagnostics;
  }

  // List element access: index must be a number
  if expr_type.is_td_list_type() {
    for idx_hir in &indices {
      let idx_result = actual_node_type(db, *idx_hir);
      diagnostics.extend(idx_result.diagnostics(db).iter().cloned());
      if let Some(idx_type) = idx_result.typ(db) {
        let num_type = get_num_type(db);
        if !is_subtype_of(db, &idx_type, &num_type.into()) {
          let node = idx_hir.node(db);
          let (tr_offset, tr_len) = node.trimmed_range();
          diagnostics.push(Diagnostic::IndexTypeMismatch {
            expected: "number".to_string(),
            start_offset: tr_offset,
            end_offset: tr_offset + tr_len,
          });
        }
      }
    }
    return diagnostics;
  }

  // Dict element access: index must match key type
  if let TdTypeEnum::TdDictType(dict) = &expr_type {
    if let Some(key_type) = dict.key(db).and_then(|l| l.resolve(db)) {
      for idx_hir in &indices {
        let idx_result = actual_node_type(db, *idx_hir);
        diagnostics.extend(idx_result.diagnostics(db).iter().cloned());
        if let Some(idx_type) = idx_result.typ(db)
          && !is_subtype_of(db, &idx_type, &key_type)
        {
          let node = idx_hir.node(db);
          let (tr_offset, tr_len) = node.trimmed_range();
          diagnostics.push(Diagnostic::IndexTypeMismatch {
            expected: key_type.display_name(db),
            start_offset: tr_offset,
            end_offset: tr_offset + tr_len,
          });
        }
      }
    }
    return diagnostics;
  }

  // String indexing is valid: index must be a number
  if expr_type.is_td_str_type() {
    for idx_hir in &indices {
      let idx_result = actual_node_type(db, *idx_hir);
      diagnostics.extend(idx_result.diagnostics(db).iter().cloned());
      if let Some(idx_type) = idx_result.typ(db) {
        let num_type = get_num_type(db);
        if !is_subtype_of(db, &idx_type, &num_type.into()) {
          let node = idx_hir.node(db);
          let (tr_offset, tr_len) = node.trimmed_range();
          diagnostics.push(Diagnostic::IndexTypeMismatch {
            expected: "number".to_string(),
            start_offset: tr_offset,
            end_offset: tr_offset + tr_len,
          });
        }
      }
    }
    return diagnostics;
  }

  // Not indexable
  let node = expr.node(db);
  let (tr_offset, tr_len) = node.trimmed_range();
  diagnostics.push(Diagnostic::NotIndexable {
    start_offset: tr_offset,
    end_offset: tr_offset + tr_len,
  });

  diagnostics
}

fn check_prefix<'db>(
  db: &'db TypedownDatabase,
  op: &str,
  operand: HirValue<'db>,
) -> Vec<Diagnostic> {
  let mut diagnostics = vec![];

  let tc_result = typecheck(db, operand);
  diagnostics.extend(tc_result.diagnostics(db).iter().cloned());

  let operand_result = actual_node_type(db, operand);
  let operand_type = match operand_result.typ(db) {
    Some(typ) => typ,
    None => return diagnostics,
  };

  let expected_type: TdTypeEnum = match op {
    "-" | "+" => get_num_type(db).into(),
    // ~ is logical not: accepts any type (only null and false are falsy)
    "~" => return diagnostics,
    _ => return diagnostics,
  };

  if !is_subtype_of(db, &operand_type, &expected_type) {
    let node = operand.node(db);
    let (tr_offset, tr_len) = node.trimmed_range();
    diagnostics.push(Diagnostic::OperandTypeMismatch {
      op: op.to_string(),
      expected: expected_type.display_name(db),
      start_offset: tr_offset,
      end_offset: tr_offset + tr_len,
    });
  }

  diagnostics
}

fn check_postfix<'db>(
  db: &'db TypedownDatabase,
  op: &str,
  operand: HirValue<'db>,
) -> Vec<Diagnostic> {
  let mut diagnostics = vec![];
  let tc_result = typecheck(db, operand);
  diagnostics.extend(tc_result.diagnostics(db).iter().cloned());

  if op == "?" {
    let operand_result = actual_node_type(db, operand);
    if let Some(operand_type) = operand_result.typ(db)
      && !operand_type.is_type(db)
    {
      let node = operand.node(db);
      let (tr_offset, tr_len) = node.trimmed_range();
      diagnostics.push(Diagnostic::OperandTypeMismatch {
        op: "?".to_string(),
        expected: "type".to_string(),
        start_offset: tr_offset,
        end_offset: tr_offset + tr_len,
      });
    }
  }

  diagnostics
}

fn check_binary<'db>(
  db: &'db TypedownDatabase,
  op: &str,
  left: HirValue<'db>,
  right: HirValue<'db>,
) -> Vec<Diagnostic> {
  let mut diagnostics = vec![];

  let tc_left = typecheck(db, left);
  diagnostics.extend(tc_left.diagnostics(db).iter().cloned());
  let tc_right = typecheck(db, right);
  diagnostics.extend(tc_right.diagnostics(db).iter().cloned());

  let left_type = actual_node_type(db, left).typ(db);
  let right_type = actual_node_type(db, right).typ(db);

  match op {
    // Arithmetic: both operands must be number
    "+" | "-" | "*" | "/" | "%" | "**" => {
      let num_type: TdTypeEnum = get_num_type(db).into();
      if let Some(lt) = &left_type
        && !is_subtype_of(db, lt, &num_type)
      {
        let node = left.node(db);
        let (tr_offset, tr_len) = node.trimmed_range();
        diagnostics.push(Diagnostic::OperandTypeMismatch {
          op: op.to_string(),
          expected: "number".to_string(),
          start_offset: tr_offset,
          end_offset: tr_offset + tr_len,
        });
      }
      if let Some(rt) = &right_type
        && !is_subtype_of(db, rt, &num_type)
      {
        let node = right.node(db);
        let (tr_offset, tr_len) = node.trimmed_range();
        diagnostics.push(Diagnostic::OperandTypeMismatch {
          op: op.to_string(),
          expected: "number".to_string(),
          start_offset: tr_offset,
          end_offset: tr_offset + tr_len,
        });
      }
    }
    // Logical: both operands must be boolean
    "&&" | "||" => {
      let bool_type: TdTypeEnum = get_bool_type(db).into();
      if let Some(lt) = &left_type
        && !is_subtype_of(db, lt, &bool_type)
      {
        let node = left.node(db);
        let (tr_offset, tr_len) = node.trimmed_range();
        diagnostics.push(Diagnostic::OperandTypeMismatch {
          op: op.to_string(),
          expected: "boolean".to_string(),
          start_offset: tr_offset,
          end_offset: tr_offset + tr_len,
        });
      }
      if let Some(rt) = &right_type
        && !is_subtype_of(db, rt, &bool_type)
      {
        let node = right.node(db);
        let (tr_offset, tr_len) = node.trimmed_range();
        diagnostics.push(Diagnostic::OperandTypeMismatch {
          op: op.to_string(),
          expected: "boolean".to_string(),
          start_offset: tr_offset,
          end_offset: tr_offset + tr_len,
        });
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
  declared_type: &TdTypeEnum<'db>,
  items: Vec<HirValue<'db>>,
) -> Vec<Diagnostic> {
  let mut diagnostics = vec![];

  // Get the element type from the list type
  let Some(list) = declared_type.as_td_list_type() else {
    return diagnostics;
  };

  let elem_type = match list.elem(db).and_then(|e| e.resolve(db)) {
    Some(typ) => typ,
    // Uninstantiated list: no element type constraint
    None => return diagnostics,
  };

  for item in items {
    // Recursively typecheck each item
    let tc_result = typecheck(db, item);
    diagnostics.extend(tc_result.diagnostics(db).iter().cloned());

    // Check item type against element type
    let item_result = actual_node_type(db, item);
    if let Some(item_type) = item_result.typ(db)
      && !is_subtype_of(db, &item_type, &elem_type)
    {
      let node = item.node(db);
      let (tr_offset, tr_len) = node.trimmed_range();
      diagnostics.push(Diagnostic::ElementTypeMismatch {
        expected: elem_type.display_name(db),
        start_offset: tr_offset,
        end_offset: tr_offset + tr_len,
      });
    }
  }

  diagnostics
}

#[cfg(test)]
mod tests {
  use crate::db::{
    derived::typechecker::typecheck::typecheck, fixtures::load_vault_fixture, utils::lower_file,
  };
  use crate::syntax::diagnostic::Diagnostic;

  // Mapping without _type: infers product type, no validation errors
  #[test]
  fn typecheck_mapping_without_type_infers_product_no_errors() {
    let (db, project, file) = load_vault_fixture("typecheck/my_vault", "literal_value.td");
    let (hir, _) = lower_file(&db, project, file);
    let result = typecheck(&db, hir.unwrap());
    assert!(
      result.diagnostics(&db).is_empty(),
      "mapping without _type infers product type, no errors expected: {:?}",
      result.diagnostics(&db)
    );
  }

  // _type references a non-existent schema
  #[test]
  fn typecheck_unresolved_type_has_diagnostics() {
    let (db, project, file) = load_vault_fixture("typecheck/my_vault", "unresolved_type.td");
    let (hir, _) = lower_file(&db, project, file);
    let result = typecheck(&db, hir.unwrap());
    assert!(
      !result.diagnostics(&db).is_empty(),
      "expected diagnostics for unresolved schema"
    );
  }

  #[test]
  fn typecheck_mapping_with_ident_value() {
    let (db, project, file) = load_vault_fixture("typecheck/my_vault", "ident_value.td");
    let (hir, _) = lower_file(&db, project, file);
    let result = typecheck(&db, hir.unwrap());
    assert!(
      result.diagnostics(&db).is_empty(),
      "expected no diagnostics, got: {:?}",
      result.diagnostics(&db)
    );
  }

  #[test]
  fn typecheck_excess_field_has_diagnostics() {
    let (db, project, file) = load_vault_fixture("typecheck/my_vault", "excess_field.td");
    let (hir, _) = lower_file(&db, project, file);
    let result = typecheck(&db, hir.unwrap());
    let diags = result.diagnostics(&db);
    assert!(
      diags
        .iter()
        .any(|d| matches!(d, Diagnostic::UnknownField { field, on_type, .. } if field == "favorite_color" && on_type == "Person")),
      "expected UnknownField for 'favorite_color', got: {:?}",
      diags
    );
  }

  #[test]
  fn typecheck_schema_missing_properties_has_diagnostics() {
    let (db, project, file) =
      load_vault_fixture("typecheck/my_vault", "schema_missing_properties.td");
    let (hir, _) = lower_file(&db, project, file);
    let result = typecheck(&db, hir.unwrap());
    let diags = result.diagnostics(&db);
    assert!(
      diags.iter().any(
        |d| matches!(d, Diagnostic::MissingRequiredField { field, .. } if field == "properties")
      ),
      "expected MissingRequiredField for 'properties', got: {:?}",
      diags
    );
  }

  #[test]
  fn typecheck_valid_person_no_errors() {
    let (db, project, file) = load_vault_fixture("typecheck/my_vault", "valid_person.td");
    let (hir, _) = lower_file(&db, project, file);
    let result = typecheck(&db, hir.unwrap());
    assert!(
      result.diagnostics(&db).is_empty(),
      "valid Person should have no errors: {:?}",
      result.diagnostics(&db)
    );
  }

  #[test]
  fn typecheck_wrong_field_type_has_diagnostics() {
    let (db, project, file) = load_vault_fixture("typecheck/my_vault", "wrong_field_type.td");
    let (hir, _) = lower_file(&db, project, file);
    let result = typecheck(&db, hir.unwrap());
    let diags = result.diagnostics(&db);
    assert!(
      diags.iter().any(|d| matches!(d, Diagnostic::FieldTypeMismatch { field, expected, .. } if field == "name" && expected == "string")),
      "expected FieldTypeMismatch for 'name' with expected 'string', got: {:?}",
      diags
    );
  }

  #[test]
  fn typecheck_nested_valid_no_errors() {
    let (db, project, file) = load_vault_fixture("typecheck/my_vault", "nested_valid.td");
    let (hir, _) = lower_file(&db, project, file);
    let result = typecheck(&db, hir.unwrap());
    assert!(
      result.diagnostics(&db).is_empty(),
      "valid nested PersonWithAddress should have no errors: {:?}",
      result.diagnostics(&db)
    );
  }

  #[test]
  fn typecheck_nested_wrong_type_has_diagnostics() {
    let (db, project, file) = load_vault_fixture("typecheck/my_vault", "nested_wrong_type.td");
    let (hir, _) = lower_file(&db, project, file);
    let result = typecheck(&db, hir.unwrap());
    let diags = result.diagnostics(&db);
    assert!(
      diags
        .iter()
        .any(|d| matches!(d, Diagnostic::FieldTypeMismatch { field, .. } if field == "address")),
      "expected FieldTypeMismatch for 'address', got: {:?}",
      diags
    );
  }

  #[test]
  fn typecheck_prefix_valid() {
    let (db, project, file) = load_vault_fixture("typecheck/my_vault", "unary_valid.td");
    let (hir, _) = lower_file(&db, project, file);
    let result = typecheck(&db, hir.unwrap());
    assert!(
      result.diagnostics(&db).is_empty(),
      "unary minus on number should have no errors: {:?}",
      result.diagnostics(&db)
    );
  }

  #[test]
  fn typecheck_prefix_wrong_type() {
    let (db, project, file) = load_vault_fixture("typecheck/my_vault", "unary_wrong_type.td");
    let (hir, _) = lower_file(&db, project, file);
    let result = typecheck(&db, hir.unwrap());
    assert!(
      result
        .diagnostics(&db)
        .iter()
        .any(|d| matches!(d, Diagnostic::OperandTypeMismatch { .. })),
      "expected OperandTypeMismatch"
    );
  }

  #[test]
  fn typecheck_binary_valid() {
    let (db, project, file) = load_vault_fixture("typecheck/my_vault", "binary_valid.td");
    let (hir, _) = lower_file(&db, project, file);
    let result = typecheck(&db, hir.unwrap());
    assert!(
      result.diagnostics(&db).is_empty(),
      "binary addition should have no errors: {:?}",
      result.diagnostics(&db)
    );
  }

  #[test]
  fn typecheck_binary_wrong_type() {
    let (db, project, file) = load_vault_fixture("typecheck/my_vault", "binary_wrong_type.td");
    let (hir, _) = lower_file(&db, project, file);
    let result = typecheck(&db, hir.unwrap());
    assert!(
      result
        .diagnostics(&db)
        .iter()
        .any(|d| matches!(d, Diagnostic::OperandTypeMismatch { .. })),
      "expected OperandTypeMismatch"
    );
  }

  #[test]
  fn typecheck_math_field_valid() {
    let (db, project, file) = load_vault_fixture("typecheck/my_vault", "valid_math.td");
    let (hir, _) = lower_file(&db, project, file);
    let result = typecheck(&db, hir.unwrap());
    assert!(
      result.diagnostics(&db).is_empty(),
      "math field should typecheck: {:?}",
      result.diagnostics(&db)
    );
  }

  #[test]
  fn typecheck_markdown_body_with_interpolation() {
    let (db, project, file) = load_vault_fixture("typecheck/my_vault", "valid_markdown.td");
    let (hir, _) = lower_file(&db, project, file);
    let result = typecheck(&db, hir.unwrap());
    assert!(
      result.diagnostics(&db).is_empty(),
      "markdown body should typecheck: {:?}",
      result.diagnostics(&db)
    );
  }

  #[test]
  fn typecheck_literal_type_valid() {
    let (db, project, file) = load_vault_fixture("typecheck/my_vault", "valid_status.td");
    let (hir, _) = lower_file(&db, project, file);
    let result = typecheck(&db, hir.unwrap());
    assert!(
      result.diagnostics(&db).is_empty(),
      "state: \"draft\" should match: {:?}",
      result.diagnostics(&db)
    );
  }

  #[test]
  fn typecheck_literal_type_mismatch() {
    let (db, project, file) = load_vault_fixture("typecheck/my_vault", "invalid_status.td");
    let (hir, _) = lower_file(&db, project, file);
    let result = typecheck(&db, hir.unwrap());
    assert!(
      result
        .diagnostics(&db)
        .iter()
        .any(|d| matches!(d, Diagnostic::FieldTypeMismatch { field, .. } if field == "state")),
      "state mismatch expected"
    );
  }

  #[test]
  fn typecheck_date_time_fields_accept_quoted_strings() {
    let (db, project, file) = load_vault_fixture("typecheck/my_vault", "valid_event.td");
    let (hir, _) = lower_file(&db, project, file);
    let result = typecheck(&db, hir.unwrap());
    assert!(
      result.diagnostics(&db).is_empty(),
      "date/time fields should accept strings: {:?}",
      result.diagnostics(&db)
    );
  }

  #[test]
  fn typecheck_string_with_inline_math_no_errors() {
    let (db, project, file) = load_vault_fixture("typecheck/my_vault", "math_in_string.td");
    let (hir, _) = lower_file(&db, project, file);
    let result = typecheck(&db, hir.unwrap());
    assert!(
      result.diagnostics(&db).is_empty(),
      "string with inline math: {:?}",
      result.diagnostics(&db)
    );
  }

  #[test]
  fn typecheck_string_with_multiple_inline_math_no_errors() {
    let (db, project, file) = load_vault_fixture("typecheck/my_vault", "math_mixed_string.td");
    let (hir, _) = lower_file(&db, project, file);
    let result = typecheck(&db, hir.unwrap());
    assert!(
      result.diagnostics(&db).is_empty(),
      "multiple inline math: {:?}",
      result.diagnostics(&db)
    );
  }

  #[test]
  fn typecheck_math_only_string_as_math_field() {
    let (db, project, file) = load_vault_fixture("typecheck/my_vault", "math_only_string.td");
    let (hir, _) = lower_file(&db, project, file);
    let result = typecheck(&db, hir.unwrap());
    assert!(
      result.diagnostics(&db).is_empty(),
      "math-only string: {:?}",
      result.diagnostics(&db)
    );
  }

  #[test]
  fn typecheck_schema_with_list_type_no_errors() {
    let (db, project, file) = load_vault_fixture("typecheck/my_vault", "_types/WithListType.td");
    let (hir, _) = lower_file(&db, project, file);
    let result = typecheck(&db, hir.unwrap());
    assert!(
      result.diagnostics(&db).is_empty(),
      "schema with list[string]: {:?}",
      result.diagnostics(&db)
    );
  }

  #[test]
  fn typecheck_circular_schema_no_errors() {
    let (db, project, file) = load_vault_fixture("typecheck/my_vault", "_types/CircularA.td");
    let (hir, _) = lower_file(&db, project, file);
    let result = typecheck(&db, hir.unwrap());
    assert!(
      result.diagnostics(&db).is_empty(),
      "circular schema: {:?}",
      result.diagnostics(&db)
    );
  }

  #[test]
  fn typecheck_schema_with_bare_user_type_ref() {
    let (db, project, file) = load_vault_fixture("typecheck/my_vault", "_types/WithBareRef.td");
    let (hir, _) = lower_file(&db, project, file);
    let result = typecheck(&db, hir.unwrap());
    assert!(
      result.diagnostics(&db).is_empty(),
      "bare ref: {:?}",
      result.diagnostics(&db)
    );
  }

  #[test]
  fn typecheck_schema_with_self_ref() {
    let (db, project, file) = load_vault_fixture("typecheck/my_vault", "_types/SelfRef.td");
    let (hir, _) = lower_file(&db, project, file);
    let result = typecheck(&db, hir.unwrap());
    assert!(
      result.diagnostics(&db).is_empty(),
      "self ref: {:?}",
      result.diagnostics(&db)
    );
  }

  #[test]
  fn typecheck_schema_with_list_of_user_type_no_errors() {
    let (db, project, file) = load_vault_fixture("typecheck/my_vault", "_types/WithRefList.td");
    let (hir, _) = lower_file(&db, project, file);
    let result = typecheck(&db, hir.unwrap());
    assert!(
      result.diagnostics(&db).is_empty(),
      "list[Person]: {:?}",
      result.diagnostics(&db)
    );
  }

  #[test]
  fn typecheck_circular_content_no_errors() {
    let (db, project, file) = load_vault_fixture("typecheck/my_vault", "circular_ref.td");
    let (hir, _) = lower_file(&db, project, file);
    let result = typecheck(&db, hir.unwrap());
    assert!(
      result.diagnostics(&db).is_empty(),
      "circular ref: {:?}",
      result.diagnostics(&db)
    );
  }

  #[test]
  fn typecheck_schema_simple_props_no_errors() {
    let (db, project, file) = load_vault_fixture("typecheck/my_vault", "_types/SimpleProps.td");
    let (hir, _) = lower_file(&db, project, file);
    let result = typecheck(&db, hir.unwrap());
    assert!(
      result.diagnostics(&db).is_empty(),
      "simple props: {:?}",
      result.diagnostics(&db)
    );
  }

  #[test]
  fn typecheck_schema_with_optional_no_errors() {
    let (db, project, file) = load_vault_fixture("typecheck/my_vault", "_types/WithOptional.td");
    let (hir, _) = lower_file(&db, project, file);
    let hir = hir.unwrap();
    let result = typecheck(&db, hir);
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
    let (hir, _) = lower_file(&db, project, file);
    let result = typecheck(&db, hir.unwrap());
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
    let (hir, _) = lower_file(&db, project, file);
    let result = typecheck(&db, hir.unwrap());
    assert!(
      result.diagnostics(&db).is_empty(),
      "content with postfix optional fields should have no errors: {:?}",
      result.diagnostics(&db)
    );
  }

  #[test]
  fn typecheck_schema_with_pipe_union_no_errors() {
    let (db, project, file) = load_vault_fixture("typecheck/my_vault", "_types/WithPipeUnion.td");
    let (hir, _) = lower_file(&db, project, file);
    let result = typecheck(&db, hir.unwrap());
    assert!(
      result.diagnostics(&db).is_empty(),
      "pipe union schema: {:?}",
      result.diagnostics(&db)
    );
  }

  #[test]
  fn typecheck_content_with_pipe_union_valid() {
    let (db, project, file) = load_vault_fixture("typecheck/my_vault", "valid_pipe_union.td");
    let (hir, _) = lower_file(&db, project, file);
    let result = typecheck(&db, hir.unwrap());
    assert!(
      result.diagnostics(&db).is_empty(),
      "valid pipe union content: {:?}",
      result.diagnostics(&db)
    );
  }

  #[test]
  fn typecheck_content_with_pipe_union_invalid() {
    let (db, project, file) = load_vault_fixture("typecheck/my_vault", "invalid_pipe_union.td");
    let (hir, _) = lower_file(&db, project, file);
    let result = typecheck(&db, hir.unwrap());
    assert!(
      !result.diagnostics(&db).is_empty(),
      "invalid pipe union content should have errors"
    );
  }

  #[test]
  fn typecheck_schema_with_explicit_type_tag_no_errors() {
    let (db, project, file) =
      load_vault_fixture("typecheck/my_vault", "_types/WithExplicitTypeTag.td");
    let (hir, _) = lower_file(&db, project, file);
    let result = typecheck(&db, hir.unwrap());
    assert!(
      result.diagnostics(&db).is_empty(),
      "explicit type tag: {:?}",
      result.diagnostics(&db)
    );
  }

  #[test]
  fn typecheck_schema_with_union_type_no_errors() {
    let (db, project, file) = load_vault_fixture("typecheck/my_vault", "_types/WithUnion.td");
    let (hir, _) = lower_file(&db, project, file);
    let result = typecheck(&db, hir.unwrap());
    assert!(
      result.diagnostics(&db).is_empty(),
      "union: {:?}",
      result.diagnostics(&db)
    );
  }

  #[test]
  fn typecheck_schema_with_nested_type_no_errors() {
    let (db, project, file) = load_vault_fixture("typecheck/my_vault", "_types/WithNestedType.td");
    let (hir, _) = lower_file(&db, project, file);
    let result = typecheck(&db, hir.unwrap());
    assert!(
      result.diagnostics(&db).is_empty(),
      "nested type: {:?}",
      result.diagnostics(&db)
    );
  }

  #[test]
  fn typecheck_schema_with_literal_type_no_errors() {
    let (db, project, file) = load_vault_fixture("typecheck/my_vault", "_types/WithLiteralType.td");
    let (hir, _) = lower_file(&db, project, file);
    let result = typecheck(&db, hir.unwrap());
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
    let (hir, _) = lower_file(&db, project, file);
    let result = typecheck(&db, hir.unwrap());
    assert!(
      !result.diagnostics(&db).is_empty(),
      "non-mapping properties should error"
    );
  }

  #[test]
  fn typecheck_schema_missing_properties_field() {
    let (db, project, file) =
      load_vault_fixture("typecheck/my_vault", "_types/MissingProperties.td");
    let (hir, _) = lower_file(&db, project, file);
    let result = typecheck(&db, hir.unwrap());
    assert!(
      result.diagnostics(&db).iter().any(
        |d| matches!(d, Diagnostic::MissingRequiredField { field, .. } if field == "properties")
      ),
      "missing properties"
    );
  }

  #[test]
  fn typecheck_schema_prop_descriptor_extra_field_no_typecheck_errors() {
    let (db, project, file) =
      load_vault_fixture("typecheck/my_vault", "_types/PropDescriptorExtraField.td");
    let (hir, _) = lower_file(&db, project, file);
    let result = typecheck(&db, hir.unwrap());
    assert!(
      result.diagnostics(&db).is_empty(),
      "extra fields: {:?}",
      result.diagnostics(&db)
    );
  }

  #[test]
  fn typecheck_schema_prop_descriptor_missing_type_has_errors() {
    let (db, project, file) =
      load_vault_fixture("typecheck/my_vault", "_types/PropDescriptorMissingType.td");
    let (hir, _) = lower_file(&db, project, file);
    let result = typecheck(&db, hir.unwrap());
    assert!(
      !result.diagnostics(&db).is_empty(),
      "missing type in prop descriptor should error"
    );
  }

  #[test]
  fn typecheck_valid_list_sum_no_errors() {
    let (db, project, file) = load_vault_fixture("typecheck/narrow_vault", "valid_list_sum.td");
    let (hir, _) = lower_file(&db, project, file);
    let result = typecheck(&db, hir.unwrap());
    assert!(
      result.diagnostics(&db).is_empty(),
      "string | number should accept \"hello\": {:?}",
      result.diagnostics(&db)
    );
  }

  #[test]
  fn typecheck_invalid_list_sum_has_errors() {
    let (db, project, file) = load_vault_fixture("typecheck/narrow_vault", "invalid_list_sum.td");
    let (hir, _) = lower_file(&db, project, file);
    let result = typecheck(&db, hir.unwrap());
    assert!(!result.diagnostics(&db).is_empty(), "should reject true");
  }

  #[test]
  fn typecheck_valid_dict_sum_no_errors() {
    let (db, project, file) = load_vault_fixture("typecheck/narrow_vault", "valid_dict_sum.td");
    let (hir, _) = lower_file(&db, project, file);
    let result = typecheck(&db, hir.unwrap());
    assert!(
      result.diagnostics(&db).is_empty(),
      "dict sum: {:?}",
      result.diagnostics(&db)
    );
  }

  #[test]
  fn typecheck_invalid_dict_sum_has_errors() {
    let (db, project, file) = load_vault_fixture("typecheck/narrow_vault", "invalid_dict_sum.td");
    let (hir, _) = lower_file(&db, project, file);
    let result = typecheck(&db, hir.unwrap());
    assert!(!result.diagnostics(&db).is_empty(), "should reject boolean");
  }

  #[test]
  fn typecheck_mixed_union_accepts_string() {
    let (db, project, file) = load_vault_fixture("typecheck/narrow_vault", "mixed_union_string.td");
    let (hir, _) = lower_file(&db, project, file);
    let result = typecheck(&db, hir.unwrap());
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
    let (hir, _) = lower_file(&db, project, file);
    let result = typecheck(&db, hir.unwrap());
    assert!(
      result.diagnostics(&db).is_empty(),
      "mixed union literal: {:?}",
      result.diagnostics(&db)
    );
  }

  #[test]
  fn typecheck_mixed_union_accepts_number() {
    let (db, project, file) = load_vault_fixture("typecheck/narrow_vault", "mixed_union_number.td");
    let (hir, _) = lower_file(&db, project, file);
    let result = typecheck(&db, hir.unwrap());
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
    let (hir, _) = lower_file(&db, project, file);
    let result = typecheck(&db, hir.unwrap());
    assert!(!result.diagnostics(&db).is_empty(), "should reject true");
  }

  #[test]
  fn typecheck_nested_product_valid() {
    let (db, project, file) = load_vault_fixture("typecheck/narrow_vault", "nested_valid.td");
    let (hir, _) = lower_file(&db, project, file);
    let result = typecheck(&db, hir.unwrap());
    assert!(
      result.diagnostics(&db).is_empty(),
      "nested valid: {:?}",
      result.diagnostics(&db)
    );
  }

  #[test]
  fn typecheck_nested_product_wrong_field_type() {
    let (db, project, file) = load_vault_fixture("typecheck/narrow_vault", "nested_wrong_type.td");
    let (hir, _) = lower_file(&db, project, file);
    let result = typecheck(&db, hir.unwrap());
    assert!(!result.diagnostics(&db).is_empty(), "should fail");
  }

  #[test]
  fn typecheck_contrived_valid() {
    let (db, project, file) = load_vault_fixture("typecheck/narrow_vault", "contrived_valid.td");
    let (hir, _) = lower_file(&db, project, file);
    let result = typecheck(&db, hir.unwrap());
    assert!(
      result.diagnostics(&db).is_empty(),
      "contrived valid: {:?}",
      result.diagnostics(&db)
    );
  }

  #[test]
  fn typecheck_contrived_wrong_literal_num() {
    let (db, project, file) =
      load_vault_fixture("typecheck/narrow_vault", "contrived_wrong_literal.td");
    let (hir, _) = lower_file(&db, project, file);
    let result = typecheck(&db, hir.unwrap());
    assert!(
      !result.diagnostics(&db).is_empty(),
      "literal_num: 99 should fail"
    );
  }

  #[test]
  fn typecheck_contrived_missing_required_nested() {
    let (db, project, file) =
      load_vault_fixture("typecheck/narrow_vault", "contrived_missing_required.td");
    let (hir, _) = lower_file(&db, project, file);
    let result = typecheck(&db, hir.unwrap());
    assert!(
      !result.diagnostics(&db).is_empty(),
      "missing required should fail"
    );
  }

  #[test]
  fn typecheck_contrived_mixed_accepts_literal_num() {
    let (db, project, file) =
      load_vault_fixture("typecheck/narrow_vault", "contrived_mixed_num.td");
    let (hir, _) = lower_file(&db, project, file);
    let result = typecheck(&db, hir.unwrap());
    assert!(
      result.diagnostics(&db).is_empty(),
      "mixed num: {:?}",
      result.diagnostics(&db)
    );
  }

  #[test]
  fn typecheck_contrived_mixed_accepts_literal_bool() {
    let (db, project, file) =
      load_vault_fixture("typecheck/narrow_vault", "contrived_mixed_bool.td");
    let (hir, _) = lower_file(&db, project, file);
    let result = typecheck(&db, hir.unwrap());
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
    let (hir, _) = lower_file(&db, project, file);
    let result = typecheck(&db, hir.unwrap());
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
    let (hir, _) = lower_file(&db, project, file);
    let result = typecheck(&db, hir.unwrap());
    assert!(
      result.diagnostics(&db).is_empty(),
      "age: 10 + 20 should pass: {:?}",
      result.diagnostics(&db)
    );
  }

  // Binary expression with wrong operand type should fail
  #[test]
  fn typecheck_binary_expr_wrong_operand_type() {
    let (db, project, file) =
      load_vault_fixture("typecheck/my_vault", "binary_schema_wrong_operand.td");
    let (hir, _) = lower_file(&db, project, file);
    let result = typecheck(&db, hir.unwrap());
    assert!(
      result
        .diagnostics(&db)
        .iter()
        .any(|d| matches!(d, Diagnostic::OperandTypeMismatch { .. })),
      "1 + true should report operand mismatch"
    );
  }

  // Binary expression result assigned to wrong field type should fail
  #[test]
  fn typecheck_binary_expr_wrong_field_type() {
    let (db, project, file) =
      load_vault_fixture("typecheck/my_vault", "binary_schema_wrong_field_type.td");
    let (hir, _) = lower_file(&db, project, file);
    let result = typecheck(&db, hir.unwrap());
    assert!(
      result
        .diagnostics(&db)
        .iter()
        .any(|d| matches!(d, Diagnostic::FieldTypeMismatch { field, .. } if field == "name")),
      "name: 1 + 2 should report field type mismatch"
    );
  }

  // Parenthesized expression in schema field should pass
  #[test]
  fn typecheck_paren_expr_in_schema_field() {
    let (db, project, file) = load_vault_fixture("typecheck/my_vault", "paren_schema_valid.td");
    let (hir, _) = lower_file(&db, project, file);
    let result = typecheck(&db, hir.unwrap());
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
    let (hir, _) = lower_file(&db, project, file);
    let result = typecheck(&db, hir.unwrap());
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
    let (hir, _) = lower_file(&db, project, file);
    let result = typecheck(&db, hir.unwrap());
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
    let (hir, _) = lower_file(&db, project, file);
    let result = typecheck(&db, hir.unwrap());
    assert!(
      result
        .diagnostics(&db)
        .iter()
        .any(|d| matches!(d, Diagnostic::OperandTypeMismatch { .. })),
      "(1 + true) * 3 should report operand mismatch"
    );
  }

  // Comparison expression is valid schemaless
  #[test]
  fn typecheck_comparison_valid() {
    let (db, project, file) = load_vault_fixture("typecheck/my_vault", "comparison_valid.td");
    let (hir, _) = lower_file(&db, project, file);
    let result = typecheck(&db, hir.unwrap());
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
    let (hir, _) = lower_file(&db, project, file);
    let result = typecheck(&db, hir.unwrap());
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
    let (hir, _) = lower_file(&db, project, file);
    let result = typecheck(&db, hir.unwrap());
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
    let (hir, _) = lower_file(&db, project, file);
    let result = typecheck(&db, hir.unwrap());
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
    let (hir, _) = lower_file(&db, project, file);
    let result = typecheck(&db, hir.unwrap());
    assert!(
      result
        .diagnostics(&db)
        .iter()
        .any(|d| matches!(d, Diagnostic::OperandTypeMismatch { .. })),
      "1 && 2 should report operand mismatch"
    );
  }

  // Closure referencing self should typecheck
  #[test]
  fn typecheck_closure_self_ref() {
    let (db, project, file) = load_vault_fixture("typecheck/my_vault", "closure_self_ref.td");
    let (hir, _) = lower_file(&db, project, file);
    let result = typecheck(&db, hir.unwrap());
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
    let (hir, _) = lower_file(&db, project, file);
    let result = typecheck(&db, hir.unwrap());
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
    let (hir, _) = lower_file(&db, project, file);
    let result = typecheck(&db, hir.unwrap());
    assert!(
      result.diagnostics(&db).is_empty(),
      "file with valid imports should typecheck cleanly: {:?}",
      result.diagnostics(&db)
    );
  }

  // File with _imports has type error when imported field type mismatches schema
  #[test]
  fn typecheck_with_imports_wrong_type_has_diagnostics() {
    let (db, project, file) =
      load_vault_fixture("typecheck/my_vault", "with_imports_wrong_type.td");
    let (hir, _) = lower_file(&db, project, file);
    let result = typecheck(&db, hir.unwrap());
    assert!(
      result
        .diagnostics(&db)
        .iter()
        .any(|d| matches!(d, Diagnostic::FieldTypeMismatch { .. })),
      "should have FieldTypeMismatch for string assigned to number field: {:?}",
      result.diagnostics(&db)
    );
  }
}
