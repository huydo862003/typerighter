//! Tracked query for typechecking
// I think this is the idea of bidirectional typechecking

use std::collections::HashSet;

use crate::syntax::diagnostic::Diagnostic;
use typedown_macros::query_derived;

use crate::db::TypedownDatabase;
use crate::db::derived::get_builtin_types::{get_bool_type, get_num_type};
use crate::db::derived::name_resolver::referee::referee;
use crate::db::derived::typechecker::actual_node_type_member::actual_node_type_member;
use crate::db::derived::typechecker::expected_node_type_member::expected_node_type_member;
use std::collections::HashMap;

use crate::db::types::{
  HirValue, HirValueKind, InterpolatedPart, MemberType, TdTypeEnum, TdTypeLike, TypeMember,
  TypeMemberDescriptors, TypecheckResult, member_type_display_name,
};
use crate::db::utils::typecheck::{
  lift_member_type, lift_type_member_result, member_types_compatible,
};
use typedown_incremental::QueryDatabase;

#[query_derived]
pub fn typecheck(db: &TypedownDatabase, hir: HirValue) -> TypecheckResult {
  let type_result = actual_node_type_member(db, hir);
  let mut diagnostics = type_result.diagnostics(db).clone();

  // Use expected type from schema if available, otherwise fall back to inferred type
  let declared_member_type = if let Some(member) = expected_node_type_member(db, hir).member(db) {
    member.typ(db)
  } else {
    match type_result.member(db) {
      Some(member) => member.typ(db),
      None => return TypecheckResult::new(db, diagnostics),
    }
  };

  // Validate structure based on the node kind
  match hir.kind(db) {
    // Check mapping fields against declared schema type
    HirValueKind::Mapping(entries) => {
      diagnostics.extend(check_mapping_fields(
        db,
        hir,
        &entries,
        &declared_member_type,
      ));
    }
    // Check tag inner matches the tag's schema
    HirValueKind::Tag { inner, .. } => {
      if let Some(typ) = lift_member_type(db, &declared_member_type) {
        diagnostics.extend(check_tag(db, &typ, *inner));
      }
    }
    // Check call arity and arg types against function signature
    HirValueKind::Call { callee, args } => {
      diagnostics.extend(check_call(db, *callee, args));
    }
    // Check each item against the list's element type
    HirValueKind::Sequence(items) => {
      if let Some(typ) = lift_member_type(db, &declared_member_type) {
        diagnostics.extend(check_sequence(db, &typ, items));
      }
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
    // Check unary operand type
    HirValueKind::Unary { op, operand } => {
      diagnostics.extend(check_unary(db, &op, *operand));
    }
    // Check binary operand types
    HirValueKind::Binary { op, left, right } => {
      diagnostics.extend(check_binary(db, &op, *left, *right));
    }
    // Check index types against container key types
    HirValueKind::Index { expr, indices } => {
      diagnostics.extend(check_index(db, *expr, indices));
    }
    _ => {}
  }

  TypecheckResult::new(db, diagnostics)
}

// Extract the declared fields from a member type for mapping validation
fn extract_declared_fields(
  db: &TypedownDatabase,
  member_type: &MemberType,
) -> HashMap<String, TypeMember> {
  match member_type {
    MemberType::Structural(fields) => fields.clone(),
    MemberType::Simple(_) => {
      let typ = match member_type.evaluate_simple(db) {
        Some(t) => t,
        None => return HashMap::new(),
      };
      if let Some(product) = typ.as_td_product_type() {
        return product.fields(db);
      }
      if typ.is_td_schema_type() {
        return vec!["properties"]
          .into_iter()
          .filter_map(|name| {
            typ
              .get_owned_field_type_member(db, name)
              .map(|member| (name.to_string(), member))
          })
          .collect();
      }
      HashMap::new()
    }
    _ => HashMap::new(),
  }
}

fn check_mapping_fields(
  db: &TypedownDatabase,
  mapping_hir: HirValue,
  entries: &[(String, HirValue)],
  expected_member_type: &MemberType,
) -> Vec<Diagnostic> {
  let mut diagnostics = vec![];
  let declared_fields = extract_declared_fields(db, expected_member_type);

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
    if let Some(member) = declared_fields.get(key) {
      // Recursively typecheck the field value
      let tc_result = typecheck(db, *value_hir);
      diagnostics.extend(tc_result.diagnostics(db).iter().cloned());

      // Check synthesized type against expected field type
      let value_result = actual_node_type_member(db, *value_hir);
      let is_optional = member
        .descriptors(db)
        .contains(TypeMemberDescriptors::OPTIONAL);
      match value_result.member(db) {
        Some(actual_member) => {
          if !member_types_compatible(db, &member.typ(db), &actual_member.typ(db)) {
            let node = value_hir.node(db);
            let (tr_offset, tr_len) = node.trimmed_range();
            diagnostics.push(Diagnostic::FieldTypeMismatch {
              field: key.clone(),
              expected: member_type_display_name(db, &member.typ(db)),
              start_offset: tr_offset,
              end_offset: tr_offset + tr_len,
            });
          }
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
            expected: member_type_display_name(db, &member.typ(db)),
            start_offset: tr_offset,
            end_offset: tr_offset + tr_len,
          });
        }
        None => {}
      }
    }
  }

  // Check required fields are present (null values are checked above)
  let mapping_node = mapping_hir.node(db);
  let (tr_offset, tr_len) = mapping_node.trimmed_range();
  let present_keys: HashSet<&str> = entries.iter().map(|(key, _)| key.as_str()).collect();

  for (field_name, member) in declared_fields {
    let is_optional = member
      .descriptors(db)
      .contains(TypeMemberDescriptors::OPTIONAL);
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

fn check_tag(
  db: &TypedownDatabase,
  expected_type: &TdTypeEnum,
  inner: HirValue,
) -> Vec<Diagnostic> {
  let mut diagnostics = vec![];
  let inner_result = actual_node_type_member(db, inner);
  diagnostics.extend(inner_result.diagnostics(db).iter().cloned());
  if let Some(actual_type) = lift_type_member_result(db, &inner_result)
    && !expected_type.is_compatible_with(db, &actual_type)
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

fn check_call(db: &TypedownDatabase, callee: HirValue, args: Vec<HirValue>) -> Vec<Diagnostic> {
  let mut diagnostics = vec![];

  let callee_result = actual_node_type_member(db, callee);
  diagnostics.extend(callee_result.diagnostics(db).iter().cloned());

  let callee_type = match lift_type_member_result(db, &callee_result) {
    Some(typ) => typ,
    None => return diagnostics,
  };

  let Some(func) = callee_type.as_td_func_type() else {
    let node = callee.node(db);
    let (tr_offset, tr_len) = node.trimmed_range();
    diagnostics.push(Diagnostic::NotCallable {
      start_offset: tr_offset,
      end_offset: tr_offset + tr_len,
    });
    return diagnostics;
  };

  let sig = func.signature(db);
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
    let arg_result = actual_node_type_member(db, *arg_hir);
    diagnostics.extend(arg_result.diagnostics(db).iter().cloned());
    if let Some(arg_type) = lift_type_member_result(db, &arg_result)
      && !param.is_compatible_with(db, &arg_type)
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

fn check_index(db: &TypedownDatabase, expr: HirValue, indices: Vec<HirValue>) -> Vec<Diagnostic> {
  let mut diagnostics = vec![];

  let expr_result = actual_node_type_member(db, expr);
  diagnostics.extend(expr_result.diagnostics(db).iter().cloned());

  let expr_type = match lift_type_member_result(db, &expr_result) {
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
      let idx_result = actual_node_type_member(db, *idx_hir);
      diagnostics.extend(idx_result.diagnostics(db).iter().cloned());
      if let Some(idx_type) = lift_type_member_result(db, &idx_result) {
        let num_type = get_num_type(db);
        if !num_type.is_compatible_with(db, &idx_type) {
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
    if let Some(key_type) = dict.key(db) {
      for idx_hir in &indices {
        let idx_result = actual_node_type_member(db, *idx_hir);
        diagnostics.extend(idx_result.diagnostics(db).iter().cloned());
        if let Some(idx_type) = lift_type_member_result(db, &idx_result)
          && !key_type.is_compatible_with(db, &idx_type)
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
      let idx_result = actual_node_type_member(db, *idx_hir);
      diagnostics.extend(idx_result.diagnostics(db).iter().cloned());
      if let Some(idx_type) = lift_type_member_result(db, &idx_result) {
        let num_type = get_num_type(db);
        if !num_type.is_compatible_with(db, &idx_type) {
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

fn check_unary(db: &TypedownDatabase, op: &str, operand: HirValue) -> Vec<Diagnostic> {
  let mut diagnostics = vec![];

  let tc_result = typecheck(db, operand);
  diagnostics.extend(tc_result.diagnostics(db).iter().cloned());

  let operand_result = actual_node_type_member(db, operand);
  let operand_type = match lift_type_member_result(db, &operand_result) {
    Some(typ) => typ,
    None => return diagnostics,
  };

  let expected_type: TdTypeEnum = match op {
    "-" | "+" => get_num_type(db).into(),
    // ~ is logical not: accepts any type (only null and false are falsy)
    "~" => return diagnostics,
    _ => return diagnostics,
  };

  if !expected_type.is_compatible_with(db, &operand_type) {
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

fn check_binary(
  db: &TypedownDatabase,
  op: &str,
  left: HirValue,
  right: HirValue,
) -> Vec<Diagnostic> {
  let mut diagnostics = vec![];

  let tc_left = typecheck(db, left);
  diagnostics.extend(tc_left.diagnostics(db).iter().cloned());
  let tc_right = typecheck(db, right);
  diagnostics.extend(tc_right.diagnostics(db).iter().cloned());

  let left_type = lift_type_member_result(db, &actual_node_type_member(db, left));
  let right_type = lift_type_member_result(db, &actual_node_type_member(db, right));

  match op {
    // Arithmetic: both operands must be number
    "+" | "-" | "*" | "/" | "%" | "**" => {
      let num_type: TdTypeEnum = get_num_type(db).into();
      if let Some(lt) = &left_type
        && !num_type.is_compatible_with(db, lt)
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
        && !num_type.is_compatible_with(db, rt)
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
    // Consider allow truthy and falsy?
    "&&" | "||" => {
      let bool_type: TdTypeEnum = get_bool_type(db).into();
      if let Some(lt) = &left_type
        && !bool_type.is_compatible_with(db, lt)
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
        && !bool_type.is_compatible_with(db, rt)
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
    // :)) not sure
    "==" | "!=" | "<" | ">" | "<=" | ">=" => {}
    _ => {}
  }

  diagnostics
}

fn check_sequence(
  db: &TypedownDatabase,
  declared_type: &TdTypeEnum,
  items: Vec<HirValue>,
) -> Vec<Diagnostic> {
  let mut diagnostics = vec![];

  // Get the element type from the list type
  let Some(list) = declared_type.as_td_list_type() else {
    return diagnostics;
  };

  let elem_type = match list.elem(db) {
    Some(typ) => typ,
    // Uninstantiated list: no element type constraint
    None => return diagnostics,
  };

  for item in items {
    // Recursively typecheck each item
    let tc_result = typecheck(db, item);
    diagnostics.extend(tc_result.diagnostics(db).iter().cloned());

    // Check item type against element type
    let item_result = actual_node_type_member(db, item);
    if let Some(item_type) = lift_type_member_result(db, &item_result)
      && !elem_type.is_compatible_with(db, &item_type)
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
    let (db, project, file) = load_vault_fixture("typecheck/my_vault", "content/literal_value.td");
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
    let (db, project, file) =
      load_vault_fixture("typecheck/my_vault", "content/unresolved_type.td");
    let (hir, _) = lower_file(&db, project, file);
    let result = typecheck(&db, hir.unwrap());
    assert!(
      !result.diagnostics(&db).is_empty(),
      "expected diagnostics for unresolved schema"
    );
  }

  // Mapping with identifier value that resolves to nothing
  // No typecheck error here because the file has no _type, so no schema to check against
  #[test]
  fn typecheck_mapping_with_ident_value() {
    let (db, project, file) = load_vault_fixture("typecheck/my_vault", "content/ident_value.td");
    let (hir, _) = lower_file(&db, project, file);
    let result = typecheck(&db, hir.unwrap());
    assert!(
      result.diagnostics(&db).is_empty(),
      "expected no diagnostics, got: {:?}",
      result.diagnostics(&db)
    );
  }

  // Schema with _type: Schema but missing required 'properties' field
  #[test]
  fn typecheck_schema_missing_properties_has_diagnostics() {
    let (db, project, file) =
      load_vault_fixture("typecheck/my_vault", "content/schema_missing_properties.td");
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

  // Typecheck a valid document against a user-defined schema
  #[test]
  fn typecheck_valid_person_no_errors() {
    let (db, project, file) = load_vault_fixture("typecheck/my_vault", "content/valid_person.td");
    let (hir, _) = lower_file(&db, project, file);
    let result = typecheck(&db, hir.unwrap());
    assert!(
      result.diagnostics(&db).is_empty(),
      "valid Person should have no errors: {:?}",
      result.diagnostics(&db)
    );
  }

  // Field type mismatch: name expects string, got number
  #[test]
  fn typecheck_wrong_field_type_has_diagnostics() {
    let (db, project, file) =
      load_vault_fixture("typecheck/my_vault", "content/wrong_field_type.td");
    let (hir, _) = lower_file(&db, project, file);
    let result = typecheck(&db, hir.unwrap());
    let diags = result.diagnostics(&db);
    assert!(
      diags.iter().any(|d| matches!(d, Diagnostic::FieldTypeMismatch { field, expected, .. } if field == "name" && expected == "string")),
      "expected FieldTypeMismatch for 'name' with expected 'string', got: {:?}",
      diags
    );
  }

  // Recursive typecheck for nested inline object with valid types
  #[test]
  fn typecheck_nested_valid_no_errors() {
    let (db, project, file) = load_vault_fixture("typecheck/my_vault", "content/nested_valid.td");
    let (hir, _) = lower_file(&db, project, file);
    let result = typecheck(&db, hir.unwrap());
    assert!(
      result.diagnostics(&db).is_empty(),
      "valid nested PersonWithAddress should have no errors: {:?}",
      result.diagnostics(&db)
    );
  }

  // Recursive typecheck for nested inline object with wrong field type (street: 42 instead of string)
  #[test]
  fn typecheck_nested_wrong_type_has_diagnostics() {
    let (db, project, file) =
      load_vault_fixture("typecheck/my_vault", "content/nested_wrong_type.td");
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

  // Unary minus on number: no errors
  #[test]
  fn typecheck_unary_valid() {
    let (db, project, file) = load_vault_fixture("typecheck/my_vault", "content/unary_valid.td");
    let (hir, _) = lower_file(&db, project, file);
    let result = typecheck(&db, hir.unwrap());
    assert!(
      result.diagnostics(&db).is_empty(),
      "unary minus on number should have no errors: {:?}",
      result.diagnostics(&db)
    );
  }

  // Unary minus on boolean: OperandTypeMismatch
  #[test]
  fn typecheck_unary_wrong_type() {
    let (db, project, file) =
      load_vault_fixture("typecheck/my_vault", "content/unary_wrong_type.td");
    let (hir, _) = lower_file(&db, project, file);
    let result = typecheck(&db, hir.unwrap());
    let diags = result.diagnostics(&db);
    assert!(
      diags
        .iter()
        .any(|d| matches!(d, Diagnostic::OperandTypeMismatch { .. })),
      "expected OperandTypeMismatch for unary minus on boolean, got: {:?}",
      diags
    );
  }

  // Binary addition of numbers: no errors
  #[test]
  fn typecheck_binary_valid() {
    let (db, project, file) = load_vault_fixture("typecheck/my_vault", "content/binary_valid.td");
    let (hir, _) = lower_file(&db, project, file);
    let result = typecheck(&db, hir.unwrap());
    assert!(
      result.diagnostics(&db).is_empty(),
      "binary addition of numbers should have no errors: {:?}",
      result.diagnostics(&db)
    );
  }

  // Binary addition with boolean operand: OperandTypeMismatch
  #[test]
  fn typecheck_binary_wrong_type() {
    let (db, project, file) =
      load_vault_fixture("typecheck/my_vault", "content/binary_wrong_type.td");
    let (hir, _) = lower_file(&db, project, file);
    let result = typecheck(&db, hir.unwrap());
    let diags = result.diagnostics(&db);
    assert!(
      diags
        .iter()
        .any(|d| matches!(d, Diagnostic::OperandTypeMismatch { .. })),
      "expected OperandTypeMismatch for binary addition with boolean, got: {:?}",
      diags
    );
  }

  #[test]
  fn typecheck_math_field_valid() {
    let (db, project, file) = load_vault_fixture("typecheck/my_vault", "content/valid_math.td");
    let (hir, _) = lower_file(&db, project, file);
    let result = typecheck(&db, hir.unwrap());
    assert!(
      result.diagnostics(&db).is_empty(),
      "math field should typecheck with no errors: {:?}",
      result.diagnostics(&db)
    );
  }

  #[test]
  fn typecheck_markdown_body_with_interpolation() {
    let (db, project, file) = load_vault_fixture("typecheck/my_vault", "content/valid_markdown.td");
    let (hir, _) = lower_file(&db, project, file);
    let result = typecheck(&db, hir.unwrap());
    assert!(
      result.diagnostics(&db).is_empty(),
      "markdown body with interpolation should typecheck with no errors: {:?}",
      result.diagnostics(&db)
    );
  }

  // Quoted string values for date/time/datetime fields should pass typechecking
  // because actual_node_type_member deduces the specific subtype from ISO format
  #[test]
  fn typecheck_literal_type_valid() {
    let (db, project, file) = load_vault_fixture("typecheck/my_vault", "content/valid_status.td");
    let (hir, _) = lower_file(&db, project, file);
    let result = typecheck(&db, hir.unwrap());
    assert!(
      result.diagnostics(&db).is_empty(),
      "state: \"draft\" should match literal type \"draft\": {:?}",
      result.diagnostics(&db)
    );
  }

  #[test]
  fn typecheck_literal_type_mismatch() {
    let (db, project, file) = load_vault_fixture("typecheck/my_vault", "content/invalid_status.td");
    let (hir, _) = lower_file(&db, project, file);
    let result = typecheck(&db, hir.unwrap());
    let diags = result.diagnostics(&db);
    assert!(
      diags
        .iter()
        .any(|d| matches!(d, Diagnostic::FieldTypeMismatch { field, .. } if field == "state")),
      "state: \"published\" should fail literal type \"draft\": {:?}",
      diags
    );
  }

  #[test]
  fn typecheck_date_time_fields_accept_quoted_strings() {
    let (db, project, file) = load_vault_fixture("typecheck/my_vault", "content/valid_event.td");
    let (hir, _) = lower_file(&db, project, file);
    let result = typecheck(&db, hir.unwrap());
    assert!(
      result.diagnostics(&db).is_empty(),
      "date/time/datetime fields should accept quoted string values: {:?}",
      result.diagnostics(&db)
    );
  }

  // String field containing text with inline math lowers to Interpolated (a string subtype)
  #[test]
  fn typecheck_string_with_inline_math_no_errors() {
    let (db, project, file) = load_vault_fixture("typecheck/my_vault", "content/math_in_string.td");
    let (hir, _) = lower_file(&db, project, file);
    let result = typecheck(&db, hir.unwrap());
    assert!(
      result.diagnostics(&db).is_empty(),
      "string with inline math should have no type errors: {:?}",
      result.diagnostics(&db)
    );
  }

  // String field containing multiple inline math expressions still accepted as string
  #[test]
  fn typecheck_string_with_multiple_inline_math_no_errors() {
    let (db, project, file) =
      load_vault_fixture("typecheck/my_vault", "content/math_mixed_string.td");
    let (hir, _) = lower_file(&db, project, file);
    let result = typecheck(&db, hir.unwrap());
    assert!(
      result.diagnostics(&db).is_empty(),
      "string with multiple inline math should have no type errors: {:?}",
      result.diagnostics(&db)
    );
  }

  // String containing only a math literal still lowers to Math type
  #[test]
  fn typecheck_math_only_string_as_math_field() {
    let (db, project, file) =
      load_vault_fixture("typecheck/my_vault", "content/math_only_string.td");
    let (hir, _) = lower_file(&db, project, file);
    let result = typecheck(&db, hir.unwrap());
    assert!(
      result.diagnostics(&db).is_empty(),
      "math-only string in math field should have no type errors: {:?}",
      result.diagnostics(&db)
    );
  }

  // Schema property descriptor tests: valid cases

  #[test]
  fn typecheck_schema_with_list_type_no_errors() {
    let (db, project, file) = load_vault_fixture("typecheck/my_vault", "schemas/WithListType.td");
    let (hir, _) = lower_file(&db, project, file);
    let result = typecheck(&db, hir.unwrap());
    assert!(
      result.diagnostics(&db).is_empty(),
      "schema with list[string] should have no errors: {:?}",
      result.diagnostics(&db)
    );
  }

  #[test]
  fn typecheck_circular_schema_no_errors() {
    let (db, project, file) = load_vault_fixture("typecheck/my_vault", "schemas/CircularA.td");
    let (hir, _) = lower_file(&db, project, file);
    let result = typecheck(&db, hir.unwrap());
    assert!(
      result.diagnostics(&db).is_empty(),
      "circular schema with list[CircularB] should have no errors: {:?}",
      result.diagnostics(&db)
    );
  }

  #[test]
  fn typecheck_schema_with_bare_user_type_ref() {
    let (db, project, file) = load_vault_fixture("typecheck/my_vault", "schemas/WithBareRef.td");
    let (hir, _) = lower_file(&db, project, file);
    let result = typecheck(&db, hir.unwrap());
    assert!(
      result.diagnostics(&db).is_empty(),
      "schema with bare Person ref should have no errors: {:?}",
      result.diagnostics(&db)
    );
  }

  #[test]
  fn typecheck_schema_with_self_ref() {
    let (db, project, file) = load_vault_fixture("typecheck/my_vault", "schemas/SelfRef.td");
    let (hir, _) = lower_file(&db, project, file);
    let result = typecheck(&db, hir.unwrap());
    assert!(
      result.diagnostics(&db).is_empty(),
      "schema with self-ref and list[SelfRef] should have no errors: {:?}",
      result.diagnostics(&db)
    );
  }

  #[test]
  fn typecheck_schema_with_list_of_user_type_no_errors() {
    let (db, project, file) = load_vault_fixture("typecheck/my_vault", "schemas/WithRefList.td");
    let (hir, _) = lower_file(&db, project, file);
    let result = typecheck(&db, hir.unwrap());
    assert!(
      result.diagnostics(&db).is_empty(),
      "schema with list[Person] should have no errors: {:?}",
      result.diagnostics(&db)
    );
  }

  #[test]
  fn typecheck_circular_content_no_errors() {
    let (db, project, file) = load_vault_fixture("typecheck/my_vault", "content/circular_ref.td");
    let (hir, _) = lower_file(&db, project, file);
    let result = typecheck(&db, hir.unwrap());
    assert!(
      result.diagnostics(&db).is_empty(),
      "content with circular refs should have no errors: {:?}",
      result.diagnostics(&db)
    );
  }

  #[test]
  fn typecheck_schema_simple_props_no_errors() {
    let (db, project, file) = load_vault_fixture("typecheck/my_vault", "schemas/SimpleProps.td");
    let (hir, _) = lower_file(&db, project, file);
    let result = typecheck(&db, hir.unwrap());
    assert!(
      result.diagnostics(&db).is_empty(),
      "schema with simple property types should have no errors: {:?}",
      result.diagnostics(&db)
    );
  }

  #[test]
  fn typecheck_schema_with_optional_no_errors() {
    let (db, project, file) = load_vault_fixture("typecheck/my_vault", "schemas/WithOptional.td");
    let (hir, _) = lower_file(&db, project, file);
    let result = typecheck(&db, hir.unwrap());
    assert!(
      result.diagnostics(&db).is_empty(),
      "schema with optional property should have no errors: {:?}",
      result.diagnostics(&db)
    );
  }

  #[test]
  fn typecheck_schema_with_explicit_type_tag_no_errors() {
    let (db, project, file) =
      load_vault_fixture("typecheck/my_vault", "schemas/WithExplicitTypeTag.td");
    let (hir, _) = lower_file(&db, project, file);
    let result = typecheck(&db, hir.unwrap());
    assert!(
      result.diagnostics(&db).is_empty(),
      "schema with !type tags should have no errors: {:?}",
      result.diagnostics(&db)
    );
  }

  #[test]
  fn typecheck_schema_with_union_type_no_errors() {
    let (db, project, file) = load_vault_fixture("typecheck/my_vault", "schemas/WithUnion.td");
    let (hir, _) = lower_file(&db, project, file);
    let result = typecheck(&db, hir.unwrap());
    assert!(
      result.diagnostics(&db).is_empty(),
      "schema with union type should have no errors: {:?}",
      result.diagnostics(&db)
    );
  }

  #[test]
  fn typecheck_schema_with_nested_type_no_errors() {
    let (db, project, file) = load_vault_fixture("typecheck/my_vault", "schemas/WithNestedType.td");
    let (hir, _) = lower_file(&db, project, file);
    let result = typecheck(&db, hir.unwrap());
    assert!(
      result.diagnostics(&db).is_empty(),
      "schema with nested inline type should have no errors: {:?}",
      result.diagnostics(&db)
    );
  }

  #[test]
  fn typecheck_schema_with_literal_type_no_errors() {
    let (db, project, file) =
      load_vault_fixture("typecheck/my_vault", "schemas/WithLiteralType.td");
    let (hir, _) = lower_file(&db, project, file);
    let result = typecheck(&db, hir.unwrap());
    assert!(
      result.diagnostics(&db).is_empty(),
      "schema with literal types should have no errors: {:?}",
      result.diagnostics(&db)
    );
  }

  // Schema property descriptor tests: negative cases

  #[test]
  fn typecheck_schema_properties_not_mapping_has_errors() {
    let (db, project, file) =
      load_vault_fixture("typecheck/my_vault", "schemas/PropertiesNotMapping.td");
    let (hir, _) = lower_file(&db, project, file);
    let result = typecheck(&db, hir.unwrap());
    let diags = result.diagnostics(&db);
    assert!(
      !diags.is_empty(),
      "schema with non-mapping properties should have errors"
    );
  }

  #[test]
  fn typecheck_schema_missing_properties_field() {
    let (db, project, file) =
      load_vault_fixture("typecheck/my_vault", "schemas/MissingProperties.td");
    let (hir, _) = lower_file(&db, project, file);
    let result = typecheck(&db, hir.unwrap());
    let diags = result.diagnostics(&db);
    assert!(
      diags.iter().any(
        |d| matches!(d, Diagnostic::MissingRequiredField { field, .. } if field == "properties")
      ),
      "schema without properties should have MissingRequiredField: {:?}",
      diags
    );
  }

  // Property descriptor structural validation (extra fields, missing type) is handled
  // by evaluate_type::resolve_property_descriptor, not the typechecker. These tests
  // verify the typechecker does not produce false errors for these cases.

  #[test]
  fn typecheck_schema_prop_descriptor_extra_field_no_typecheck_errors() {
    let (db, project, file) =
      load_vault_fixture("typecheck/my_vault", "schemas/PropDescriptorExtraField.td");
    let (hir, _) = lower_file(&db, project, file);
    let result = typecheck(&db, hir.unwrap());
    assert!(
      result.diagnostics(&db).is_empty(),
      "extra fields in property descriptor are ignored by typechecker: {:?}",
      result.diagnostics(&db)
    );
  }

  #[test]
  fn typecheck_schema_prop_descriptor_missing_type_has_errors() {
    let (db, project, file) =
      load_vault_fixture("typecheck/my_vault", "schemas/PropDescriptorMissingType.td");
    let (hir, _) = lower_file(&db, project, file);
    let result = typecheck(&db, hir.unwrap());
    assert!(
      !result.diagnostics(&db).is_empty(),
      "missing type in property descriptor should produce typecheck errors"
    );
  }

  #[test]
  fn typecheck_valid_list_sum_no_errors() {
    let (db, project, file) =
      load_vault_fixture("typecheck/narrow_vault", "content/valid_list_sum.td");
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
    let (db, project, file) =
      load_vault_fixture("typecheck/narrow_vault", "content/invalid_list_sum.td");
    let (hir, _) = lower_file(&db, project, file);
    let result = typecheck(&db, hir.unwrap());
    assert!(
      !result.diagnostics(&db).is_empty(),
      "string | number should reject true"
    );
  }

  #[test]
  fn typecheck_valid_dict_sum_no_errors() {
    let (db, project, file) =
      load_vault_fixture("typecheck/narrow_vault", "content/valid_dict_sum.td");
    let (hir, _) = lower_file(&db, project, file);
    let result = typecheck(&db, hir.unwrap());
    assert!(
      result.diagnostics(&db).is_empty(),
      "dict with string and number values should match product {{ author: string, version: number }}: {:?}",
      result.diagnostics(&db)
    );
  }

  #[test]
  fn typecheck_invalid_dict_sum_has_errors() {
    let (db, project, file) =
      load_vault_fixture("typecheck/narrow_vault", "content/invalid_dict_sum.td");
    let (hir, _) = lower_file(&db, project, file);
    let result = typecheck(&db, hir.unwrap());
    assert!(
      !result.diagnostics(&db).is_empty(),
      "product {{ author: string, version: number }} should reject boolean value"
    );
  }

  // Mixed union: string | number | 'special'
  #[test]
  fn typecheck_mixed_union_accepts_string() {
    let (db, project, file) =
      load_vault_fixture("typecheck/narrow_vault", "content/mixed_union_string.td");
    let (hir, _) = lower_file(&db, project, file);
    let result = typecheck(&db, hir.unwrap());
    assert!(
      result.diagnostics(&db).is_empty(),
      "string | number | 'special' should accept \"hello\": {:?}",
      result.diagnostics(&db)
    );
  }

  #[test]
  fn typecheck_mixed_union_accepts_literal() {
    let (db, project, file) =
      load_vault_fixture("typecheck/narrow_vault", "content/mixed_union_literal.td");
    let (hir, _) = lower_file(&db, project, file);
    let result = typecheck(&db, hir.unwrap());
    assert!(
      result.diagnostics(&db).is_empty(),
      "string | number | 'special' should accept \"special\": {:?}",
      result.diagnostics(&db)
    );
  }

  #[test]
  fn typecheck_mixed_union_accepts_number() {
    let (db, project, file) =
      load_vault_fixture("typecheck/narrow_vault", "content/mixed_union_number.td");
    let (hir, _) = lower_file(&db, project, file);
    let result = typecheck(&db, hir.unwrap());
    assert!(
      result.diagnostics(&db).is_empty(),
      "string | number | 'special' should accept 42: {:?}",
      result.diagnostics(&db)
    );
  }

  #[test]
  fn typecheck_mixed_union_rejects_bool() {
    let (db, project, file) =
      load_vault_fixture("typecheck/narrow_vault", "content/mixed_union_invalid.td");
    let (hir, _) = lower_file(&db, project, file);
    let result = typecheck(&db, hir.unwrap());
    assert!(
      !result.diagnostics(&db).is_empty(),
      "string | number | 'special' should reject true"
    );
  }

  // Nested product type
  #[test]
  fn typecheck_nested_product_valid() {
    let (db, project, file) =
      load_vault_fixture("typecheck/narrow_vault", "content/nested_valid.td");
    let (hir, _) = lower_file(&db, project, file);
    let result = typecheck(&db, hir.unwrap());
    assert!(
      result.diagnostics(&db).is_empty(),
      "nested product with valid fields should pass: {:?}",
      result.diagnostics(&db)
    );
  }

  #[test]
  fn typecheck_nested_product_wrong_field_type() {
    let (db, project, file) =
      load_vault_fixture("typecheck/narrow_vault", "content/nested_wrong_type.td");
    let (hir, _) = lower_file(&db, project, file);
    let result = typecheck(&db, hir.unwrap());
    assert!(
      !result.diagnostics(&db).is_empty(),
      "nested product with number in string field should fail"
    );
  }

  // Contrived schema: literal types, mixed union, nested with optional
  #[test]
  fn typecheck_contrived_valid() {
    let (db, project, file) =
      load_vault_fixture("typecheck/narrow_vault", "content/contrived_valid.td");
    let (hir, _) = lower_file(&db, project, file);
    let result = typecheck(&db, hir.unwrap());
    assert!(
      result.diagnostics(&db).is_empty(),
      "contrived valid should pass: {:?}",
      result.diagnostics(&db)
    );
  }

  #[test]
  fn typecheck_contrived_wrong_literal_num() {
    let (db, project, file) = load_vault_fixture(
      "typecheck/narrow_vault",
      "content/contrived_wrong_literal.td",
    );
    let (hir, _) = lower_file(&db, project, file);
    let result = typecheck(&db, hir.unwrap());
    assert!(
      !result.diagnostics(&db).is_empty(),
      "literal_num: 99 should fail when schema expects 42"
    );
  }

  #[test]
  fn typecheck_contrived_missing_required_nested() {
    let (db, project, file) = load_vault_fixture(
      "typecheck/narrow_vault",
      "content/contrived_missing_required.td",
    );
    let (hir, _) = lower_file(&db, project, file);
    let result = typecheck(&db, hir.unwrap());
    assert!(
      !result.diagnostics(&db).is_empty(),
      "missing required_field in nested product should fail"
    );
  }

  // Mixed union [string, 0, true]: literal num 0 should match
  #[test]
  fn typecheck_contrived_mixed_accepts_literal_num() {
    let (db, project, file) =
      load_vault_fixture("typecheck/narrow_vault", "content/contrived_mixed_num.td");
    let (hir, _) = lower_file(&db, project, file);
    let result = typecheck(&db, hir.unwrap());
    assert!(
      result.diagnostics(&db).is_empty(),
      "[string, 0, true] should accept 0: {:?}",
      result.diagnostics(&db)
    );
  }

  // Mixed union [string, 0, true]: literal bool true should match
  #[test]
  fn typecheck_contrived_mixed_accepts_literal_bool() {
    let (db, project, file) =
      load_vault_fixture("typecheck/narrow_vault", "content/contrived_mixed_bool.td");
    let (hir, _) = lower_file(&db, project, file);
    let result = typecheck(&db, hir.unwrap());
    assert!(
      result.diagnostics(&db).is_empty(),
      "[string, 0, true] should accept true: {:?}",
      result.diagnostics(&db)
    );
  }

  // Fref with narrower union: Article.status is 'draft'|'published'|'archived',
  // Summary.status is 'draft'|'published'. Fref-ing the narrower field should be valid
  #[test]
  fn typecheck_fref_narrower_union_field() {
    let (db, project, file) =
      load_vault_fixture("typecheck/narrow_vault", "content/article_fref_status.td");
    let (hir, _) = lower_file(&db, project, file);
    let result = typecheck(&db, hir.unwrap());
    assert!(
      result.diagnostics(&db).is_empty(),
      "fref to narrower union field should be valid: {:?}",
      result.diagnostics(&db)
    );
  }
}
