//! Evaluate a schema symbol to extract the type it defines.

use std::collections::HashMap;

use crate::syntax::diagnostic::Diagnostic;
use typedown_macros::query_derived;

use crate::db::TypedownDatabase;
use crate::db::derived::evaluate::evaluate_node::evaluate_node;
use crate::db::derived::get_builtin_types::{
  get_bool_type, get_date_type, get_datetime_type, get_dict_type, get_list_type, get_literal_type,
  get_math_type, get_null_type, get_num_type, get_object_type, get_schema_type, get_str_type,
  get_sum_type, get_time_type, get_type_type,
};
use crate::db::derived::name_resolver::referee::referee;
use crate::db::derived::schema_property::get_schema_property_type;
use crate::db::derived::typechecker::actual_node_type::actual_node_type;
use crate::db::typecheck::utils::is_subtype_of;
use crate::db::types::{
  BuiltinSchemaKind, File, HirValue, HirValueKind, LazyType, LiteralValue, Project,
  PropertyDescriptor, RuntimeScope, Symbol, SymbolKind, TdBlobType, TdProductType, TdStaticType,
  TdStructuralType, TdTypeEnum, TypeResult,
};
use crate::db::utils::lower_file;
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
        BuiltinSchemaKind::Object => get_object_type(db).into(),
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
    if let Some(desc) = resolve_property_descriptor(db, prop_hir, &mut diagnostics) {
      fields.insert(prop_name.clone(), desc);
    }
  }

  TypeResult::new(
    db,
    Some(
      TdProductType::new(
        db,
        Some(schema_name),
        get_schema_type(db).into(),
        fields,
        HashMap::new(),
      )
      .into(),
    ),
    diagnostics,
  )
}

// Process a property descriptor like `{ type: string, default: "hello" }`
// Returns Option<PropertyDescriptor>
pub(crate) fn resolve_property_descriptor(
  db: &TypedownDatabase,
  hir: HirValue,
  diagnostics: &mut Vec<Diagnostic>,
) -> Option<PropertyDescriptor> {
  let entries = match hir.kind(db) {
    HirValueKind::Mapping(entries) => entries,
    _ => return None,
  };

  let mut field_type: Option<LazyType> = None;
  let mut default_val: Option<HirValue> = None;

  for (key, value) in &entries {
    match key.as_str() {
      "type" => {
        field_type = resolve_type_lazy(db, *value, diagnostics);
      }
      "default" => {
        default_val = Some(*value);
      }
      _ => {}
    }
  }

  if let (Some(lazy), Some(def_hir)) = (&field_type, default_val)
    && let Some(declared_type) = lazy.resolve(db)
  {
    let actual_res = actual_node_type(db, def_hir);
    diagnostics.extend(actual_res.diagnostics(db).iter().cloned());
    if actual_res
      .typ(db)
      .is_some_and(|actual_type| !is_subtype_of(db, &actual_type, &declared_type))
    {
      let node = def_hir.node(db);
      let (tr_offset, tr_len) = node.trimmed_range();
      diagnostics.push(Diagnostic::FieldTypeMismatch {
        field: "default".to_string(),
        expected: declared_type.display_name(db),
        start_offset: tr_offset,
        end_offset: tr_offset + tr_len,
      });
    }
  }

  let default_obj =
    default_val.and_then(|def_hir| evaluate_node(db, def_hir, RuntimeScope::empty(db)).value(db));

  field_type.map(|lazy| PropertyDescriptor {
    field_type: lazy,
    default_value: default_obj,
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
        if let Some(desc) = resolve_property_descriptor(db, value_hir, diagnostics) {
          fields.insert(key.clone(), desc.field_type);
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
      let inst_result = base_type.instantiate(db, arg_types);
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
  use super::*;
  use crate::db::typecheck::utils::validate_type_params;
  use crate::db::types::derived::object_system::TdStaticType;
  use crate::db::types::{
    TdObjectEnum, TdRuntimeObject, TdTypeEnum, TypeParams, TypeVariable, make_property_descriptors,
  };
  use crate::syntax::diagnostic::Diagnostic;

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
      LiteralValue, Project, RuntimeScope, Symbol, SymbolKind, TdBoolObj, TdNumObj, TdProductType,
      TdStrObj, TdStructuralType, TdTypeType,
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
  fn evaluate_type_schema_with_valid_default_fixture() {
    let (db, project, file) = load_vault_fixture("evaluate/my_vault", "schemas/DefaultValid.td");
    let symbol = file_symbol(&db, project, file).value(&db).unwrap();
    let result = evaluate_type(&db, symbol);
    assert!(
      result.diagnostics(&db).is_empty(),
      "valid default fixture should have no diagnostics: {:?}",
      result.diagnostics(&db)
    );
    assert!(result.typ(&db).is_some());
  }

  #[test]
  fn evaluate_type_schema_with_mismatched_default_fixture() {
    let (db, project, file) = load_vault_fixture("evaluate/my_vault", "schemas/DefaultInvalid.td");
    let symbol = file_symbol(&db, project, file).value(&db).unwrap();
    let result = evaluate_type(&db, symbol);
    assert_eq!(
      result.diagnostics(&db),
      &[Diagnostic::FieldTypeMismatch {
        field: "default".to_string(),
        expected: "string".to_string(),
        start_offset: 70,
        end_offset: 73,
      }]
    );
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
    let dn = |t: TdTypeEnum| t.display_name(&db);
    assert_eq!(dn(get_str_type(&db).into()), "string");
    assert_eq!(dn(get_num_type(&db).into()), "number");
    assert_eq!(dn(get_bool_type(&db).into()), "boolean");
    assert_eq!(dn(get_date_type(&db).into()), "date");
    assert_eq!(dn(get_datetime_type(&db).into()), "datetime");
    assert_eq!(dn(get_time_type(&db).into()), "time");
    assert_eq!(dn(get_list_type(&db).into()), "list");
    assert_eq!(dn(get_dict_type(&db).into()), "dict");
    assert_eq!(dn(get_type_type(&db).into()), "type");
    assert_eq!(dn(get_schema_type(&db).into()), "schema");
    assert_eq!(dn(get_never_type(&db).into()), "never");
    assert_eq!(dn(get_null_type(&db).into()), "null");
  }

  #[test]
  fn display_name_literal_types() {
    let db = make_db();
    let dn = |t: TdTypeEnum| t.display_name(&db);
    assert_eq!(
      dn(get_literal_type(&db, LiteralValue::Str("draft".to_string())).into()),
      "\"draft\""
    );
    assert_eq!(
      dn(get_literal_type(&db, LiteralValue::Num("42".to_string())).into()),
      "42"
    );
    assert_eq!(
      dn(get_literal_type(&db, LiteralValue::Bool(true)).into()),
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
    let sum_type: TdTypeEnum = sum.into();
    assert_eq!(sum_type.display_name(&db), "string | number");
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
    let structural_type: TdTypeEnum = structural.into();
    assert_eq!(structural_type.display_name(&db), "{ name: string }");
  }

  #[test]
  fn display_name_instantiated_list() {
    let db = make_db();
    let list_str = TdTypeEnum::from(get_list_type(&db))
      .instantiate(&db, vec![LazyType::eager(get_str_type(&db).into())]);
    assert_eq!(list_str.typ(&db).display_name(&db), "list[string]");
  }

  #[test]
  fn display_name_instantiated_dict() {
    let db = make_db();
    let dict_str_num = TdTypeEnum::from(get_dict_type(&db)).instantiate(
      &db,
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
  fn evaluate_type_instantiate_bounded_type_violating_bound_produces_diagnostic() {
    let db = make_db();
    let num_type = TdTypeEnum::from(get_num_type(&db));
    let str_type = TdTypeEnum::from(get_str_type(&db));

    let params = TypeParams::new(
      &db,
      vec![TypeVariable::get(&db, Some(LazyType::eager(num_type)))],
      vec![],
    );
    let diagnostics = validate_type_params(&db, Some(&params), &[LazyType::eager(str_type)]);
    assert_eq!(diagnostics.len(), 1);
    assert!(
      matches!(
        diagnostics[0],
        Diagnostic::TypeArgBoundViolation { index: 0, .. }
      ),
      "expected TypeArgBoundViolation diagnostic in evaluate_type"
    );
  }

  #[test]
  fn display_name_user_defined_schema() {
    let (db, project, file) = load_vault_fixture("evaluate/my_vault", "schemas/Person.td");
    let symbol = file_symbol(&db, project, file).value(&db).unwrap();
    let typ = evaluate_type(&db, symbol).typ(&db).unwrap();
    assert_eq!(typ.display_name(&db), "Person");
  }

  #[test]
  fn display_name_anonymous_product() {
    let db = make_db();
    let product = TdProductType::new(
      &db,
      None,
      get_type_type(&db).into(),
      make_property_descriptors(
        &db,
        HashMap::from([(
          "name".to_string(),
          LazyType::eager(get_str_type(&db).into()),
        )]),
      ),
      HashMap::new(),
    );
    let product_type: TdTypeEnum = product.into();
    assert_eq!(product_type.display_name(&db), "{ name: string }");
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
    let obj = construct_from_hir(&db, hir.unwrap(), RuntimeScope::empty(&db), &mut vec![]).unwrap();
    let name_obj = obj.get_owned_field(&db, "name").unwrap();
    let name = name_obj.as_td_str_obj().unwrap();
    assert_eq!(name.value(&db), "Alice");
  }

  // List construct from a sequence
  #[test]
  fn construct_list() {
    let db = make_db();
    let list_num = TdTypeEnum::from(get_list_type(&db))
      .instantiate(&db, vec![LazyType::eager(get_num_type(&db).into())]);
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

  #[test]
  fn construct_object_type_fallback_to_dict() {
    let db = make_db();
    let hir = make_hir(&db, "---\nname: \"Alice\"\nage: 42\n---");
    let val_hir = get_field_hir(&db, hir, "name");
    let obj = construct_from_hir(&db, val_hir, RuntimeScope::empty(&db), &mut vec![]).unwrap();
    assert_eq!(obj.as_td_str_obj().unwrap().value(&db), "Alice");
  }

  #[test]
  fn construct_type_type() {
    let (db, project, file) = load_vault_fixture("evaluate/my_vault", "schemas/Person.td");
    let (hir, _) = lower_file(&db, project, file);
    let obj = construct_from_hir(&db, hir.unwrap(), RuntimeScope::empty(&db), &mut vec![]).unwrap();
    assert!(
      obj
        .as_td_type_obj()
        .and_then(|t| t.as_td_product_type())
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
    let _ = construct_from_hir(&db, hir.unwrap(), RuntimeScope::empty(&db), &mut vec![]);
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
    let typ = status_field.field_type.resolve(&db).unwrap();
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
    let typ = value_field.field_type.resolve(&db).unwrap();
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

  #[test]
  fn evaluate_closure_call_simple_arithmetic() {
    let db = make_db();
    let hir = make_hir(
      &db,
      r#"---
result: ((x) -> x + 1)(3)
---"#,
    );
    let field = get_field_hir(&db, hir, "result");
    let obj = construct_from_hir(&db, field, RuntimeScope::empty(&db), &mut vec![]).unwrap();
    assert_eq!(obj.as_td_num_obj().unwrap().value(&db), 4.0);
  }

  #[test]
  fn evaluate_closure_call_two_params() {
    let db = make_db();
    let hir = make_hir(
      &db,
      r#"---
result: ((x, y) -> x + y)(10, 20)
---"#,
    );
    let field = get_field_hir(&db, hir, "result");
    let obj = construct_from_hir(&db, field, RuntimeScope::empty(&db), &mut vec![]).unwrap();
    assert_eq!(obj.as_td_num_obj().unwrap().value(&db), 30.0);
  }

  #[test]
  fn evaluate_closure_identity() {
    let db = make_db();
    let hir = make_hir(
      &db,
      r#"---
result: ((x) -> x)("hello")
---"#,
    );
    let field = get_field_hir(&db, hir, "result");
    let obj = construct_from_hir(&db, field, RuntimeScope::empty(&db), &mut vec![]).unwrap();
    assert_eq!(obj.as_td_str_obj().unwrap().value(&db), "hello");
  }

  // Nested closure captures outer param via RuntimeScope parent chain
  #[test]
  fn evaluate_nested_closure() {
    let db = make_db();
    let hir = make_hir(
      &db,
      r#"---
result: ((x) -> (y) -> x + y)(10)(20)
---"#,
    );
    let field = get_field_hir(&db, hir, "result");
    let obj = construct_from_hir(&db, field, RuntimeScope::empty(&db), &mut vec![]).unwrap();
    assert_eq!(obj.as_td_num_obj().unwrap().value(&db), 30.0);
  }

  #[test]
  fn evaluate_closure_as_value() {
    let db = make_db();
    let hir = make_hir(
      &db,
      r#"---
f: (x) -> x + 1
---"#,
    );
    let field = get_field_hir(&db, hir, "f");
    let obj = construct_from_hir(&db, field, RuntimeScope::empty(&db), &mut vec![]).unwrap();
    assert!(obj.as_td_func_obj().is_some());
  }

  // Closure with boolean logic
  #[test]
  fn evaluate_closure_boolean_logic() {
    let db = make_db();
    let hir = make_hir(
      &db,
      r#"---
result: ((x, y) -> x && y)(true, false)
---"#,
    );
    let field = get_field_hir(&db, hir, "result");
    let obj = construct_from_hir(&db, field, RuntimeScope::empty(&db), &mut vec![]).unwrap();
    assert!(!obj.as_td_bool_obj().unwrap().value(&db));
  }

  // Closure passed to another closure
  #[test]
  fn evaluate_closure_higher_order() {
    let db = make_db();
    let hir = make_hir(
      &db,
      r#"---
result: ((f, x) -> f(x))((x) -> x + 10, 5)
---"#,
    );
    let field = get_field_hir(&db, hir, "result");
    let obj = construct_from_hir(&db, field, RuntimeScope::empty(&db), &mut vec![]).unwrap();
    assert_eq!(obj.as_td_num_obj().unwrap().value(&db), 15.0);
  }

  // Closure with comparison
  #[test]
  fn evaluate_closure_comparison() {
    let db = make_db();
    let hir = make_hir(
      &db,
      r#"---
result: ((x) -> x > 5)(10)
---"#,
    );
    let field = get_field_hir(&db, hir, "result");
    let obj = construct_from_hir(&db, field, RuntimeScope::empty(&db), &mut vec![]).unwrap();
    assert!(obj.as_td_bool_obj().unwrap().value(&db));
  }

  // Closure referencing self evaluates correctly
  #[test]
  fn evaluate_closure_self_ref() {
    let (db, project, file) =
      load_vault_fixture("typecheck/my_vault", "content/closure_self_ref.td");
    let (hir, _) = lower_file(&db, project, file);
    let hir = hir.unwrap();
    let field = get_field_hir(&db, hir, "b");
    let obj = construct_from_hir(&db, field, RuntimeScope::empty(&db), &mut vec![]).unwrap();
    assert_eq!(obj.as_td_num_obj().unwrap().value(&db), 31.0);
  }

  // Closure captures self from defining file, not call site
  // Construct closure from TwoNums file (a: 30), extract it, call it manually
  #[test]
  fn evaluate_closure_captures_defining_file_self() {
    let (db, _project, _file) =
      load_vault_fixture("typecheck/my_vault", "content/closure_self_ref.td");
    // Construct a standalone closure that references self.a
    // Use a separate content string in the same vault so self resolves to the same file
    let closure_hir = make_hir(
      &db,
      r#"---
f: (x) -> self.a + x
---"#,
    );
    let closure_field = get_field_hir(&db, closure_hir, "f");
    // Construct closure: self resolves via referee to the closure's own file (no _type, no self.a)
    // So this should return None since self.a doesn't exist on a schemaless file
    let obj =
      construct_from_hir(&db, closure_field, RuntimeScope::empty(&db), &mut vec![]).unwrap();
    let func = obj.as_td_func_obj().unwrap();
    let result = func
      .call(&db, None, vec![TdNumObj::new(&db, 5.0).into()])
      .ok();
    // self.a is not available in the schemaless make_hir file, so call returns None
    assert!(
      result.is_none(),
      "self should bind to the defining file, not the call site"
    );
  }

  #[test]
  fn evaluate_schema_with_valid_default_no_diagnostics() {
    let db = make_db();
    let hir = make_hir(
      &db,
      r#"---
_type: schema
properties:
  age:
    type: number
    default: 42
---"#,
    );
    let mut diagnostics = vec![];
    let entries = match hir.kind(&db) {
      HirValueKind::Mapping(entries) => entries,
      _ => panic!("expected mapping"),
    };
    let (_, props_hir) = entries.iter().find(|(k, _)| k == "properties").unwrap();
    let props_entries = match props_hir.kind(&db) {
      HirValueKind::Mapping(entries) => entries,
      _ => panic!("expected mapping"),
    };
    let (_, prop_hir) = props_entries.iter().find(|(k, _)| k == "age").unwrap();

    let lazy = resolve_property_descriptor(&db, *prop_hir, &mut diagnostics);
    assert!(lazy.is_some());
    assert!(
      diagnostics.is_empty(),
      "valid default should produce no diagnostics: {:?}",
      diagnostics
    );
  }

  #[test]
  fn evaluate_schema_with_invalid_default_emits_diagnostic() {
    let db = make_db();
    let hir = make_hir(
      &db,
      r#"---
_type: schema
properties:
  age:
    type: number
    default: "not a number"
---"#,
    );
    let mut diagnostics = vec![];
    let entries = match hir.kind(&db) {
      HirValueKind::Mapping(entries) => entries,
      _ => panic!("expected mapping"),
    };
    let (_, props_hir) = entries.iter().find(|(k, _)| k == "properties").unwrap();
    let props_entries = match props_hir.kind(&db) {
      HirValueKind::Mapping(entries) => entries,
      _ => panic!("expected mapping"),
    };
    let (_, prop_hir) = props_entries.iter().find(|(k, _)| k == "age").unwrap();

    let lazy = resolve_property_descriptor(&db, *prop_hir, &mut diagnostics);
    assert!(lazy.is_some());
    assert_eq!(
      diagnostics,
      vec![Diagnostic::FieldTypeMismatch {
        field: "default".to_string(),
        expected: "number".to_string(),
        start_offset: 67,
        end_offset: 81,
      }]
    );
  }
}
