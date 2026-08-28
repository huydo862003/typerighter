//! Tracked query to get the actual (bottom-up) type of a HIR value
// I think this is the idea of bidirectional typechecking

use std::collections::HashMap;

use crate::db::TypedownDatabase;
use crate::db::derived::evaluate::evaluate_type::evaluate_type;
use crate::db::derived::get_builtin_types::{
  get_bool_type, get_date_type, get_datetime_type, get_func_type, get_list_type, get_literal_type,
  get_math_type, get_null_type, get_num_type, get_str_type, get_sum_type, get_time_type,
};
use crate::db::derived::get_vault_config::get_vault_config;
use crate::db::derived::name_resolver::file_symbol::file_symbol;
use crate::db::derived::name_resolver::referee::referee;
use crate::db::derived::typechecker::expected_node_type::expected_node_type;
use crate::db::derived::typechecker::get_symbol_type::get_symbol_type;
use crate::db::types::derived::object_system::{
  TdProductType, TdStaticType, is_valid_iso_date, is_valid_iso_datetime, is_valid_iso_time,
};
use crate::db::types::{
  BuiltinMacroKind, FuncSignature, HirValue, HirValueKind, LazyType, LiteralValue, SymbolKind,
  TdTypeEnum, TypeResult,
};
use crate::syntax::diagnostic::Diagnostic;
use typedown_incremental::QueryDatabase;
use typedown_macros::query_derived;

// Infer the type of an HIR bottom-up from its structure
// Exception: closures read expected(closure) to get param types (see README.md)
#[query_derived]
pub fn actual_node_type<'db>(db: &'db TypedownDatabase, hir: HirValue<'db>) -> TypeResult<'db> {
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
    HirValueKind::Prefix { op, .. } => get_prefix_type(db, &op),
    HirValueKind::Postfix { op, operand } => get_postfix_type(db, &op, *operand),
    HirValueKind::Binary { op, left, right } => get_binary_type(db, &op, *left, *right),
    HirValueKind::Math(_) => TypeResult::new(db, Some(get_math_type(db).into()), vec![]),
    HirValueKind::Markdown(_) => TypeResult::new(db, Some(get_str_type(db).into()), vec![]),
    HirValueKind::Closure { params, body } => get_closure_type(db, hir, params, *body),
  }
}

// Helper to get the type of a mapping
fn get_mapping_type<'db>(
  db: &'db TypedownDatabase,
  _hir: HirValue<'db>,
  entries: Vec<(String, HirValue<'db>)>,
) -> TypeResult<'db> {
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
    Some(TdProductType::new(db, None, fields).into()),
    diagnostics,
  )
}

// Resolve a tag expression like !Person { name: "John" }
fn get_tag_type<'db>(db: &'db TypedownDatabase, tag: HirValue<'db>) -> TypeResult<'db> {
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

// Synthesize the result type of a prefix expression
fn get_prefix_type<'db>(db: &'db TypedownDatabase, op: &str) -> TypeResult<'db> {
  match op {
    "-" | "+" => TypeResult::new(db, Some(get_num_type(db).into()), vec![]),
    "~" => TypeResult::new(db, Some(get_bool_type(db).into()), vec![]),
    _ => TypeResult::new(db, None, vec![]),
  }
}

// Synthesize the result type of a postfix expression
fn get_postfix_type<'db>(
  db: &'db TypedownDatabase,
  op: &str,
  operand: HirValue<'db>,
) -> TypeResult<'db> {
  match op {
    // T? is a type operator, its result is the operand type
    "?" => actual_node_type(db, operand),
    _ => TypeResult::new(db, None, vec![]),
  }
}

// Synthesize the result type of a binary expression
fn get_binary_type<'db>(
  db: &'db TypedownDatabase,
  op: &str,
  left: HirValue<'db>,
  right: HirValue<'db>,
) -> TypeResult<'db> {
  // Field access needs actual(left) to look up the field type
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

  match op {
    "+" | "-" | "*" | "/" | "%" | "**" => {
      TypeResult::new(db, Some(get_num_type(db).into()), vec![])
    }
    "==" | "!=" | "<" | ">" | "<=" | ">=" => {
      TypeResult::new(db, Some(get_bool_type(db).into()), vec![])
    }
    "&&" | "||" => TypeResult::new(db, Some(get_bool_type(db).into()), vec![]),
    _ => TypeResult::new(db, None, vec![]),
  }
}

// Helper to get the type of a sequence
fn get_sequence_type<'db>(db: &'db TypedownDatabase, items: Vec<HirValue<'db>>) -> TypeResult<'db> {
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
  let list_type = get_list_type(db).instantiate(db, vec![elem]).typ(db);
  TypeResult::new(db, Some(list_type), diagnostics)
}

// Helper to get the type of a call expression
fn get_call_type<'db>(
  db: &'db TypedownDatabase,
  callee: HirValue<'db>,
  args: Vec<HirValue<'db>>,
) -> TypeResult<'db> {
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

  let arg_types: Vec<TdTypeEnum> = args
    .iter()
    .filter_map(|arg| actual_node_type(db, *arg).typ(db))
    .collect();

  if let Some(sig) = callee_type.call_type(db, arg_types) {
    return TypeResult::new(db, Some(sig.ret(db)), diagnostics);
  }

  TypeResult::new(db, None, diagnostics)
}

fn get_macro_call_type<'db>(
  db: &'db TypedownDatabase,
  kind: BuiltinMacroKind,
  args: Vec<HirValue<'db>>,
) -> TypeResult<'db> {
  match kind {
    BuiltinMacroKind::Fref => get_fref_type(db, args),
  }
}

// fref("file.td") returns link[T] where T is the target file's schema type
fn get_fref_type<'db>(db: &'db TypedownDatabase, args: Vec<HirValue<'db>>) -> TypeResult<'db> {
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
  let root_dir = get_vault_config(db, project).root_dir(db);
  let target_path = root_dir.join(&path_str);

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

// Helper to get the type of an index expression
fn get_index_type<'db>(
  db: &'db TypedownDatabase,
  expr: HirValue<'db>,
  indices: Vec<HirValue<'db>>,
) -> TypeResult<'db> {
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
    let inst_result =
      expr_type.instantiate(db, arg_types.into_iter().map(LazyType::eager).collect());
    diagnostics.extend(inst_result.diagnostics(db).iter().cloned());
    return TypeResult::new(db, Some(inst_result.typ(db)), diagnostics);
  }

  let key_type = indices
    .first()
    .and_then(|idx| actual_node_type(db, *idx).typ(db))
    .unwrap_or_else(|| get_num_type(db).into());

  if let Some(sig) = expr_type.index_type(db, &key_type) {
    return TypeResult::new(db, Some(sig.ret(db)), diagnostics);
  }

  TypeResult::new(db, None, diagnostics)
}

// actual(closure) = fn(params from expected, return from actual(body))
fn get_closure_type<'db>(
  db: &'db TypedownDatabase,
  hir: HirValue<'db>,
  params: Vec<String>,
  body: HirValue<'db>,
) -> TypeResult<'db> {
  let expected = expected_node_type(db, hir).typ(db);
  let param_types = match expected {
    Some(TdTypeEnum::TdFuncType(f)) => f.signature(db).params(db),
    _ => return TypeResult::new(db, None, vec![]),
  };

  if param_types.len() != params.len() {
    let node = hir.node(db);
    let (tr_offset, tr_len) = node.trimmed_range();
    return TypeResult::new(
      db,
      None,
      vec![Diagnostic::WrongArgCount {
        expected: param_types.len(),
        got: params.len(),
        start_offset: tr_offset,
        end_offset: tr_offset + tr_len,
      }],
    );
  }

  let body_result = actual_node_type(db, body);
  let ret = match body_result.typ(db) {
    Some(t) => t,
    None => return TypeResult::new(db, None, body_result.diagnostics(db).clone()),
  };

  let sig = FuncSignature::new(db, param_types, ret);
  let func_type = get_func_type(db, sig);
  TypeResult::new(
    db,
    Some(func_type.into()),
    body_result.diagnostics(db).clone(),
  )
}

#[cfg(test)]
mod tests {
  use crate::db::derived::get_builtin_types::{
    get_func_type, get_literal_type, get_schema_meta_type, get_str_type,
  };
  use crate::db::types::TdTypeEnum;
  use crate::db::types::derived::object_system::TdStaticType;
  use crate::db::types::{File, FileHandle, FileMetadata, FuncSignature, LiteralValue, Project};
  use std::{collections::HashMap, path::PathBuf};

  use crate::db::{QueryStorage, TypedownDatabase, utils::lower_file};

  use crate::db::{fixtures::load_vault_fixture, types::HirValueKind};

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

  fn is_literal_bool(db: &TypedownDatabase, typ: &TdTypeEnum<'_>, expected: bool) -> bool {
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
    let (db, project, file) = load_vault_fixture("typecheck/narrow_vault", "anonymous_mapping.td");
    let (hir, _) = lower_file(&db, project, file);
    let hir = hir.expect("should parse");
    let result = actual_node_type(&db, hir);
    let typ = result.typ(&db).expect("should infer a type");
    let product = typ
      .as_td_product_type()
      .expect("anonymous mapping should be Product");
    let fields = product.get_fields(&db);

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
    let schema_file_path = vault.join("_types/Person.td");

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
    let expected = Some(TdTypeEnum::from(get_schema_meta_type(&db)));
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
    let (db, project, file) = load_vault_fixture("typecheck/my_vault", "valid_person.td");
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
    let (db, project, file) = load_vault_fixture("typecheck/narrow_vault", "anonymous_mapping.td");
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
    let (db, project, file) = load_vault_fixture("typecheck/narrow_vault", "anonymous_mapping.td");
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
    let (db, project, file) = load_vault_fixture("typecheck/my_vault", "valid_event.td");
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
      load_vault_fixture("typecheck/narrow_vault", "article_fref_status.td");
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
    let (db, project, file) = load_vault_fixture("typecheck/my_vault", "valid_person.td");
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

  #[test]
  fn actual_node_type_to_string_field_access_returns_func_type() {
    let (db, _, _) = load_vault_fixture("typecheck/my_vault", "valid_person.td");
    let num_lit: TdTypeEnum<'_> = get_literal_type(&db, LiteralValue::Num("42".to_string())).into();

    let field_type = num_lit
      .lookup_field_type(&db, "to_string")
      .expect("should have to_string method");
    let expected_sig = FuncSignature::new(&db, vec![], get_str_type(&db).into());
    let expected_func_type: TdTypeEnum = get_func_type(&db, expected_sig).into();

    assert_eq!(field_type, expected_func_type);
  }

  #[test]
  fn actual_node_type_method_call_to_string_returns_string_type() {
    let (db, project, file) = load_vault_fixture("typecheck/my_vault", "method_call.td");
    let (hir, _) = lower_file(&db, project, file);
    let hir = hir.expect("should parse");
    if let HirValueKind::Mapping(entries) = hir.kind(&db) {
      let res_hir = entries
        .iter()
        .find(|(k, _)| k == "result")
        .map(|(_, v)| *v)
        .unwrap();
      let result = actual_node_type(&db, res_hir);
      let typ = result.typ(&db).expect("should have a type");
      let str_type: TdTypeEnum = get_str_type(&db).into();
      assert_eq!(typ, str_type);
    }
  }
}
