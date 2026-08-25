//! Evaluate a schema symbol to extract the type it defines.

use std::collections::HashMap;

use crate::syntax::diagnostic::Diagnostic;
use typedown_macros::query_derived;

use crate::db::TypedownDatabase;
use crate::db::derived::evaluate::evaluate_node::evaluate_node;
use crate::db::derived::get_builtin_types::{
  get_bool_type, get_date_type, get_datetime_type, get_dict_type, get_list_type, get_literal_type,
  get_math_type, get_null_type, get_num_type, get_object_type, get_schema_meta_type, get_str_type,
  get_sum_type, get_time_type, get_type_type,
};
use crate::db::derived::name_resolver::referee::referee;
use crate::db::derived::name_resolver::scope::get_file_runtime_scope;
use crate::db::derived::schema_property::get_schema_property_type;
use crate::db::derived::typechecker::actual_node_type::actual_node_type;
use crate::db::typecheck::utils::is_subtype_of;
use crate::db::types::{
  BuiltinSchemaKind, File, HirValue, HirValueKind, LazyType, LiteralValue, Project,
  PropertyDescriptor, Symbol, SymbolKind, TdBlobType, TdProductType, TdSchemaType, TdStaticType,
  TdTypeEnum, TypeResult,
};
use crate::db::utils::lower_file;
use std::collections::HashSet;
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
        BuiltinSchemaKind::Schema => get_schema_meta_type(db).into(),
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

  // Resolve _extends if present
  let (inherited_fields, parent_type) =
    resolve_parent_schema(db, &schema_name, &entries, &mut diagnostics);

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
      // No own properties: return with only inherited fields
      return TypeResult::new(
        db,
        Some(
          TdSchemaType::new(
            db,
            schema_name,
            inherited_fields,
            HashMap::new(),
            parent_type,
          )
          .into(),
        ),
        diagnostics,
      );
    }
  };

  // Start with inherited fields, then overlay own fields
  let mut fields = inherited_fields.clone();

  for (prop_name, prop_hir) in &properties_entries {
    let node = prop_hir.node(db);
    let (tr_offset, tr_len) = node.trimmed_range();
    if let Some(desc) = resolve_property_descriptor(db, *prop_hir, &mut diagnostics) {
      // Validate that a redefined inherited field type is a subtype of the parent field type
      if let Some(parent_desc) = inherited_fields.get(prop_name)
        && let (Some(child_type), Some(parent_field_type)) = (
          desc.field_type.resolve(db),
          parent_desc.field_type.resolve(db),
        )
        && !is_subtype_of(db, &child_type, &parent_field_type)
      {
        let parent_name = parent_type
          .as_ref()
          .and_then(|p| p.as_td_schema_type())
          .map(|p| p.name(db))
          .unwrap_or_default();
        diagnostics.push(Diagnostic::FieldRefinementViolation {
          field: prop_name.clone(),
          parent_schema: parent_name,
          expected: parent_field_type.display_name(db),
          got: child_type.display_name(db),
          start_offset: tr_offset,
          end_offset: tr_offset + tr_len,
        });
      }
      fields.insert(prop_name.clone(), desc);
    }
  }

  TypeResult::new(
    db,
    Some(TdSchemaType::new(db, schema_name, fields, HashMap::new(), parent_type).into()),
    diagnostics,
  )
}

// Resolve _extends parent, pre-walking to detect cycles
fn resolve_parent_schema(
  db: &TypedownDatabase,
  current_name: &str,
  entries: &[(String, HirValue)],
  diagnostics: &mut Vec<Diagnostic>,
) -> (HashMap<String, PropertyDescriptor>, Option<TdTypeEnum>) {
  let Some((_, extends_hir)) = entries.iter().find(|(key, _)| key == "_extends") else {
    return (HashMap::new(), None);
  };

  let resolved = referee(db, *extends_hir);
  let Some(parent_symbol) = resolved.value(db) else {
    let node = extends_hir.node(db);
    let (tr_offset, tr_len) = node.trimmed_range();
    diagnostics.push(Diagnostic::UnresolvedExtends {
      name: node.text(),
      start_offset: tr_offset,
      end_offset: tr_offset + tr_len,
    });
    return (HashMap::new(), None);
  };

  if !matches!(parent_symbol.kind(db), SymbolKind::UserDefinedSchema(..)) {
    let node = extends_hir.node(db);
    let (tr_offset, tr_len) = node.trimmed_range();
    diagnostics.push(Diagnostic::UnresolvedExtends {
      name: node.text(),
      start_offset: tr_offset,
      end_offset: tr_offset + tr_len,
    });
    return (HashMap::new(), None);
  }

  // Pre-walk the _extends chain to detect cycles before calling evaluate_type
  if let Some(cycle) = detect_extends_cycle(db, current_name, parent_symbol) {
    diagnostics.push(Diagnostic::CircularExtension {
      name: current_name.to_string(),
      cycle,
    });
    return (HashMap::new(), None);
  }

  let parent_result = evaluate_type(db, parent_symbol);
  diagnostics.extend(parent_result.diagnostics(db).iter().cloned());

  let Some(parent_type) = parent_result.typ(db) else {
    return (HashMap::new(), None);
  };

  let Some(parent_schema) = parent_type.as_td_schema_type() else {
    return (HashMap::new(), None);
  };

  (parent_schema.fields(db), Some(parent_type))
}

// Walk _extends by reading raw HIR to detect cycles without triggering evaluate_type
fn detect_extends_cycle(
  db: &TypedownDatabase,
  start_name: &str,
  mut current_symbol: Symbol,
) -> Option<Vec<String>> {
  let mut visited = HashSet::new();
  visited.insert(start_name.to_string());

  loop {
    let name = current_symbol.name(db);
    if visited.contains(&name) {
      let mut cycle: Vec<String> = visited.into_iter().collect();
      cycle.sort();
      cycle.push(name);
      return Some(cycle);
    }
    visited.insert(name);

    let SymbolKind::UserDefinedSchema(project, file) = current_symbol.kind(db) else {
      return None;
    };

    let (hir, _) = lower_file(db, project, file);
    let hir = hir?;
    let HirValueKind::Mapping(entries) = hir.kind(db) else {
      return None;
    };
    let (_, extends_hir) = entries.iter().find(|(k, _)| k == "_extends")?;

    let resolved = referee(db, *extends_hir);
    let next_symbol = resolved.value(db)?;
    if !matches!(next_symbol.kind(db), SymbolKind::UserDefinedSchema(..)) {
      return None;
    }
    current_symbol = next_symbol;
  }
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
  let mut computed_fn_val: Option<HirValue> = None;

  for (key, value) in &entries {
    match key.as_str() {
      "type" => {
        field_type = resolve_type_lazy(db, *value, diagnostics);
      }
      "default" => {
        default_val = Some(*value);
      }
      "computed" => {
        computed_fn_val = Some(*value);
      }
      _ => {}
    }
  }

  if default_val.is_some()
    && let Some(computed_hir) = computed_fn_val
  {
    let node = computed_hir.node(db);
    let (tr_offset, tr_len) = node.trimmed_range();
    diagnostics.push(Diagnostic::FieldTypeMismatch {
      field: "computed".to_string(),
      expected: "property cannot specify both default and computed".to_string(),
      start_offset: tr_offset,
      end_offset: tr_offset + tr_len,
    });
    return None;
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

  let default_obj = default_val.and_then(|def_hir| {
    let file_scope = get_file_runtime_scope(db, def_hir.project(db), def_hir.file(db));
    evaluate_node(db, def_hir, file_scope).value(db)
  });

  let computed_fn = computed_fn_val.and_then(|computed_hir| {
    let scope = get_file_runtime_scope(db, computed_hir.project(db), computed_hir.file(db));
    let res = evaluate_node(db, computed_hir, scope);
    diagnostics.extend(res.diagnostics(db).iter().cloned());
    let val = res.value(db)?;
    let Some(func_obj) = val.as_td_func_obj() else {
      let node = computed_hir.node(db);
      let (tr_offset, tr_len) = node.trimmed_range();
      diagnostics.push(Diagnostic::FieldTypeMismatch {
        field: "computed".to_string(),
        expected: "function".to_string(),
        start_offset: tr_offset,
        end_offset: tr_offset + tr_len,
      });
      return None;
    };

    let sig = func_obj.signature(db);
    let param_count = match computed_hir.kind(db) {
      HirValueKind::Closure { ref params, .. } => params.len(),
      _ => sig.params(db).len(),
    };
    if param_count != 1 {
      let node = computed_hir.node(db);
      let (tr_offset, tr_len) = node.trimmed_range();
      diagnostics.push(Diagnostic::FieldTypeMismatch {
        field: "computed".to_string(),
        expected: "function expecting 1 parameter".to_string(),
        start_offset: tr_offset,
        end_offset: tr_offset + tr_len,
      });
      return None;
    }

    let ret_type = match computed_hir.kind(db) {
      HirValueKind::Closure { body, .. } => actual_node_type(db, *body).typ(db),
      _ => Some(sig.ret(db)),
    };
    if let Some(ref lazy) = field_type
      && let Some(declared_type) = lazy.resolve(db)
      && let Some(ret_type) = ret_type
      && !is_subtype_of(db, &ret_type, &declared_type)
    {
      let node = computed_hir.node(db);
      let (tr_offset, tr_len) = node.trimmed_range();
      diagnostics.push(Diagnostic::FieldTypeMismatch {
        field: "computed".to_string(),
        expected: declared_type.display_name(db),
        start_offset: tr_offset,
        end_offset: tr_offset + tr_len,
      });
    }
    Some(val)
  });

  if field_type.is_none()
    && let Some(ref computed_enum) = computed_fn
    && let Some(func_obj) = computed_enum.as_td_func_obj()
  {
    field_type = Some(LazyType::eager(func_obj.signature(db).ret(db)));
  }

  field_type.map(|lazy| PropertyDescriptor {
    field_type: lazy,
    default_value: default_obj,
    computed_fn,
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
      Some(LazyType::eager(TdProductType::new(db, None, fields).into()))
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
  use crate::db::typecheck::utils::{is_subtype_of, validate_type_params};
  use crate::db::types::{TdObjectEnum, TdRuntimeObject, TdTypeEnum, TypeParams, TypeVariable};
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
      LiteralValue, Project, Symbol, SymbolKind, TdBoolObj, TdNumObj, TdProductType, TdStrObj,
      TdTypeType,
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
    assert!(result.typ(&db) == Some(TdTypeEnum::from(get_schema_meta_type(&db))));
    assert!(result.diagnostics(&db).is_empty());
  }

  #[test]
  fn evaluate_user_defined_schema_returns_schema_type() {
    let (db, project, file) = load_vault_fixture("evaluate/my_vault", "_types/Person.td");
    let symbol = file_symbol(&db, project, file).value(&db).unwrap();
    let result = evaluate_type(&db, symbol);
    assert!(result.typ(&db).unwrap().is_td_schema_type());
  }

  #[test]
  fn evaluate_user_defined_schema_has_declared_fields() {
    let (db, project, file) = load_vault_fixture("evaluate/my_vault", "_types/Person.td");
    let symbol = file_symbol(&db, project, file).value(&db).unwrap();
    let result = evaluate_type(&db, symbol);
    let typ = result.typ(&db).unwrap();
    let schema = typ.as_td_schema_type().unwrap();
    assert!(schema.fields(&db).contains_key("name"));
    assert!(schema.fields(&db).contains_key("age"));
  }

  // Schema where property types use the explicit `!type` tag: `type: !type string`
  #[test]
  fn evaluate_schema_with_explicit_type_tag() {
    let (db, project, file) =
      load_vault_fixture("evaluate/my_vault", "_types/PersonExplicitType.td");
    let symbol = file_symbol(&db, project, file).value(&db).unwrap();
    let result = evaluate_type(&db, symbol);
    assert!(
      result.diagnostics(&db).is_empty(),
      "{:?}",
      result.diagnostics(&db)
    );
    let typ = result.typ(&db).unwrap();
    let schema = typ.as_td_schema_type().unwrap();
    assert!(schema.fields(&db).contains_key("name"));
    assert!(schema.fields(&db).contains_key("age"));
  }

  #[test]
  fn evaluate_type_no_properties_returns_empty_schema() {
    let (db, project, file) = load_vault_fixture("evaluate/my_vault", "_types/NoProperties.td");
    let symbol = file_symbol(&db, project, file).value(&db).unwrap();
    let result = evaluate_type(&db, symbol);
    let typ = result.typ(&db).unwrap();
    let schema = typ.as_td_schema_type().unwrap();
    assert!(schema.fields(&db).is_empty());
  }

  #[test]
  fn evaluate_type_wrong_properties_type_has_diagnostics() {
    let (db, project, file) = load_vault_fixture("evaluate/my_vault", "_types/WrongProperties.td");
    let symbol = file_symbol(&db, project, file).value(&db).unwrap();
    assert!(!evaluate_type(&db, symbol).diagnostics(&db).is_empty());
  }

  #[test]
  fn evaluate_type_wrong_property_descriptor_has_diagnostics() {
    let (db, project, file) =
      load_vault_fixture("evaluate/my_vault", "_types/WrongPropertyDescriptor.td");
    let symbol = file_symbol(&db, project, file).value(&db).unwrap();
    assert!(!evaluate_type(&db, symbol).diagnostics(&db).is_empty());
  }

  #[test]
  fn evaluate_type_schema_with_valid_default_fixture() {
    let (db, project, file) = load_vault_fixture("evaluate/my_vault", "_types/DefaultValid.td");
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
    let (db, project, file) = load_vault_fixture("evaluate/my_vault", "_types/DefaultInvalid.td");
    let symbol = file_symbol(&db, project, file).value(&db).unwrap();
    let result = evaluate_type(&db, symbol);
    let diags = result.diagnostics(&db);
    assert_eq!(diags.len(), 1);
    assert!(
      matches!(
        &diags[0],
        Diagnostic::FieldTypeMismatch { field, expected, .. }
          if field == "default" && expected == "string"
      ),
      "expected FieldTypeMismatch for 'default', got {:?}",
      diags
    );
  }

  #[test]
  fn evaluate_type_list_field_in_schema() {
    let (db, project, file) = load_vault_fixture("evaluate/my_vault", "_types/WithListField.td");
    let symbol = file_symbol(&db, project, file).value(&db).unwrap();
    let result = evaluate_type(&db, symbol);
    assert!(
      result.diagnostics(&db).is_empty(),
      "{:?}",
      result.diagnostics(&db)
    );
    let typ = result.typ(&db).unwrap();
    let schema = typ.as_td_schema_type().unwrap();
    assert!(schema.fields(&db).contains_key("tags"));
    assert!(schema.fields(&db).contains_key("scores"));
  }

  #[test]
  fn evaluate_type_circular_schema_refs() {
    let (db, project, file_a) = load_vault_fixture("evaluate/my_vault", "_types/SchemaA.td");
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
  fn extends_inherits_parent_fields() {
    let (db, project, file) = load_vault_fixture("evaluate/my_vault", "_types/Student.td");
    let symbol = file_symbol(&db, project, file).value(&db).unwrap();
    let result = evaluate_type(&db, symbol);
    assert!(
      result.diagnostics(&db).is_empty(),
      "{:?}",
      result.diagnostics(&db)
    );
    let typ = result.typ(&db).unwrap();
    let schema = typ.as_td_schema_type().unwrap();
    let fields = schema.get_fields(&db);
    // Inherited from Person
    assert!(
      fields.contains_key("name"),
      "should inherit name from Person"
    );
    assert!(fields.contains_key("age"), "should inherit age from Person");
    // Own field
    assert!(
      fields.contains_key("student_id"),
      "should have own student_id field"
    );
  }

  #[test]
  fn extends_sets_parent_type() {
    let (db, project, file) = load_vault_fixture("evaluate/my_vault", "_types/Student.td");
    let symbol = file_symbol(&db, project, file).value(&db).unwrap();
    let result = evaluate_type(&db, symbol);
    let typ = result.typ(&db).unwrap();
    let schema = typ.as_td_schema_type().unwrap();
    let parent = schema.parent_type(&db);
    assert!(parent.is_some(), "Student should have a parent type");
    let parent_name = parent.unwrap().as_td_schema_type().map(|p| p.name(&db));
    assert_eq!(parent_name.as_deref(), Some("Person"));
  }

  #[test]
  fn extends_student_is_subtype_of_person() {
    let (db, project, file) = load_vault_fixture("evaluate/my_vault", "_types/Student.td");
    let person_file = project
      .files(&db)
      .iter()
      .find(|(p, _)| p.ends_with("Person.td"))
      .map(|(_, f)| *f)
      .unwrap();
    let student_symbol = file_symbol(&db, project, file).value(&db).unwrap();
    let person_symbol = file_symbol(&db, project, person_file).value(&db).unwrap();
    let student_type = evaluate_type(&db, student_symbol).typ(&db).unwrap();
    let person_type = evaluate_type(&db, person_symbol).typ(&db).unwrap();
    assert!(
      is_subtype_of(&db, &student_type, &person_type),
      "Student should be a subtype of Person"
    );
    assert!(
      !is_subtype_of(&db, &person_type, &student_type),
      "Person should not be a subtype of Student"
    );
  }

  #[test]
  fn extends_unresolved_emits_diagnostic() {
    let (db, project, file) = load_vault_fixture("evaluate/extends_vault", "_types/Child.td");
    let symbol = file_symbol(&db, project, file).value(&db).unwrap();
    let result = evaluate_type(&db, symbol);
    let diags = result.diagnostics(&db);
    assert!(
      diags
        .iter()
        .any(|d| matches!(d, Diagnostic::UnresolvedExtends { .. })),
      "expected UnresolvedExtends diagnostic, got {diags:?}"
    );
  }

  #[test]
  fn extends_circular_emits_diagnostic() {
    let (db, project, file) = load_vault_fixture("evaluate/extends_vault", "_types/CycleA.td");
    let symbol = file_symbol(&db, project, file).value(&db).unwrap();
    let result = evaluate_type(&db, symbol);
    let diags = result.diagnostics(&db);
    assert!(
      diags
        .iter()
        .any(|d| matches!(d, Diagnostic::CircularExtension { .. })),
      "expected CircularExtension diagnostic, got {diags:?}"
    );
  }

  #[test]
  fn extends_field_widening_emits_diagnostic() {
    let (db, project, file) = load_vault_fixture("evaluate/extends_vault", "_types/WidenChild.td");
    let symbol = file_symbol(&db, project, file).value(&db).unwrap();
    let result = evaluate_type(&db, symbol);
    let diags = result.diagnostics(&db);
    assert!(
      diags.iter().any(
        |d| matches!(d, Diagnostic::FieldRefinementViolation { field, .. } if field == "status")
      ),
      "expected FieldRefinementViolation for status, got {diags:?}"
    );
  }

  #[test]
  fn extends_override_default_no_diagnostic() {
    let (db, project, file) = load_vault_fixture("evaluate/extends_vault", "_types/NarrowChild.td");
    let symbol = file_symbol(&db, project, file).value(&db).unwrap();
    let result = evaluate_type(&db, symbol);
    assert!(
      result.diagnostics(&db).is_empty(),
      "overriding default should not produce diagnostics, got {:?}",
      result.diagnostics(&db)
    );
  }

  #[test]
  fn extends_transitive_chain_inherits_all_fields() {
    let (db, project, file) = load_vault_fixture("evaluate/my_vault", "_types/GradStudent.td");
    let symbol = file_symbol(&db, project, file).value(&db).unwrap();
    let result = evaluate_type(&db, symbol);
    assert!(
      result.diagnostics(&db).is_empty(),
      "{:?}",
      result.diagnostics(&db)
    );
    let fields = result
      .typ(&db)
      .unwrap()
      .as_td_schema_type()
      .unwrap()
      .get_fields(&db);
    // Inherited transitively from Person via Student
    assert!(fields.contains_key("name"));
    assert!(fields.contains_key("age"));
    // Inherited from Student
    assert!(fields.contains_key("student_id"));
    // Own field
    assert!(fields.contains_key("thesis_topic"));
  }

  #[test]
  fn extends_transitive_nominal_subtyping() {
    let (db, project, file) = load_vault_fixture("evaluate/my_vault", "_types/GradStudent.td");
    let person_file = project
      .files(&db)
      .iter()
      .find(|(p, _)| p.ends_with("Person.td"))
      .map(|(_, f)| *f)
      .unwrap();
    let grad_symbol = file_symbol(&db, project, file).value(&db).unwrap();
    let person_symbol = file_symbol(&db, project, person_file).value(&db).unwrap();
    let grad_type = evaluate_type(&db, grad_symbol).typ(&db).unwrap();
    let person_type = evaluate_type(&db, person_symbol).typ(&db).unwrap();
    assert!(is_subtype_of(&db, &grad_type, &person_type));
  }

  #[test]
  fn extends_builtin_type_emits_unresolved_diagnostic() {
    // _extends: string is not a user-defined schema so it should fail
    let (db, project, file) =
      load_vault_fixture("evaluate/extends_vault", "_types/ExtendsBuiltin.td");
    let symbol = file_symbol(&db, project, file).value(&db).unwrap();
    let result = evaluate_type(&db, symbol);
    let diags = result.diagnostics(&db);
    assert!(
      diags
        .iter()
        .any(|d| matches!(d, Diagnostic::UnresolvedExtends { .. })),
      "expected UnresolvedExtends for builtin _extends, got {diags:?}"
    );
  }

  #[test]
  fn extends_identical_field_type_no_diagnostic() {
    // Redefining an inherited field with the same type is silently allowed
    let (db, project, file) =
      load_vault_fixture("evaluate/extends_vault", "_types/RedundantChild.td");
    let symbol = file_symbol(&db, project, file).value(&db).unwrap();
    let result = evaluate_type(&db, symbol);
    assert!(
      result.diagnostics(&db).is_empty(),
      "redundant field redefinition should not produce diagnostics, got {:?}",
      result.diagnostics(&db)
    );
  }

  #[test]
  fn extends_no_own_properties_inherits_all() {
    // A schema with _extends but no properties block still inherits all parent fields
    let (db, project, file) =
      load_vault_fixture("evaluate/extends_vault", "_types/NoPropsChild.td");
    let symbol = file_symbol(&db, project, file).value(&db).unwrap();
    let result = evaluate_type(&db, symbol);
    assert!(
      result.diagnostics(&db).is_empty(),
      "{:?}",
      result.diagnostics(&db)
    );
    let fields = result
      .typ(&db)
      .unwrap()
      .as_td_schema_type()
      .unwrap()
      .get_fields(&db);
    assert!(fields.contains_key("name"));
    assert!(fields.contains_key("status"));
  }

  #[test]
  fn extends_literal_narrowing_no_diagnostic() {
    // Narrowing a string field to a string literal is a valid refinement
    let (db, project, file) =
      load_vault_fixture("evaluate/extends_vault", "_types/LiteralNarrowChild.td");
    let symbol = file_symbol(&db, project, file).value(&db).unwrap();
    let result = evaluate_type(&db, symbol);
    assert!(
      result.diagnostics(&db).is_empty(),
      "literal narrowing should not produce diagnostics, got {:?}",
      result.diagnostics(&db)
    );
  }

  #[test]
  fn extends_field_narrowed_to_subschema_no_diagnostic() {
    // EntityChild narrows its inherited `entity: Base` field to `entity: Extended`
    // Extended _extends Base so this is a valid refinement
    let (db, project, file) = load_vault_fixture("evaluate/extends_vault", "_types/EntityChild.td");
    let symbol = file_symbol(&db, project, file).value(&db).unwrap();
    let result = evaluate_type(&db, symbol);
    assert!(
      result.diagnostics(&db).is_empty(),
      "narrowing a field to a subschema should not produce diagnostics, got {:?}",
      result.diagnostics(&db)
    );
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
    assert_eq!(dn(get_schema_meta_type(&db).into()), "schema");
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
  fn display_name_product_type() {
    let db = make_db();
    let product = TdProductType::new(
      &db,
      None,
      HashMap::from([(
        "name".to_string(),
        LazyType::eager(get_str_type(&db).into()),
      )]),
    );
    let product_type: TdTypeEnum = product.into();
    assert_eq!(product_type.display_name(&db), "{ name: string }");
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
    let (db, project, file) = load_vault_fixture("evaluate/my_vault", "_types/Person.td");
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
      HashMap::from([(
        "name".to_string(),
        LazyType::eager(get_str_type(&db).into()),
      )]),
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
    let (db, project, file) = load_vault_fixture("evaluate/my_vault", "valid_person.td");
    let (hir, _) = lower_file(&db, project, file);
    let scope = get_file_runtime_scope(&db, project, file);
    let obj = construct_from_hir(&db, hir.unwrap(), scope, &mut vec![]).unwrap();
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
    let (db, project, file) = load_vault_fixture("evaluate/my_vault", "_types/Person.td");
    let symbol = file_symbol(&db, project, file).value(&db).unwrap();
    let typ = evaluate_type(&db, symbol).typ(&db).unwrap();
    let schema = typ.as_td_schema_type().unwrap();
    assert!(schema.fields(&db).contains_key("name"));
  }

  #[test]
  fn construct_object_type_fallback_to_dict() {
    let db = make_db();
    let hir = make_hir(&db, "---\nname: \"Alice\"\nage: 42\n---");
    let val_hir = get_field_hir(&db, hir, "name");
    let scope = get_file_runtime_scope(&db, val_hir.project(&db), val_hir.file(&db));
    let obj = construct_from_hir(&db, val_hir, scope, &mut vec![]).unwrap();
    assert_eq!(obj.as_td_str_obj().unwrap().value(&db), "Alice");
  }

  #[test]
  fn construct_type_type() {
    let (db, project, file) = load_vault_fixture("evaluate/my_vault", "_types/Person.td");
    let (hir, _) = lower_file(&db, project, file);
    let scope = get_file_runtime_scope(&db, project, file);
    let obj = construct_from_hir(&db, hir.unwrap(), scope, &mut vec![]).unwrap();
    assert!(
      obj
        .as_td_type_obj()
        .and_then(|t| t.as_td_schema_type())
        .unwrap()
        .fields(&db)
        .contains_key("name")
    );
  }

  #[test]
  fn construct_type_type_rejects_non_schema() {
    let (db, project, file) = load_vault_fixture("evaluate/my_vault", "valid_person.td");
    let (hir, _) = lower_file(&db, project, file);
    let scope = get_file_runtime_scope(&db, project, file);
    assert!(TdTypeType::get(&db).construct(&db, vec![]).is_none());
    let _ = construct_from_hir(&db, hir.unwrap(), scope, &mut vec![]);
  }

  #[test]
  fn evaluate_type_fref_resolves_referenced_type() {
    let (db, project, file) = load_vault_fixture("evaluate/my_vault", "with_fref.td");
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
    let (db, project, file) = load_vault_fixture("evaluate/my_vault", "icon.svg");
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
    let (db, project, file) = load_vault_fixture("evaluate/my_vault", "_types/Status.td");
    let symbol = file_symbol(&db, project, file).value(&db).unwrap();
    let result = evaluate_type(&db, symbol);
    let typ = result.typ(&db).unwrap();
    let schema = typ.as_td_schema_type().unwrap();
    let status_field = schema.fields(&db).get("status").unwrap().clone();
    let typ = status_field.field_type.resolve(&db).unwrap();
    let sum = typ.as_td_sum_type().expect("status should be a sum type");
    assert_eq!(sum.members(&db).len(), 3, "status should have 3 members");
  }

  // Mixed union where type is a union of literal and simple types
  #[test]
  fn evaluate_mixed_union_schema() {
    let (db, project, file) = load_vault_fixture("evaluate/my_vault", "_types/Mixed.td");
    let symbol = file_symbol(&db, project, file).value(&db).unwrap();
    let result = evaluate_type(&db, symbol);
    let typ = result.typ(&db).unwrap();
    let schema = typ.as_td_schema_type().unwrap();
    let value_field = schema.fields(&db).get("value").unwrap().clone();
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
    let scope = get_file_runtime_scope(&db, field.project(&db), field.file(&db));
    let obj = construct_from_hir(&db, field, scope, &mut vec![]).unwrap();
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
    let scope = get_file_runtime_scope(&db, field.project(&db), field.file(&db));
    let obj = construct_from_hir(&db, field, scope, &mut vec![]).unwrap();
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
    let scope = get_file_runtime_scope(&db, field.project(&db), field.file(&db));
    let obj = construct_from_hir(&db, field, scope, &mut vec![]).unwrap();
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
    let scope = get_file_runtime_scope(&db, field.project(&db), field.file(&db));
    let obj = construct_from_hir(&db, field, scope, &mut vec![]).unwrap();
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
    let scope = get_file_runtime_scope(&db, field.project(&db), field.file(&db));
    let obj = construct_from_hir(&db, field, scope, &mut vec![]).unwrap();
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
    let scope = get_file_runtime_scope(&db, field.project(&db), field.file(&db));
    let obj = construct_from_hir(&db, field, scope, &mut vec![]).unwrap();
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
    let scope = get_file_runtime_scope(&db, field.project(&db), field.file(&db));
    let obj = construct_from_hir(&db, field, scope, &mut vec![]).unwrap();
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
    let scope = get_file_runtime_scope(&db, field.project(&db), field.file(&db));
    let obj = construct_from_hir(&db, field, scope, &mut vec![]).unwrap();
    assert!(obj.as_td_bool_obj().unwrap().value(&db));
  }

  // Closure referencing self evaluates correctly
  #[test]
  fn evaluate_closure_self_ref() {
    let (db, project, file) = load_vault_fixture("typecheck/my_vault", "closure_self_ref.td");
    let symbol = file_symbol(&db, project, file).value(&db).unwrap();
    let resource = evaluate_resource(&db, symbol).value(&db).unwrap();
    let b_val = resource
      .get_owned_field(&db, "b")
      .unwrap()
      .as_td_num_obj()
      .unwrap()
      .value(&db);
    assert_eq!(b_val, 31.0);
  }

  // Closure captures self from defining file, not call site
  // Construct closure from TwoNums file (a: 30), extract it, call it manually
  #[test]
  fn evaluate_closure_captures_defining_file_self() {
    let (db, project, file) = load_vault_fixture("typecheck/my_vault", "closure_self_ref.td");
    let symbol = file_symbol(&db, project, file).value(&db).unwrap();
    let resource = evaluate_resource(&db, symbol).value(&db).unwrap();
    let b_val = resource
      .get_owned_field(&db, "b")
      .unwrap()
      .as_td_num_obj()
      .unwrap()
      .value(&db);
    assert_eq!(b_val, 31.0);
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

  #[test]
  fn evaluate_schema_with_computed_field() {
    let (db, project, file) = load_vault_fixture("evaluate/my_vault", "_types/ComputedValid.td");
    let symbol = file_symbol(&db, project, file).value(&db).unwrap();
    let result = evaluate_type(&db, symbol);
    assert!(result.diagnostics(&db).is_empty());
    let typ = result.typ(&db).unwrap();
    let schema = typ.as_td_schema_type().unwrap();
    let desc = schema.fields(&db).get("fullName").cloned().unwrap();
    assert!(
      desc.computed_fn.is_some(),
      "computed_fn should be populated"
    );
  }

  #[test]
  fn evaluate_schema_with_invalid_computed_return_type_emits_diagnostic() {
    let db = make_db();
    let hir = make_hir(
      &db,
      r#"---
_type: schema
properties:
  fullName:
    type: number
    computed: (r) -> "hello"
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
    let (_, prop_hir) = props_entries.iter().find(|(k, _)| k == "fullName").unwrap();

    let desc = resolve_property_descriptor(&db, *prop_hir, &mut diagnostics);
    assert!(desc.is_some());
    assert_eq!(
      diagnostics,
      vec![Diagnostic::FieldTypeMismatch {
        field: "computed".to_string(),
        expected: "number".to_string(),
        start_offset: 72,
        end_offset: 87,
      }]
    );
  }

  #[test]
  fn evaluate_schema_with_invalid_computed_param_count_emits_diagnostic() {
    let db = make_db();
    let hir = make_hir(
      &db,
      r#"---
_type: schema
properties:
  fullName:
    type: string
    computed: (a, b) -> a + b
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
    let (_, prop_hir) = props_entries.iter().find(|(k, _)| k == "fullName").unwrap();

    let desc = resolve_property_descriptor(&db, *prop_hir, &mut diagnostics);
    assert!(desc.is_some());
    assert_eq!(
      diagnostics,
      vec![Diagnostic::FieldTypeMismatch {
        field: "computed".to_string(),
        expected: "function expecting 1 parameter".to_string(),
        start_offset: 72,
        end_offset: 88,
      }]
    );
  }

  #[test]
  fn evaluate_schema_with_default_and_computed_emits_diagnostic() {
    let db = make_db();
    let hir = make_hir(
      &db,
      r#"---
_type: schema
properties:
  fullName:
    type: string
    default: "Alice"
    computed: (r) -> "hello"
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
    let (_, prop_hir) = props_entries.iter().find(|(k, _)| k == "fullName").unwrap();

    let desc = resolve_property_descriptor(&db, *prop_hir, &mut diagnostics);
    assert!(desc.is_none());
    assert_eq!(
      diagnostics,
      vec![Diagnostic::FieldTypeMismatch {
        field: "computed".to_string(),
        expected: "property cannot specify both default and computed".to_string(),
        start_offset: 93,
        end_offset: 108,
      }]
    );
  }
}
