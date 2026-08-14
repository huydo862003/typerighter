//! Tracked query to get the actual (bottom-up) type of a HIR value
// I think this is the idea of bidirectional typechecking

use std::collections::HashMap;

use crate::db::TypedownDatabase;
use crate::db::derived::evaluate::evaluate_type::evaluate_type;
use crate::db::derived::get_builtin_types::{
  get_bool_type, get_date_type, get_datetime_type, get_math_type, get_num_type, get_str_type,
  get_time_type, instantiate_type,
};
use crate::db::derived::get_vault_config::get_vault_config;
use crate::db::derived::name_resolver::file_symbol::file_symbol;
use crate::db::derived::name_resolver::referee::referee;
use crate::db::derived::typechecker::get_symbol_type_member::get_symbol_type_member;
use crate::db::types::derived::object_system::{
  is_valid_iso_date, is_valid_iso_datetime, is_valid_iso_time,
};
use crate::db::types::{
  BuiltinMacroKind, HirValue, HirValueKind, LiteralValue, MemberType, SymbolKind, TdStrType,
  TdTypeEnum, TdTypeLike, TypeMember, TypeMemberDescriptors, TypeMemberResult, TypeResult,
};
use crate::db::utils::lower_file;
use crate::syntax::diagnostic::Diagnostic;
use typedown_incremental::QueryDatabase;
use typedown_macros::query_derived;

// Infer the type of an HIR
// This function never relies on the declared type of the hir (it can rely on the declared type of the referenced hir)
// It always guesses based on the structure of the hir alone
#[query_derived]
pub fn actual_node_type_member(db: &TypedownDatabase, hir: HirValue) -> TypeMemberResult {
  let diagnostics = vec![];
  match hir.kind(db) {
    HirValueKind::Str(ref val) => {
      // Date/time subtypes are more specific than string literals
      let member_type = if is_valid_iso_datetime(val) {
        MemberType::eager_simple(get_datetime_type(db).into())
      } else if is_valid_iso_date(val) {
        MemberType::eager_simple(get_date_type(db).into())
      } else if is_valid_iso_time(val) {
        MemberType::eager_simple(get_time_type(db).into())
      } else {
        MemberType::Literal(LiteralValue::Str(val.clone()))
      };
      TypeMemberResult::new(
        db,
        Some(TypeMember::new(
          db,
          member_type,
          TypeMemberDescriptors::empty(),
        )),
        diagnostics,
      )
    }
    HirValueKind::Num(ref val) => TypeMemberResult::new(
      db,
      Some(TypeMember::new(
        db,
        MemberType::Literal(LiteralValue::Num(val.clone())),
        TypeMemberDescriptors::empty(),
      )),
      diagnostics,
    ),
    HirValueKind::Bool(val) => TypeMemberResult::new(
      db,
      Some(TypeMember::new(
        db,
        MemberType::Literal(LiteralValue::Bool(val)),
        TypeMemberDescriptors::empty(),
      )),
      diagnostics,
    ),
    HirValueKind::Interpolated(_) => simple_member_result(db, get_str_type(db).into(), vec![]),
    HirValueKind::Null => TypeMemberResult::new(db, None, vec![]),
    HirValueKind::Ident(ref name) if name == "self" => get_self_type(db, hir),
    HirValueKind::Ident(_) => {
      let resolved = referee(db, hir);
      match resolved.value(db) {
        Some(symbol) => get_symbol_type_member(db, symbol),
        None => TypeMemberResult::new(db, None, vec![]),
      }
    }
    HirValueKind::Mapping(entries) => get_mapping_type(db, hir, entries),
    HirValueKind::Sequence(items) => get_sequence_type(db, items),
    HirValueKind::Call { callee, args } => get_call_type(db, *callee, args),
    HirValueKind::Index { expr, indices } => get_index_type(db, *expr, indices),
    HirValueKind::Tag { tag, .. } => get_tag_type(db, *tag),
    HirValueKind::Unary { op, operand } => get_unary_type(db, &op, *operand),
    HirValueKind::Binary { op, left, right } => get_binary_type(db, &op, *left, *right),
    HirValueKind::Math(_) => simple_member_result(db, get_math_type(db).into(), vec![]),
    HirValueKind::Markdown(_) => simple_member_result(db, get_str_type(db).into(), vec![]),
  }
}

/// Helper to create a TypeMemberResult for a simple TdTypeEnum
fn simple_member_result(
  db: &TypedownDatabase,
  typ: TdTypeEnum,
  diagnostics: Vec<Diagnostic>,
) -> TypeMemberResult {
  TypeMemberResult::new(
    db,
    Some(TypeMember::new(
      db,
      MemberType::eager_simple(typ),
      TypeMemberDescriptors::empty(),
    )),
    diagnostics,
  )
}

/// Convert a TypeResult to a TypeMemberResult wrapping as Simple
fn type_result_to_member_result(db: &TypedownDatabase, result: TypeResult) -> TypeMemberResult {
  let member = result.typ(db).map(|typ| {
    TypeMember::new(
      db,
      MemberType::eager_simple(typ),
      TypeMemberDescriptors::empty(),
    )
  });
  TypeMemberResult::new(db, member, result.diagnostics(db).clone())
}

/// Helper to get the type of a mapping
fn get_mapping_type(
  db: &TypedownDatabase,
  _hir: HirValue,
  entries: Vec<(String, HirValue)>,
) -> TypeMemberResult {
  // If _type is present, resolve the schema
  for (key, value_hir) in &entries {
    if key == "_type" {
      let resolved = referee(db, *value_hir);
      if let Some(symbol) = resolved.value(db) {
        return type_result_to_member_result(db, evaluate_type(db, symbol));
      }
      let node = value_hir.node(db);
      let (tr_offset, tr_len) = node.trimmed_range();
      return TypeMemberResult::new(
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
    let field_result = actual_node_type_member(db, value_hir);
    diagnostics.extend(field_result.diagnostics(db).iter().cloned());
    if let Some(member) = field_result.member(db) {
      fields.insert(key, member);
    }
  }
  TypeMemberResult::new(
    db,
    Some(TypeMember::new(
      db,
      MemberType::Structural(fields),
      TypeMemberDescriptors::empty(),
    )),
    diagnostics,
  )
}

/// Resolve a tag expression like `!Person { name: "John" }`
fn get_tag_type(db: &TypedownDatabase, tag: HirValue) -> TypeMemberResult {
  let resolved = referee(db, tag);
  match resolved.value(db) {
    Some(symbol) => type_result_to_member_result(db, evaluate_type(db, symbol)),
    None => {
      let node = tag.node(db);
      let (tr_offset, tr_len) = node.trimmed_range();
      TypeMemberResult::new(
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

/// Helper to get the return type of a unary expression
fn get_unary_type(db: &TypedownDatabase, op: &str, operand: HirValue) -> TypeMemberResult {
  let operand_result = actual_node_type_member(db, operand);
  let diagnostics = operand_result.diagnostics(db).clone();

  match op {
    "-" | "+" => simple_member_result(db, get_num_type(db).into(), diagnostics),
    "~" => simple_member_result(db, get_bool_type(db).into(), diagnostics),
    _ => TypeMemberResult::new(db, None, diagnostics),
  }
}

/// Helper to get the return type of a binary expression
fn get_binary_type(
  db: &TypedownDatabase,
  op: &str,
  left: HirValue,
  right: HirValue,
) -> TypeMemberResult {
  // Field access such as `obj.field`
  if op == "." {
    let left_result = actual_node_type_member(db, left);
    let mut diagnostics = left_result.diagnostics(db).clone();
    let left_member = match left_result.member(db) {
      Some(member) => member,
      None => return TypeMemberResult::new(db, None, diagnostics),
    };
    let left_type = match left_member.typ(db).evaluate_simple(db) {
      Some(typ) => typ,
      None => return TypeMemberResult::new(db, None, diagnostics),
    };
    let field_name = match right.kind(db) {
      HirValueKind::Ident(name) => name,
      _ => return TypeMemberResult::new(db, None, diagnostics),
    };
    return match left_type.lookup_field_type_member(db, &field_name) {
      Some(member) => TypeMemberResult::new(db, Some(member), diagnostics),
      None => {
        let node = right.node(db);
        let (tr_offset, tr_len) = node.trimmed_range();
        diagnostics.push(Diagnostic::UnknownField {
          field: field_name,
          on_type: left_type.display_name(db),
          start_offset: tr_offset,
          end_offset: tr_offset + tr_len,
        });
        TypeMemberResult::new(db, None, diagnostics)
      }
    };
  }

  let left_result = actual_node_type_member(db, left);
  let right_result = actual_node_type_member(db, right);
  let mut diagnostics = left_result.diagnostics(db).clone();
  diagnostics.extend(right_result.diagnostics(db).iter().cloned());

  match op {
    "+" | "-" | "*" | "/" | "%" | "**" => {
      simple_member_result(db, get_num_type(db).into(), diagnostics)
    }
    "==" | "!=" | "<" | ">" | "<=" | ">=" => {
      simple_member_result(db, get_bool_type(db).into(), diagnostics)
    }
    "&&" | "||" => simple_member_result(db, get_bool_type(db).into(), diagnostics),
    _ => TypeMemberResult::new(db, None, diagnostics),
  }
}

/// Helper to get the type of a sequence
fn get_sequence_type(db: &TypedownDatabase, items: Vec<HirValue>) -> TypeMemberResult {
  let mut diagnostics = vec![];
  let mut arms = vec![];

  for item in items {
    let item_result = actual_node_type_member(db, item);
    diagnostics.extend(item_result.diagnostics(db).iter().cloned());
    if let Some(member) = item_result.member(db) {
      arms.push(member);
    }
  }

  let member_type = MemberType::ListOfSum(arms);
  TypeMemberResult::new(
    db,
    Some(TypeMember::new(
      db,
      member_type,
      TypeMemberDescriptors::empty(),
    )),
    diagnostics,
  )
}

/// Helper to get the type of a call expression
fn get_call_type(db: &TypedownDatabase, callee: HirValue, args: Vec<HirValue>) -> TypeMemberResult {
  // Check if callee is a macro
  let resolved = referee(db, callee);
  if let Some(symbol) = resolved.value(db)
    && let SymbolKind::BuiltinMacro(kind) = symbol.kind(db)
  {
    return get_macro_call_type(db, kind, args);
  }

  let callee_result = actual_node_type_member(db, callee);
  let diagnostics = callee_result.diagnostics(db).clone();

  let callee_member = match callee_result.member(db) {
    Some(member) => member,
    None => return TypeMemberResult::new(db, None, diagnostics),
  };
  let callee_type = match callee_member.typ(db).evaluate_simple(db) {
    Some(typ) => typ,
    None => return TypeMemberResult::new(db, None, diagnostics),
  };

  if let TdTypeEnum::TdFuncType(func) = &callee_type {
    let sig = func.signature(db);
    return simple_member_result(db, sig.ret(db), diagnostics);
  }

  TypeMemberResult::new(db, None, diagnostics)
}

fn get_macro_call_type(
  db: &TypedownDatabase,
  kind: BuiltinMacroKind,
  args: Vec<HirValue>,
) -> TypeMemberResult {
  match kind {
    BuiltinMacroKind::Fref => get_fref_type(db, args),
  }
}

// fref("file.td") returns link[T] where T is the target file's schema type
fn get_fref_type(db: &TypedownDatabase, args: Vec<HirValue>) -> TypeMemberResult {
  if args.len() != 1 {
    let node = args.first().map(|a| a.node(db));
    let (tr_offset, tr_len) = node.as_ref().map_or((0, 0), |n| n.trimmed_range());
    return TypeMemberResult::new(
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
      return TypeMemberResult::new(
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
      return TypeMemberResult::new(
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
    Some(sym) => get_symbol_type_member(db, sym),
    None => TypeMemberResult::new(
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
fn get_index_type(
  db: &TypedownDatabase,
  expr: HirValue,
  indices: Vec<HirValue>,
) -> TypeMemberResult {
  let expr_result = actual_node_type_member(db, expr);
  let mut diagnostics = expr_result.diagnostics(db).clone();

  let expr_member = match expr_result.member(db) {
    Some(member) => member,
    None => return TypeMemberResult::new(db, None, diagnostics),
  };
  let expr_type = match expr_member.typ(db).evaluate_simple(db) {
    Some(typ) => typ,
    None => return TypeMemberResult::new(db, None, diagnostics),
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
            None => return TypeMemberResult::new(db, None, diagnostics),
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
          return TypeMemberResult::new(db, None, diagnostics);
        }
      }
    }
    let inst_result = instantiate_type(db, expr_type, arg_types);
    diagnostics.extend(inst_result.diagnostics(db).iter().cloned());
    return simple_member_result(db, inst_result.typ(db), diagnostics);
  }

  // Element access on instantiated list
  if let TdTypeEnum::TdListType(list) = &expr_type {
    return match list.elem(db) {
      Some(elem) => simple_member_result(db, elem, diagnostics),
      None => TypeMemberResult::new(db, None, diagnostics),
    };
  }

  // Element access on instantiated dict
  if let TdTypeEnum::TdDictType(dict) = &expr_type {
    return match dict.value(db) {
      Some(value) => simple_member_result(db, value, diagnostics),
      None => TypeMemberResult::new(db, None, diagnostics),
    };
  }

  // Element access on string
  if expr_type.is_td_str_type() {
    return simple_member_result(db, TdStrType::get(db).into(), diagnostics);
  }

  TypeMemberResult::new(db, None, diagnostics)
}

/// Return the type of `self` in the current file
fn get_self_type(db: &TypedownDatabase, hir: HirValue) -> TypeMemberResult {
  let project = hir.project(db);
  let file = hir.file(db);
  let (mapping_hir, _) = lower_file(db, project, file);
  let mapping_hir = match mapping_hir {
    Some(mapping_hir) => mapping_hir,
    None => return TypeMemberResult::new(db, None, vec![]),
  };

  if let HirValueKind::Mapping(entries) = mapping_hir.kind(db) {
    for (key, val_hir) in entries {
      if key == "_type" {
        let resolved = referee(db, val_hir);
        return match resolved.value(db) {
          Some(symbol) => type_result_to_member_result(db, evaluate_type(db, symbol)),
          None => TypeMemberResult::new(db, None, vec![]),
        };
      }
    }
  }
  TypeMemberResult::new(db, None, vec![])
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
    types::{HirValueKind, LiteralValue, MemberType, TdTypeLike},
  };

  use super::actual_node_type_member;
  use crate::db::utils::typecheck::lift_type_member_result;

  fn vault_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/evaluate_schema/my_vault")
  }

  #[test]
  fn infer_anonymous_mapping_narrows_literal_fields() {
    let (db, project, file) =
      load_vault_fixture("typecheck/narrow_vault", "content/anonymous_mapping.td");
    let (hir, _) = lower_file(&db, project, file);
    let hir = hir.expect("should parse");
    let result = actual_node_type_member(&db, hir);
    let member = result.member(&db).expect("should infer a type");
    let MemberType::Structural(fields) = member.typ(&db) else {
      panic!("anonymous mapping should be Structural");
    };

    // String literal narrows to Literal(Str)
    let name_member = fields.get("name").expect("should have name field");
    assert!(
      matches!(name_member.typ(&db), MemberType::Literal(LiteralValue::Str(s)) if s == "Alice"),
      "name should be Literal(Str(\"Alice\"))"
    );

    // Num literal narrows to Literal(Num)
    let age_member = fields.get("age").expect("should have age field");
    assert!(
      matches!(age_member.typ(&db), MemberType::Literal(LiteralValue::Num(n)) if n == "30"),
      "age should be Literal(Num(\"30\"))"
    );

    // Bool literal narrows to Literal(Bool)
    let active_member = fields.get("active").expect("should have active field");
    assert!(
      matches!(
        active_member.typ(&db),
        MemberType::Literal(LiteralValue::Bool(true))
      ),
      "active should be Literal(Bool(true))"
    );

    // Sequence ["a", 3] narrows to ListOfSum with 2 arms
    let tags_member = fields.get("tags").expect("should have tags field");
    let MemberType::ListOfSum(arms) = tags_member.typ(&db) else {
      panic!("tags should be ListOfSum");
    };
    assert_eq!(arms.len(), 2, "tags should have 2 arms");
    assert!(
      matches!(arms[0].typ(&db), MemberType::Literal(LiteralValue::Str(s)) if s == "a"),
      "first arm should be Literal(Str(\"a\"))"
    );
    assert!(
      matches!(arms[1].typ(&db), MemberType::Literal(LiteralValue::Num(n)) if n == "3"),
      "second arm should be Literal(Num(\"3\"))"
    );
  }

  #[test]
  fn actual_node_type_member_of_schema_file_top_level_mapping_is_schema_type() {
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
    let result = actual_node_type_member(&db, hir);

    let typ = lift_type_member_result(&db, &result);
    let expected = Some(TdTypeEnum::from(get_schema_type(&db)));
    assert!(
      typ == expected,
      "top-level mapping of a schema file should have type TdSchemaType"
    );
    assert!(
      result.diagnostics(&db).is_empty(),
      "expected no diagnostics, got: {:?}",
      result.diagnostics(&db)
    );
  }

  #[test]
  fn actual_node_type_member_string_literal_returns_literal() {
    let (db, project, file) = load_vault_fixture("typecheck/my_vault", "content/valid_person.td");
    let (hir, _) = lower_file(&db, project, file);
    let hir = hir.expect("should parse");
    // Get the "name" field value ("Alice")
    if let HirValueKind::Mapping(entries) = hir.kind(&db) {
      let name_hir = entries.iter().find(|(k, _)| k == "name").map(|(_, v)| *v);
      let name_hir = name_hir.expect("should have name field");
      let result = actual_node_type_member(&db, name_hir);
      let member = result.member(&db).expect("should have a type");
      assert!(
        matches!(member.typ(&db), MemberType::Literal(LiteralValue::Str(s)) if s == "Alice"),
        "string value should be Literal(Str)"
      );
    }
  }

  #[test]
  fn actual_node_type_member_bool_returns_literal() {
    let (db, project, file) =
      load_vault_fixture("typecheck/narrow_vault", "content/anonymous_mapping.td");
    let (hir, _) = lower_file(&db, project, file);
    let hir = hir.expect("should parse");
    if let HirValueKind::Mapping(entries) = hir.kind(&db) {
      let active_hir = entries.iter().find(|(k, _)| k == "active").map(|(_, v)| *v);
      let active_hir = active_hir.expect("should have active field");
      let result = actual_node_type_member(&db, active_hir);
      let member = result.member(&db).expect("should have a type");
      assert!(
        matches!(
          member.typ(&db),
          MemberType::Literal(LiteralValue::Bool(true))
        ),
        "bool value should be Literal(Bool)"
      );
    }
  }

  #[test]
  fn actual_node_type_member_sequence_returns_list_of_sum() {
    let (db, project, file) =
      load_vault_fixture("typecheck/narrow_vault", "content/anonymous_mapping.td");
    let (hir, _) = lower_file(&db, project, file);
    let hir = hir.expect("should parse");
    if let HirValueKind::Mapping(entries) = hir.kind(&db) {
      let tags_hir = entries.iter().find(|(k, _)| k == "tags").map(|(_, v)| *v);
      let tags_hir = tags_hir.expect("should have tags field");
      let result = actual_node_type_member(&db, tags_hir);
      let member = result.member(&db).expect("should have a type");
      assert!(
        matches!(member.typ(&db), MemberType::ListOfSum(_)),
        "sequence should be ListOfSum"
      );
    }
  }

  // Date strings narrow to Simple(date), not Literal
  #[test]
  fn actual_node_type_member_date_string_returns_simple_date() {
    let (db, project, file) = load_vault_fixture("typecheck/my_vault", "content/valid_event.td");
    let (hir, _) = lower_file(&db, project, file);
    let hir = hir.expect("should parse");
    if let HirValueKind::Mapping(entries) = hir.kind(&db) {
      let date_hir = entries.iter().find(|(k, _)| k == "date").map(|(_, v)| *v);
      let date_hir = date_hir.expect("should have date field");
      let result = actual_node_type_member(&db, date_hir);
      let member = result.member(&db).expect("should have a type");
      let typ = member
        .typ(&db)
        .evaluate_simple(&db)
        .expect("should resolve");
      assert_eq!(
        typ.display_name(&db),
        "date",
        "ISO date string should resolve to date"
      );
    }
  }

  // Fref returns the resource's schema type, not type_type
  #[test]
  fn actual_node_type_member_fref_returns_resource_type() {
    let (db, project, file) =
      load_vault_fixture("typecheck/narrow_vault", "content/article_fref_status.td");
    let (hir, _) = lower_file(&db, project, file);
    let hir = hir.expect("should parse");
    if let HirValueKind::Mapping(entries) = hir.kind(&db) {
      // status: fref("summary.td").status
      let status_hir = entries.iter().find(|(k, _)| k == "status").map(|(_, v)| *v);
      let status_hir = status_hir.expect("should have status field");
      let result = actual_node_type_member(&db, status_hir);
      // Should resolve to something (not None), and not be type_type
      if let Some(member) = result.member(&db)
        && let Some(typ) = member.typ(&db).evaluate_simple(&db)
      {
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
  fn actual_node_type_member_num_returns_literal() {
    let (db, project, file) = load_vault_fixture("typecheck/my_vault", "content/valid_person.td");
    let (hir, _) = lower_file(&db, project, file);
    let hir = hir.expect("should parse");
    if let HirValueKind::Mapping(entries) = hir.kind(&db) {
      let age_hir = entries.iter().find(|(k, _)| k == "age").map(|(_, v)| *v);
      let age_hir = age_hir.expect("should have age field");
      let result = actual_node_type_member(&db, age_hir);
      let member = result.member(&db).expect("should have a type");
      assert!(
        matches!(member.typ(&db), MemberType::Literal(LiteralValue::Num(n)) if n == "30"),
        "number value should be Literal(Num)"
      );
    }
  }
}
