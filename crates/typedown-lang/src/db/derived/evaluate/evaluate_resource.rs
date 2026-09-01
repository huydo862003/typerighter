//! Evaluate a resource file into typed objects

use std::collections::HashMap;

use typedown_macros::query_derived;

use crate::db::TypedownDatabase;
use crate::db::derived::evaluate::evaluate_node::evaluate_node;
use crate::db::types::{
  ResourceResult, Symbol, SymbolKind, TdBlobObj, TdObjectEnum, TdProductObj, TdProductType,
  TdSchemaObj,
};
use crate::db::utils::{is_schemaless_file, lower_file};
use typedown_incremental::QueryDatabase;

use crate::db::derived::name_resolver::scope::get_file_runtime_scope;

#[query_derived]
pub fn evaluate_resource<'db>(
  db: &'db TypedownDatabase,
  symbol: Symbol<'db>,
) -> ResourceResult<'db> {
  if let SymbolKind::Asset(asset_kind, _project, file) = symbol.kind(db) {
    let blob = TdBlobObj::new(db, asset_kind, file);
    return ResourceResult::new(db, Some(blob.into()), vec![]);
  }

  let (project, file) = match symbol.kind(db) {
    SymbolKind::UserDefinedResource(project, file) => (project, file),
    _ => return ResourceResult::new(db, None, vec![]),
  };

  let (hir, mut diagnostics) = lower_file(db, project, file);
  let hir = match hir {
    Some(hir) => hir,
    None => return ResourceResult::new(db, None, diagnostics),
  };

  let file_scope = get_file_runtime_scope(db, project, file);
  let node_result = evaluate_node(db, hir, file_scope);
  diagnostics.extend(node_result.diagnostics(db).iter().cloned());

  let is_schemaless = is_schemaless_file(db, project, file);

  // Stamp the file symbol so serialization can detect fref origins
  let value = match node_result.value(db) {
    Some(TdObjectEnum::TdSchemaObj(obj)) => Some(
      TdSchemaObj::new(
        db,
        obj.schema(db),
        project,
        Some(symbol),
        obj.builtins(db),
        obj.fields(db),
      )
      .into(),
    ),
    Some(TdObjectEnum::TdProductObj(obj)) => {
      let file_sym = if is_schemaless { Some(symbol) } else { None };
      Some(
        TdProductObj::new(
          db,
          obj.product_type(db),
          file_sym,
          obj.builtins(db),
          obj.fields(db),
        )
        .into(),
      )
    }
    // Schemaless files with no type produce a DictObj, convert to ProductObj
    Some(TdObjectEnum::TdDictObj(dict)) if is_schemaless => {
      let mut builtins = HashMap::new();
      let mut fields = HashMap::new();
      for (key, val) in dict.entries(db) {
        if key.starts_with('_') {
          builtins.insert(key, val);
        } else {
          fields.insert(key, val);
        }
      }
      Some(
        TdProductObj::new(
          db,
          TdProductType::new(db, None, HashMap::new()).into(),
          Some(symbol),
          builtins,
          fields,
        )
        .into(),
      )
    }
    other => other,
  };

  ResourceResult::new(db, value, diagnostics)
}

#[cfg(test)]
mod tests {
  use std::path::PathBuf;

  use crate::db::types::{
    AssetKind, File, FileHandle, FileMetadata, Project, Symbol, SymbolKind, TdObjectEnum,
    TdRuntimeObject,
  };
  use crate::syntax::diagnostic::Diagnostic;

  use crate::db::{
    QueryStorage, TypedownDatabase,
    derived::evaluate::evaluate_node::evaluate_node,
    derived::evaluate::evaluate_resource::evaluate_resource,
    derived::name_resolver::file_symbol::file_symbol,
    derived::name_resolver::scope::get_file_runtime_scope,
    derived::typechecker::typecheck::typecheck,
    fixtures::{load_vault_fixture, make_blob_obj},
    types::HirValueKind,
    utils::lower_file,
  };

  // A valid resource with _type produces an object with the declared fields
  #[test]
  fn evaluate_resource_valid_person() {
    let (db, project, file) = load_vault_fixture("evaluate/my_vault", "valid_person.td");
    let symbol = file_symbol(&db, project, file)
      .value(&db)
      .expect("file_symbol should return a resource symbol");

    let result = evaluate_resource(&db, symbol);
    assert!(
      result.value(&db).is_some(),
      "should produce an object, diagnostics: {:?}",
      result.diagnostics(&db)
    );
    let obj = result.value(&db).unwrap();
    let name_obj = obj.get_owned_field(&db, "name").expect("should have name");
    let name_str = name_obj.as_td_str_obj().expect("expected TdStrObj");
    assert_eq!(name_str.value(&db), "Alice");
  }

  // A field value that doesn't match the declared schema type produces diagnostics
  #[test]
  fn evaluate_resource_wrong_type_has_diagnostics() {
    let (db, project, file) = load_vault_fixture("evaluate/my_vault", "wrong_field_type.td");
    let symbol = file_symbol(&db, project, file)
      .value(&db)
      .expect("file_symbol should return a resource symbol");

    let result = evaluate_resource(&db, symbol);
    assert!(
      !result.diagnostics(&db).is_empty(),
      "should have diagnostics for wrong field type"
    );
  }

  // A schema file placed outside _types is treated as a resource, not a schema, but evaluate_resource still produces a value (a TdProductType)
  #[test]
  fn schema_in_root_dir_is_resource() {
    let (db, project, file) = load_vault_fixture("evaluate/my_vault", "schema_in_content.td");
    let symbol = file_symbol(&db, project, file)
      .value(&db)
      .expect("file_symbol should return a symbol");

    assert!(
      symbol.kind(&db).is_resource(),
      "schema file in content dir should be a resource symbol"
    );

    let result = evaluate_resource(&db, symbol);
    assert!(
      result.value(&db).is_some(),
      "should produce an object, diagnostics: {:?}",
      result.diagnostics(&db)
    );
    let obj = result.value(&db).unwrap();
    let schema_type = obj
      .as_td_type_obj()
      .and_then(|t| t.as_td_schema_type())
      .expect("expected TdSchemaType");
    assert!(
      schema_type.fields(&db).contains_key("title"),
      "should have title field"
    );
  }

  // Circular fref does not cause infinite recursion due to lazy evaluation
  #[test]
  fn circular_fref_does_not_panic() {
    let (db, project, file) = load_vault_fixture("evaluate/my_vault", "circular_a.td");
    let symbol = file_symbol(&db, project, file)
      .value(&db)
      .expect("should return a resource symbol");

    let result = evaluate_resource(&db, symbol);
    assert!(
      result.value(&db).is_some(),
      "circular fref should still produce an object"
    );

    // Access a non-fref field to verify the object works
    let obj = result.value(&db).unwrap();
    let name_obj = obj.get_owned_field(&db, "name").expect("should have name");
    let name_str = name_obj.as_td_str_obj().expect("expected TdStrObj");
    assert_eq!(name_str.value(&db), "Alice");
  }

  // Lazy field access: accessing the fref field evaluates the target on both sides
  #[test]
  fn lazy_fref_field_access() {
    let (db, project, file_a) = load_vault_fixture("evaluate/my_vault", "circular_a.td");
    let symbol_a = file_symbol(&db, project, file_a)
      .value(&db)
      .expect("should return a resource symbol");

    let result_a = evaluate_resource(&db, symbol_a);
    let alice = result_a.value(&db).unwrap();

    // Alice -> friend -> Bob
    let friend = alice
      .get_owned_field(&db, "friend")
      .expect("should have friend");
    let friend_name = friend
      .get_owned_field(&db, "name")
      .expect("friend should have name");
    let friend_name_str = friend_name.as_td_str_obj().expect("expected TdStrObj");
    assert_eq!(friend_name_str.value(&db), "Bob");

    // Bob -> friend -> Alice (circular, should not panic)
    let friend_of_friend = friend
      .get_owned_field(&db, "friend")
      .expect("Bob should have friend");
    let fof_name = friend_of_friend
      .get_owned_field(&db, "name")
      .expect("should have name");
    let fof_name_str = fof_name.as_td_str_obj().expect("expected TdStrObj");
    assert_eq!(fof_name_str.value(&db), "Alice");
  }

  // str.to_string() returns the same string value
  #[test]
  fn str_to_string_produces_same_value() {
    let (db, project, file) = load_vault_fixture("evaluate/my_vault", "str_method_call.td");
    let symbol = file_symbol(&db, project, file)
      .value(&db)
      .expect("file_symbol should return a resource symbol");
    let result = evaluate_resource(&db, symbol);
    let obj = result.value(&db).expect("should produce an object");
    let result_field = obj
      .get_owned_field(&db, "result")
      .expect("should have result field");
    let str_obj = result_field.as_td_str_obj().expect("expected TdStrObj");
    assert_eq!(str_obj.value(&db), "hello");
  }

  // num.to_string() returns the decimal representation, without trailing .0 for integers
  #[test]
  fn num_to_string_produces_string_repr() {
    let (db, project, file) = load_vault_fixture("evaluate/my_vault", "num_method_call.td");
    let symbol = file_symbol(&db, project, file)
      .value(&db)
      .expect("file_symbol should return a resource symbol");
    let result = evaluate_resource(&db, symbol);
    let obj = result.value(&db).expect("should produce an object");
    let result_field = obj
      .get_owned_field(&db, "result")
      .expect("should have result field");
    let str_obj = result_field.as_td_str_obj().expect("expected TdStrObj");
    assert_eq!(str_obj.value(&db), "42");
  }

  // bool.to_string() returns "true" or "false"
  #[test]
  fn bool_to_string_produces_string_repr() {
    let (db, project, file) = load_vault_fixture("evaluate/my_vault", "bool_method_call.td");
    let symbol = file_symbol(&db, project, file)
      .value(&db)
      .expect("file_symbol should return a resource symbol");
    let result = evaluate_resource(&db, symbol);
    let obj = result.value(&db).expect("should produce an object");
    let result_field = obj
      .get_owned_field(&db, "result")
      .expect("should have result field");
    let str_obj = result_field.as_td_str_obj().expect("expected TdStrObj");
    assert_eq!(str_obj.value(&db), "true");
  }

  #[test]
  fn evaluate_resource_with_computed_field() {
    let (db, project, file) = load_vault_fixture("evaluate/my_vault", "computed_resource.td");
    let symbol = file_symbol(&db, project, file)
      .value(&db)
      .expect("file_symbol should return a resource symbol");
    let result = evaluate_resource(&db, symbol);
    let obj = result.value(&db).expect("should produce an object");
    let full_name = obj
      .get_owned_field(&db, "fullName")
      .expect("should evaluate computed fullName field");
    let str_obj = full_name.as_td_str_obj().expect("expected TdStrObj");
    assert_eq!(str_obj.value(&db), "Alice Smith");
  }

  // fref("file.td").prop evaluates the referenced resource and accesses a field on it
  #[test]
  fn fref_prop_accesses_field_on_referenced_resource() {
    let (db, project, file) = load_vault_fixture("evaluate/my_vault", "fref_prop.td");
    let symbol = file_symbol(&db, project, file)
      .value(&db)
      .expect("file_symbol should return a resource symbol");
    let result = evaluate_resource(&db, symbol);
    let obj = result.value(&db).expect("should produce an object");
    let result_field = obj
      .get_owned_field(&db, "result")
      .expect("should have result field");
    let str_obj = result_field.as_td_str_obj().expect("expected TdStrObj");
    assert_eq!(str_obj.value(&db), "Alice");
  }

  // self.field accesses a field on the current resource object
  #[test]
  fn self_ref_accesses_own_field() {
    let (db, project, file) = load_vault_fixture("evaluate/my_vault", "self_ref.td");
    let symbol = file_symbol(&db, project, file)
      .value(&db)
      .expect("file_symbol should return a resource symbol");
    let result = evaluate_resource(&db, symbol);
    let obj = result.value(&db).expect("should produce an object");
    let result_field = obj
      .get_owned_field(&db, "result")
      .expect("should have result field");
    let str_obj = result_field.as_td_str_obj().expect("expected TdStrObj");
    assert_eq!(str_obj.value(&db), "Alice");
  }

  // String interpolation evaluates embedded expressions and concatenates the parts
  #[test]
  fn str_interp_evaluates_expr_parts() {
    let (db, project, file) = load_vault_fixture("evaluate/my_vault", "str_interp.td");
    let symbol = file_symbol(&db, project, file)
      .value(&db)
      .expect("file_symbol should return a resource symbol");
    let result = evaluate_resource(&db, symbol);
    let obj = result.value(&db).expect("should produce an object");
    let result_field = obj
      .get_owned_field(&db, "result")
      .expect("should have result field");
    let str_obj = result_field.as_td_str_obj().expect("expected TdStrObj");
    assert_eq!(str_obj.value(&db), "hello 42");
  }

  fn get_num_field(db: &TypedownDatabase, obj: &TdObjectEnum, field: &str) -> f64 {
    let field_obj = obj.get_owned_field(db, field).expect("should have field");
    let num = field_obj.as_td_num_obj().expect("should be TdNumObj");
    num.value(db)
  }

  fn get_bool_field(db: &TypedownDatabase, obj: &TdObjectEnum, field: &str) -> bool {
    let field_obj = obj.get_owned_field(db, field).expect("should have field");
    let b = field_obj.as_td_bool_obj().expect("should be TdBoolObj");
    b.value(db)
  }

  // 1 + 2 evaluates to 3
  #[test]
  fn binary_add_evaluates_to_sum() {
    let (db, project, file) = load_vault_fixture("evaluate/my_vault", "binary_valid.td");
    let symbol = file_symbol(&db, project, file).value(&db).unwrap();
    let obj = evaluate_resource(&db, symbol).value(&db).unwrap();
    assert_eq!(get_num_field(&db, &obj, "result"), 3.0);
  }

  // -, *, /, %, ** all produce the expected numeric result
  #[test]
  fn arithmetic_ops_evaluate_correctly() {
    let (db, project, file) = load_vault_fixture("evaluate/my_vault", "arithmetic_ops.td");
    let symbol = file_symbol(&db, project, file).value(&db).unwrap();
    let obj = evaluate_resource(&db, symbol).value(&db).unwrap();
    assert_eq!(get_num_field(&db, &obj, "sub"), 7.0);
    assert_eq!(get_num_field(&db, &obj, "mul"), 12.0);
    assert_eq!(get_num_field(&db, &obj, "div"), 2.5);
    assert_eq!(get_num_field(&db, &obj, "mod"), 1.0);
    assert_eq!(get_num_field(&db, &obj, "pow"), 256.0);
  }

  // Unary - negates the number
  #[test]
  fn unary_negation_evaluates_correctly() {
    let (db, project, file) = load_vault_fixture("evaluate/my_vault", "unary_valid.td");
    let symbol = file_symbol(&db, project, file).value(&db).unwrap();
    let obj = evaluate_resource(&db, symbol).value(&db).unwrap();
    assert_eq!(get_num_field(&db, &obj, "result"), -42.0);
  }

  // <, >, ==, !=, <=, >= all produce bool results for numeric operands
  #[test]
  fn comparison_ops_evaluate_correctly() {
    let (db, project, file) = load_vault_fixture("evaluate/my_vault", "comparison_ops.td");
    let symbol = file_symbol(&db, project, file).value(&db).unwrap();
    let obj = evaluate_resource(&db, symbol).value(&db).unwrap();
    assert!(get_bool_field(&db, &obj, "lt"));
    assert!(get_bool_field(&db, &obj, "gt"));
    assert!(get_bool_field(&db, &obj, "eq"));
    assert!(get_bool_field(&db, &obj, "ne"));
    assert!(get_bool_field(&db, &obj, "le"));
    assert!(get_bool_field(&db, &obj, "ge"));
  }

  // && and || produce the expected bool result
  #[test]
  fn logical_ops_evaluate_correctly() {
    let (db, project, file) = load_vault_fixture("evaluate/my_vault", "logical_ops.td");
    let symbol = file_symbol(&db, project, file).value(&db).unwrap();
    let obj = evaluate_resource(&db, symbol).value(&db).unwrap();
    assert!(!get_bool_field(&db, &obj, "and_false"));
    assert!(get_bool_field(&db, &obj, "or_true"));
  }

  // Unary + is identity; ~ is logical not (falsy: null/false; truthy: everything else)
  #[test]
  fn unary_extras_evaluate_correctly() {
    let (db, project, file) = load_vault_fixture("evaluate/my_vault", "unary_extras.td");
    let symbol = file_symbol(&db, project, file).value(&db).unwrap();
    let obj = evaluate_resource(&db, symbol).value(&db).unwrap();
    assert_eq!(get_num_field(&db, &obj, "pos"), 5.0);
    assert!(get_bool_field(&db, &obj, "logical_not_false"));
    assert!(!get_bool_field(&db, &obj, "logical_not_true"));
    assert!(!get_bool_field(&db, &obj, "logical_not_num"));
  }

  // String comparison operators work lexicographically
  #[test]
  fn str_comparison_evaluates_correctly() {
    let (db, project, file) = load_vault_fixture("evaluate/my_vault", "str_comparison.td");
    let symbol = file_symbol(&db, project, file).value(&db).unwrap();
    let obj = evaluate_resource(&db, symbol).value(&db).unwrap();
    assert!(get_bool_field(&db, &obj, "eq"));
    assert!(get_bool_field(&db, &obj, "ne"));
    assert!(get_bool_field(&db, &obj, "lt"));
  }

  // list[n] evaluates the list and returns the nth element
  #[test]
  fn list_index_evaluates_correctly() {
    let (db, project, file) = load_vault_fixture("evaluate/my_vault", "list_index.td");
    let symbol = file_symbol(&db, project, file).value(&db).unwrap();
    let obj = evaluate_resource(&db, symbol).value(&db).unwrap();
    assert_eq!(get_num_field(&db, &obj, "result"), 20.0);
  }

  // out-of-bounds index on list and string evaluates to undefined and emits a diagnostic
  #[test]
  fn index_out_of_bounds_emits_diagnostic() {
    let (db, project, file) = load_vault_fixture("evaluate/my_vault", "index_oob.td");
    let (hir, _) = lower_file(&db, project, file);
    let hir = hir.unwrap();

    // Extract field HIRs from the top-level mapping
    let HirValueKind::Mapping(entries) = hir.kind(&db) else {
      panic!("expected mapping at top level");
    };
    let field_hirs: std::collections::HashMap<_, _> = entries.into_iter().collect();

    let file_scope = get_file_runtime_scope(&db, project, file);
    let list_result = evaluate_node(&db, field_hirs["list_oob"], file_scope);
    assert!(
      list_result.value(&db).is_none(),
      "list OOB should be undefined"
    );
    assert!(
      list_result.diagnostics(&db).iter().any(|d| matches!(
        d,
        Diagnostic::IndexOutOfBounds {
          index: 99,
          length: 3,
          ..
        }
      )),
      "expected IndexOutOfBounds(99, 3) for list, got: {:?}",
      list_result.diagnostics(&db)
    );

    let str_result = evaluate_node(&db, field_hirs["str_oob"], file_scope);
    assert!(
      str_result.value(&db).is_none(),
      "string OOB should be undefined"
    );
    assert!(
      str_result.diagnostics(&db).iter().any(|d| matches!(
        d,
        Diagnostic::IndexOutOfBounds {
          index: 99,
          length: 5,
          ..
        }
      )),
      "expected IndexOutOfBounds(99, 5) for string, got: {:?}",
      str_result.diagnostics(&db)
    );
  }

  // string[n] returns the nth character as a string
  #[test]
  fn str_index_evaluates_correctly() {
    let (db, project, file) = load_vault_fixture("evaluate/my_vault", "str_index.td");
    let symbol = file_symbol(&db, project, file).value(&db).unwrap();
    let obj = evaluate_resource(&db, symbol).value(&db).unwrap();
    let result = obj.get_owned_field(&db, "result").unwrap();
    let str_obj = result.as_td_str_obj().expect("expected TdStrObj");
    assert_eq!(str_obj.value(&db), "e");
  }

  // Tag expressions like !str "Alice" strip the tag and evaluate the inner value
  #[test]
  fn tag_expr_strips_tag_and_evaluates_inner() {
    let (db, project, file) = load_vault_fixture("evaluate/my_vault", "tag_expr.td");
    let symbol = file_symbol(&db, project, file).value(&db).unwrap();
    let obj = evaluate_resource(&db, symbol).value(&db).unwrap();
    let name = obj.get_owned_field(&db, "name").unwrap();
    let name_str = name.as_td_str_obj().expect("expected TdStrObj");
    assert_eq!(name_str.value(&db), "Alice");
    assert_eq!(get_num_field(&db, &obj, "age"), 30.0);
  }

  // A math field evaluates to TdMathObj with the correct value
  #[test]
  fn evaluate_math_field() {
    let (db, project, file) = load_vault_fixture("evaluate/my_vault", "valid_math_eval.td");
    let symbol = file_symbol(&db, project, file).value(&db).unwrap();
    let obj = evaluate_resource(&db, symbol).value(&db).unwrap();
    let formula = obj
      .get_owned_field(&db, "formula")
      .expect("should have formula field");
    let math_obj = formula.as_td_math_obj().expect("expected TdMathObj");
    assert_eq!(math_obj.value(&db), "E = mc^2");
  }

  // _content is injected from the markdown body and evaluates to a string
  #[test]
  fn evaluate_content_field() {
    let (db, project, file) = load_vault_fixture("evaluate/my_vault", "md_with_content.td");
    let symbol = file_symbol(&db, project, file).value(&db).unwrap();
    let obj = evaluate_resource(&db, symbol).value(&db).unwrap();
    let content = obj
      .get_builtin_field(&db, "_content")
      .expect("should have _content field");
    let str_obj = content.as_td_str_obj().expect("expected TdStrObj");
    assert!(str_obj.value(&db).contains("Hello world"));
  }

  // String field with inline math evaluates to a concatenated string
  #[test]
  fn evaluate_string_with_inline_math() {
    let (db, project, file) = load_vault_fixture("evaluate/my_vault", "str_with_inline_math.td");
    let symbol = file_symbol(&db, project, file).value(&db).unwrap();
    let obj = evaluate_resource(&db, symbol).value(&db).unwrap();
    let name = obj
      .get_owned_field(&db, "name")
      .expect("should have name field");
    let str_obj = name.as_td_str_obj().expect("expected TdStrObj");
    assert_eq!(str_obj.value(&db), "The judgment $\\vdash$ holds");
  }

  // String field with multiple inline math expressions evaluates to a concatenated string
  #[test]
  fn evaluate_string_with_multiple_inline_math() {
    let (db, project, file) = load_vault_fixture("evaluate/my_vault", "str_with_multiple_math.td");
    let symbol = file_symbol(&db, project, file).value(&db).unwrap();
    let obj = evaluate_resource(&db, symbol).value(&db).unwrap();
    let name = obj
      .get_owned_field(&db, "name")
      .expect("should have name field");
    let str_obj = name.as_td_str_obj().expect("expected TdStrObj");
    let val = str_obj.value(&db);
    assert!(
      val.contains("$\\Gamma \\vdash J$") && val.contains("$\\Gamma \\vdash K$"),
      "expected math content wrapped in $ delimiters, got: {}",
      val
    );
  }

  // An asset symbol evaluates to a TdBlobObj with the correct format field
  #[test]
  fn evaluate_asset_produces_blob() {
    let db = TypedownDatabase {
      storage: QueryStorage::default(),
    };
    let path = PathBuf::from("/vault/_assets/photo.png");
    let file = File::new(&db, FileHandle::Path(path.clone(), FileMetadata::default()));
    let project = Project::new(
      &db,
      PathBuf::from("/vault"),
      [(path, file)].into_iter().collect(),
    );
    let symbol = Symbol::new(
      &db,
      SymbolKind::Asset(AssetKind::Png, project, file),
      "photo".to_string(),
      "@vault::_assets/photo.png".to_string(),
    );

    let result = evaluate_resource(&db, symbol);
    assert!(
      result.diagnostics(&db).is_empty(),
      "should have no diagnostics"
    );
    let obj = result.value(&db).expect("should produce a blob object");
    assert!(obj.as_td_blob_obj().is_some(), "expected TdBlobObj");

    let format = obj
      .get_owned_field(&db, "format")
      .expect("should have format field");
    let format_str = format.as_td_str_obj().expect("expected TdStrObj");
    assert_eq!(format_str.value(&db), "png");
  }

  // An SVG file in the fixture vault is loaded as an asset and evaluates to a blob
  #[test]
  fn asset_file_loaded_and_evaluated() {
    let (db, project, file) = load_vault_fixture("evaluate/my_vault", "icon.svg");
    let symbol = file_symbol(&db, project, file)
      .value(&db)
      .expect("file_symbol should return a symbol");

    assert!(symbol.kind(&db).is_asset(), "should be an asset symbol");

    let result = evaluate_resource(&db, symbol);
    assert!(
      result.diagnostics(&db).is_empty(),
      "should have no diagnostics"
    );
    let obj = result.value(&db).expect("should produce a blob object");
    assert!(obj.as_td_blob_obj().is_some(), "expected TdBlobObj");

    let format = obj
      .get_owned_field(&db, "format")
      .expect("should have format field");
    let format_str = format.as_td_str_obj().expect("expected TdStrObj");
    assert_eq!(format_str.value(&db), "svg");
  }

  // Each AssetKind produces the correct format string
  #[test]
  fn blob_format_matches_asset_kind() {
    let db = TypedownDatabase {
      storage: QueryStorage::default(),
    };
    let file = File::new(
      &db,
      FileHandle::Path(PathBuf::from("/dummy"), FileMetadata::default()),
    );

    let cases = [
      (AssetKind::Pdf, "pdf"),
      (AssetKind::Svg, "svg"),
      (AssetKind::Png, "png"),
      (AssetKind::Jpg, "jpg"),
      (AssetKind::Webp, "webp"),
      (AssetKind::UnknownBinary, "unknown"),
    ];

    for (kind, expected_format) in cases {
      let blob = make_blob_obj(&db, kind, file);
      let format = TdObjectEnum::from(blob)
        .get_owned_field(&db, "format")
        .expect("should have format");
      let format_str = format.as_td_str_obj().expect("expected TdStrObj");
      assert_eq!(
        format_str.value(&db),
        expected_format,
        "format mismatch for {:?}",
        kind
      );
    }
  }

  // null evaluates to TdNullObj
  #[test]
  fn null_evaluates_to_null_obj() {
    let (db, project, file) = load_vault_fixture("evaluate/null_type", "null_value.td");
    let symbol = file_symbol(&db, project, file).value(&db).unwrap();
    let obj = evaluate_resource(&db, symbol).value(&db).unwrap();
    let result = obj
      .get_owned_field(&db, "result")
      .expect("should have result");
    assert!(
      result.as_td_null_obj().is_some(),
      "null should evaluate to TdNullObj"
    );
  }

  // string? field accepts null
  #[test]
  fn optional_field_accepts_null() {
    let (db, project, file) = load_vault_fixture("evaluate/null_type", "with_null.td");
    let symbol = file_symbol(&db, project, file).value(&db).unwrap();
    let obj = evaluate_resource(&db, symbol).value(&db).unwrap();
    let name = obj.get_owned_field(&db, "name").expect("should have name");
    assert_eq!(name.as_td_str_obj().unwrap().value(&db), "Alice");
    let nickname = obj
      .get_owned_field(&db, "nickname")
      .expect("should have nickname");
    assert!(
      nickname.as_td_null_obj().is_some(),
      "null value should be TdNullObj"
    );
  }

  // string? field accepts a string value
  #[test]
  fn optional_field_accepts_value() {
    let (db, project, file) = load_vault_fixture("evaluate/null_type", "with_value.td");
    let symbol = file_symbol(&db, project, file).value(&db).unwrap();
    let obj = evaluate_resource(&db, symbol).value(&db).unwrap();
    let nickname = obj
      .get_owned_field(&db, "nickname")
      .expect("should have nickname");
    let str_obj = nickname.as_td_str_obj().expect("should be a string");
    assert_eq!(str_obj.value(&db), "Bobby");
  }

  // string? field can be omitted
  #[test]
  fn optional_field_can_be_omitted() {
    let (db, project, file) = load_vault_fixture("evaluate/null_type", "without_optional.td");
    let symbol = file_symbol(&db, project, file).value(&db).unwrap();
    let result = evaluate_resource(&db, symbol);
    assert!(
      result.value(&db).is_some(),
      "resource with omitted optional field should evaluate"
    );
  }

  // string? field with wrong type produces typecheck error
  #[test]
  fn optional_field_wrong_type_has_diagnostics() {
    let (db, project, file) = load_vault_fixture("evaluate/null_type", "wrong_type.td");
    let (hir, _) = lower_file(&db, project, file);
    let result = typecheck(&db, hir.unwrap());
    assert!(
      !result.diagnostics(&db).is_empty(),
      "number in string? field should produce diagnostics"
    );
  }

  // _type: Schema? should produce a diagnostic
  #[test]
  fn nullable_type_ref_has_diagnostics() {
    let (db, project, file) = load_vault_fixture("evaluate/null_type", "nullable_type_ref.td");
    let (hir, _) = lower_file(&db, project, file);
    let result = typecheck(&db, hir.unwrap());
    let diags = result.diagnostics(&db);
    assert!(
      diags
        .iter()
        .any(|d| matches!(d, Diagnostic::UnresolvedSchema { .. })),
      "_type: Schema? should produce UnresolvedSchema: {:?}",
      diags
    );
  }

  // Missing a required field (name: string) produces MissingRequiredField diagnostic
  #[test]
  fn missing_required_field_has_diagnostics() {
    let (db, project, file) = load_vault_fixture("evaluate/null_type", "missing_required.td");
    let (hir, _) = lower_file(&db, project, file);
    let result = typecheck(&db, hir.unwrap());
    let diags = result.diagnostics(&db);
    assert!(
      diags
        .iter()
        .any(|d| matches!(d, Diagnostic::MissingRequiredField { field, .. } if field == "name")),
      "missing required field 'name' should produce diagnostic: {:?}",
      diags
    );
  }

  // Missing a nullable field (nickname: string?) does NOT produce a diagnostic
  #[test]
  fn missing_nullable_field_no_diagnostics() {
    let (db, project, file) = load_vault_fixture("evaluate/null_type", "without_optional.td");
    let (hir, _) = lower_file(&db, project, file);
    let result = typecheck(&db, hir.unwrap());
    let diags = result.diagnostics(&db);
    assert!(
      !diags.iter().any(
        |d| matches!(d, Diagnostic::MissingRequiredField { field, .. } if field == "nickname")
      ),
      "missing nullable field should not produce diagnostic: {:?}",
      diags
    );
  }

  // Missing field on a product object evaluates to null at runtime
  #[test]
  fn missing_field_evaluates_to_null() {
    let (db, project, file) = load_vault_fixture("evaluate/null_type", "without_optional.td");
    let symbol = file_symbol(&db, project, file).value(&db).unwrap();
    let obj = evaluate_resource(&db, symbol).value(&db).unwrap();
    let nickname = obj
      .get_owned_field(&db, "nickname")
      .expect("missing field should return null, not None");
    assert!(
      nickname.as_td_null_obj().is_some(),
      "missing field should evaluate to TdNullObj"
    );
  }

  // Missing a field with a default value does NOT produce a MissingRequiredField diagnostic
  #[test]
  fn missing_field_with_default_no_diagnostics() {
    let (db, project, file) = load_vault_fixture("evaluate/my_vault", "default_resource.td");
    let (hir, _) = lower_file(&db, project, file);
    let result = typecheck(&db, hir.unwrap());
    let diags = result.diagnostics(&db);
    assert!(
      diags.is_empty(),
      "missing field with default should not produce diagnostics: {:?}",
      diags
    );
  }

  // Missing field with a default value evaluates to default_value at runtime
  #[test]
  fn missing_field_evaluates_to_default_value() {
    let (db, project, file) = load_vault_fixture("evaluate/my_vault", "default_resource.td");
    let symbol = file_symbol(&db, project, file).value(&db).unwrap();
    let obj = evaluate_resource(&db, symbol).value(&db).unwrap();
    let status_obj = obj
      .get_owned_field(&db, "status")
      .expect("field with default should return evaluated default object");
    let status_str = status_obj
      .as_td_str_obj()
      .expect("expected TdStrObj for status default");
    assert_eq!(status_str.value(&db), "draft");

    let count_obj = obj
      .get_owned_field(&db, "count")
      .expect("field with default should return evaluated default object");
    let count_num = count_obj
      .as_td_num_obj()
      .expect("expected TdNumObj for count default");
    assert_eq!(count_num.value(&db), 0.0);
  }

  #[test]
  fn imports_resolve_module_fields() {
    let (db, project, file) = load_vault_fixture("evaluate/my_vault", "with_imports.td");
    let symbol = file_symbol(&db, project, file)
      .value(&db)
      .expect("file_symbol should return a resource symbol");

    let result = evaluate_resource(&db, symbol);
    assert!(
      result.value(&db).is_some(),
      "should produce an object, diagnostics: {:?}",
      result.diagnostics(&db)
    );
    let obj = result.value(&db).unwrap();
    let brand = obj
      .get_owned_field(&db, "brand")
      .expect("should have brand field");
    let brand_str = brand.as_td_str_obj().expect("expected TdStrObj");
    assert_eq!(brand_str.value(&db), "#4F6BCA");
  }

  #[test]
  #[cfg(feature = "export")]
  fn imports_stripped_from_export() {
    use crate::integrations::export::export_resource;

    let (db, project, file) = load_vault_fixture("evaluate/my_vault", "with_imports.td");
    let result = export_resource(&db, project, file);
    let exported = result.expect("file with imports should export");
    let header = exported
      .header
      .as_object()
      .expect("header should be object");
    assert!(
      !header.contains_key("_imports"),
      "_imports should be stripped from export"
    );
    assert_eq!(
      header.get("brand").and_then(|v| v.as_str()),
      Some("#4F6BCA"),
      "brand should resolve to imported color value"
    );
  }
}
