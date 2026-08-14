//! Evaluate a schema symbol to extract the type it defines.

use std::collections::HashMap;

use crate::syntax::diagnostic::Diagnostic;
use typedown_macros::query_derived;

use crate::db::TypedownDatabase;
use crate::db::derived::get_builtin_types::{
  get_bool_type, get_date_type, get_datetime_type, get_dict_type, get_list_type, get_math_type,
  get_num_type, get_schema_type, get_str_type, get_time_type, get_type_type, instantiate_type,
};
use crate::db::derived::name_resolver::referee::referee;
use crate::db::derived::schema_property::get_schema_property_type;
use crate::db::types::{
  BuiltinSchemaKind, File, HirValue, HirValueKind, LazyType, LiteralValue, MemberType, Project,
  Symbol, SymbolKind, TdBlobType, TdProductType, TdTypeEnum, TdTypeLike, TypeMember,
  TypeMemberDescriptors, TypeResult,
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
      };
      TypeResult::new(db, Some(typ), vec![])
    }
    SymbolKind::UserDefinedSchema(project, file) => {
      evaluate_user_defined_schema(db, symbol.name(db), project, file)
    }
    SymbolKind::Asset(_, _, _) => TypeResult::new(db, Some(TdBlobType::get(db).into()), vec![]),
    SymbolKind::UserDefinedResource(_, _) | SymbolKind::BuiltinMacro(_) => {
      TypeResult::new(db, None, vec![])
    }
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
    None => {
      // Schema with no properties: empty product type
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
    if let Some((member_type, descriptors)) =
      resolve_property_descriptor(db, prop_hir, &mut diagnostics)
    {
      fields.insert(
        prop_name.clone(),
        TypeMember::new(db, member_type, descriptors),
      );
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

// Process a property descriptor like `{ type: string, optional: true }`
pub(crate) fn resolve_property_descriptor(
  db: &TypedownDatabase,
  hir: HirValue,
  diagnostics: &mut Vec<Diagnostic>,
) -> Option<(MemberType, TypeMemberDescriptors)> {
  let entries = match hir.kind(db) {
    HirValueKind::Mapping(entries) => entries,
    _ => return None,
  };

  let mut field_type: Option<MemberType> = None;
  let mut descriptors = TypeMemberDescriptors::empty();

  for (key, value) in &entries {
    match key.as_str() {
      "type" => {
        field_type = resolve_type_member(db, *value, diagnostics);
      }
      "optional" => {
        if let HirValueKind::Bool(true) = value.kind(db) {
          descriptors |= TypeMemberDescriptors::OPTIONAL;
        }
      }
      _ => {}
    }
  }

  field_type.map(|typ| (typ, descriptors))
}

fn resolve_type_member(
  db: &TypedownDatabase,
  hir: HirValue,
  diagnostics: &mut Vec<Diagnostic>,
) -> Option<MemberType> {
  match hir.kind(db) {
    // If there are some special macros, handle it here
    // HirValueKind::Call(...)

    // `!type expr` is redundant but valid: strip the tag and recurse on the inner value
    HirValueKind::Tag { tag, inner } => {
      if matches!(tag.kind(db), HirValueKind::Ident(ref name) if name == "type") {
        return resolve_type_member(db, *inner, diagnostics);
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

    // Simple type reference like `type: string`
    HirValueKind::Ident(_) => {
      let resolved = referee(db, hir);
      match resolved.value(db) {
        Some(symbol) => match symbol.kind(db) {
          SymbolKind::UserDefinedSchema(_, _) => Some(MemberType::Simple(LazyType::lazy(symbol))),
          _ => {
            let result = evaluate_type(db, symbol);
            diagnostics.extend(result.diagnostics(db).iter().cloned());
            result
              .typ(db)
              .map(|t| MemberType::Simple(LazyType::eager(t)))
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
        if let Some(member_type) = resolve_type_member(db, item, diagnostics) {
          members.push(TypeMember::new(
            db,
            member_type,
            TypeMemberDescriptors::empty(),
          ));
        }
      }
      if members.is_empty() {
        None
      } else {
        Some(MemberType::Sum(members))
      }
    }
    // Inline object like `type: { name: { type: string }, age: { type: number } }`
    // Each entry is a property descriptor with `type` and optional `optional`
    HirValueKind::Mapping(entries) => {
      let mut fields = HashMap::new();
      for (key, value_hir) in entries {
        if let Some((member_type, descriptors)) =
          resolve_property_descriptor(db, value_hir, diagnostics)
        {
          fields.insert(key.clone(), TypeMember::new(db, member_type, descriptors));
        }
      }
      Some(MemberType::Structural(fields))
    }
    // Generic type instantiation like `type: list[string]`
    HirValueKind::Index { expr, indices } => {
      let base = resolve_type_member(db, *expr, diagnostics)?;
      let base_type = match base {
        MemberType::Simple(lazy) => lazy.resolve(db)?,
        _ => return None,
      };
      if base_type.arity(db) == 0 {
        return Some(MemberType::Simple(LazyType::eager(base_type)));
      }
      let mut arg_types = vec![];
      for idx_hir in indices {
        let resolved = referee(db, idx_hir);
        match resolved.value(db) {
          Some(symbol) => match symbol.kind(db) {
            SymbolKind::UserDefinedSchema(_, _) => {
              arg_types.push(
                TdProductType::new(
                  db,
                  Some(symbol.name(db)),
                  get_schema_type(db).into(),
                  HashMap::new(),
                  HashMap::new(),
                )
                .into(),
              );
            }
            _ => {
              let result = evaluate_type(db, symbol);
              diagnostics.extend(result.diagnostics(db).iter().cloned());
              if let Some(typ) = result.typ(db) {
                arg_types.push(typ);
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
      Some(MemberType::Simple(LazyType::eager(inst_result.typ(db))))
    }
    // Literal types
    HirValueKind::Str(val) => Some(MemberType::Literal(LiteralValue::Str(val))),
    HirValueKind::Num(val) => Some(MemberType::Literal(LiteralValue::Num(val))),
    HirValueKind::Bool(val) => Some(MemberType::Literal(LiteralValue::Bool(val))),
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
    derived::typechecker::actual_node_type_member::actual_node_type_member,
    fixtures::load_vault_fixture,
    types::{
      BuiltinSchemaKind, File, FileHandle, FileMetadata, HirValue, HirValueKind, LazyType,
      LiteralValue, MemberType, Project, Symbol, SymbolKind, TdBoolObj, TdNumObj, TdProductType,
      TdStrObj, TdTypeLike, TdTypeType, TypeMember, TypeMemberDescriptors,
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

    let expected = Some(TdTypeEnum::from(get_schema_type(&db)));
    assert!(
      result.typ(&db) == expected,
      "builtin Schema symbol should evaluate to TdSchemaType"
    );
    assert!(
      result.diagnostics(&db).is_empty(),
      "expected no diagnostics"
    );
  }

  #[test]
  fn evaluate_user_defined_schema_returns_product_type() {
    let (db, project, file) = load_vault_fixture("evaluate/my_vault", "schemas/Person.td");
    let symbol = file_symbol(&db, project, file).value(&db).unwrap();

    let result = evaluate_type(&db, symbol);

    let typ = result.typ(&db).expect("should return a type");
    assert!(
      typ.is_td_product_type(),
      "user-defined schema should evaluate to TdProductType"
    );
  }

  #[test]
  fn evaluate_user_defined_schema_has_declared_fields() {
    let (db, project, file) = load_vault_fixture("evaluate/my_vault", "schemas/Person.td");
    let symbol = file_symbol(&db, project, file).value(&db).unwrap();

    let result = evaluate_type(&db, symbol);
    let typ = result.typ(&db).unwrap();
    let product = typ.as_td_product_type().expect("expected TdProductType");
    let fields = product.fields(&db);
    assert!(fields.contains_key("name"), "should have 'name' field");
    assert!(fields.contains_key("age"), "should have 'age' field");
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
      "schema with !type tags should have no diagnostics: {:?}",
      result.diagnostics(&db)
    );
    let typ = result.typ(&db).expect("should return a type");
    let product = typ.as_td_product_type().expect("expected TdProductType");
    let fields = product.fields(&db);
    assert!(fields.contains_key("name"), "should have 'name' field");
    assert!(fields.contains_key("age"), "should have 'age' field");
  }

  #[test]
  fn evaluate_type_no_properties_returns_empty_product() {
    let (db, project, file) = load_vault_fixture("evaluate/my_vault", "schemas/NoProperties.td");
    let symbol = file_symbol(&db, project, file).value(&db).unwrap();

    let result = evaluate_type(&db, symbol);
    let typ = result.typ(&db).expect("should return a type");
    let product = typ.as_td_product_type().expect("expected TdProductType");
    assert!(
      product.fields(&db).is_empty(),
      "schema with no properties should have empty fields"
    );
  }

  #[test]
  fn evaluate_type_wrong_properties_type_has_diagnostics() {
    let (db, project, file) = load_vault_fixture("evaluate/my_vault", "schemas/WrongProperties.td");
    let symbol = file_symbol(&db, project, file).value(&db).unwrap();

    let result = evaluate_type(&db, symbol);
    assert!(
      !result.diagnostics(&db).is_empty(),
      "schema with non-mapping properties should have diagnostics"
    );
  }

  #[test]
  fn evaluate_type_wrong_property_descriptor_has_diagnostics() {
    let (db, project, file) =
      load_vault_fixture("evaluate/my_vault", "schemas/WrongPropertyDescriptor.td");
    let symbol = file_symbol(&db, project, file).value(&db).unwrap();

    let result = evaluate_type(&db, symbol);
    assert!(
      !result.diagnostics(&db).is_empty(),
      "schema with unresolved property type should have diagnostics: {:?}",
      result.diagnostics(&db)
    );
  }

  #[test]
  fn evaluate_type_list_field_in_schema() {
    let (db, project, file) = load_vault_fixture("evaluate/my_vault", "schemas/WithListField.td");
    let symbol = file_symbol(&db, project, file).value(&db).unwrap();

    let result = evaluate_type(&db, symbol);
    assert!(
      result.diagnostics(&db).is_empty(),
      "schema with list[string] should have no diagnostics: {:?}",
      result.diagnostics(&db)
    );
    let typ = result.typ(&db).expect("should return a type");
    let product = typ.as_td_product_type().expect("expected TdProductType");
    let fields = product.fields(&db);
    assert!(fields.contains_key("tags"), "should have tags field");
    assert!(fields.contains_key("scores"), "should have scores field");
  }

  #[test]
  fn evaluate_type_circular_schema_refs() {
    let (db, project, file_a) = load_vault_fixture("evaluate/my_vault", "schemas/SchemaA.td");
    let symbol_a = file_symbol(&db, project, file_a).value(&db).unwrap();
    let result_a = evaluate_type(&db, symbol_a);
    assert!(
      result_a.diagnostics(&db).is_empty(),
      "circular schema A should have no diagnostics: {:?}",
      result_a.diagnostics(&db)
    );

    let file_b = project
      .files(&db)
      .iter()
      .find(|(path, _)| path.ends_with("SchemaB.td"))
      .map(|(_, f)| *f)
      .expect("SchemaB.td should exist");
    let symbol_b = file_symbol(&db, project, file_b).value(&db).unwrap();
    let result_b = evaluate_type(&db, symbol_b);
    assert!(
      result_b.diagnostics(&db).is_empty(),
      "circular schema B should have no diagnostics: {:?}",
      result_b.diagnostics(&db)
    );
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
  }

  #[test]
  fn display_name_instantiated_list() {
    let db = make_db();

    let list_str = instantiate_type(
      &db,
      get_list_type(&db).into(),
      vec![get_str_type(&db).into()],
    );
    assert_eq!(list_str.typ(&db).display_name(&db), "list[string]");
  }

  #[test]
  fn display_name_instantiated_dict() {
    let db = make_db();

    let dict_str_num = instantiate_type(
      &db,
      get_dict_type(&db).into(),
      vec![get_str_type(&db).into(), get_num_type(&db).into()],
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

    let result = evaluate_type(&db, symbol);
    let typ = result.typ(&db).unwrap();
    assert_eq!(typ.display_name(&db), "Person");
  }

  #[test]
  fn display_name_anonymous_product() {
    let db = make_db();

    let product = TdProductType::new(
      &db,
      None,
      get_type_type(&db).into(),
      HashMap::from([(
        "name".to_string(),
        TypeMember::new(
          &db,
          MemberType::Simple(LazyType::eager(get_str_type(&db).into())),
          TypeMemberDescriptors::empty(),
        ),
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
    let str_type = get_str_type(&db);
    let obj = str_type
      .construct(&db, vec![TdStrObj::new(&db, "hello".to_string()).into()])
      .expect("should construct");
    let str_obj = obj.as_td_str_obj().expect("expected TdStrObj");
    assert_eq!(str_obj.value(&db), "hello");
  }

  #[test]
  fn construct_num() {
    let db = make_db();
    let num_type = get_num_type(&db);
    let obj = num_type
      .construct(&db, vec![TdNumObj::new(&db, 42.0).into()])
      .expect("should construct");
    let num_obj = obj.as_td_num_obj().expect("expected TdNumObj");
    assert_eq!(num_obj.value(&db), 42.0);
  }

  #[test]
  fn construct_bool() {
    let db = make_db();
    let bool_type = get_bool_type(&db);
    let obj = bool_type
      .construct(&db, vec![TdBoolObj::new(&db, true).into()])
      .expect("should construct");
    let bool_obj = obj.as_td_bool_obj().expect("expected TdBoolObj");
    assert!(bool_obj.value(&db));
  }

  #[test]
  fn construct_str_returns_none_for_wrong_type() {
    let db = make_db();
    let str_type = get_str_type(&db);
    assert!(
      str_type
        .construct(&db, vec![TdNumObj::new(&db, 42.0).into()])
        .is_none(),
      "str construct should reject TdNumObj"
    );
  }

  // Product type construct from a mapping
  #[test]
  fn construct_product() {
    let (db, project, file) = load_vault_fixture("evaluate/my_vault", "content/valid_person.td");
    let (hir, _) = lower_file(&db, project, file);
    let hir = hir.unwrap();

    let obj = construct_from_hir(&db, hir, &mut vec![]).expect("should construct product");
    let name_obj = obj
      .get_owned_field(&db, "name")
      .expect("should have name field");
    let name_str = name_obj.as_td_str_obj().expect("expected TdStrObj");
    assert_eq!(name_str.value(&db), "Alice");
  }

  // List construct from a sequence
  #[test]
  fn construct_list() {
    let db = make_db();
    let list_num = instantiate_type(
      &db,
      get_list_type(&db).into(),
      vec![get_num_type(&db).into()],
    );
    let items: Vec<TdObjectEnum> = vec![
      TdNumObj::new(&db, 1.0).into(),
      TdNumObj::new(&db, 2.0).into(),
      TdNumObj::new(&db, 3.0).into(),
    ];
    assert!(list_num.typ(&db).construct(&db, items).is_some());
  }

  // Schema construct via evaluate_type
  #[test]
  fn construct_schema() {
    let (db, project, file) = load_vault_fixture("evaluate/my_vault", "schemas/Person.td");
    let symbol = file_symbol(&db, project, file).value(&db).unwrap();

    // evaluate_type uses resolve_property_descriptor and builds TdProductType
    let result = evaluate_type(&db, symbol);
    let obj = result.typ(&db).expect("should construct schema");
    let product_type = obj.as_td_product_type().expect("expected TdProductType");
    assert!(
      product_type.fields(&db).contains_key("name"),
      "should have name field"
    );
    assert!(
      product_type.fields(&db).contains_key("age"),
      "should have age field"
    );
  }

  // TdObjectType construct passes through when given exactly one arg
  #[test]
  fn construct_object_type_fallback_to_dict() {
    let db = make_db();
    let hir = make_hir(
      &db,
      r#"---
name: "Alice"
age: 42
---"#,
    );
    let val_hir = get_field_hir(&db, hir, "name");

    // construct_from_hir evaluates str HIR to TdStrObj
    let obj =
      construct_from_hir(&db, val_hir, &mut vec![]).expect("should construct via delegation");
    let str_obj = obj.as_td_str_obj().expect("expected TdStrObj");
    assert_eq!(str_obj.value(&db), "Alice");
  }

  // TdTypeType::construct returns None (HIR path lives in utils.rs)
  #[test]
  fn construct_type_type() {
    let (db, project, file) = load_vault_fixture("evaluate/my_vault", "schemas/Person.td");
    let (hir, _) = lower_file(&db, project, file);
    let hir = hir.unwrap();

    let obj = construct_from_hir(&db, hir, &mut vec![]).expect("should construct type from schema");
    let product_type = obj.as_td_product_type().expect("expected TdProductType");
    assert!(
      product_type.fields(&db).contains_key("name"),
      "should have name field"
    );
    assert!(
      product_type.fields(&db).contains_key("age"),
      "should have age field"
    );
  }

  // TdTypeType::construct returns None; construct_from_hir returns None for non-schema mappings
  #[test]
  fn construct_type_type_rejects_non_schema() {
    let (db, project, file) = load_vault_fixture("evaluate/my_vault", "content/valid_person.td");
    let (hir, _) = lower_file(&db, project, file);
    let hir = hir.unwrap();

    let type_type = TdTypeType::get(&db);
    assert!(
      type_type.construct(&db, vec![]).is_none(),
      "TdTypeType construct should return None"
    );
    // construct_from_hir on a non-schema mapping should produce a product/dict, not a type
    // (it no longer returns None, it returns a TdProductObj or TdDictObj)
    let _ = construct_from_hir(&db, hir, &mut vec![]);
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

    let type_result = actual_node_type_member(&db, friend_hir);
    let member = type_result.member(&db).expect("fref should return a type");
    let MemberType::Simple(lazy) = member.typ(&db) else {
      panic!("expected Simple type");
    };
    let typ = lazy.resolve(&db).expect("expected Simple type");
    assert_eq!(typ.display_name(&db), "Person");
  }

  // Asset symbol evaluates to TdBlobType
  #[test]
  fn evaluate_type_asset_returns_blob_type() {
    let (db, project, file) = load_vault_fixture("evaluate/my_vault", "content/icon.svg");
    let symbol = file_symbol(&db, project, file).value(&db).unwrap();

    assert!(symbol.kind(&db).is_asset(), "should be an asset symbol");

    let result = evaluate_type(&db, symbol);
    assert!(
      result.diagnostics(&db).is_empty(),
      "should have no diagnostics"
    );
    let typ = result.typ(&db).expect("should return a type");
    assert!(typ.is_td_blob_type(), "asset should evaluate to TdBlobType");
    assert_eq!(typ.display_name(&db), "blob");

    // Evaluate the asset as a resource and check the format field value
    let result = evaluate_resource(&db, symbol);
    let obj = result.value(&db).expect("should produce a blob object");
    let format = obj
      .get_owned_field(&db, "format")
      .expect("should have format field");
    let format_str = format.as_td_str_obj().expect("expected TdStrObj");
    assert_eq!(format_str.value(&db), "svg");
  }

  // Enum schema where type is a union of string literals
  #[test]
  fn evaluate_enum_schema() {
    let (db, project, file) = load_vault_fixture("evaluate/my_vault", "schemas/Status.td");
    let symbol = file_symbol(&db, project, file).value(&db).unwrap();

    let result = evaluate_type(&db, symbol);
    let typ = result.typ(&db).expect("should produce a type");
    let product = typ.as_td_product_type().expect("expected TdProductType");
    let fields = product.fields(&db);
    let status_field = fields.get("status").expect("should have status field");

    assert!(
      matches!(status_field.typ(&db), MemberType::Sum(members) if members.len() == 3),
      "status should be a union of 3 literal types"
    );
  }

  // Mixed union where type is a union of literal and simple types
  #[test]
  fn evaluate_mixed_union_schema() {
    let (db, project, file) = load_vault_fixture("evaluate/my_vault", "schemas/Mixed.td");
    let symbol = file_symbol(&db, project, file).value(&db).unwrap();

    let result = evaluate_type(&db, symbol);
    let typ = result.typ(&db).expect("should produce a type");
    let product = typ.as_td_product_type().expect("expected TdProductType");
    let fields = product.fields(&db);
    let value_field = fields.get("value").expect("should have value field");

    // Should be Sum(['draft', number, boolean])
    match value_field.typ(&db) {
      MemberType::Sum(members) => {
        assert_eq!(members.len(), 3, "should have 3 members in union");
        assert!(
          matches!(members[0].typ(&db), MemberType::Literal(LiteralValue::Str(s)) if s == "draft"),
          "first member should be literal 'draft'"
        );
        assert!(
          matches!(members[1].typ(&db), MemberType::Simple(_)),
          "second member should be a simple type"
        );
        assert!(
          matches!(members[2].typ(&db), MemberType::Simple(_)),
          "third member should be a simple type"
        );
      }
      _ => panic!("expected Sum"),
    }
  }
}
