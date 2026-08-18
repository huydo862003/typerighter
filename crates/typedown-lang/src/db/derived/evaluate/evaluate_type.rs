//! Evaluate a schema symbol to extract the type it defines.

use std::collections::HashMap;

use crate::syntax::diagnostic::Diagnostic;
use typedown_macros::query_derived;

use crate::db::TypedownDatabase;
use crate::db::derived::get_builtin_types::{
  get_bool_type, get_date_type, get_datetime_type, get_dict_type, get_list_type, get_literal_type,
  get_math_type, get_null_type, get_num_type, get_schema_type, get_str_type, get_sum_type,
  get_time_type, get_type_type, instantiate_type,
};
use crate::db::derived::name_resolver::referee::referee;
use crate::db::derived::schema_property::get_schema_property_type;
use crate::db::types::{
  BuiltinSchemaKind, File, HirValue, HirValueKind, LazyType, LiteralValue, Project, Symbol,
  SymbolKind, TdBlobType, TdProductType, TdStructuralType, TdTypeEnum, TdTypeLike, TypeResult,
};
use crate::db::utils::lower_file;
use crate::db::utils::typecheck::is_nullable;
use typedown_incremental::QueryDatabase;

#[query_derived]
pub fn evaluate_type(db: &TypedownDatabase, symbol: Symbol) -> TypeResult {
  match symbol.kind(db) {
    SymbolKind::BuiltinSchema(kind) => {
      let typ: TdTypeEnum = match kind {
        BuiltinSchemaKind::Str => get_str_type(db).into(),
        BuiltinSchemaKind::Num => get_num_type(db).into(),
        BuiltinSchemaKind::Bool => get_bool_type(db).into(),
        BuiltinSchemaKind::Date => get_date_type(db).into(),
        BuiltinSchemaKind::DateTime => get_datetime_type(db).into(),
        BuiltinSchemaKind::Time => get_time_type(db).into(),
        BuiltinSchemaKind::List => get_list_type(db).into(),
        BuiltinSchemaKind::Dict => get_dict_type(db).into(),
        BuiltinSchemaKind::Math => get_math_type(db).into(),
        BuiltinSchemaKind::Schema => get_schema_type(db).into(),
        BuiltinSchemaKind::TypeType => get_type_type(db).into(),
        BuiltinSchemaKind::SchemaProperty => get_schema_property_type(db).into(),
      };
      TypeResult::new(db, Some(typ), vec![])
    }
    SymbolKind::UserDefinedSchema(project, file) => {
      evaluate_user_defined_schema(db, symbol.name(db), project, file)
    }
    SymbolKind::Asset(_, _, _) => TypeResult::new(db, Some(TdBlobType::get(db).into()), vec![]),
    SymbolKind::UserDefinedResource(_, _)
    | SymbolKind::BuiltinMacro(_)
    | SymbolKind::BuiltinGlobal(_) => TypeResult::new(db, None, vec![]),
    SymbolKind::FnParam(_, _, _) => TypeResult::new(db, None, vec![]),
  }
}

fn evaluate_user_defined_schema(
  db: &TypedownDatabase,
  schema_name: String,
  project: Project,
  file: File,
) -> TypeResult {
  let mut diagnostics = vec![];

  // Parse file and lower frontmatter to HIR
  let (hir, _) = lower_file(db, project, file);
  let hir = match hir {
    Some(hir) => hir,
    None => return TypeResult::new(db, None, vec![]),
  };

  // Extract entries from the frontmatter mapping
  let entries = match hir.kind(db) {
    HirValueKind::Mapping(entries) => entries,
    _ => return TypeResult::new(db, None, diagnostics),
  };

  // Find the "properties" entry
  let properties_hir = entries.iter().find(|(key, _)| key == "properties");
  let properties_entries = match properties_hir {
    Some((_, props_hir)) => match props_hir.kind(db) {
      HirValueKind::Mapping(entries) => entries,
      _ => {
        let node = props_hir.node(db);
        let (tr_offset, tr_len) = node.trimmed_range();
        diagnostics.push(Diagnostic::FieldTypeMismatch {
          field: "properties".to_string(),
          expected: "mapping".to_string(),
          start_offset: tr_offset,
          end_offset: tr_offset + tr_len,
        });
        return TypeResult::new(db, None, diagnostics);
      }
    },
    // Schema with no properties: empty product type
    None => {
      return TypeResult::new(
        db,
        Some(
          TdProductType::new(
            db,
            Some(schema_name.clone()),
            get_schema_type(db).into(),
            None,
            HashMap::new(),
            HashMap::new(),
          )
          .into(),
        ),
        diagnostics,
      );
    }
  };

  // The resulting fields of the product/schema type
  let mut fields = HashMap::new();

  // Loop through the declared props
  for (prop_name, prop_hir) in properties_entries {
    if let Some(lazy) = resolve_property_descriptor(db, prop_hir, &mut diagnostics) {
      fields.insert(prop_name.clone(), lazy);
    }
  }

  TypeResult::new(
    db,
    Some(
      TdProductType::new(
        db,
        Some(schema_name),
        get_schema_type(db).into(),
        None,
        fields,
        HashMap::new(),
      )
      .into(),
    ),
    diagnostics,
  )
}

// Process a property descriptor like `{ type: string, optional: true }`
// Returns a LazyType representing the field type
pub(crate) fn resolve_property_descriptor(
  db: &TypedownDatabase,
  hir: HirValue,
  diagnostics: &mut Vec<Diagnostic>,
) -> Option<LazyType> {
  let entries = match hir.kind(db) {
    HirValueKind::Mapping(entries) => entries,
    _ => return None,
  };

  let mut field_type: Option<LazyType> = None;
  let mut is_optional = false;

  for (key, value) in &entries {
    match key.as_str() {
      "type" => {
        field_type = resolve_type_lazy(db, *value, diagnostics);
      }
      "optional" => {
        if let HirValueKind::Bool(true) = value.kind(db) {
          is_optional = true;
        }
      }
      _ => {}
    }
  }

  field_type.map(|lazy| {
    if is_optional && !lazy.as_eager().is_some_and(|t| is_nullable(db, t)) {
      let null_lazy = LazyType::eager(get_null_type(db).into());
      LazyType::eager(get_sum_type(db, vec![lazy, null_lazy]).into())
    } else {
      lazy
    }
  })
}

fn resolve_type_lazy(
  db: &TypedownDatabase,
  hir: HirValue,
  diagnostics: &mut Vec<Diagnostic>,
) -> Option<LazyType> {
  match hir.kind(db) {
    // `!type expr` is redundant but valid: strip the tag and recurse on the inner value
    HirValueKind::Tag { tag, inner } => {
      if matches!(tag.kind(db), HirValueKind::Ident(ref name) if name == "type") {
        return resolve_type_lazy(db, *inner, diagnostics);
      }
      let node = hir.node(db);
      let (tr_offset, tr_len) = node.trimmed_range();
      diagnostics.push(Diagnostic::FieldTypeMismatch {
        field: "type".to_string(),
        expected: "type expression".to_string(),
        start_offset: tr_offset,
        end_offset: tr_offset + tr_len,
      });
      None
    }

    // Desugar T? to Sum([T, null])
    HirValueKind::Postfix { op, operand } if op == "?" => {
      let inner = resolve_type_lazy(db, *operand, diagnostics)?;
      let null_lazy = LazyType::eager(get_null_type(db).into());
      Some(LazyType::eager(
        get_sum_type(db, vec![inner, null_lazy]).into(),
      ))
    }

    // Simple type reference like `type: string`
    HirValueKind::Ident(_) => {
      let resolved = referee(db, hir);
      match resolved.value(db) {
        Some(symbol) => match symbol.kind(db) {
          SymbolKind::UserDefinedSchema(_, _) => Some(LazyType::lazy(symbol)),
          _ => {
            let result = evaluate_type(db, symbol);
            diagnostics.extend(result.diagnostics(db).iter().cloned());
            result.typ(db).map(LazyType::eager)
          }
        },
        None => {
          let node = hir.node(db);
          let (tr_offset, tr_len) = node.trimmed_range();
          diagnostics.push(Diagnostic::UnresolvedSchema {
            name: node.text(),
            start_offset: tr_offset,
            end_offset: tr_offset + tr_len,
          });
          None
        }
      }
    }
    // Union type like `type: [string, number]`
    HirValueKind::Sequence(items) => {
      let mut members = vec![];
      for item in items {
        if let Some(lazy) = resolve_type_lazy(db, item, diagnostics) {
          members.push(lazy);
        }
      }
      if members.is_empty() {
        None
      } else {
        Some(LazyType::eager(
          get_sum_type(db, members.into_iter().collect()).into(),
        ))
      }
    }
    // Inline object like `type: { name: { type: string }, age: { type: number } }`
    HirValueKind::Mapping(entries) => {
      let mut fields = HashMap::new();
      for (key, value_hir) in entries {
        if let Some(lazy) = resolve_property_descriptor(db, value_hir, diagnostics) {
          fields.insert(key.clone(), lazy);
        }
      }
      Some(LazyType::eager(TdStructuralType::new(db, fields).into()))
    }
    // Generic type instantiation like `type: list[string]`
    HirValueKind::Index { expr, indices } => {
      let base = resolve_type_lazy(db, *expr, diagnostics)?;
      let base_type = base.resolve(db)?;
      if base_type.arity(db) == 0 {
        return Some(LazyType::eager(base_type));
      }
      let mut arg_types = vec![];
      for idx_hir in indices {
        let resolved = referee(db, idx_hir);
        match resolved.value(db) {
          Some(symbol) => match symbol.kind(db) {
            SymbolKind::UserDefinedSchema(_, _) => {
              arg_types.push(LazyType::lazy(symbol));
            }
            _ => {
              let result = evaluate_type(db, symbol);
              diagnostics.extend(result.diagnostics(db).iter().cloned());
              if let Some(typ) = result.typ(db) {
                arg_types.push(LazyType::eager(typ));
              }
            }
          },
          None => {
            let node = idx_hir.node(db);
            let (tr_offset, tr_len) = node.trimmed_range();
            diagnostics.push(Diagnostic::UnresolvedSchema {
              name: node.text(),
              start_offset: tr_offset,
              end_offset: tr_offset + tr_len,
            });
            return None;
          }
        }
      }
      let inst_result = instantiate_type(db, base_type, arg_types);
      diagnostics.extend(inst_result.diagnostics(db).iter().cloned());
      Some(LazyType::eager(inst_result.typ(db)))
    }
    // Literal types
    HirValueKind::Str(val) => Some(LazyType::eager(
      get_literal_type(db, LiteralValue::Str(val)).into(),
    )),
    HirValueKind::Num(val) => Some(LazyType::eager(
      get_literal_type(db, LiteralValue::Num(val)).into(),
    )),
    HirValueKind::Bool(val) => Some(LazyType::eager(
      get_literal_type(db, LiteralValue::Bool(val)).into(),
    )),
    _ => {
      let node = hir.node(db);
      let (tr_offset, tr_len) = node.trimmed_range();
      diagnostics.push(Diagnostic::FieldTypeMismatch {
        field: "type".to_string(),
        expected: "type expression".to_string(),
        start_offset: tr_offset,
        end_offset: tr_offset + tr_len,
      });
      None
    }
  }
}

#[cfg(test)]
mod tests {
  use crate::db::types::{TdObjectEnum, TdObjectLike, TdTypeEnum};
  use std::collections::HashMap;
  use std::path::PathBuf;

  use crate::db::{
    QueryStorage, TypedownDatabase,
    derived::evaluate::evaluate_resource::evaluate_resource,
    derived::evaluate::evaluate_type::evaluate_type,
    derived::evaluate::utils::construct_from_hir,
    derived::get_builtin_types::*,
    derived::name_resolver::file_symbol::file_symbol,
    derived::typechecker::actual_node_type::actual_node_type,
    fixtures::load_vault_fixture,
    types::{
      BuiltinSchemaKind, File, FileHandle, FileMetadata, HirValue, HirValueKind, LazyType,
      LiteralValue, Project, Symbol, SymbolKind, TdBoolObj, TdNumObj, TdProductType, TdStrObj,
      TdStructuralType, TdTypeLike, TdTypeType,
    },
    utils::lower_file,
  };

  fn make_db() -> TypedownDatabase {
    TypedownDatabase {
      storage: QueryStorage::default(),
    }
  }

  #[test]
  fn evaluate_type_builtin_schema_returns_schema_type() {
    let db = make_db();
    let symbol = Symbol::new(
      &db,
      SymbolKind::BuiltinSchema(BuiltinSchemaKind::Schema),
      "schema".to_string(),
      "@builtin::schema".to_string(),
    );
    let result = evaluate_type(&db, symbol);
    assert!(result.typ(&db) == Some(TdTypeEnum::from(get_schema_type(&db))));
    assert!(result.diagnostics(&db).is_empty());
  }

  #[test]
  fn evaluate_user_defined_schema_returns_product_type() {
    let (db, project, file) = load_vault_fixture("evaluate/my_vault", "schemas/Person.td");
    let symbol = file_symbol(&db, project, file).value(&db).unwrap();
    let result = evaluate_type(&db, symbol);
    assert!(result.typ(&db).unwrap().is_td_product_type());
  }

  #[test]
  fn evaluate_user_defined_schema_has_declared_fields() {
    let (db, project, file) = load_vault_fixture("evaluate/my_vault", "schemas/Person.td");
    let symbol = file_symbol(&db, project, file).value(&db).unwrap();
    let result = evaluate_type(&db, symbol);
    let typ = result.typ(&db).unwrap();
    let product = typ.as_td_product_type().unwrap();
    assert!(product.fields(&db).contains_key("name"));
    assert!(product.fields(&db).contains_key("age"));
  }

  // Schema where property types use the explicit `!type` tag: `type: !type string`
  #[test]
  fn evaluate_schema_with_explicit_type_tag() {
    let (db, project, file) =
      load_vault_fixture("evaluate/my_vault", "schemas/PersonExplicitType.td");
    let symbol = file_symbol(&db, project, file).value(&db).unwrap();
    let result = evaluate_type(&db, symbol);
    assert!(
      result.diagnostics(&db).is_empty(),
      "{:?}",
      result.diagnostics(&db)
    );
    let typ = result.typ(&db).unwrap();
    let product = typ.as_td_product_type().unwrap();
    assert!(product.fields(&db).contains_key("name"));
    assert!(product.fields(&db).contains_key("age"));
  }

  #[test]
  fn evaluate_type_no_properties_returns_empty_product() {
    let (db, project, file) = load_vault_fixture("evaluate/my_vault", "schemas/NoProperties.td");
    let symbol = file_symbol(&db, project, file).value(&db).unwrap();
    let result = evaluate_type(&db, symbol);
    let typ = result.typ(&db).unwrap();
    let product = typ.as_td_product_type().unwrap();
    assert!(product.fields(&db).is_empty());
  }

  #[test]
  fn evaluate_type_wrong_properties_type_has_diagnostics() {
    let (db, project, file) = load_vault_fixture("evaluate/my_vault", "schemas/WrongProperties.td");
    let symbol = file_symbol(&db, project, file).value(&db).unwrap();
    assert!(!evaluate_type(&db, symbol).diagnostics(&db).is_empty());
  }

  #[test]
  fn evaluate_type_wrong_property_descriptor_has_diagnostics() {
    let (db, project, file) =
      load_vault_fixture("evaluate/my_vault", "schemas/WrongPropertyDescriptor.td");
    let symbol = file_symbol(&db, project, file).value(&db).unwrap();
    assert!(!evaluate_type(&db, symbol).diagnostics(&db).is_empty());
  }

  #[test]
  fn evaluate_type_list_field_in_schema() {
    let (db, project, file) = load_vault_fixture("evaluate/my_vault", "schemas/WithListField.td");
    let symbol = file_symbol(&db, project, file).value(&db).unwrap();
    let result = evaluate_type(&db, symbol);
    assert!(
      result.diagnostics(&db).is_empty(),
      "{:?}",
      result.diagnostics(&db)
    );
    let typ = result.typ(&db).unwrap();
    let product = typ.as_td_product_type().unwrap();
    assert!(product.fields(&db).contains_key("tags"));
    assert!(product.fields(&db).contains_key("scores"));
  }

  #[test]
  fn evaluate_type_circular_schema_refs() {
    let (db, project, file_a) = load_vault_fixture("evaluate/my_vault", "schemas/SchemaA.td");
    let symbol_a = file_symbol(&db, project, file_a).value(&db).unwrap();
    assert!(evaluate_type(&db, symbol_a).diagnostics(&db).is_empty());
    let file_b = project
      .files(&db)
      .iter()
      .find(|(path, _)| path.ends_with("SchemaB.td"))
      .map(|(_, f)| *f)
      .unwrap();
    let symbol_b = file_symbol(&db, project, file_b).value(&db).unwrap();
    assert!(evaluate_type(&db, symbol_b).diagnostics(&db).is_empty());
  }

  #[test]
  fn display_name_builtin_types() {
    let db = make_db();
    assert_eq!(get_str_type(&db).display_name(&db), "string");
    assert_eq!(get_num_type(&db).display_name(&db), "number");
    assert_eq!(get_bool_type(&db).display_name(&db), "boolean");
    assert_eq!(get_date_type(&db).display_name(&db), "date");
    assert_eq!(get_datetime_type(&db).display_name(&db), "datetime");
    assert_eq!(get_time_type(&db).display_name(&db), "time");
    assert_eq!(get_list_type(&db).display_name(&db), "list");
    assert_eq!(get_dict_type(&db).display_name(&db), "dict");
    assert_eq!(get_type_type(&db).display_name(&db), "type");
    assert_eq!(get_object_type(&db).display_name(&db), "object");
    assert_eq!(get_schema_type(&db).display_name(&db), "schema");
    assert_eq!(get_never_type(&db).display_name(&db), "never");
    assert_eq!(get_null_type(&db).display_name(&db), "null");
  }

  #[test]
  fn display_name_literal_types() {
    let db = make_db();
    assert_eq!(
      get_literal_type(&db, LiteralValue::Str("draft".to_string())).display_name(&db),
      "\"draft\""
    );
    assert_eq!(
      get_literal_type(&db, LiteralValue::Num("42".to_string())).display_name(&db),
      "42"
    );
    assert_eq!(
      get_literal_type(&db, LiteralValue::Bool(true)).display_name(&db),
      "true"
    );
  }

  #[test]
  fn display_name_sum_type() {
    let db = make_db();
    let sum = get_sum_type(
      &db,
      vec![
        LazyType::eager(get_str_type(&db).into()),
        LazyType::eager(get_num_type(&db).into()),
      ],
    );
    assert_eq!(sum.display_name(&db), "string | number");
  }

  #[test]
  fn display_name_structural_type() {
    let db = make_db();
    let structural = TdStructuralType::new(
      &db,
      HashMap::from([(
        "name".to_string(),
        LazyType::eager(get_str_type(&db).into()),
      )]),
    );
    assert_eq!(structural.display_name(&db), "{ name: string }");
  }

  #[test]
  fn display_name_instantiated_list() {
    let db = make_db();
    let list_str = instantiate_type(
      &db,
      get_list_type(&db).into(),
      vec![LazyType::eager(get_str_type(&db).into())],
    );
    assert_eq!(list_str.typ(&db).display_name(&db), "list[string]");
  }

  #[test]
  fn display_name_instantiated_dict() {
    let db = make_db();
    let dict_str_num = instantiate_type(
      &db,
      get_dict_type(&db).into(),
      vec![
        LazyType::eager(get_str_type(&db).into()),
        LazyType::eager(get_num_type(&db).into()),
      ],
    );
    assert_eq!(
      dict_str_num.typ(&db).display_name(&db),
      "dict[string, number]"
    );
  }

  #[test]
  fn display_name_user_defined_schema() {
    let (db, project, file) = load_vault_fixture("evaluate/my_vault", "schemas/Person.td");
    let symbol = file_symbol(&db, project, file).value(&db).unwrap();
    assert_eq!(
      evaluate_type(&db, symbol)
        .typ(&db)
        .unwrap()
        .display_name(&db),
      "Person"
    );
  }

  #[test]
  fn display_name_anonymous_product() {
    let db = make_db();
    let product = TdProductType::new(
      &db,
      None,
      get_type_type(&db).into(),
      None,
      HashMap::from([(
        "name".to_string(),
        LazyType::eager(get_str_type(&db).into()),
      )]),
      HashMap::new(),
    );
    assert_eq!(product.display_name(&db), "{ name: string }");
  }

  // Helper to create an HirValue from a frontmatter string
  fn make_hir(db: &TypedownDatabase, content: &str) -> HirValue {
    let file = File::new(
      db,
      FileHandle::Content(
        PathBuf::from("test.td"),
        content.to_string(),
        FileMetadata::default(),
      ),
    );
    let project = Project::new(db, PathBuf::new(), HashMap::new());
    let (hir, _) = lower_file(db, project, file);
    hir.unwrap()
  }

  // Helper to get a specific field's HirValue from a frontmatter mapping
  fn get_field_hir(db: &TypedownDatabase, hir: HirValue, field: &str) -> HirValue {
    match hir.kind(db) {
      HirValueKind::Mapping(entries) => entries.into_iter().find(|(k, _)| k == field).unwrap().1,
      _ => panic!("expected mapping"),
    }
  }

  #[test]
  fn construct_str() {
    let db = make_db();
    let obj = get_str_type(&db)
      .construct(&db, vec![TdStrObj::new(&db, "hello".to_string()).into()])
      .unwrap();
    assert_eq!(obj.as_td_str_obj().unwrap().value(&db), "hello");
  }

  #[test]
  fn construct_num() {
    let db = make_db();
    let obj = get_num_type(&db)
      .construct(&db, vec![TdNumObj::new(&db, 42.0).into()])
      .unwrap();
    assert_eq!(obj.as_td_num_obj().unwrap().value(&db), 42.0);
  }

  #[test]
  fn construct_bool() {
    let db = make_db();
    let obj = get_bool_type(&db)
      .construct(&db, vec![TdBoolObj::new(&db, true).into()])
      .unwrap();
    assert!(obj.as_td_bool_obj().unwrap().value(&db));
  }

  #[test]
  fn construct_str_returns_none_for_wrong_type() {
    let db = make_db();
    assert!(
      get_str_type(&db)
        .construct(&db, vec![TdNumObj::new(&db, 42.0).into()])
        .is_none()
    );
  }

  // Product type construct from a mapping
  #[test]
  fn construct_product() {
    let (db, project, file) = load_vault_fixture("evaluate/my_vault", "content/valid_person.td");
    let (hir, _) = lower_file(&db, project, file);
    let obj = construct_from_hir(&db, hir.unwrap(), &mut vec![]).unwrap();
    let name_obj = obj.get_owned_field(&db, "name").unwrap();
    let name = name_obj.as_td_str_obj().unwrap();
    assert_eq!(name.value(&db), "Alice");
  }

  // List construct from a sequence
  #[test]
  fn construct_list() {
    let db = make_db();
    let list_num = instantiate_type(
      &db,
      get_list_type(&db).into(),
      vec![LazyType::eager(get_num_type(&db).into())],
    );
    let items: Vec<TdObjectEnum> = vec![
      TdNumObj::new(&db, 1.0).into(),
      TdNumObj::new(&db, 2.0).into(),
    ];
    assert!(list_num.typ(&db).construct(&db, items).is_some());
  }

  // Schema construct via evaluate_type
  #[test]
  fn construct_schema() {
    let (db, project, file) = load_vault_fixture("evaluate/my_vault", "schemas/Person.td");
    let symbol = file_symbol(&db, project, file).value(&db).unwrap();
    let typ = evaluate_type(&db, symbol).typ(&db).unwrap();
    let product = typ.as_td_product_type().unwrap();
    assert!(product.fields(&db).contains_key("name"));
  }

  // TdObjectType construct passes through when given exactly one arg
  #[test]
  fn construct_object_type_fallback_to_dict() {
    let db = make_db();
    let hir = make_hir(&db, "---\nname: \"Alice\"\nage: 42\n---");
    let val_hir = get_field_hir(&db, hir, "name");
    let obj = construct_from_hir(&db, val_hir, &mut vec![]).unwrap();
    assert_eq!(obj.as_td_str_obj().unwrap().value(&db), "Alice");
  }

  #[test]
  fn construct_type_type() {
    let (db, project, file) = load_vault_fixture("evaluate/my_vault", "schemas/Person.td");
    let (hir, _) = lower_file(&db, project, file);
    let obj = construct_from_hir(&db, hir.unwrap(), &mut vec![]).unwrap();
    assert!(
      obj
        .as_td_product_type()
        .unwrap()
        .fields(&db)
        .contains_key("name")
    );
  }

  #[test]
  fn construct_type_type_rejects_non_schema() {
    let (db, project, file) = load_vault_fixture("evaluate/my_vault", "content/valid_person.td");
    let (hir, _) = lower_file(&db, project, file);
    assert!(TdTypeType::get(&db).construct(&db, vec![]).is_none());
    let _ = construct_from_hir(&db, hir.unwrap(), &mut vec![]);
  }

  #[test]
  fn evaluate_type_fref_resolves_referenced_type() {
    let (db, project, file) = load_vault_fixture("evaluate/my_vault", "content/with_fref.td");
    let (hir, _) = lower_file(&db, project, file);
    let hir = hir.unwrap();
    let friend_hir = match hir.kind(&db) {
      HirValueKind::Mapping(entries) => entries.into_iter().find(|(k, _)| k == "friend").unwrap().1,
      _ => panic!("expected mapping"),
    };
    let type_result = actual_node_type(&db, friend_hir);
    let typ = type_result.typ(&db).expect("fref should return a type");
    assert_eq!(typ.display_name(&db), "Person");
  }

  #[test]
  fn evaluate_type_asset_returns_blob_type() {
    let (db, project, file) = load_vault_fixture("evaluate/my_vault", "content/icon.svg");
    let symbol = file_symbol(&db, project, file).value(&db).unwrap();
    assert!(symbol.kind(&db).is_asset());
    let result = evaluate_type(&db, symbol);
    assert!(result.diagnostics(&db).is_empty());
    assert!(result.typ(&db).unwrap().is_td_blob_type());
    let obj = evaluate_resource(&db, symbol).value(&db).unwrap();
    let format_obj = obj.get_owned_field(&db, "format").unwrap();
    let format = format_obj.as_td_str_obj().unwrap();
    assert_eq!(format.value(&db), "svg");
  }

  // Enum schema where type is a union of string literals
  #[test]
  fn evaluate_enum_schema() {
    let (db, project, file) = load_vault_fixture("evaluate/my_vault", "schemas/Status.td");
    let symbol = file_symbol(&db, project, file).value(&db).unwrap();
    let result = evaluate_type(&db, symbol);
    let typ = result.typ(&db).unwrap();
    let product = typ.as_td_product_type().unwrap();
    let status_field = product.fields(&db).get("status").unwrap().clone();
    let typ = status_field.resolve(&db).unwrap();
    let sum = typ.as_td_sum_type().expect("status should be a sum type");
    assert_eq!(sum.members(&db).len(), 3, "status should have 3 members");
  }

  // Mixed union where type is a union of literal and simple types
  #[test]
  fn evaluate_mixed_union_schema() {
    let (db, project, file) = load_vault_fixture("evaluate/my_vault", "schemas/Mixed.td");
    let symbol = file_symbol(&db, project, file).value(&db).unwrap();
    let result = evaluate_type(&db, symbol);
    let typ = result.typ(&db).unwrap();
    let product = typ.as_td_product_type().unwrap();
    let value_field = product.fields(&db).get("value").unwrap().clone();
    let typ = value_field.resolve(&db).unwrap();
    let sum = typ.as_td_sum_type().expect("value should be a sum type");
    assert_eq!(sum.members(&db).len(), 3, "should have 3 members");
    let has_draft = sum.members(&db).iter().any(|m| {
      m.resolve(&db).is_some_and(|t| {
        t.as_td_literal_type()
          .is_some_and(|lit| lit.value(&db) == LiteralValue::Str("draft".to_string()))
      })
    });
    assert!(has_draft, "sum members should contain 'draft'");
  }
}
