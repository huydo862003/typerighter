use std::collections::HashMap;

use typedown_incremental::QueryDatabase;
use typedown_macros::query_derived;

use crate::db::TypedownDatabase;
use crate::db::derived::get_builtin_types::{
  get_list_type, get_null_type, get_num_type, get_object_type, get_schema_type, get_str_type,
  get_type_type,
};
use crate::db::types::{
  FuncSignature, LazyType, NativeFnKind, TdFuncObj, TdProductType, TdSumType,
};

#[query_derived]
pub fn get_vault_type(db: &TypedownDatabase) -> TdProductType {
  let list_type = get_list_type(db);
  let num_type = get_num_type(db);
  let object_type = get_object_type(db);

  // The filter argument type: { schema?: schema, path?: string }
  let filter_type = TdProductType::new(
    db,
    Some("VaultFilter".to_string()),
    get_type_type(db).into(),
    None,
    HashMap::from([
      (
        "schema".to_string(),
        LazyType::eager(
          TdSumType::new(
            db,
            vec![
              LazyType::eager(get_schema_type(db).into()),
              LazyType::eager(get_null_type(db).into()),
            ],
          )
          .into(),
        ),
      ),
      (
        "path".to_string(),
        LazyType::eager(
          TdSumType::new(
            db,
            vec![
              LazyType::eager(get_str_type(db).into()),
              LazyType::eager(get_null_type(db).into()),
            ],
          )
          .into(),
        ),
      ),
    ]),
    HashMap::new(),
  );

  let files_sig = FuncSignature::new(db, vec![], list_type.into());
  let files = TdFuncObj::new(
    db,
    "files".to_string(),
    object_type.into(),
    files_sig,
    NativeFnKind::VaultFiles,
  );

  let files_where_sig = FuncSignature::new(db, vec![filter_type.into()], list_type.into());
  let files_where = TdFuncObj::new(
    db,
    "files_where".to_string(),
    object_type.into(),
    files_where_sig,
    NativeFnKind::VaultFilesWhere,
  );

  let count_sig = FuncSignature::new(db, vec![], num_type.into());
  let count = TdFuncObj::new(
    db,
    "count".to_string(),
    object_type.into(),
    count_sig,
    NativeFnKind::VaultCount,
  );

  let count_where_sig = FuncSignature::new(db, vec![filter_type.into()], num_type.into());
  let count_where = TdFuncObj::new(
    db,
    "count_where".to_string(),
    object_type.into(),
    count_where_sig,
    NativeFnKind::VaultCountWhere,
  );

  let vtable = HashMap::from([
    ("files".to_string(), files),
    ("files_where".to_string(), files_where),
    ("count".to_string(), count),
    ("count_where".to_string(), count_where),
  ]);

  TdProductType::new(
    db,
    Some("vault".to_string()),
    get_type_type(db).into(),
    None,
    HashMap::new(),
    vtable,
  )
}

#[cfg(test)]
mod tests {
  use crate::db::TypedownDatabase;
  use crate::db::derived::evaluate::evaluate_resource::evaluate_resource;
  use crate::db::derived::name_resolver::file_symbol::file_symbol;
  use crate::db::derived::typechecker::typecheck::typecheck;
  use crate::db::fixtures::load_vault_fixture;
  use crate::db::types::{TdObjectEnum, TdObjectLike};
  use crate::db::utils::lower_file;

  fn get_num_field(db: &TypedownDatabase, obj: &TdObjectEnum, field: &str) -> f64 {
    let field_obj = obj.get_owned_field(db, field).expect("should have field");
    let num = field_obj.as_td_num_obj().expect("should be TdNumObj");
    num.value(db)
  }

  fn get_list_len(db: &TypedownDatabase, obj: &TdObjectEnum, field: &str) -> usize {
    let field_obj = obj.get_owned_field(db, field).expect("should have field");
    let list = field_obj.as_td_list_obj().expect("should be TdListObj");
    list.len(db)
  }

  fn eval_vault_file(file_name: &str) -> (TypedownDatabase, TdObjectEnum) {
    let (db, project, file) =
      load_vault_fixture("evaluate/vault_builtin", &format!("content/{}", file_name));
    let symbol = file_symbol(&db, project, file)
      .value(&db)
      .expect("should return a resource symbol");
    let obj = evaluate_resource(&db, symbol)
      .value(&db)
      .expect("should produce an object");
    (db, obj)
  }

  // vault.files() returns all content files (excluding the query file itself)
  #[test]
  fn vault_files_returns_all_content() {
    let (db, obj) = eval_vault_file("vault_files.td");
    let len = get_list_len(&db, &obj, "all");
    // 3 typed files + vault_files.td itself
    assert!(
      len >= 3,
      "vault.files() should return at least 3 files, got {}",
      len
    );
  }

  // vault.count() returns the total count
  #[test]
  fn vault_count_returns_total() {
    let (db, obj) = eval_vault_file("vault_files.td");
    let total = get_num_field(&db, &obj, "total");
    assert!(
      total >= 3.0,
      "vault.count() should be at least 3, got {}",
      total
    );
  }

  // vault.files_where({ schema: Article }) filters by schema
  #[test]
  fn vault_files_where_filters_by_schema() {
    let (db, obj) = eval_vault_file("vault_files.td");
    let len = get_list_len(&db, &obj, "articles");
    assert_eq!(len, 2, "should have exactly 2 articles");
  }

  // vault.count_where({ schema: Article }) counts by schema
  #[test]
  fn vault_count_where_filters_by_schema() {
    let (db, obj) = eval_vault_file("vault_files.td");
    let count = get_num_field(&db, &obj, "article_count");
    assert_eq!(count, 2.0, "should have exactly 2 articles");
  }

  // vault.count_where({ schema: Note }) counts a different schema
  #[test]
  fn vault_count_where_note_schema() {
    let (db, obj) = eval_vault_file("vault_files.td");
    let count = get_num_field(&db, &obj, "note_count");
    assert_eq!(count, 1.0, "should have exactly 1 note");
  }

  // Filtered counts add up to less than or equal to total
  #[test]
  fn filtered_counts_consistent_with_total() {
    let (db, obj) = eval_vault_file("vault_files.td");
    let total = get_num_field(&db, &obj, "total");
    let articles = get_num_field(&db, &obj, "article_count");
    let notes = get_num_field(&db, &obj, "note_count");
    assert!(
      articles + notes <= total,
      "filtered counts should not exceed total"
    );
  }

  // vault.count_where({ path: "blog" }) filters by path prefix
  #[test]
  fn vault_count_where_filters_by_path() {
    let (db, obj) = eval_vault_file("vault_files.td");
    let count = get_num_field(&db, &obj, "blog_count");
    assert_eq!(count, 1.0, "should have exactly 1 file in blog/");
  }

  // vault.count_where({ schema: Article, path: "blog" }) combines both filters
  #[test]
  fn vault_count_where_combines_schema_and_path() {
    let (db, obj) = eval_vault_file("vault_files.td");
    let count = get_num_field(&db, &obj, "blog_articles");
    assert_eq!(count, 1.0, "should have exactly 1 article in blog/");
  }

  // { schema: "not_a_schema" } should produce diagnostics because schema
  // field expects a schema type, not a string
  #[test]
  fn vault_files_where_wrong_schema_type_has_diagnostics() {
    let (db, project, file) = load_vault_fixture(
      "evaluate/vault_builtin",
      "content/vault_bad_schema_filter.td",
    );
    let (hir, _) = lower_file(&db, project, file);
    let result = typecheck(&db, hir.unwrap());
    assert!(
      !result.diagnostics(&db).is_empty(),
      "string instead of schema type should produce diagnostics"
    );
  }

  // { path: 42 } should produce typecheck diagnostics
  #[test]
  fn vault_files_where_wrong_path_type_has_diagnostics() {
    let (db, project, file) =
      load_vault_fixture("evaluate/vault_builtin", "content/vault_bad_path_filter.td");
    let (hir, _) = lower_file(&db, project, file);
    let result = typecheck(&db, hir.unwrap());
    let diags = result.diagnostics(&db);
    assert!(
      !diags.is_empty(),
      "number instead of string for path should produce diagnostics"
    );
  }

  // vault.files_where(42) passes a non-object arg
  #[test]
  fn vault_files_where_non_object_arg_has_diagnostics() {
    let (db, project, file) =
      load_vault_fixture("evaluate/vault_builtin", "content/vault_wrong_arg_type.td");
    let (hir, _) = lower_file(&db, project, file);
    let result = typecheck(&db, hir.unwrap());
    assert!(
      !result.diagnostics(&db).is_empty(),
      "number arg instead of filter object should produce diagnostics"
    );
  }

  // vault.files("extra", "args") passes wrong number of args
  #[test]
  fn vault_files_wrong_arity_has_diagnostics() {
    let (db, project, file) =
      load_vault_fixture("evaluate/vault_builtin", "content/vault_wrong_arity.td");
    let (hir, _) = lower_file(&db, project, file);
    let result = typecheck(&db, hir.unwrap());
    assert!(
      !result.diagnostics(&db).is_empty(),
      "wrong number of args should produce diagnostics"
    );
  }

  // Valid vault calls produce no typecheck diagnostics
  #[test]
  fn vault_files_no_typecheck_errors() {
    let (db, project, file) =
      load_vault_fixture("evaluate/vault_builtin", "content/vault_files.td");
    let (hir, _) = lower_file(&db, project, file);
    let result = typecheck(&db, hir.unwrap());
    assert!(
      result.diagnostics(&db).is_empty(),
      "valid vault calls should have no errors: {:?}",
      result.diagnostics(&db)
    );
  }

  // vault.files_where({}) with empty filter is valid, no diagnostics
  #[test]
  fn vault_files_where_empty_filter_no_diagnostics() {
    let (db, project, file) =
      load_vault_fixture("evaluate/vault_builtin", "content/vault_empty_filter.td");
    let (hir, _) = lower_file(&db, project, file);
    let result = typecheck(&db, hir.unwrap());
    assert!(
      result.diagnostics(&db).is_empty(),
      "empty filter should be valid: {:?}",
      result.diagnostics(&db)
    );
  }
}
