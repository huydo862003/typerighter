//! Evaluate a schema symbol to extract the type it defines.

use std::collections::BTreeMap;

use crate::syntax::diagnostic::Diagnostic;
use typedown_macros::query_derived;

use crate::db::TypedownDatabase;
use crate::db::derived::evaluate::evaluate_node::evaluate_node;
use crate::db::derived::get_builtin_typs::{
  get_bool_typ, get_date_typ, get_datetime_typ, get_dict_typ, get_list_typ, get_lit_typ,
  get_math_typ, get_null_typ, get_number_typ, get_obj_typ, get_schema_meta_typ, get_string_typ,
  get_sum_typ, get_time_typ, get_typ_typ,
};
use crate::db::derived::name_resolver::referee::referee;
use crate::db::derived::name_resolver::scope::get_file_runtime_scope;
use crate::db::derived::schema_prop::get_schema_prop_typ;
use crate::db::derived::typechecker::actual_node_typ::actual_node_typ;
use crate::db::types::typecheck::is_subtype_of;
use crate::db::types::{
  BuiltinSchemaKind, File, HirValue, HirValueKind, LazyTyp, LitValue, Project, PropertyDescriptor,
  Symbol, SymbolKind, TdBlobTyp, TdObjEnum, TdProductTyp, TdSchemaTyp, TdStaticTyp, TdTypEnum,
  TypResult,
};
use crate::db::utils::lower_file;
use std::collections::HashSet;
use typedown_incremental::QueryDatabase;
use typedown_types::either::Either;

#[query_derived(no_hash)]
pub fn evaluate_typ<'db>(db: &'db TypedownDatabase, symbol: Symbol<'db>) -> TypResult<'db> {
  match symbol.kind(db) {
    SymbolKind::BuiltinSchema(kind) => {
      let typ: TdTypEnum = match kind {
        BuiltinSchemaKind::String => get_string_typ(db).into(),
        BuiltinSchemaKind::Number => get_number_typ(db).into(),
        BuiltinSchemaKind::Bool => get_bool_typ(db).into(),
        BuiltinSchemaKind::Date => get_date_typ(db).into(),
        BuiltinSchemaKind::DateTime => get_datetime_typ(db).into(),
        BuiltinSchemaKind::Time => get_time_typ(db).into(),
        BuiltinSchemaKind::List => get_list_typ(db).into(),
        BuiltinSchemaKind::Dict => get_dict_typ(db).into(),
        BuiltinSchemaKind::Math => get_math_typ(db).into(),
        BuiltinSchemaKind::Schema => get_schema_meta_typ(db).into(),
        BuiltinSchemaKind::TypTyp => get_typ_typ(db).into(),
        BuiltinSchemaKind::SchemaProp => get_schema_prop_typ(db).into(),
        BuiltinSchemaKind::Obj => get_obj_typ(db).into(),
      };
      TypResult::new(db, Some(typ), vec![])
    }
    SymbolKind::UserDefinedSchema(project, file) => {
      evaluate_user_defined_schema(db, symbol.name(db), project, file)
    }
    SymbolKind::Asset(_, _, _) => TypResult::new(db, Some(TdBlobTyp::get(db).into()), vec![]),
    SymbolKind::UserDefinedResource(_, _)
    | SymbolKind::BuiltinMacro(_)
    | SymbolKind::BuiltinGlobal(_) => TypResult::new(db, None, vec![]),
    SymbolKind::FuncParam(_, _, _) => TypResult::new(db, None, vec![]),
  }
}

fn evaluate_user_defined_schema<'db>(
  db: &'db TypedownDatabase,
  schema_name: String,
  project: Project,
  file: File,
) -> TypResult<'db> {
  let mut diagnostics = vec![];

  // Parse file and lower frontmatter to HIR
  let (hir, _) = lower_file(db, project, file);
  let hir = match hir {
    Some(hir) => hir,
    None => return TypResult::new(db, None, vec![]),
  };

  // Extract entries from the frontmatter mapping
  let child_entries = match hir.kind(db) {
    HirValueKind::Mapping(entries) => entries,
    _ => return TypResult::new(db, None, diagnostics),
  };

  // Resolve _extends if present
  let (inherited_fields, parent_typ) =
    resolve_parent_schema(db, &schema_name, &child_entries, &mut diagnostics);

  let child_builtins: BTreeMap<String, Either<HirValue, TdObjEnum>> = child_entries
    .iter()
    .filter(|(key, _)| key.starts_with('_'))
    .map(|(key, value)| (key.clone(), Either::Left(*value)))
    .collect();

  // Find the "properties" entry
  let child_properties_hir = child_entries.iter().find(|(key, _)| key == "properties");
  let child_properties_entries = match child_properties_hir {
    Some((_, props_hir)) => match props_hir.kind(db) {
      HirValueKind::Mapping(entries) => entries,
      _ => {
        let node = props_hir.node(db);
        let (trimmed_offset, trimmed_len) = node.trimmed_range();
        diagnostics.push(Diagnostic::FieldTypMismatch {
          field: "properties".to_string(),
          expected: "mapping".to_string(),
          start_offset: trimmed_offset,
          end_offset: trimmed_offset + trimmed_len,
        });
        return TypResult::new(db, None, diagnostics);
      }
    },
    None => {
      // No own properties: return with only inherited fields
      return TypResult::new(
        db,
        Some(
          TdSchemaTyp::new(
            db,
            schema_name,
            child_builtins,
            inherited_fields,
            BTreeMap::new(),
            parent_typ,
          )
          .into(),
        ),
        diagnostics,
      );
    }
  };

  // Start with inherited fields, then overlay own fields
  let mut child_fields = inherited_fields.clone();
  let mut seen_child_props: HashSet<&str> = HashSet::new();

  for (prop_name, prop_hir) in &child_properties_entries {
    let node = prop_hir.node(db);
    let (trimmed_offset, trimmed_len) = node.trimmed_range();

    if !seen_child_props.insert(prop_name.as_str()) {
      diagnostics.push(Diagnostic::DuplicateKey {
        key: prop_name.clone(),
        start_offset: trimmed_offset,
        end_offset: trimmed_offset + trimmed_len,
      });
      continue;
    }
    if let Some(child_descriptor) = resolve_property_descriptor(db, *prop_hir, &mut diagnostics) {
      // Validate that a redefined inherited field type is a subtype of the parent field type
      if let Some(parent_descriptor) = inherited_fields.get(prop_name)
        && let (Some(child_field_typ), Some(parent_field_typ)) = (
          child_descriptor.field_typ.resolve(db),
          parent_descriptor.field_typ.resolve(db),
        )
        && !is_subtype_of(db, &child_field_typ, &parent_field_typ)
      {
        let parent_name = parent_typ
          .as_ref()
          .and_then(|typ| typ.as_td_schema_typ())
          .map(|schema| schema.name(db))
          .unwrap_or_default();
        diagnostics.push(Diagnostic::FieldRefinementViolation {
          field: prop_name.clone(),
          parent_schema: parent_name,
          expected: parent_field_typ.display_name(db),
          got: child_field_typ.display_name(db),
          start_offset: trimmed_offset,
          end_offset: trimmed_offset + trimmed_len,
        });
      }
      child_fields.insert(prop_name.clone(), child_descriptor);
    }
  }

  TypResult::new(
    db,
    Some(
      TdSchemaTyp::new(
        db,
        schema_name,
        child_builtins,
        child_fields,
        BTreeMap::new(),
        parent_typ,
      )
      .into(),
    ),
    diagnostics,
  )
}

// Resolve _extends parent, pre-walking to detect cycles
fn resolve_parent_schema<'db>(
  db: &'db TypedownDatabase,
  child_name: &str,
  child_entries: &[(String, HirValue<'db>)],
  diagnostics: &mut Vec<Diagnostic>,
) -> (
  BTreeMap<String, PropertyDescriptor<'db>>, // Inherited fields
  Option<TdTypEnum<'db>>,                    // The parent schema
) {
  let Some((_, extends_hir)) = child_entries.iter().find(|(key, _)| key == "_extends") else {
    return (BTreeMap::new(), None);
  };

  let resolved_parent_symbol_result = referee(db, *extends_hir);
  let Some(parent_symbol) = resolved_parent_symbol_result.value(db) else {
    let node = extends_hir.node(db);
    let (trimmed_offset, trimmed_len) = node.trimmed_range();
    diagnostics.push(Diagnostic::UnresolvedExtends {
      name: node.text(),
      start_offset: trimmed_offset,
      end_offset: trimmed_offset + trimmed_len,
    });
    return (BTreeMap::new(), None);
  };

  if !matches!(parent_symbol.kind(db), SymbolKind::UserDefinedSchema(..)) {
    let node = extends_hir.node(db);
    let (trimmed_offset, trimmed_len) = node.trimmed_range();
    diagnostics.push(Diagnostic::UnresolvedExtends {
      name: node.text(),
      start_offset: trimmed_offset,
      end_offset: trimmed_offset + trimmed_len,
    });
    return (BTreeMap::new(), None);
  }

  // Pre-walk the _extends chain to detect cycles before calling evaluate_typ
  if let Some(cycle) = detect_extends_cycle(db, child_name, parent_symbol) {
    diagnostics.push(Diagnostic::CircularExtension {
      name: child_name.to_string(),
      cycle,
    });
    return (BTreeMap::new(), None);
  }

  let parent_result = evaluate_typ(db, parent_symbol);
  diagnostics.extend(parent_result.diagnostics(db).iter().cloned());

  let Some(parent_typ) = parent_result.typ(db) else {
    return (BTreeMap::new(), None);
  };

  let Some(parent_schema) = parent_typ.as_td_schema_typ() else {
    return (BTreeMap::new(), None);
  };

  (parent_schema.fields(db), Some(parent_typ))
}

// Walk _extends by reading raw HIR to detect cycles without triggering evaluate_typ
fn detect_extends_cycle<'db>(
  db: &'db TypedownDatabase,
  start_name: &str,
  mut current_symbol: Symbol<'db>,
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
pub(crate) fn resolve_property_descriptor<'db>(
  db: &'db TypedownDatabase,
  hir: HirValue<'db>,
  diagnostics: &mut Vec<Diagnostic>,
) -> Option<PropertyDescriptor<'db>> {
  let kind = hir.kind(db);
  let entries = match kind {
    HirValueKind::Mapping(entries) => entries,
    _ => return None,
  };

  let mut field_typ: Option<LazyTyp> = None;
  let mut default_value: Option<HirValue> = None;
  let mut computed_func_value: Option<HirValue> = None;

  for (key, value) in &entries {
    match key.as_str() {
      "type" => {
        field_typ = resolve_typ_lazy(db, *value, diagnostics);
      }
      "default" => {
        default_value = Some(*value);
      }
      "computed" => {
        computed_func_value = Some(*value);
      }
      _ => {}
    }
  }

  if default_value.is_some()
    && let Some(computed_hir) = computed_func_value
  {
    let node = computed_hir.node(db);
    let (trimmed_offset, trimmed_len) = node.trimmed_range();
    diagnostics.push(Diagnostic::FieldTypMismatch {
      field: "computed".to_string(),
      expected: "property cannot specify both default and computed".to_string(),
      start_offset: trimmed_offset,
      end_offset: trimmed_offset + trimmed_len,
    });
    return None;
  }

  if let (Some(lazy), Some(definition_hir)) = (&field_typ, default_value)
    && let Some(declared_typ) = lazy.resolve(db)
  {
    let actual_typ_result = actual_node_typ(db, definition_hir);
    diagnostics.extend(actual_typ_result.diagnostics(db).iter().cloned());
    if actual_typ_result
      .typ(db)
      .is_some_and(|actual_typ| !is_subtype_of(db, &actual_typ, &declared_typ))
    {
      let node = definition_hir.node(db);
      let (trimmed_offset, trimmed_len) = node.trimmed_range();
      diagnostics.push(Diagnostic::FieldTypMismatch {
        field: "default".to_string(),
        expected: declared_typ.display_name(db),
        start_offset: trimmed_offset,
        end_offset: trimmed_offset + trimmed_len,
      });
    }
  }

  let default_obj = default_value.and_then(|def_hir| {
    let file_scope = get_file_runtime_scope(db, def_hir.project(db), def_hir.node(db).owner_file);
    evaluate_node(db, def_hir, file_scope).value(db)
  });

  let computed_func = computed_func_value.and_then(|computed_hir| {
    let scope = get_file_runtime_scope(
      db,
      computed_hir.project(db),
      computed_hir.node(db).owner_file,
    );
    let result = evaluate_node(db, computed_hir, scope);
    diagnostics.extend(result.diagnostics(db).iter().cloned());
    let value = result.value(db)?;
    let Some(func_obj) = value.as_td_func_obj() else {
      let node = computed_hir.node(db);
      let (trimmed_offset, trimmed_len) = node.trimmed_range();
      diagnostics.push(Diagnostic::FieldTypMismatch {
        field: "computed".to_string(),
        expected: "function".to_string(),
        start_offset: trimmed_offset,
        end_offset: trimmed_offset + trimmed_len,
      });
      return None;
    };

    let signature = func_obj.signature(db);
    let param_count = match computed_hir.kind(db) {
      HirValueKind::Closure { ref params, .. } => params.len(),
      _ => signature.params(db).len(),
    };
    if param_count != 1 {
      let node = computed_hir.node(db);
      let (trimmed_offset, trimmed_len) = node.trimmed_range();
      diagnostics.push(Diagnostic::FieldTypMismatch {
        field: "computed".to_string(),
        expected: "function expecting 1 parameter".to_string(),
        start_offset: trimmed_offset,
        end_offset: trimmed_offset + trimmed_len,
      });
      return None;
    }

    let ret_typ = match computed_hir.kind(db) {
      HirValueKind::Closure { body, .. } => actual_node_typ(db, *body).typ(db),
      _ => Some(signature.ret(db)),
    };
    if let Some(ref lazy) = field_typ
      && let Some(declared_typ) = lazy.resolve(db)
      && let Some(ret_typ) = ret_typ
      && !is_subtype_of(db, &ret_typ, &declared_typ)
    {
      let node = computed_hir.node(db);
      let (trimmed_offset, trimmed_len) = node.trimmed_range();
      diagnostics.push(Diagnostic::FieldTypMismatch {
        field: "computed".to_string(),
        expected: declared_typ.display_name(db),
        start_offset: trimmed_offset,
        end_offset: trimmed_offset + trimmed_len,
      });
    }
    Some(value)
  });

  if field_typ.is_none()
    && let Some(ref computed_enum) = computed_func
    && let Some(func_obj) = computed_enum.as_td_func_obj()
  {
    field_typ = Some(LazyTyp::eager(func_obj.signature(db).ret(db)));
  }

  field_typ.map(|lazy| PropertyDescriptor {
    field_typ: lazy,
    default_value: default_obj,
    computed_func,
  })
}

fn resolve_typ_lazy<'db>(
  db: &'db TypedownDatabase,
  hir: HirValue<'db>,
  diagnostics: &mut Vec<Diagnostic>,
) -> Option<LazyTyp<'db>> {
  match hir.kind(db) {
    // `!type expr` is redundant but valid: strip the tag and recurse on the inner value
    HirValueKind::Tag { tag, inner } => {
      if matches!(tag.kind(db), HirValueKind::Ident(ref name) if name == "type") {
        return resolve_typ_lazy(db, *inner, diagnostics);
      }
      let node = hir.node(db);
      let (trimmed_offset, trimmed_len) = node.trimmed_range();
      diagnostics.push(Diagnostic::FieldTypMismatch {
        field: "type".to_string(),
        expected: "type expression".to_string(),
        start_offset: trimmed_offset,
        end_offset: trimmed_offset + trimmed_len,
      });
      None
    }

    // Desugar A | B to Sum([A, B])
    HirValueKind::Binary { op, left, right } if op == "|" => {
      let left = resolve_typ_lazy(db, *left, diagnostics)?;
      let right = resolve_typ_lazy(db, *right, diagnostics)?;
      Some(LazyTyp::eager(get_sum_typ(db, vec![left, right]).into()))
    }
    // Desugar T? to Sum([T, null])
    HirValueKind::Postfix { op, operand } if op == "?" => {
      let inner = resolve_typ_lazy(db, *operand, diagnostics)?;
      let null_lazy = LazyTyp::eager(get_null_typ(db).into());
      Some(LazyTyp::eager(
        get_sum_typ(db, vec![inner, null_lazy]).into(),
      ))
    }

    // Simple type reference like `type: string`
    HirValueKind::Ident(_) => {
      let resolved = referee(db, hir);
      match resolved.value(db) {
        Some(symbol) => match symbol.kind(db) {
          SymbolKind::UserDefinedSchema(_, _) => Some(LazyTyp::lazy(symbol)),
          _ => {
            let result = evaluate_typ(db, symbol);
            diagnostics.extend(result.diagnostics(db).iter().cloned());
            result.typ(db).map(LazyTyp::eager)
          }
        },
        None => {
          let node = hir.node(db);
          let (trimmed_offset, trimmed_len) = node.trimmed_range();
          diagnostics.push(Diagnostic::UnresolvedSchema {
            name: node.text(),
            start_offset: trimmed_offset,
            end_offset: trimmed_offset + trimmed_len,
          });
          None
        }
      }
    }
    // Union type like `type: [string, number]`
    HirValueKind::Sequence(items) => {
      let mut members = vec![];
      for item in items {
        if let Some(lazy) = resolve_typ_lazy(db, item, diagnostics) {
          members.push(lazy);
        }
      }
      if members.is_empty() {
        None
      } else {
        Some(LazyTyp::eager(
          get_sum_typ(db, members.into_iter().collect()).into(),
        ))
      }
    }
    // Inline object like `type: { name: { type: string }, age: { type: number } }`
    HirValueKind::Mapping(entries) => {
      let mut fields = BTreeMap::new();
      for (key, value_hir) in entries {
        if let Some(descriptor) = resolve_property_descriptor(db, value_hir, diagnostics) {
          fields.insert(key.clone(), descriptor.field_typ);
        }
      }
      Some(LazyTyp::eager(TdProductTyp::new(db, None, fields).into()))
    }
    // Generic type instantiation like `type: list[string]`
    HirValueKind::Index { expr, indices } => {
      let base = resolve_typ_lazy(db, *expr, diagnostics)?;
      let base_typ = base.resolve(db)?;
      if base_typ.arity(db) == 0 {
        return Some(LazyTyp::eager(base_typ));
      }
      let mut arg_types = vec![];
      for index_hir in indices {
        arg_types.push(resolve_typ_lazy(db, index_hir, diagnostics)?);
      }
      let instantiate_result = base_typ.instantiate(db, arg_types);
      diagnostics.extend(instantiate_result.diagnostics(db).iter().cloned());
      Some(LazyTyp::eager(instantiate_result.typ(db)))
    }
    // Lit types
    HirValueKind::String(value) => Some(LazyTyp::eager(
      get_lit_typ(db, LitValue::String(value)).into(),
    )),
    HirValueKind::Number(value) => Some(LazyTyp::eager(
      get_lit_typ(db, LitValue::Number(value)).into(),
    )),
    HirValueKind::Bool(value) => Some(LazyTyp::eager(
      get_lit_typ(db, LitValue::Bool(value)).into(),
    )),
    HirValueKind::Null => Some(LazyTyp::eager(get_null_typ(db).into())),
    _ => {
      let node = hir.node(db);
      let (trimmed_offset, trimmed_len) = node.trimmed_range();
      diagnostics.push(Diagnostic::FieldTypMismatch {
        field: "type".to_string(),
        expected: "type expression".to_string(),
        start_offset: trimmed_offset,
        end_offset: trimmed_offset + trimmed_len,
      });
      None
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::db::types::typecheck::{is_subtype_of, validate_typ_params};
  use crate::db::types::{TdObjEnum, TdRuntimeObj, TdTypEnum, TypParams, TypVariable};
  use crate::syntax::diagnostic::Diagnostic;

  use std::collections::{BTreeMap, HashMap};
  use std::path::PathBuf;

  use crate::db::{
    QueryStorage, TypedownDatabase,
    derived::evaluate::evaluate_resource::evaluate_resource,
    derived::evaluate::evaluate_typ::evaluate_typ,
    derived::evaluate::utils::construct_from_hir,
    derived::get_builtin_typs::*,
    derived::name_resolver::file_symbol::file_symbol,
    derived::typechecker::actual_node_typ::actual_node_typ,
    fixtures::load_vault_fixture,
    types::{
      BuiltinSchemaKind, File, FileHandle, FileMetadata, HirValue, HirValueKind, LazyTyp, LitValue,
      Project, Symbol, SymbolKind, TdBoolObj, TdNumberObj, TdProductTyp, TdStringObj, TdTypTyp,
    },
    utils::lower_file,
  };

  fn make_db() -> TypedownDatabase {
    TypedownDatabase {
      storage: QueryStorage::default(),
    }
  }

  fn make_project(db: &TypedownDatabase) -> Project {
    Project::new(db, PathBuf::from("/test"), HashMap::new())
  }

  #[test]
  fn evaluate_typ_builtin_schema_returns_schema_typ() {
    let db = make_db();
    let symbol = Symbol::new(
      &db,
      SymbolKind::BuiltinSchema(BuiltinSchemaKind::Schema),
      "schema".to_string(),
      "@builtin::schema".to_string(),
    );
    let result = evaluate_typ(&db, symbol);
    assert!(result.typ(&db) == Some(TdTypEnum::from(get_schema_meta_typ(&db))));
    assert!(result.diagnostics(&db).is_empty());
  }

  #[test]
  fn evaluate_user_defined_schema_returns_schema_typ() {
    let (db, project, file) = load_vault_fixture("evaluate/my_vault", "_types/Person.td");
    let symbol = file_symbol(&db, project, file).value(&db).unwrap();
    let result = evaluate_typ(&db, symbol);
    assert!(result.typ(&db).unwrap().is_td_schema_typ());
  }

  #[test]
  fn evaluate_user_defined_schema_has_declared_fields() {
    let (db, project, file) = load_vault_fixture("evaluate/my_vault", "_types/Person.td");
    let symbol = file_symbol(&db, project, file).value(&db).unwrap();
    let result = evaluate_typ(&db, symbol);
    let typ = result.typ(&db).unwrap();
    let schema = typ.as_td_schema_typ().unwrap();
    assert!(schema.fields(&db).contains_key("name"));
    assert!(schema.fields(&db).contains_key("age"));
  }

  // Schema where property types use the explicit `!type` tag: `type: !type string`
  #[test]
  fn evaluate_schema_with_explicit_typ_tag() {
    let (db, project, file) =
      load_vault_fixture("evaluate/my_vault", "_types/PersonExplicitType.td");
    let symbol = file_symbol(&db, project, file).value(&db).unwrap();
    let result = evaluate_typ(&db, symbol);
    assert!(
      result.diagnostics(&db).is_empty(),
      "{:?}",
      result.diagnostics(&db)
    );
    let typ = result.typ(&db).unwrap();
    let schema = typ.as_td_schema_typ().unwrap();
    assert!(schema.fields(&db).contains_key("name"));
    assert!(schema.fields(&db).contains_key("age"));
  }

  #[test]
  fn evaluate_typ_no_properties_returns_empty_schema() {
    let (db, project, file) = load_vault_fixture("evaluate/my_vault", "_types/NoProperties.td");
    let symbol = file_symbol(&db, project, file).value(&db).unwrap();
    let result = evaluate_typ(&db, symbol);
    let typ = result.typ(&db).unwrap();
    let schema = typ.as_td_schema_typ().unwrap();
    assert!(schema.fields(&db).is_empty());
  }

  #[test]
  fn evaluate_typ_wrong_properties_typ_has_diagnostics() {
    let (db, project, file) = load_vault_fixture("evaluate/my_vault", "_types/WrongProperties.td");
    let symbol = file_symbol(&db, project, file).value(&db).unwrap();
    assert!(!evaluate_typ(&db, symbol).diagnostics(&db).is_empty());
  }

  #[test]
  fn evaluate_typ_wrong_property_descriptor_has_diagnostics() {
    let (db, project, file) =
      load_vault_fixture("evaluate/my_vault", "_types/WrongPropertyDescriptor.td");
    let symbol = file_symbol(&db, project, file).value(&db).unwrap();
    assert!(!evaluate_typ(&db, symbol).diagnostics(&db).is_empty());
  }

  #[test]
  fn evaluate_typ_duplicate_property_has_diagnostic() {
    let (db, project, file) =
      load_vault_fixture("evaluate/my_vault", "_types/DuplicateProperty.td");
    let symbol = file_symbol(&db, project, file).value(&db).unwrap();
    let diags = evaluate_typ(&db, symbol).diagnostics(&db);
    assert!(
      diags.iter().any(
        |diagnostic| matches!(diagnostic, Diagnostic::DuplicateKey { key, .. } if key == "name")
      ),
      "expected DuplicateKey diagnostic for 'name': {diags:?}"
    );
  }

  #[test]
  fn evaluate_typ_schema_with_valid_default_fixture() {
    let (db, project, file) = load_vault_fixture("evaluate/my_vault", "_types/DefaultValid.td");
    let symbol = file_symbol(&db, project, file).value(&db).unwrap();
    let result = evaluate_typ(&db, symbol);
    assert!(
      result.diagnostics(&db).is_empty(),
      "valid default fixture should have no diagnostics: {:?}",
      result.diagnostics(&db)
    );
    assert!(result.typ(&db).is_some());
  }

  #[test]
  fn evaluate_typ_schema_with_mismatched_default_fixture() {
    let (db, project, file) = load_vault_fixture("evaluate/my_vault", "_types/DefaultInvalid.td");
    let symbol = file_symbol(&db, project, file).value(&db).unwrap();
    let result = evaluate_typ(&db, symbol);
    let diags = result.diagnostics(&db);
    assert_eq!(diags.len(), 1);
    assert!(
      matches!(
        &diags[0],
        Diagnostic::FieldTypMismatch { field, expected, .. }
          if field == "default" && expected == "string"
      ),
      "expected FieldTypeMismatch for 'default', got {:?}",
      diags
    );
  }

  #[test]
  fn evaluate_typ_list_field_in_schema() {
    let (db, project, file) = load_vault_fixture("evaluate/my_vault", "_types/WithListField.td");
    let symbol = file_symbol(&db, project, file).value(&db).unwrap();
    let result = evaluate_typ(&db, symbol);
    assert!(
      result.diagnostics(&db).is_empty(),
      "{:?}",
      result.diagnostics(&db)
    );
    let typ = result.typ(&db).unwrap();
    let schema = typ.as_td_schema_typ().unwrap();
    assert!(schema.fields(&db).contains_key("tags"));
    assert!(schema.fields(&db).contains_key("scores"));
  }

  #[test]
  fn evaluate_typ_circular_schema_refs() {
    let (db, project, file_a) = load_vault_fixture("evaluate/my_vault", "_types/SchemaA.td");
    let symbol_a = file_symbol(&db, project, file_a).value(&db).unwrap();
    assert!(evaluate_typ(&db, symbol_a).diagnostics(&db).is_empty());
    let file_b = project
      .files(&db)
      .iter()
      .find(|(path, _)| path.ends_with("SchemaB.td"))
      .map(|(_, file)| *file)
      .unwrap();
    let symbol_b = file_symbol(&db, project, file_b).value(&db).unwrap();
    assert!(evaluate_typ(&db, symbol_b).diagnostics(&db).is_empty());
  }

  #[test]
  fn extends_inherits_parent_fields() {
    let (db, project, file) = load_vault_fixture("evaluate/my_vault", "_types/Student.td");
    let symbol = file_symbol(&db, project, file).value(&db).unwrap();
    let result = evaluate_typ(&db, symbol);
    assert!(
      result.diagnostics(&db).is_empty(),
      "{:?}",
      result.diagnostics(&db)
    );
    let typ = result.typ(&db).unwrap();
    let schema = typ.as_td_schema_typ().unwrap();
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
  fn extends_sets_parent_typ() {
    let (db, project, file) = load_vault_fixture("evaluate/my_vault", "_types/Student.td");
    let symbol = file_symbol(&db, project, file).value(&db).unwrap();
    let result = evaluate_typ(&db, symbol);
    let typ = result.typ(&db).unwrap();
    let schema = typ.as_td_schema_typ().unwrap();
    let parent = schema.parent_typ(&db);
    assert!(parent.is_some(), "Student should have a parent type");
    let parent_name = parent.unwrap().as_td_schema_typ().map(|p| p.name(&db));
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
    let student_typ = evaluate_typ(&db, student_symbol).typ(&db).unwrap();
    let person_typ = evaluate_typ(&db, person_symbol).typ(&db).unwrap();
    assert!(
      is_subtype_of(&db, &student_typ, &person_typ),
      "Student should be a subtype of Person"
    );
    assert!(
      !is_subtype_of(&db, &person_typ, &student_typ),
      "Person should not be a subtype of Student"
    );
  }

  #[test]
  fn extends_unresolved_emits_diagnostic() {
    let (db, project, file) = load_vault_fixture("evaluate/extends_vault", "_types/Child.td");
    let symbol = file_symbol(&db, project, file).value(&db).unwrap();
    let result = evaluate_typ(&db, symbol);
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
    let result = evaluate_typ(&db, symbol);
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
    let result = evaluate_typ(&db, symbol);
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
    let result = evaluate_typ(&db, symbol);
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
    let result = evaluate_typ(&db, symbol);
    assert!(
      result.diagnostics(&db).is_empty(),
      "{:?}",
      result.diagnostics(&db)
    );
    let fields = result
      .typ(&db)
      .unwrap()
      .as_td_schema_typ()
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
    let grad_typ = evaluate_typ(&db, grad_symbol).typ(&db).unwrap();
    let person_typ = evaluate_typ(&db, person_symbol).typ(&db).unwrap();
    assert!(is_subtype_of(&db, &grad_typ, &person_typ));
  }

  #[test]
  fn extends_builtin_typ_emits_unresolved_diagnostic() {
    // _extends: string is not a user-defined schema so it should fail
    let (db, project, file) =
      load_vault_fixture("evaluate/extends_vault", "_types/ExtendsBuiltin.td");
    let symbol = file_symbol(&db, project, file).value(&db).unwrap();
    let result = evaluate_typ(&db, symbol);
    let diags = result.diagnostics(&db);
    assert!(
      diags
        .iter()
        .any(|d| matches!(d, Diagnostic::UnresolvedExtends { .. })),
      "expected UnresolvedExtends for builtin _extends, got {diags:?}"
    );
  }

  #[test]
  fn extends_identical_field_typ_no_diagnostic() {
    // Redefining an inherited field with the same type is silently allowed
    let (db, project, file) =
      load_vault_fixture("evaluate/extends_vault", "_types/RedundantChild.td");
    let symbol = file_symbol(&db, project, file).value(&db).unwrap();
    let result = evaluate_typ(&db, symbol);
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
    let result = evaluate_typ(&db, symbol);
    assert!(
      result.diagnostics(&db).is_empty(),
      "{:?}",
      result.diagnostics(&db)
    );
    let fields = result
      .typ(&db)
      .unwrap()
      .as_td_schema_typ()
      .unwrap()
      .get_fields(&db);
    assert!(fields.contains_key("name"));
    assert!(fields.contains_key("status"));
  }

  #[test]
  fn extends_literal_narrowing_no_diagnostic() {
    // Narrowing a string field to a string literal is a valid refinement
    let (db, project, file) =
      load_vault_fixture("evaluate/extends_vault", "_types/LitNarrowChild.td");
    let symbol = file_symbol(&db, project, file).value(&db).unwrap();
    let result = evaluate_typ(&db, symbol);
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
    let result = evaluate_typ(&db, symbol);
    assert!(
      result.diagnostics(&db).is_empty(),
      "narrowing a field to a subschema should not produce diagnostics, got {:?}",
      result.diagnostics(&db)
    );
  }

  #[test]
  fn display_name_builtin_types() {
    let db = make_db();
    let get_display_name = |typ: TdTypEnum<'_>| typ.display_name(&db);
    assert_eq!(get_display_name(get_string_typ(&db).into()), "string");
    assert_eq!(get_display_name(get_number_typ(&db).into()), "number");
    assert_eq!(get_display_name(get_bool_typ(&db).into()), "boolean");
    assert_eq!(get_display_name(get_date_typ(&db).into()), "date");
    assert_eq!(get_display_name(get_datetime_typ(&db).into()), "datetime");
    assert_eq!(get_display_name(get_time_typ(&db).into()), "time");
    assert_eq!(get_display_name(get_list_typ(&db).into()), "list");
    assert_eq!(get_display_name(get_dict_typ(&db).into()), "dict");
    assert_eq!(get_display_name(get_typ_typ(&db).into()), "type");
    assert_eq!(get_display_name(get_schema_meta_typ(&db).into()), "schema");
    assert_eq!(get_display_name(get_never_typ(&db).into()), "never");
    assert_eq!(get_display_name(get_null_typ(&db).into()), "null");
  }

  #[test]
  fn display_name_literal_types() {
    let db = make_db();
    let get_display_name = |t: TdTypEnum<'_>| t.display_name(&db);
    assert_eq!(
      get_display_name(get_lit_typ(&db, LitValue::String("draft".to_string())).into()),
      "\"draft\""
    );
    assert_eq!(
      get_display_name(get_lit_typ(&db, LitValue::Number("42".to_string())).into()),
      "42"
    );
    assert_eq!(
      get_display_name(get_lit_typ(&db, LitValue::Bool(true)).into()),
      "true"
    );
  }

  #[test]
  fn display_name_sum_typ() {
    let db = make_db();
    let sum_typ = get_sum_typ(
      &db,
      vec![
        LazyTyp::eager(get_string_typ(&db).into()),
        LazyTyp::eager(get_number_typ(&db).into()),
      ],
    );
    let sum_typ: TdTypEnum = sum_typ.into();
    assert_eq!(sum_typ.display_name(&db), "string | number");
  }

  #[test]
  fn display_name_product_typ() {
    let db = make_db();
    let product_typ = TdProductTyp::new(
      &db,
      None,
      BTreeMap::from([(
        "name".to_string(),
        LazyTyp::eager(get_string_typ(&db).into()),
      )]),
    );
    let product_typ: TdTypEnum = product_typ.into();
    assert_eq!(product_typ.display_name(&db), "{ name: string }");
  }

  #[test]
  fn display_name_instantiated_list() {
    let db = make_db();
    let list_string_typ = TdTypEnum::from(get_list_typ(&db))
      .instantiate(&db, vec![LazyTyp::eager(get_string_typ(&db).into())]);
    assert_eq!(list_string_typ.typ(&db).display_name(&db), "list[string]");
  }

  #[test]
  fn display_name_instantiated_dict() {
    let db = make_db();
    let dict_string_number = TdTypEnum::from(get_dict_typ(&db)).instantiate(
      &db,
      vec![
        LazyTyp::eager(get_string_typ(&db).into()),
        LazyTyp::eager(get_number_typ(&db).into()),
      ],
    );
    assert_eq!(
      dict_string_number.typ(&db).display_name(&db),
      "dict[string, number]"
    );
  }

  #[test]
  fn evaluate_typ_instantiate_bounded_typ_violating_bound_produces_diagnostic() {
    let db = make_db();
    let number_typ = TdTypEnum::from(get_number_typ(&db));
    let string_typ = TdTypEnum::from(get_string_typ(&db));

    let params = TypParams::new(
      &db,
      vec![TypVariable::get(&db, Some(LazyTyp::eager(number_typ)))],
      vec![],
    );
    let diagnostics = validate_typ_params(&db, Some(&params), &[LazyTyp::eager(string_typ)]);
    assert_eq!(diagnostics.len(), 1);
    assert!(
      matches!(
        diagnostics[0],
        Diagnostic::TypArgBoundViolation { index: 0, .. }
      ),
      "expected TypeArgBoundViolation diagnostic in evaluate_typ"
    );
  }

  #[test]
  fn display_name_user_defined_schema() {
    let (db, project, file) = load_vault_fixture("evaluate/my_vault", "_types/Person.td");
    let symbol = file_symbol(&db, project, file).value(&db).unwrap();
    let typ = evaluate_typ(&db, symbol).typ(&db).unwrap();
    assert_eq!(typ.display_name(&db), "Person");
  }

  #[test]
  fn display_name_anonymous_product() {
    let db = make_db();
    let product = TdProductTyp::new(
      &db,
      None,
      BTreeMap::from([(
        "name".to_string(),
        LazyTyp::eager(get_string_typ(&db).into()),
      )]),
    );
    let product_typ: TdTypEnum = product.into();
    assert_eq!(product_typ.display_name(&db), "{ name: string }");
  }

  // Helper to create an HirValue from a frontmatter string
  fn make_hir<'a>(db: &'a TypedownDatabase, content: &str) -> HirValue<'a> {
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
  fn get_field_hir<'a>(db: &'a TypedownDatabase, hir: HirValue<'a>, field: &str) -> HirValue<'a> {
    match hir.kind(db) {
      HirValueKind::Mapping(entries) => {
        entries.into_iter().find(|(key, _)| key == field).unwrap().1
      }
      _ => panic!("expected mapping"),
    }
  }

  #[test]
  fn construct_str() {
    let db = make_db();
    let obj = get_string_typ(&db)
      .construct(
        &db,
        make_project(&db),
        vec![TdStringObj::new(&db, "hello".to_string()).into()],
      )
      .unwrap();
    assert_eq!(obj.as_td_string_obj().unwrap().value(&db), "hello");
  }

  #[test]
  fn construct_num() {
    let db = make_db();
    let obj = get_number_typ(&db)
      .construct(
        &db,
        make_project(&db),
        vec![TdNumberObj::new(&db, 42.0).into()],
      )
      .unwrap();
    assert_eq!(obj.as_td_number_obj().unwrap().value(&db), 42.0);
  }

  #[test]
  fn construct_bool() {
    let db = make_db();
    let obj = get_bool_typ(&db)
      .construct(
        &db,
        make_project(&db),
        vec![TdBoolObj::new(&db, true).into()],
      )
      .unwrap();
    assert!(obj.as_td_bool_obj().unwrap().value(&db));
  }

  #[test]
  fn construct_str_returns_none_for_wrong_typ() {
    let db = make_db();
    assert!(
      get_string_typ(&db)
        .construct(
          &db,
          make_project(&db),
          vec![TdNumberObj::new(&db, 42.0).into()]
        )
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
    let name = name_obj.as_td_string_obj().unwrap();
    assert_eq!(name.value(&db), "Alice");
  }

  // List construct from a sequence
  #[test]
  fn construct_list() {
    let db = make_db();
    let list_num = TdTypEnum::from(get_list_typ(&db))
      .instantiate(&db, vec![LazyTyp::eager(get_number_typ(&db).into())]);
    let items: Vec<TdObjEnum<'_>> = vec![
      TdNumberObj::new(&db, 1.0).into(),
      TdNumberObj::new(&db, 2.0).into(),
    ];
    assert!(
      list_num
        .typ(&db)
        .construct(&db, make_project(&db), items)
        .is_some()
    );
  }

  // Schema construct via evaluate_typ
  #[test]
  fn construct_schema() {
    let (db, project, file) = load_vault_fixture("evaluate/my_vault", "_types/Person.td");
    let symbol = file_symbol(&db, project, file).value(&db).unwrap();
    let typ = evaluate_typ(&db, symbol).typ(&db).unwrap();
    let schema = typ.as_td_schema_typ().unwrap();
    assert!(schema.fields(&db).contains_key("name"));
  }

  #[test]
  fn construct_object_typ_fallback_to_dict() {
    let db = make_db();
    let hir = make_hir(&db, "---\nname: \"Alice\"\nage: 42\n---");
    let value_hir = get_field_hir(&db, hir, "name");
    let scope = get_file_runtime_scope(&db, value_hir.project(&db), value_hir.node(&db).owner_file);
    let obj = construct_from_hir(&db, value_hir, scope, &mut vec![]).unwrap();
    assert_eq!(obj.as_td_string_obj().unwrap().value(&db), "Alice");
  }

  #[test]
  fn construct_typ_typ() {
    let (db, project, file) = load_vault_fixture("evaluate/my_vault", "_types/Person.td");
    let (hir, _) = lower_file(&db, project, file);
    let scope = get_file_runtime_scope(&db, project, file);
    let obj = construct_from_hir(&db, hir.unwrap(), scope, &mut vec![]).unwrap();
    assert!(
      obj
        .as_td_typ_obj()
        .and_then(|t| t.as_td_schema_typ())
        .unwrap()
        .fields(&db)
        .contains_key("name")
    );
  }

  #[test]
  fn construct_typ_typ_rejects_non_schema() {
    let (db, project, file) = load_vault_fixture("evaluate/my_vault", "valid_person.td");
    let (hir, _) = lower_file(&db, project, file);
    let scope = get_file_runtime_scope(&db, project, file);
    assert!(
      TdTypTyp::get(&db)
        .construct(&db, make_project(&db), vec![])
        .is_none()
    );
    let _ = construct_from_hir(&db, hir.unwrap(), scope, &mut vec![]);
  }

  #[test]
  fn evaluate_typ_fref_resolves_referenced_typ() {
    let (db, project, file) = load_vault_fixture("evaluate/my_vault", "with_fref.td");
    let (hir, _) = lower_file(&db, project, file);
    let hir = hir.unwrap();
    let friend_hir = match hir.kind(&db) {
      HirValueKind::Mapping(entries) => {
        entries
          .into_iter()
          .find(|(key, _)| key == "friend")
          .unwrap()
          .1
      }
      _ => panic!("expected mapping"),
    };
    let typ_result = actual_node_typ(&db, friend_hir);
    let typ = typ_result.typ(&db).expect("fref should return a type");
    assert_eq!(typ.display_name(&db), "Person");
  }

  #[test]
  fn evaluate_typ_asset_returns_blob_typ() {
    let (db, project, file) = load_vault_fixture("evaluate/my_vault", "icon.svg");
    let symbol = file_symbol(&db, project, file).value(&db).unwrap();
    assert!(symbol.kind(&db).is_asset());
    let result = evaluate_typ(&db, symbol);
    assert!(result.diagnostics(&db).is_empty());
    assert!(result.typ(&db).unwrap().is_td_blob_typ());
    let obj = evaluate_resource(&db, symbol).value(&db).unwrap();
    let format_obj = obj.get_owned_field(&db, "format").unwrap();
    let format = format_obj.as_td_string_obj().unwrap();
    assert_eq!(format.value(&db), "svg");
  }

  // Enum schema where type is a union of string literals
  #[test]
  fn evaluate_enum_schema() {
    let (db, project, file) = load_vault_fixture("evaluate/my_vault", "_types/Status.td");
    let symbol = file_symbol(&db, project, file).value(&db).unwrap();
    let result = evaluate_typ(&db, symbol);
    let typ = result.typ(&db).unwrap();
    let schema = typ.as_td_schema_typ().unwrap();
    let status_field = schema.fields(&db).get("status").unwrap().clone();
    let typ = status_field.field_typ.resolve(&db).unwrap();
    let sum = typ.as_td_sum_typ().expect("status should be a sum type");
    assert_eq!(sum.members(&db).len(), 3, "status should have 3 members");
  }

  // Mixed union where type is a union of literal and simple types
  #[test]
  fn evaluate_mixed_union_schema() {
    let (db, project, file) = load_vault_fixture("evaluate/my_vault", "_types/Mixed.td");
    let symbol = file_symbol(&db, project, file).value(&db).unwrap();
    let result = evaluate_typ(&db, symbol);
    let typ = result.typ(&db).unwrap();
    let schema = typ.as_td_schema_typ().unwrap();
    let value_field = schema.fields(&db).get("value").unwrap().clone();
    let typ = value_field.field_typ.resolve(&db).unwrap();
    let sum = typ.as_td_sum_typ().expect("value should be a sum type");
    assert_eq!(sum.members(&db).len(), 3, "should have 3 members");
    let has_draft = sum.members(&db).iter().any(|m| {
      m.resolve(&db).is_some_and(|t| {
        t.as_td_lit_typ()
          .is_some_and(|lit| lit.value(&db) == LitValue::String("draft".to_string()))
      })
    });
    assert!(has_draft, "sum members should contain 'draft'");
  }

  #[test]
  fn evaluate_pipe_enum_schema() {
    let (db, project, file) = load_vault_fixture("evaluate/my_vault", "_types/WithPipeEnum.td");
    let symbol = file_symbol(&db, project, file).value(&db).unwrap();
    let result = evaluate_typ(&db, symbol);
    assert!(
      result.diagnostics(&db).is_empty(),
      "pipe enum: {:?}",
      result.diagnostics(&db)
    );
    let typ = result.typ(&db).unwrap();
    let schema = typ.as_td_schema_typ().unwrap();
    let fields = schema.fields(&db);

    // ('draft' | 'published' | 'archived') -> sum of 3
    let parens = fields
      .get("parens")
      .unwrap()
      .field_typ
      .resolve(&db)
      .unwrap();
    assert_eq!(parens.as_td_sum_typ().unwrap().members(&db).len(), 3);

    // 'low' | 'medium' | 'high' -> sum of 3
    let bare = fields.get("bare").unwrap().field_typ.resolve(&db).unwrap();
    assert_eq!(bare.as_td_sum_typ().unwrap().members(&db).len(), 3);

    // ('draft' | 'published' | 'archived')? -> flattened to sum('draft', 'published', 'archived', null)
    let parens_option = fields
      .get("parens_optional")
      .unwrap()
      .field_typ
      .resolve(&db)
      .unwrap();
    let parens_option_sum = parens_option.as_td_sum_typ().unwrap();
    assert_eq!(parens_option_sum.members(&db).len(), 4);

    // 'low' | 'medium' | 'high'? -> 'low' | 'medium' | ('high' | null) -> flattened to 4
    let bare_option = fields
      .get("bare_optional")
      .unwrap()
      .field_typ
      .resolve(&db)
      .unwrap();
    let bare_option_sum = bare_option.as_td_sum_typ().unwrap();
    assert_eq!(bare_option_sum.members(&db).len(), 4);

    // list['frontend' | 'backend' | 'devops'] -> list type
    let pipe_list = fields
      .get("pipe_list")
      .unwrap()
      .field_typ
      .resolve(&db)
      .unwrap();
    assert!(
      pipe_list.is_td_list_typ(),
      "pipe_list: {}",
      pipe_list.display_name(&db)
    );

    // list['frontend' | 'backend' | 'devops']? -> sum(list, null)
    let pipe_list_option = fields
      .get("pipe_list_optional")
      .unwrap()
      .field_typ
      .resolve(&db)
      .unwrap();
    let pipe_list_option_sum = pipe_list_option.as_td_sum_typ().unwrap();
    assert_eq!(pipe_list_option_sum.members(&db).len(), 2);
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
    let scope = get_file_runtime_scope(&db, field.project(&db), field.node(&db).owner_file);
    let obj = construct_from_hir(&db, field, scope, &mut vec![]).unwrap();
    assert_eq!(obj.as_td_number_obj().unwrap().value(&db), 4.0);
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
    let scope = get_file_runtime_scope(&db, field.project(&db), field.node(&db).owner_file);
    let obj = construct_from_hir(&db, field, scope, &mut vec![]).unwrap();
    assert_eq!(obj.as_td_number_obj().unwrap().value(&db), 30.0);
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
    let scope = get_file_runtime_scope(&db, field.project(&db), field.node(&db).owner_file);
    let obj = construct_from_hir(&db, field, scope, &mut vec![]).unwrap();
    assert_eq!(obj.as_td_string_obj().unwrap().value(&db), "hello");
  }

  // Nested closure captures outer param via RuntimeScope parent chain
  #[test]
  fn evaluate_nested_closure() {
    // Peak stack usage is only 36KB on Linux (valgrind massif)
    // Not sure why this overflows on Windows which defaults to 1MB stack
    // Spawn with explicit 4MB stack to avoid overflow
    let result = std::thread::Builder::new()
      .stack_size(4 * 1024 * 1024)
      .spawn(|| {
        let db = make_db();
        let hir = make_hir(
          &db,
          r#"---
result: ((x) -> (y) -> x + y)(10)(20)
---"#,
        );
        let field = get_field_hir(&db, hir, "result");
        let scope = get_file_runtime_scope(&db, field.project(&db), field.node(&db).owner_file);
        let obj = construct_from_hir(&db, field, scope, &mut vec![]).unwrap();
        assert_eq!(obj.as_td_number_obj().unwrap().value(&db), 30.0);
      })
      .unwrap()
      .join();
    result.unwrap();
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
    let scope = get_file_runtime_scope(&db, field.project(&db), field.node(&db).owner_file);
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
    let scope = get_file_runtime_scope(&db, field.project(&db), field.node(&db).owner_file);
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
    let scope = get_file_runtime_scope(&db, field.project(&db), field.node(&db).owner_file);
    let obj = construct_from_hir(&db, field, scope, &mut vec![]).unwrap();
    assert_eq!(obj.as_td_number_obj().unwrap().value(&db), 15.0);
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
    let scope = get_file_runtime_scope(&db, field.project(&db), field.node(&db).owner_file);
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
      .as_td_number_obj()
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
    let b_value = resource
      .get_owned_field(&db, "b")
      .unwrap()
      .as_td_number_obj()
      .unwrap()
      .value(&db);
    assert_eq!(b_value, 31.0);
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
      vec![Diagnostic::FieldTypMismatch {
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
    let result = evaluate_typ(&db, symbol);
    assert!(result.diagnostics(&db).is_empty());
    let typ = result.typ(&db).unwrap();
    let schema = typ.as_td_schema_typ().unwrap();
    let descriptor = schema.fields(&db).get("fullName").cloned().unwrap();
    assert!(
      descriptor.computed_func.is_some(),
      "computed_fn should be populated"
    );
  }

  #[test]
  fn evaluate_schema_with_invalid_computed_ret_typ_emits_diagnostic() {
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

    let descriptor = resolve_property_descriptor(&db, *prop_hir, &mut diagnostics);
    assert!(descriptor.is_some());
    assert_eq!(
      diagnostics,
      vec![Diagnostic::FieldTypMismatch {
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

    let descriptor = resolve_property_descriptor(&db, *prop_hir, &mut diagnostics);
    assert!(descriptor.is_some());
    assert_eq!(
      diagnostics,
      vec![Diagnostic::FieldTypMismatch {
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

    let descriptor = resolve_property_descriptor(&db, *prop_hir, &mut diagnostics);
    assert!(descriptor.is_none());
    assert_eq!(
      diagnostics,
      vec![Diagnostic::FieldTypMismatch {
        field: "computed".to_string(),
        expected: "property cannot specify both default and computed".to_string(),
        start_offset: 93,
        end_offset: 108,
      }]
    );
  }
}
