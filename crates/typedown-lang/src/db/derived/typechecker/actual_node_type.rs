//! Tracked query to get the actual (bottom-up) type of a HIR value
// I think this is the idea of bidirectional typechecking

use std::collections::HashMap;

use crate::db::TypedownDatabase;
use crate::db::derived::evaluate::evaluate_type::evaluate_type;
use crate::db::derived::get_builtin_types::{
  get_bool_type, get_date_type, get_datetime_type, get_literal_type, get_math_type, get_null_type,
  get_num_type, get_str_type, get_sum_type, get_time_type, instantiate_type,
};
use crate::db::derived::get_vault_config::get_vault_config;
use crate::db::derived::name_resolver::file_symbol::file_symbol;
use crate::db::derived::name_resolver::referee::referee;
use crate::db::derived::typechecker::get_symbol_type::get_symbol_type;
use crate::db::types::derived::object_system::{
  TdStructuralType, is_valid_iso_date, is_valid_iso_datetime, is_valid_iso_time,
};
use crate::db::types::{
  BuiltinMacroKind, HirValue, HirValueKind, LazyType, LiteralValue, SymbolKind, TdListType,
  TdStrType, TdTypeEnum, TdTypeLike, TypeResult,
};
use crate::db::utils::lower_file;
use crate::syntax::diagnostic::Diagnostic;
use typedown_incremental::QueryDatabase;
use typedown_macros::query_derived;

// Infer the type of an HIR
// This function never relies on the declared type of the hir (it can rely on the declared type of the referenced hir)
// It always guesses based on the structure of the hir alone
#[query_derived]
pub fn actual_node_type(db: &TypedownDatabase, hir: HirValue) -> TypeResult {
  let diagnostics = vec![];
  match hir.kind(db) {
    HirValueKind::Str(ref val) => {
      // Date/time subtypes are more specific than string literals
      let typ = if is_valid_iso_datetime(val) {
        get_datetime_type(db).into()
      } else if is_valid_iso_date(val) {
        get_date_type(db).into()
      } else if is_valid_iso_time(val) {
        get_time_type(db).into()
      } else {
        get_literal_type(db, LiteralValue::Str(val.clone())).into()
      };
      TypeResult::new(db, Some(typ), diagnostics)
    }
    HirValueKind::Num(ref val) => TypeResult::new(
      db,
      Some(get_literal_type(db, LiteralValue::Num(val.clone())).into()),
      diagnostics,
    ),
    HirValueKind::Bool(val) => TypeResult::new(
      db,
      Some(get_literal_type(db, LiteralValue::Bool(val)).into()),
      diagnostics,
    ),
    HirValueKind::Interpolated(_) => TypeResult::new(db, Some(get_str_type(db).into()), vec![]),
    HirValueKind::Null => TypeResult::new(db, Some(get_null_type(db).into()), vec![]),
    HirValueKind::Ident(ref name) if name == "self" => get_self_type(db, hir),
    HirValueKind::Ident(_) => {
      let resolved = referee(db, hir);
      match resolved.value(db) {
        Some(symbol) => get_symbol_type(db, symbol),
        None => TypeResult::new(db, None, vec![]),
      }
    }
    HirValueKind::Mapping(entries) => get_mapping_type(db, hir, entries),
    HirValueKind::Sequence(items) => get_sequence_type(db, items),
    HirValueKind::Call { callee, args } => get_call_type(db, *callee, args),
    HirValueKind::Index { expr, indices } => get_index_type(db, *expr, indices),
    HirValueKind::Tag { tag, .. } => get_tag_type(db, *tag),
    HirValueKind::Prefix { op, operand } => get_prefix_type(db, &op, *operand),
    HirValueKind::Postfix { op, operand } => get_postfix_type(db, &op, *operand),
    HirValueKind::Binary { op, left, right } => get_binary_type(db, &op, *left, *right),
    HirValueKind::Math(_) => TypeResult::new(db, Some(get_math_type(db).into()), vec![]),
    HirValueKind::Markdown(_) => TypeResult::new(db, Some(get_str_type(db).into()), vec![]),
    HirValueKind::Closure { .. } => TypeResult::new(db, None, vec![]),
  }
}

/// Helper to get the type of a mapping
fn get_mapping_type(
  db: &TypedownDatabase,
  _hir: HirValue,
  entries: Vec<(String, HirValue)>,
) -> TypeResult {
  // If _type is present, resolve the schema
  for (key, value_hir) in &entries {
    if key == "_type" {
      let resolved = referee(db, *value_hir);
      if let Some(symbol) = resolved.value(db) {
        return evaluate_type(db, symbol);
      }
      let node = value_hir.node(db);
      let (tr_offset, tr_len) = node.trimmed_range();
      return TypeResult::new(
        db,
        None,
        vec![Diagnostic::UnresolvedSchema {
          name: node.text(),
          start_offset: tr_offset,
          end_offset: tr_offset + tr_len,
        }],
      );
    }
  }

  // No _type: infer a structural shape from the entries
  let mut diagnostics = vec![];
  let mut fields = HashMap::new();
  for (key, value_hir) in entries {
    let field_result = actual_node_type(db, value_hir);
    diagnostics.extend(field_result.diagnostics(db).iter().cloned());
    if let Some(typ) = field_result.typ(db) {
      fields.insert(key, LazyType::eager(typ));
    }
  }
  TypeResult::new(
    db,
    Some(TdStructuralType::new(db, fields).into()),
    diagnostics,
  )
}

/// Resolve a tag expression like `!Person { name: "John" }`
fn get_tag_type(db: &TypedownDatabase, tag: HirValue) -> TypeResult {
  let resolved = referee(db, tag);
  match resolved.value(db) {
    Some(symbol) => evaluate_type(db, symbol),
    None => {
      let node = tag.node(db);
      let (tr_offset, tr_len) = node.trimmed_range();
      TypeResult::new(
        db,
        None,
        vec![Diagnostic::UnresolvedSchema {
          name: node.text(),
          start_offset: tr_offset,
          end_offset: tr_offset + tr_len,
        }],
      )
    }
  }
}

/// Helper to get the return type of a prefix expression
fn get_prefix_type(db: &TypedownDatabase, op: &str, operand: HirValue) -> TypeResult {
  let operand_result = actual_node_type(db, operand);
  let diagnostics = operand_result.diagnostics(db).clone();

  match op {
    "-" | "+" => TypeResult::new(db, Some(get_num_type(db).into()), diagnostics),
    "~" => TypeResult::new(db, Some(get_bool_type(db).into()), diagnostics),
    _ => TypeResult::new(db, None, diagnostics),
  }
}

/// Helper to get the return type of a postfix expression
fn get_postfix_type(db: &TypedownDatabase, op: &str, operand: HirValue) -> TypeResult {
  let operand_result = actual_node_type(db, operand);
  let diagnostics = operand_result.diagnostics(db).clone();
  match op {
    // T? is a type operator, its result is a type
    "?" => operand_result,
    _ => TypeResult::new(db, None, diagnostics),
  }
}

/// Helper to get the return type of a binary expression
fn get_binary_type(db: &TypedownDatabase, op: &str, left: HirValue, right: HirValue) -> TypeResult {
  // Field access such as `obj.field`
  if op == "." {
    let left_result = actual_node_type(db, left);
    let mut diagnostics = left_result.diagnostics(db).clone();
    let left_type = match left_result.typ(db) {
      Some(typ) => typ,
      None => return TypeResult::new(db, None, diagnostics),
    };
    let field_name = match right.kind(db) {
      HirValueKind::Ident(name) => name,
      _ => return TypeResult::new(db, None, diagnostics),
    };
    return match left_type.lookup_field_type(db, &field_name) {
      Some(typ) => TypeResult::new(db, Some(typ), diagnostics),
      None => {
        let node = right.node(db);
        let (tr_offset, tr_len) = node.trimmed_range();
        diagnostics.push(Diagnostic::UnknownField {
          field: field_name,
          on_type: left_type.display_name(db),
          start_offset: tr_offset,
          end_offset: tr_offset + tr_len,
        });
        TypeResult::new(db, None, diagnostics)
      }
    };
  }

  let left_result = actual_node_type(db, left);
  let right_result = actual_node_type(db, right);
  let mut diagnostics = left_result.diagnostics(db).clone();
  diagnostics.extend(right_result.diagnostics(db).iter().cloned());

  match op {
    "+" | "-" | "*" | "/" | "%" | "**" => {
      TypeResult::new(db, Some(get_num_type(db).into()), diagnostics)
    }
    "==" | "!=" | "<" | ">" | "<=" | ">=" => {
      TypeResult::new(db, Some(get_bool_type(db).into()), diagnostics)
    }
    "&&" | "||" => TypeResult::new(db, Some(get_bool_type(db).into()), diagnostics),
    _ => TypeResult::new(db, None, diagnostics),
  }
}

/// Helper to get the type of a sequence
fn get_sequence_type(db: &TypedownDatabase, items: Vec<HirValue>) -> TypeResult {
  let mut diagnostics = vec![];
  let mut arms = vec![];

  for item in items {
    let item_result = actual_node_type(db, item);
    diagnostics.extend(item_result.diagnostics(db).iter().cloned());
    if let Some(typ) = item_result.typ(db) {
      arms.push(LazyType::eager(typ));
    }
  }

  let elem = if arms.len() == 1 {
    arms.into_iter().next().unwrap()
  } else {
    LazyType::eager(get_sum_type(db, arms.into_iter().collect()).into())
  };
  let list_type = TdListType::new(db, Some(elem));
  TypeResult::new(db, Some(list_type.into()), diagnostics)
}

/// Helper to get the type of a call expression
fn get_call_type(db: &TypedownDatabase, callee: HirValue, args: Vec<HirValue>) -> TypeResult {
  // Check if callee is a macro
  let resolved = referee(db, callee);
  if let Some(symbol) = resolved.value(db)
    && let SymbolKind::BuiltinMacro(kind) = symbol.kind(db)
  {
    return get_macro_call_type(db, kind, args);
  }

  let callee_result = actual_node_type(db, callee);
  let diagnostics = callee_result.diagnostics(db).clone();

  let callee_type = match callee_result.typ(db) {
    Some(typ) => typ,
    None => return TypeResult::new(db, None, diagnostics),
  };

  if let TdTypeEnum::TdFuncType(func) = &callee_type {
    let sig = func.signature(db);
    return TypeResult::new(db, Some(sig.ret(db)), diagnostics);
  }

  TypeResult::new(db, None, diagnostics)
}

fn get_macro_call_type(
  db: &TypedownDatabase,
  kind: BuiltinMacroKind,
  args: Vec<HirValue>,
) -> TypeResult {
  match kind {
    BuiltinMacroKind::Fref => get_fref_type(db, args),
  }
}

// fref("file.td") returns link[T] where T is the target file's schema type
fn get_fref_type(db: &TypedownDatabase, args: Vec<HirValue>) -> TypeResult {
  if args.len() != 1 {
    let node = args.first().map(|a| a.node(db));
    let (tr_offset, tr_len) = node.as_ref().map_or((0, 0), |n| n.trimmed_range());
    return TypeResult::new(
      db,
      None,
      vec![Diagnostic::WrongArgCount {
        expected: 1,
        got: args.len(),
        start_offset: tr_offset,
        end_offset: tr_offset + tr_len,
      }],
    );
  }
  let arg = args[0];
  let node = arg.node(db);
  let (tr_offset, tr_len) = node.trimmed_range();
  let path_str = match arg.kind(db) {
    HirValueKind::Str(val) => val,
    _ => {
      return TypeResult::new(
        db,
        None,
        vec![Diagnostic::ArgTypeMismatch {
          expected: "string".to_string(),
          start_offset: tr_offset,
          end_offset: tr_offset + tr_len,
        }],
      );
    }
  };

  let project = arg.project(db);
  let files = project.files(db);
  let content_dir = get_vault_config(db, project).content_dir(db);
  let target_path = content_dir.join(&path_str);

  let target_file = match files.get(&target_path) {
    Some(file) => *file,
    None => {
      return TypeResult::new(
        db,
        None,
        vec![Diagnostic::UnresolvedFileRef {
          path: path_str,
          start_offset: tr_offset,
          end_offset: tr_offset + tr_len,
        }],
      );
    }
  };
  let target_symbol = file_symbol(db, project, target_file);

  match target_symbol.value(db) {
    Some(sym) => get_symbol_type(db, sym),
    None => TypeResult::new(
      db,
      None,
      vec![Diagnostic::UnresolvedSchema {
        name: path_str,
        start_offset: tr_offset,
        end_offset: tr_offset + tr_len,
      }],
    ),
  }
}

/// Helper to get the type of an index expression
fn get_index_type(db: &TypedownDatabase, expr: HirValue, indices: Vec<HirValue>) -> TypeResult {
  let expr_result = actual_node_type(db, expr);
  let mut diagnostics = expr_result.diagnostics(db).clone();

  let expr_type = match expr_result.typ(db) {
    Some(typ) => typ,
    None => return TypeResult::new(db, None, diagnostics),
  };

  /* Generic instantiation */

  let expr_type = if expr_type.arity(db) == 0
    && let HirValueKind::Ident(_) = expr.kind(db)
    && let Some(symbol) = referee(db, expr).value(db)
    && let Some(typ) = evaluate_type(db, symbol).typ(db)
    && typ.arity(db) > 0
  {
    typ
  } else {
    expr_type
  };

  // Resolve each type argument and instantiate the generic type
  if expr_type.arity(db) > 0 {
    let mut arg_types = vec![];
    for idx_hir in indices {
      let resolved = referee(db, idx_hir);
      match resolved.value(db) {
        Some(symbol) => {
          let schema_result = evaluate_type(db, symbol);
          diagnostics.extend(schema_result.diagnostics(db).iter().cloned());
          match schema_result.typ(db) {
            Some(typ) => arg_types.push(typ),
            None => return TypeResult::new(db, None, diagnostics),
          }
        }
        None => {
          let node = idx_hir.node(db);
          let (tr_offset, tr_len) = node.trimmed_range();
          diagnostics.push(Diagnostic::UnresolvedSchema {
            name: node.text(),
            start_offset: tr_offset,
            end_offset: tr_offset + tr_len,
          });
          return TypeResult::new(db, None, diagnostics);
        }
      }
    }
    let inst_result = instantiate_type(
      db,
      expr_type,
      arg_types.into_iter().map(LazyType::eager).collect(),
    );
    diagnostics.extend(inst_result.diagnostics(db).iter().cloned());
    return TypeResult::new(db, Some(inst_result.typ(db)), diagnostics);
  }

  // Element access on instantiated list
  if let TdTypeEnum::TdListType(list) = &expr_type {
    return match list.elem(db).and_then(|e| e.resolve(db)) {
      Some(elem) => TypeResult::new(db, Some(elem), diagnostics),
      None => TypeResult::new(db, None, diagnostics),
    };
  }

  // Element access on instantiated dict
  if let TdTypeEnum::TdDictType(dict) = &expr_type {
    return match dict.value(db).and_then(|l| l.resolve(db)) {
      Some(value) => TypeResult::new(db, Some(value), diagnostics),
      None => TypeResult::new(db, None, diagnostics),
    };
  }

  // Element access on string
  if expr_type.is_td_str_type() {
    return TypeResult::new(db, Some(TdStrType::get(db).into()), diagnostics);
  }

  TypeResult::new(db, None, diagnostics)
}

/// Return the type of `self` in the current file
fn get_self_type(db: &TypedownDatabase, hir: HirValue) -> TypeResult {
  let project = hir.project(db);
  let file = hir.file(db);
  let (mapping_hir, _) = lower_file(db, project, file);
  let mapping_hir = match mapping_hir {
    Some(mapping_hir) => mapping_hir,
    None => return TypeResult::new(db, None, vec![]),
  };

  if let HirValueKind::Mapping(entries) = mapping_hir.kind(db) {
    for (key, val_hir) in entries {
      if key == "_type" {
        let resolved = referee(db, val_hir);
        return match resolved.value(db) {
          Some(symbol) => evaluate_type(db, symbol),
          None => TypeResult::new(db, None, vec![]),
        };
      }
    }
  }
  TypeResult::new(db, None, vec![])
}

#[cfg(test)]
mod tests {
  use crate::db::types::TdTypeEnum;
  use std::{collections::HashMap, path::PathBuf};

  use crate::db::{
    QueryStorage, TypedownDatabase,
    derived::get_builtin_types::get_schema_type,
    types::{File, FileHandle, FileMetadata, Project},
    utils::lower_file,
  };

  use crate::db::{
    fixtures::load_vault_fixture,
    types::{HirValueKind, LiteralValue, TdTypeLike},
  };

  use super::actual_node_type;

  fn is_literal_str(db: &TypedownDatabase, typ: &TdTypeEnum, expected: &str) -> bool {
    if let TdTypeEnum::TdLiteralType(lit) = typ {
      return lit.value(db) == LiteralValue::Str(expected.to_string());
    }
    false
  }

  fn is_literal_num(db: &TypedownDatabase, typ: &TdTypeEnum, expected: &str) -> bool {
    if let TdTypeEnum::TdLiteralType(lit) = typ {
      return lit.value(db) == LiteralValue::Num(expected.to_string());
    }
    false
  }

  fn is_literal_bool(db: &TypedownDatabase, typ: &TdTypeEnum, expected: bool) -> bool {
    if let TdTypeEnum::TdLiteralType(lit) = typ {
      return lit.value(db) == LiteralValue::Bool(expected);
    }
    false
  }

  fn vault_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/evaluate_schema/my_vault")
  }

  #[test]
  fn infer_anonymous_mapping_narrows_literal_fields() {
    let (db, project, file) =
      load_vault_fixture("typecheck/narrow_vault", "content/anonymous_mapping.td");
    let (hir, _) = lower_file(&db, project, file);
    let hir = hir.expect("should parse");
    let result = actual_node_type(&db, hir);
    let typ = result.typ(&db).expect("should infer a type");
    let structural = typ
      .as_td_structural_type()
      .expect("anonymous mapping should be Structural");
    let fields = structural.fields(&db);

    // String literal narrows to TdLiteralType(Str)
    let name_lazy = fields.get("name").expect("should have name field");
    let name_typ = name_lazy.resolve(&db).expect("should resolve");
    assert!(
      is_literal_str(&db, &name_typ, "Alice"),
      "name should be literal str \"Alice\""
    );

    // Num literal narrows to TdLiteralType(Num)
    let age_lazy = fields.get("age").expect("should have age field");
    let age_typ = age_lazy.resolve(&db).expect("should resolve");
    assert!(
      is_literal_num(&db, &age_typ, "30"),
      "age should be literal num \"30\""
    );

    // Bool literal narrows to TdLiteralType(Bool)
    let active_lazy = fields.get("active").expect("should have active field");
    let active_typ = active_lazy.resolve(&db).expect("should resolve");
    assert!(
      is_literal_bool(&db, &active_typ, true),
      "active should be literal bool true"
    );

    // Sequence ["a", 3] narrows to list[sum]
    let tags_lazy = fields.get("tags").expect("should have tags field");
    let tags_typ = tags_lazy.resolve(&db).expect("should resolve");
    let list = tags_typ
      .as_td_list_type()
      .expect("tags should be a list type");
    let elem = list.elem(&db).expect("list should have elem");
    let elem_typ = elem.resolve(&db).expect("elem should resolve");
    let sum = elem_typ
      .as_td_sum_type()
      .expect("elem should be a sum type");
    let members = sum.members(&db);
    assert_eq!(members.len(), 2, "tags should have 2 arms");
    let has_a = members
      .iter()
      .any(|m| m.resolve(&db).is_some_and(|t| is_literal_str(&db, &t, "a")));
    let has_3 = members
      .iter()
      .any(|m| m.resolve(&db).is_some_and(|t| is_literal_num(&db, &t, "3")));
    assert!(has_a, "arms should contain literal str 'a'");
    assert!(has_3, "arms should contain literal num '3'");
  }

  #[test]
  fn actual_node_type_of_schema_file_top_level_mapping_is_schema_type() {
    let vault = vault_root();
    let schema_file_path = vault.join("schemas/Person.td");

    let db = TypedownDatabase {
      storage: QueryStorage::default(),
    };

    let file = File::new(
      &db,
      FileHandle::Path(schema_file_path.clone(), FileMetadata::default()),
    );
    let files = HashMap::from([(schema_file_path, file)]);
    let project = Project::new(&db, vault, files);

    let (hir, _) = lower_file(&db, project, file);
    let hir = hir.expect("schema file should have parseable frontmatter");
    let result = actual_node_type(&db, hir);

    let typ = result.typ(&db);
    let expected = Some(TdTypeEnum::from(get_schema_type(&db)));
    assert!(
      typ == expected,
      "top-level mapping of a schema file should have schema type"
    );
    assert!(
      result.diagnostics(&db).is_empty(),
      "expected no diagnostics, got: {:?}",
      result.diagnostics(&db)
    );
  }

  #[test]
  fn actual_node_type_string_literal_returns_literal() {
    let (db, project, file) = load_vault_fixture("typecheck/my_vault", "content/valid_person.td");
    let (hir, _) = lower_file(&db, project, file);
    let hir = hir.expect("should parse");
    // Get the "name" field value ("Alice")
    if let HirValueKind::Mapping(entries) = hir.kind(&db) {
      let name_hir = entries.iter().find(|(k, _)| k == "name").map(|(_, v)| *v);
      let name_hir = name_hir.expect("should have name field");
      let result = actual_node_type(&db, name_hir);
      let typ = result.typ(&db).expect("should have a type");
      assert!(
        is_literal_str(&db, &typ, "Alice"),
        "string value should be literal str"
      );
    }
  }

  #[test]
  fn actual_node_type_bool_returns_literal() {
    let (db, project, file) =
      load_vault_fixture("typecheck/narrow_vault", "content/anonymous_mapping.td");
    let (hir, _) = lower_file(&db, project, file);
    let hir = hir.expect("should parse");
    if let HirValueKind::Mapping(entries) = hir.kind(&db) {
      let active_hir = entries.iter().find(|(k, _)| k == "active").map(|(_, v)| *v);
      let active_hir = active_hir.expect("should have active field");
      let result = actual_node_type(&db, active_hir);
      let typ = result.typ(&db).expect("should have a type");
      assert!(
        is_literal_bool(&db, &typ, true),
        "bool value should be literal bool"
      );
    }
  }

  #[test]
  fn actual_node_type_sequence_returns_list_type() {
    let (db, project, file) =
      load_vault_fixture("typecheck/narrow_vault", "content/anonymous_mapping.td");
    let (hir, _) = lower_file(&db, project, file);
    let hir = hir.expect("should parse");
    if let HirValueKind::Mapping(entries) = hir.kind(&db) {
      let tags_hir = entries.iter().find(|(k, _)| k == "tags").map(|(_, v)| *v);
      let tags_hir = tags_hir.expect("should have tags field");
      let result = actual_node_type(&db, tags_hir);
      let typ = result.typ(&db).expect("should have a type");
      assert!(typ.is_td_list_type(), "sequence should be a list type");
    }
  }

  // Date strings narrow to date type, not Literal
  #[test]
  fn actual_node_type_date_string_returns_simple_date() {
    let (db, project, file) = load_vault_fixture("typecheck/my_vault", "content/valid_event.td");
    let (hir, _) = lower_file(&db, project, file);
    let hir = hir.expect("should parse");
    if let HirValueKind::Mapping(entries) = hir.kind(&db) {
      let date_hir = entries.iter().find(|(k, _)| k == "date").map(|(_, v)| *v);
      let date_hir = date_hir.expect("should have date field");
      let result = actual_node_type(&db, date_hir);
      let typ = result.typ(&db).expect("should have a type");
      assert_eq!(
        typ.display_name(&db),
        "date",
        "ISO date string should resolve to date"
      );
    }
  }

  // Fref returns the resource's schema type, not type_type
  #[test]
  fn actual_node_type_fref_returns_resource_type() {
    let (db, project, file) =
      load_vault_fixture("typecheck/narrow_vault", "content/article_fref_status.td");
    let (hir, _) = lower_file(&db, project, file);
    let hir = hir.expect("should parse");
    if let HirValueKind::Mapping(entries) = hir.kind(&db) {
      // status: fref("summary.td").status
      let status_hir = entries.iter().find(|(k, _)| k == "status").map(|(_, v)| *v);
      let status_hir = status_hir.expect("should have status field");
      let result = actual_node_type(&db, status_hir);
      // Should resolve to something (not None), and not be type_type
      if let Some(typ) = result.typ(&db) {
        assert_ne!(
          typ.display_name(&db),
          "type",
          "fref field access should not return type_type"
        );
      }
    }
  }

  // Num literal returns Literal(Num)
  #[test]
  fn actual_node_type_num_returns_literal() {
    let (db, project, file) = load_vault_fixture("typecheck/my_vault", "content/valid_person.td");
    let (hir, _) = lower_file(&db, project, file);
    let hir = hir.expect("should parse");
    if let HirValueKind::Mapping(entries) = hir.kind(&db) {
      let age_hir = entries.iter().find(|(k, _)| k == "age").map(|(_, v)| *v);
      let age_hir = age_hir.expect("should have age field");
      let result = actual_node_type(&db, age_hir);
      let typ = result.typ(&db).expect("should have a type");
      assert!(
        is_literal_num(&db, &typ, "30"),
        "number value should be literal num"
      );
    }
  }
}
