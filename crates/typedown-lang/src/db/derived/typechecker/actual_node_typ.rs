//! Tracked query to get the actual (bottom-up) type of a HIR value
// I think this is the idea of bidirectional typechecking

use std::collections::BTreeMap;

use crate::db::TypedownDatabase;
use crate::db::derived::evaluate::evaluate_typ::evaluate_typ;
use crate::db::derived::get_builtin_typs::{
  get_bool_typ, get_date_typ, get_datetime_typ, get_func_typ, get_list_typ, get_lit_typ,
  get_math_typ, get_null_typ, get_number_typ, get_string_typ, get_sum_typ, get_time_typ,
  get_typ_typ,
};
use crate::db::derived::get_vault_config::get_vault_config;
use crate::db::derived::name_resolver::file_symbol::file_symbol;
use crate::db::derived::name_resolver::referee::referee;
use crate::db::derived::typechecker::expected_node_typ::expected_node_typ;
use crate::db::derived::typechecker::get_symbol_typ::get_symbol_typ;
use crate::db::types::derived::obj_system::{
  TdProductTyp, TdStaticTyp, is_valid_iso_date, is_valid_iso_datetime, is_valid_iso_time,
};
use crate::db::utils::resolve_fref_path;
use crate::db::types::{
  BuiltinMacroKind, FuncSignature, HirValue, HirValueKind, LazyTyp, LitValue, SymbolKind,
  TdTypEnum, TypResult,
};
use crate::syntax::diagnostic::Diagnostic;
use typedown_incremental::QueryDatabase;
use typedown_macros::query_derived;

// Infer the type of an HIR bottom-up from its structure
// Exception: closures read expected(closure) to get param types (see README.md)
#[query_derived(no_hash)]
pub fn actual_node_typ<'db>(db: &'db TypedownDatabase, hir: HirValue<'db>) -> TypResult<'db> {
  let diagnostics = vec![];
  match hir.kind(db) {
    HirValueKind::String(ref val) => {
      // Date/time subtypes are more specific than string literals
      let typ = if is_valid_iso_datetime(val) {
        get_datetime_typ(db).into()
      } else if is_valid_iso_date(val) {
        get_date_typ(db).into()
      } else if is_valid_iso_time(val) {
        get_time_typ(db).into()
      } else {
        get_lit_typ(db, LitValue::String(val.clone())).into()
      };
      TypResult::new(db, Some(typ), diagnostics)
    }
    HirValueKind::Number(ref val) => TypResult::new(
      db,
      Some(get_lit_typ(db, LitValue::Number(val.clone())).into()),
      diagnostics,
    ),
    HirValueKind::Bool(val) => TypResult::new(
      db,
      Some(get_lit_typ(db, LitValue::Bool(val)).into()),
      diagnostics,
    ),
    HirValueKind::Interpolated(_) => TypResult::new(db, Some(get_string_typ(db).into()), vec![]),
    HirValueKind::Null => TypResult::new(db, Some(get_null_typ(db).into()), vec![]),
    HirValueKind::Ident(_) => {
      let resolved = referee(db, hir);
      match resolved.value(db) {
        Some(symbol) => get_symbol_typ(db, symbol),
        None => TypResult::new(db, None, vec![]),
      }
    }
    HirValueKind::Mapping(entries) => get_mapping_typ(db, hir, entries),
    HirValueKind::Sequence(items) => get_sequence_typ(db, items),
    HirValueKind::Call { callee, args } => get_call_typ(db, *callee, args),
    HirValueKind::Index { expr, indices } => get_index_typ(db, *expr, indices),
    HirValueKind::Tag { tag, .. } => get_tag_typ(db, *tag),
    HirValueKind::Prefix { op, .. } => get_prefix_typ(db, &op),
    HirValueKind::Postfix { op, operand } => get_postfix_typ(db, &op, *operand),
    HirValueKind::Binary { op, left, right } => get_binary_typ(db, &op, *left, *right),
    HirValueKind::Math(_) => TypResult::new(db, Some(get_math_typ(db).into()), vec![]),
    HirValueKind::Markdown(_) => TypResult::new(db, Some(get_string_typ(db).into()), vec![]),
    HirValueKind::Closure { params, body } => get_closure_typ(db, hir, params, *body),
  }
}

// Helper to get the type of a mapping
fn get_mapping_typ<'db>(
  db: &'db TypedownDatabase,
  _hir: HirValue<'db>,
  entries: Vec<(String, HirValue<'db>)>,
) -> TypResult<'db> {
  // If _type is present, resolve the schema
  for (key, value_hir) in &entries {
    if key == "_type" {
      let resolved = referee(db, *value_hir);
      if let Some(symbol) = resolved.value(db) {
        return evaluate_typ(db, symbol);
      }
      let node = value_hir.node(db);
      let (trimmed_offset, trimmed_len) = node.trimmed_range();
      return TypResult::new(
        db,
        None,
        vec![Diagnostic::UnresolvedSchema {
          name: node.text(),
          start_offset: trimmed_offset,
          end_offset: trimmed_offset + trimmed_len,
        }],
      );
    }
  }

  // No _type: infer a structural shape from the entries
  let mut diagnostics = vec![];
  let mut fields = BTreeMap::new();
  for (key, value_hir) in entries {
    let field_result = actual_node_typ(db, value_hir);
    diagnostics.extend(field_result.diagnostics(db).iter().cloned());
    if let Some(typ) = field_result.typ(db) {
      fields.insert(key, LazyTyp::eager(typ));
    }
  }
  TypResult::new(
    db,
    Some(TdProductTyp::new(db, None, fields).into()),
    diagnostics,
  )
}

// Resolve a tag expression like !Person { name: "John" }
fn get_tag_typ<'db>(db: &'db TypedownDatabase, tag: HirValue<'db>) -> TypResult<'db> {
  let resolved = referee(db, tag);
  match resolved.value(db) {
    Some(symbol) => evaluate_typ(db, symbol),
    None => {
      let node = tag.node(db);
      let (trimmed_offset, trimmed_len) = node.trimmed_range();
      TypResult::new(
        db,
        None,
        vec![Diagnostic::UnresolvedSchema {
          name: node.text(),
          start_offset: trimmed_offset,
          end_offset: trimmed_offset + trimmed_len,
        }],
      )
    }
  }
}

// Synthesize the result type of a prefix expression
fn get_prefix_typ<'db>(db: &'db TypedownDatabase, op: &str) -> TypResult<'db> {
  match op {
    "-" | "+" => TypResult::new(db, Some(get_number_typ(db).into()), vec![]),
    "~" => TypResult::new(db, Some(get_bool_typ(db).into()), vec![]),
    _ => TypResult::new(db, None, vec![]),
  }
}

// Synthesize the result type of a postfix expression
fn get_postfix_typ<'db>(
  db: &'db TypedownDatabase,
  op: &str,
  operand: HirValue<'db>,
) -> TypResult<'db> {
  match op {
    // T? is a type operator, its result is the operand type
    "?" => actual_node_typ(db, operand),
    _ => TypResult::new(db, None, vec![]),
  }
}

// Synthesize the result type of a binary expression
fn get_binary_typ<'db>(
  db: &'db TypedownDatabase,
  op: &str,
  left: HirValue<'db>,
  right: HirValue<'db>,
) -> TypResult<'db> {
  // Field access needs actual(left) to look up the field type
  if op == "." {
    let left_result = actual_node_typ(db, left);
    let mut diagnostics = left_result.diagnostics(db).clone();
    let left_typ = match left_result.typ(db) {
      Some(typ) => typ,
      None => return TypResult::new(db, None, diagnostics),
    };
    let field_name = match right.kind(db) {
      HirValueKind::Ident(name) => name,
      _ => return TypResult::new(db, None, diagnostics),
    };
    return match left_typ.lookup_field_typ(db, &field_name) {
      Some(typ) => TypResult::new(db, Some(typ), diagnostics),
      None => {
        let node = right.node(db);
        let (trimmed_offset, trimmed_len) = node.trimmed_range();
        diagnostics.push(Diagnostic::UnknownField {
          field: field_name,
          on_typ: left_typ.display_name(db),
          start_offset: trimmed_offset,
          end_offset: trimmed_offset + trimmed_len,
        });
        TypResult::new(db, None, diagnostics)
      }
    };
  }

  match op {
    "+" | "-" | "*" | "/" | "%" | "**" => {
      TypResult::new(db, Some(get_number_typ(db).into()), vec![])
    }
    "==" | "!=" | "<" | ">" | "<=" | ">=" => {
      TypResult::new(db, Some(get_bool_typ(db).into()), vec![])
    }
    "&&" | "||" => TypResult::new(db, Some(get_bool_typ(db).into()), vec![]),
    // FIXME: validate both operands are type expressions
    "|" => TypResult::new(db, Some(get_typ_typ(db).into()), vec![]),
    _ => TypResult::new(db, None, vec![]),
  }
}

// Helper to get the type of a sequence
fn get_sequence_typ<'db>(db: &'db TypedownDatabase, items: Vec<HirValue<'db>>) -> TypResult<'db> {
  let mut diagnostics = vec![];
  let mut arms = vec![];

  for item in items {
    let item_result = actual_node_typ(db, item);
    diagnostics.extend(item_result.diagnostics(db).iter().cloned());
    if let Some(typ) = item_result.typ(db) {
      arms.push(LazyTyp::eager(typ));
    }
  }

  let element = if arms.len() == 1 {
    arms.into_iter().next().unwrap()
  } else {
    LazyTyp::eager(get_sum_typ(db, arms.into_iter().collect()).into())
  };
  let list_typ = get_list_typ(db).instantiate(db, vec![element]).typ(db);
  TypResult::new(db, Some(list_typ), diagnostics)
}

// Helper to get the type of a call expression
fn get_call_typ<'db>(
  db: &'db TypedownDatabase,
  callee: HirValue<'db>,
  args: Vec<HirValue<'db>>,
) -> TypResult<'db> {
  // Check if callee is a macro
  let resolved = referee(db, callee);
  if let Some(symbol) = resolved.value(db)
    && let SymbolKind::BuiltinMacro(kind) = symbol.kind(db)
  {
    return get_macro_call_typ(db, kind, args);
  }

  let callee_result = actual_node_typ(db, callee);
  let diagnostics = callee_result.diagnostics(db).clone();

  let callee_typ = match callee_result.typ(db) {
    Some(typ) => typ,
    None => return TypResult::new(db, None, diagnostics),
  };

  let arg_typs: Vec<TdTypEnum> = args
    .iter()
    .filter_map(|arg| actual_node_typ(db, *arg).typ(db))
    .collect();

  if let Some(signature) = callee_typ.call_typ(db, arg_typs) {
    return TypResult::new(db, Some(signature.ret(db)), diagnostics);
  }

  TypResult::new(db, None, diagnostics)
}

fn get_macro_call_typ<'db>(
  db: &'db TypedownDatabase,
  kind: BuiltinMacroKind,
  args: Vec<HirValue<'db>>,
) -> TypResult<'db> {
  match kind {
    BuiltinMacroKind::Fref => get_fref_typ(db, args),
  }
}

// fref("file.td") returns link[T] where T is the target file's schema type
fn get_fref_typ<'db>(db: &'db TypedownDatabase, args: Vec<HirValue<'db>>) -> TypResult<'db> {
  if args.len() != 1 {
    let node = args.first().map(|a| a.node(db));
    let (trimmed_offset, trimmed_len) = node.as_ref().map_or((0, 0), |n| n.trimmed_range());
    return TypResult::new(
      db,
      None,
      vec![Diagnostic::WrongArgCount {
        expected: 1,
        got: args.len(),
        start_offset: trimmed_offset,
        end_offset: trimmed_offset + trimmed_len,
      }],
    );
  }
  let arg = args[0];
  let node = arg.node(db);
  let (trimmed_offset, trimmed_len) = node.trimmed_range();
  let path_string = match arg.kind(db) {
    HirValueKind::String(value) => value,
    _ => {
      return TypResult::new(
        db,
        None,
        vec![Diagnostic::ArgTypMismatch {
          expected: "string".to_string(),
          start_offset: trimmed_offset,
          end_offset: trimmed_offset + trimmed_len,
        }],
      );
    }
  };

  let project = arg.project(db);
  let files = project.files(db);
  let root_dir = get_vault_config(db, project).root_dir(db);
  let file_dir = arg
    .node(db)
    .owner_file
    .handle(db)
    .path()
    .and_then(|p| p.parent().map(|d| d.to_path_buf()))
    .unwrap_or_else(|| root_dir.clone());
  let target_path = resolve_fref_path(&path_string, &file_dir, &root_dir);

  let target_file = match files.get(&target_path) {
    Some(file) => *file,
    None => {
      return TypResult::new(
        db,
        None,
        vec![Diagnostic::UnresolvedFileRef {
          path: path_string,
          start_offset: trimmed_offset,
          end_offset: trimmed_offset + trimmed_len,
        }],
      );
    }
  };
  let target_symbol = file_symbol(db, project, target_file);

  match target_symbol.value(db) {
    Some(symbol) => get_symbol_typ(db, symbol),
    None => TypResult::new(
      db,
      None,
      vec![Diagnostic::UnresolvedSchema {
        name: path_string,
        start_offset: trimmed_offset,
        end_offset: trimmed_offset + trimmed_len,
      }],
    ),
  }
}

// Helper to get the type of an index expression
fn get_index_typ<'db>(
  db: &'db TypedownDatabase,
  expr: HirValue<'db>,
  indices: Vec<HirValue<'db>>,
) -> TypResult<'db> {
  let expr_result = actual_node_typ(db, expr);
  let mut diagnostics = expr_result.diagnostics(db).clone();

  let expr_typ = match expr_result.typ(db) {
    Some(typ) => typ,
    None => return TypResult::new(db, None, diagnostics),
  };

  /* Generic instantiation */

  let expr_typ = if expr_typ.arity(db) == 0
    && let HirValueKind::Ident(_) = expr.kind(db)
    && let Some(symbol) = referee(db, expr).value(db)
    && let Some(typ) = evaluate_typ(db, symbol).typ(db)
    && typ.arity(db) > 0
  {
    typ
  } else {
    expr_typ
  };

  // Resolve each type argument and instantiate the generic type
  if expr_typ.arity(db) > 0 {
    let mut arg_typs = vec![];
    for index_hir in indices {
      let index_result = actual_node_typ(db, index_hir);
      diagnostics.extend(index_result.diagnostics(db).iter().cloned());
      match index_result.typ(db) {
        Some(typ) => arg_typs.push(typ),
        None => return TypResult::new(db, None, diagnostics),
      }
    }
    let instantiate_result =
      expr_typ.instantiate(db, arg_typs.into_iter().map(LazyTyp::eager).collect());
    diagnostics.extend(instantiate_result.diagnostics(db).iter().cloned());
    return TypResult::new(db, Some(instantiate_result.typ(db)), diagnostics);
  }

  let key_typ = indices
    .first()
    .and_then(|index| actual_node_typ(db, *index).typ(db))
    .unwrap_or_else(|| get_number_typ(db).into());

  if let Some(signature) = expr_typ.index_typ(db, &key_typ) {
    return TypResult::new(db, Some(signature.ret(db)), diagnostics);
  }

  TypResult::new(db, None, diagnostics)
}

// actual(closure) = fn(params from expected, return from actual(body))
fn get_closure_typ<'db>(
  db: &'db TypedownDatabase,
  hir: HirValue<'db>,
  params: Vec<String>,
  body: HirValue<'db>,
) -> TypResult<'db> {
  let expected = expected_node_typ(db, hir).typ(db);
  let param_typs = match expected {
    Some(TdTypEnum::TdFuncTyp(func)) => func.signature(db).params(db),
    _ => return TypResult::new(db, None, vec![]),
  };

  if param_typs.len() != params.len() {
    let node = hir.node(db);
    let (trimmed_offset, trimmed_len) = node.trimmed_range();
    return TypResult::new(
      db,
      None,
      vec![Diagnostic::WrongArgCount {
        expected: param_typs.len(),
        got: params.len(),
        start_offset: trimmed_offset,
        end_offset: trimmed_offset + trimmed_len,
      }],
    );
  }

  let body_result = actual_node_typ(db, body);
  let ret = match body_result.typ(db) {
    Some(typ) => typ,
    None => return TypResult::new(db, None, body_result.diagnostics(db).clone()),
  };

  let signature = FuncSignature::new(db, param_typs, ret);
  let func_typ = get_func_typ(db, signature);
  TypResult::new(
    db,
    Some(func_typ.into()),
    body_result.diagnostics(db).clone(),
  )
}

#[cfg(test)]
mod tests {
  use crate::db::derived::get_builtin_typs::{
    get_func_typ, get_lit_typ, get_schema_meta_typ, get_string_typ,
  };
  use crate::db::types::derived::obj_system::TdStaticTyp;
  use crate::db::types::{File, FileHandle, FileMetadata, LitValue, Project};
  use crate::db::types::{FuncSignature, TdTypEnum};
  use std::{collections::HashMap, path::PathBuf};

  use crate::db::{QueryStorage, TypedownDatabase, utils::lower_file};
  use crate::syntax::diagnostic::Diagnostic;

  use crate::db::{fixtures::load_vault_fixture, types::HirValueKind};

  use super::actual_node_typ;

  fn is_lit_string(db: &TypedownDatabase, typ: &TdTypEnum, expected: &str) -> bool {
    if let TdTypEnum::TdLitTyp(lit) = typ {
      return lit.value(db) == LitValue::String(expected.to_string());
    }
    false
  }

  fn is_lit_number(db: &TypedownDatabase, typ: &TdTypEnum, expected: &str) -> bool {
    if let TdTypEnum::TdLitTyp(lit) = typ {
      return lit.value(db) == LitValue::Number(expected.to_string());
    }
    false
  }

  fn is_lit_bool(db: &TypedownDatabase, typ: &TdTypEnum<'_>, expected: bool) -> bool {
    if let TdTypEnum::TdLitTyp(lit) = typ {
      return lit.value(db) == LitValue::Bool(expected);
    }
    false
  }

  fn vault_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/evaluate_schema/my_vault")
  }

  #[test]
  fn infer_anonymous_mapping_narrows_lit_fields() {
    let (db, project, file) = load_vault_fixture("typecheck/narrow_vault", "anonymous_mapping.td");
    let (hir, _) = lower_file(&db, project, file);
    let hir = hir.expect("should parse");
    let result_typ = actual_node_typ(&db, hir);
    let typ = result_typ.typ(&db).expect("should infer a type");
    let product = typ
      .as_td_product_typ()
      .expect("anonymous mapping should be Product");
    let fields = product.get_fields(&db);

    // String literal narrows to TdLiteralType(Str)
    let name_lazy = fields.get("name").expect("should have name field");
    let name_typ = name_lazy.resolve(&db).expect("should resolve");
    assert!(
      is_lit_string(&db, &name_typ, "Alice"),
      "name should be literal str \"Alice\""
    );

    // Num literal narrows to TdLiteralType(Num)
    let age_lazy = fields.get("age").expect("should have age field");
    let age_typ = age_lazy.resolve(&db).expect("should resolve");
    assert!(
      is_lit_number(&db, &age_typ, "30"),
      "age should be literal num \"30\""
    );

    // Bool literal narrows to TdLiteralType(Bool)
    let active_lazy = fields.get("active").expect("should have active field");
    let active_typ = active_lazy.resolve(&db).expect("should resolve");
    assert!(
      is_lit_bool(&db, &active_typ, true),
      "active should be literal bool true"
    );

    // Sequence ["a", 3] narrows to list[sum]
    let tags_lazy = fields.get("tags").expect("should have tags field");
    let tags_typ = tags_lazy.resolve(&db).expect("should resolve");
    let list = tags_typ
      .as_td_list_typ()
      .expect("tags should be a list type");
    let elem = list.element(&db).expect("list should have elem");
    let elem_typ = elem.resolve(&db).expect("elem should resolve");
    let sum = elem_typ.as_td_sum_typ().expect("elem should be a sum type");
    let members = sum.members(&db);
    assert_eq!(members.len(), 2, "tags should have 2 arms");
    let has_a = members.iter().any(|member| {
      member
        .resolve(&db)
        .is_some_and(|t| is_lit_string(&db, &t, "a"))
    });
    let has_3 = members.iter().any(|member| {
      member
        .resolve(&db)
        .is_some_and(|t| is_lit_number(&db, &t, "3"))
    });
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
    let result = actual_node_typ(&db, hir);

    let typ = result.typ(&db);
    let expected_typ = Some(TdTypEnum::from(get_schema_meta_typ(&db)));
    assert!(
      typ == expected_typ,
      "top-level mapping of a schema file should have schema type"
    );
    assert!(
      result.diagnostics(&db).is_empty(),
      "expected no diagnostics, got: {:?}",
      result.diagnostics(&db)
    );
  }

  #[test]
  fn actual_node_typ_string_lit_returns_lit() {
    let (db, project, file) = load_vault_fixture("typecheck/my_vault", "valid_person.td");
    let (hir, _) = lower_file(&db, project, file);
    let hir = hir.expect("should parse");
    // Get the "name" field value ("Alice")
    if let HirValueKind::Mapping(entries) = hir.kind(&db) {
      let name_hir = entries
        .iter()
        .find(|(key, _)| key == "name")
        .map(|(_, value)| *value);
      let name_hir = name_hir.expect("should have name field");
      let result_typ = actual_node_typ(&db, name_hir);
      let typ = result_typ.typ(&db).expect("should have a type");
      assert!(
        is_lit_string(&db, &typ, "Alice"),
        "string value should be literal str"
      );
    }
  }

  #[test]
  fn actual_node_typ_bool_returns_lit() {
    let (db, project, file) = load_vault_fixture("typecheck/narrow_vault", "anonymous_mapping.td");
    let (hir, _) = lower_file(&db, project, file);
    let hir = hir.expect("should parse");
    if let HirValueKind::Mapping(entries) = hir.kind(&db) {
      let active_hir = entries
        .iter()
        .find(|(key, _)| key == "active")
        .map(|(_, value)| *value);
      let active_hir = active_hir.expect("should have active field");
      let result_typ = actual_node_typ(&db, active_hir);
      let typ = result_typ.typ(&db).expect("should have a type");
      assert!(
        is_lit_bool(&db, &typ, true),
        "bool value should be literal bool"
      );
    }
  }

  #[test]
  fn actual_node_typ_sequence_returns_list_typ() {
    let (db, project, file) = load_vault_fixture("typecheck/narrow_vault", "anonymous_mapping.td");
    let (hir, _) = lower_file(&db, project, file);
    let hir = hir.expect("should parse");
    if let HirValueKind::Mapping(entries) = hir.kind(&db) {
      let tags_hir = entries
        .iter()
        .find(|(key, _)| key == "tags")
        .map(|(_, value)| *value);
      let tags_hir = tags_hir.expect("should have tags field");
      let result_typ = actual_node_typ(&db, tags_hir);
      let typ = result_typ.typ(&db).expect("should have a type");
      assert!(typ.is_td_list_typ(), "sequence should be a list type");
    }
  }

  // Date strings narrow to date type, not Literal
  #[test]
  fn actual_node_typ_date_string_returns_simple_date() {
    let (db, project, file) = load_vault_fixture("typecheck/my_vault", "valid_event.td");
    let (hir, _) = lower_file(&db, project, file);
    let hir = hir.expect("should parse");
    if let HirValueKind::Mapping(entries) = hir.kind(&db) {
      let date_hir = entries
        .iter()
        .find(|(key, _)| key == "date")
        .map(|(_, value)| *value);
      let date_hir = date_hir.expect("should have date field");
      let result_typ = actual_node_typ(&db, date_hir);
      let typ = result_typ.typ(&db).expect("should have a type");
      assert_eq!(
        typ.display_name(&db),
        "date",
        "ISO date string should resolve to date"
      );
    }
  }

  // Fref returns the resource's schema type, not type_type
  #[test]
  fn actual_node_typ_fref_returns_resource_typ() {
    let (db, project, file) =
      load_vault_fixture("typecheck/narrow_vault", "article_fref_status.td");
    let (hir, _) = lower_file(&db, project, file);
    let hir = hir.expect("should parse");
    if let HirValueKind::Mapping(entries) = hir.kind(&db) {
      // status: fref("summary.td").status
      let status_hir = entries
        .iter()
        .find(|(key, _)| key == "status")
        .map(|(_, value)| *value);
      let status_hir = status_hir.expect("should have status field");
      let result_typ = actual_node_typ(&db, status_hir);
      // Should resolve to something (not None), and not be type_type
      if let Some(typ) = result_typ.typ(&db) {
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
  fn actual_node_typ_num_returns_lit() {
    let (db, project, file) = load_vault_fixture("typecheck/my_vault", "valid_person.td");
    let (hir, _) = lower_file(&db, project, file);
    let hir = hir.expect("should parse");
    if let HirValueKind::Mapping(entries) = hir.kind(&db) {
      let age_hir = entries
        .iter()
        .find(|(key, _)| key == "age")
        .map(|(_, value)| *value);
      let age_hir = age_hir.expect("should have age field");
      let result_typ = actual_node_typ(&db, age_hir);
      let typ = result_typ.typ(&db).expect("should have a type");
      assert!(
        is_lit_number(&db, &typ, "30"),
        "number value should be literal num"
      );
    }
  }

  #[test]
  fn actual_node_typ_to_string_field_access_returns_func_type() {
    let (db, _, _) = load_vault_fixture("typecheck/my_vault", "valid_person.td");
    let number_lit: TdTypEnum<'_> = get_lit_typ(&db, LitValue::Number("42".to_string())).into();

    let field_typ = number_lit
      .lookup_field_typ(&db, "to_string")
      .expect("should have to_string method");
    let expected_signature = FuncSignature::new(&db, vec![], get_string_typ(&db).into());
    let expected_func_typ: TdTypEnum = get_func_typ(&db, expected_signature).into();

    assert_eq!(field_typ, expected_func_typ);
  }

  // fref("./peer.td") in a file inside subdir/ resolves relative to that file, not vault root
  #[test]
  fn actual_node_typ_relative_fref_resolves_from_file_directory() {
    let (db, project, file) =
      load_vault_fixture("typecheck/narrow_vault", "subdir/with_relative_fref.td");
    let (hir, _) = lower_file(&db, project, file);
    let hir = hir.expect("should parse");
    let related_hir = match hir.kind(&db) {
      HirValueKind::Mapping(entries) => entries
        .into_iter()
        .find(|(k, _)| k == "related")
        .map(|(_, v)| v)
        .expect("should have 'related' field"),
      _ => panic!("expected mapping"),
    };
    let result = actual_node_typ(&db, related_hir);
    let has_unresolved = result
      .diagnostics(&db)
      .iter()
      .any(|d| matches!(d, Diagnostic::UnresolvedFileRef { .. }));
    assert!(
      !has_unresolved,
      "fref(\"./peer.td\") should resolve relative to subdir/, not vault root: {:?}",
      result.diagnostics(&db)
    );
    assert!(
      result.typ(&db).is_some(),
      "fref(\"./peer.td\") should have a resolved type"
    );
  }

  #[test]
  fn actual_node_type_method_call_to_string_returns_string_type() {
    let (db, project, file) = load_vault_fixture("typecheck/my_vault", "method_call.td");
    let (hir, _) = lower_file(&db, project, file);
    let hir = hir.expect("should parse");
    if let HirValueKind::Mapping(entries) = hir.kind(&db) {
      let result_hir = entries
        .iter()
        .find(|(key, _)| key == "result")
        .map(|(_, value)| *value)
        .unwrap();
      let result = actual_node_typ(&db, result_hir);
      let typ = result.typ(&db).expect("should have a type");
      let string_typ: TdTypEnum = get_string_typ(&db).into();
      assert_eq!(typ, string_typ);
    }
  }
}
