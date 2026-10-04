use std::collections::BTreeMap;

use crate::db::TypedownDatabase;
use crate::db::derived::evaluate::evaluate_node::evaluate_node;
use crate::db::derived::evaluate::evaluate_resource::evaluate_resource;
use crate::db::derived::evaluate::evaluate_typ::{evaluate_typ, resolve_property_descriptor};
use crate::db::derived::get_builtin_typs::{get_never_typ, get_null_typ, get_sum_typ};
use crate::db::derived::get_vault_config::get_vault_config;
use crate::db::derived::icon::{ICON_ENTRIES, get_icon_module_typ};
use crate::db::derived::name_resolver::file_symbol::file_symbol;
use crate::db::derived::name_resolver::referee::referee;
use crate::db::derived::typechecker::actual_node_typ::actual_node_typ;
use crate::db::types::{
  BuiltinGlobalKind, BuiltinMacroKind, FuncKind, HirValue, HirValueKind, InterpolatedPart, LazyTyp,
  PropertyDescriptor, RuntimeScope, SymbolKind, TdBoolObj, TdDictObj, TdFuncObj, TdFuncTyp,
  TdIconObj, TdListObj, TdMathObj, TdNullObj, TdNumberObj, TdObjEnum, TdProductObj, TdRuntimeObj,
  TdSchemaObj, TdSchemaTyp, TdStaticTyp, TdStringObj, TdTypEnum, TdVaultObj,
};
use crate::syntax::diagnostic::Diagnostic;
use typedown_types::either::Either;

pub(crate) fn construct_from_hir<'db>(
  db: &'db TypedownDatabase,
  hir: HirValue<'db>,
  runtime_scope: RuntimeScope<'db>,
  diagnostics: &mut Vec<Diagnostic>,
) -> Option<TdObjEnum<'db>> {
  match hir.kind(db) {
    HirValueKind::Null => {
      return Some(TdNullObj::get(db).into());
    }
    // Ident: check runtime scope first (for closure params), then normal resolution
    HirValueKind::Ident(ref name) => {
      if let Some(obj) = runtime_scope.lookup(db, name) {
        return Some(obj);
      }
      let resolved = referee(db, hir);
      if let Some(symbol) = resolved.value(db) {
        match symbol.kind(db) {
          SymbolKind::BuiltinGlobal(kind) => {
            return match kind {
              BuiltinGlobalKind::Vault => Some(TdVaultObj::new(db, hir.project(db)).into()),
              BuiltinGlobalKind::Icon => {
                let mut fields = BTreeMap::new();
                for entry in ICON_ENTRIES {
                  let obj =
                    TdIconObj::new(db, entry.name.to_string(), entry.lucide_name.to_string());
                  fields.insert(entry.name.to_string(), Either::Right(obj.into()));
                }
                let module_typ = get_icon_module_typ(db).into();
                Some(TdProductObj::new(db, module_typ, None, BTreeMap::new(), fields).into())
              }
            };
          }
          // Schema identifiers evaluate to the schema type as an object
          SymbolKind::UserDefinedSchema(_, _) | SymbolKind::BuiltinSchema(_) => {
            return evaluate_typ(db, symbol).typ(db).map(TdObjEnum::from);
          }
          // Resource identifiers (including self) evaluate to the resource object
          SymbolKind::UserDefinedResource(_, _) => {
            let resource = evaluate_resource(db, symbol);
            return resource.value(db);
          }
          _ => {}
        }
      }
    }
    // Tag expressions: the tag is a type hint for the typechecker
    HirValueKind::Tag { inner, .. } => {
      let node = evaluate_node(db, *inner, runtime_scope);
      return node.value(db);
    }
    // Field access: obj.field
    HirValueKind::Binary { op, left, right } if op == "." => {
      if let HirValueKind::Ident(field_name) = right.kind(db) {
        let left_node = evaluate_node(db, *left, runtime_scope);
        let this = left_node.value(db)?;
        return this.lookup_field(db, &field_name);
      }
    }
    // Arithmetic, comparison, and logical binary operators
    HirValueKind::Binary { op, left, right } => {
      return evaluate_binary(db, &op, *left, *right, runtime_scope);
    }
    // Prefix operators
    HirValueKind::Prefix { op, operand } => {
      return evaluate_prefix(db, &op, *operand, runtime_scope);
    }
    // Postfix operators
    HirValueKind::Postfix { op, operand } => {
      return evaluate_postfix(db, &op, *operand, runtime_scope);
    }
    // Index access: list[n] or dict["key"]
    HirValueKind::Index { expr, indices } => {
      return evaluate_index(db, *expr, indices, runtime_scope, diagnostics);
    }
    HirValueKind::Call { callee, args } => {
      match callee.kind(db) {
        // Method call: obj.method(args)
        HirValueKind::Binary { op, left, right } if op == "." => {
          if let HirValueKind::Ident(method_name) = right.kind(db) {
            let left_node = evaluate_node(db, *left, runtime_scope);
            let this = left_node.value(db)?;
            let func_obj = this.lookup_method(db, &method_name)?;
            let arg_objs: Vec<_> = args
              .into_iter()
              .filter_map(|arg| {
                let node = evaluate_node(db, arg, runtime_scope);
                node.value(db)
              })
              .collect();
            let scope = runtime_scope.scope(db);
            let project = scope.project(db);
            return func_obj.call(db, project, Some(this), arg_objs).ok();
          }
        }
        // Macro calls: pass raw HIR args (macros need project context from HIR)
        _ => {
          let resolved = referee(db, *callee);
          if let Some(symbol) = resolved.value(db)
            && let SymbolKind::BuiltinMacro(kind) = symbol.kind(db)
          {
            return construct_macro(db, kind, args);
          }
          // Plain function call: evaluate callee, call it via protocol
          let callee_node = evaluate_node(db, *callee, runtime_scope);
          let callee_obj = callee_node.value(db)?;
          let arg_objs: Vec<_> = args
            .into_iter()
            .filter_map(|arg| {
              let node = evaluate_node(db, arg, runtime_scope);
              node.value(db)
            })
            .collect();
          let scope = runtime_scope.scope(db);
          let project = scope.project(db);
          return callee_obj.call(db, project, None, arg_objs).ok();
        }
      }
    }
    // Closure: create a TdFuncObj capturing the defining scope
    HirValueKind::Closure { ref params, .. } => {
      let func_typ = match actual_node_typ(db, hir).typ(db) {
        Some(TdTypEnum::TdFuncTyp(f)) => f,
        // No expected type context: assume never for params and return
        _ => {
          let never: TdTypEnum = get_never_typ(db).into();
          let param_typs = vec![never.clone(); params.len()];
          TdFuncTyp::get(db, param_typs, never)
        }
      };
      let func_obj = TdFuncObj::new(
        db,
        "<closure>".to_string(),
        func_typ.signature(db),
        FuncKind::UserDefined(hir, runtime_scope),
      );
      return Some(func_obj.into());
    }
    _ => {}
  }

  // Anonymous mappings become product objects
  let typ_result = actual_node_typ(db, hir);
  if let HirValueKind::Mapping(entries) = hir.kind(db)
    && typ_result.typ(db).is_some_and(|t| t.is_td_product_typ())
  {
    let mut builtins = BTreeMap::new();
    let mut fields = BTreeMap::new();
    for (key, value_hir) in entries {
      if key.starts_with('_') {
        builtins.insert(key, Either::Left(value_hir));
      } else {
        fields.insert(key, Either::Left(value_hir));
      }
    }
    let product_typ = typ_result.typ(db).unwrap();
    return Some(TdProductObj::new(db, product_typ, None, builtins, fields).into());
  }

  // Normal construction: convert HIR to args, then call construct
  let raw_typ = typ_result.typ(db)?;
  let typ = match raw_typ.runtime_typ(db) {
    Some(typ) => typ,
    None => {
      let (start, len) = hir.node(db).trimmed_range();
      diagnostics.push(Diagnostic::NotConstructible {
        type_name: raw_typ.display_name(db),
        start_offset: start,
        end_offset: start + len,
      });
      return None;
    }
  };
  match hir.kind(db) {
    HirValueKind::String(val) => {
      typ.construct(db, hir.project(db), vec![TdStringObj::new(db, val).into()])
    }
    HirValueKind::Number(val) => {
      let num: f64 = val.parse().unwrap_or(0.0);
      typ.construct(db, hir.project(db), vec![TdNumberObj::new(db, num).into()])
    }
    HirValueKind::Bool(val) => {
      typ.construct(db, hir.project(db), vec![TdBoolObj::new(db, val).into()])
    }
    HirValueKind::Math(val) => {
      typ.construct(db, hir.project(db), vec![TdMathObj::new(db, val).into()])
    }
    HirValueKind::Interpolated(parts) => {
      let obj = evaluate_interpolated(db, runtime_scope, parts)?;
      typ.construct(db, hir.project(db), vec![obj])
    }
    HirValueKind::Sequence(items) => {
      if typ.is_td_list_typ() {
        let hir_items = items.into_iter().map(Either::Left).collect();
        return Some(TdListObj::new(db, hir_items).into());
      }
      let args: Vec<_> = items
        .into_iter()
        .filter_map(|item| evaluate_node(db, item, runtime_scope).value(db))
        .collect();
      typ.construct(db, hir.project(db), args)
    }
    HirValueKind::Mapping(entries) => evaluate_mapping(db, &typ, entries),
    HirValueKind::Markdown(parts) => {
      let obj = evaluate_interpolated(db, runtime_scope, parts)?;
      typ.construct(db, hir.project(db), vec![obj])
    }
    _ => None,
  }
}

fn evaluate_prefix<'db>(
  db: &'db TypedownDatabase,
  op: &str,
  operand: HirValue<'db>,
  runtime_scope: RuntimeScope<'db>,
) -> Option<TdObjEnum<'db>> {
  let operand_obj = evaluate_node(db, operand, runtime_scope).value(db)?;
  match op {
    "-" | "+" => {
      let num = operand_obj.as_td_number_obj()?;
      let val = num.value(db);
      let result = match op {
        "-" => -val,
        "+" => val,
        _ => unreachable!(),
      };
      Some(TdNumberObj::new(db, result).into())
    }
    // Logical not: only null and false are falsy, everything else is truthy
    "~" => {
      let is_falsy = operand_obj.as_td_bool_obj().is_some_and(|b| !b.value(db));
      Some(TdBoolObj::new(db, is_falsy).into())
    }
    _ => None,
  }
}

fn evaluate_postfix<'db>(
  db: &'db TypedownDatabase,
  op: &str,
  operand: HirValue<'db>,
  runtime_scope: RuntimeScope<'db>,
) -> Option<TdObjEnum<'db>> {
  match op {
    // T? evaluates to Sum([T, null]) as a type object
    "?" => {
      let inner = evaluate_node(db, operand, runtime_scope).value(db)?;
      let inner_typ = inner.into_td_typ_obj().ok()?;
      Some(
        get_sum_typ(
          db,
          vec![
            LazyTyp::eager(inner_typ),
            LazyTyp::eager(get_null_typ(db).into()),
          ],
        )
        .into(),
      )
    }
    _ => None,
  }
}

fn evaluate_binary<'db>(
  db: &'db TypedownDatabase,
  op: &str,
  left: HirValue<'db>,
  right: HirValue<'db>,
  runtime_scope: RuntimeScope<'db>,
) -> Option<TdObjEnum<'db>> {
  let left_obj = evaluate_node(db, left, runtime_scope).value(db)?;
  let right_obj = evaluate_node(db, right, runtime_scope).value(db)?;
  match op {
    "+" | "-" | "*" | "/" | "%" | "**" => {
      let lnum = left_obj.as_td_number_obj()?;
      let rnum = right_obj.as_td_number_obj()?;
      let lval = lnum.value(db);
      let rval = rnum.value(db);
      let result = match op {
        "+" => lval + rval,
        "-" => lval - rval,
        "*" => lval * rval,
        "/" => lval / rval,
        "%" => lval % rval,
        "**" => lval.powf(rval),
        _ => unreachable!(),
      };
      Some(TdNumberObj::new(db, result).into())
    }
    "==" | "!=" | "<" | ">" | "<=" | ">=" => {
      let result = compare_objects(db, op, &left_obj, &right_obj);
      Some(TdBoolObj::new(db, result).into())
    }
    "&&" | "||" => {
      let lbool = left_obj.as_td_bool_obj()?;
      let rbool = right_obj.as_td_bool_obj()?;
      let result = match op {
        "&&" => lbool.value(db) && rbool.value(db),
        "||" => lbool.value(db) || rbool.value(db),
        _ => unreachable!(),
      };
      Some(TdBoolObj::new(db, result).into())
    }
    _ => None,
  }
}

fn compare_objects<'db>(
  db: &'db TypedownDatabase,
  op: &str,
  left: &TdObjEnum<'db>,
  right: &TdObjEnum<'db>,
) -> bool {
  match op {
    "==" => TdRuntimeObj::eq(left, db, right),
    "!=" => !TdRuntimeObj::eq(left, db, right),
    "<" => left.lt(db, right),
    ">" => left.gt(db, right),
    "<=" => left.le(db, right),
    ">=" => left.ge(db, right),
    _ => false,
  }
}

fn evaluate_index<'db>(
  db: &'db TypedownDatabase,
  expr: HirValue<'db>,
  indices: Vec<HirValue<'db>>,
  runtime_scope: RuntimeScope<'db>,
  diagnostics: &mut Vec<Diagnostic>,
) -> Option<TdObjEnum<'db>> {
  if indices.len() != 1 {
    return None;
  }
  let index_hir = indices[0];
  let container = evaluate_node(db, expr, runtime_scope).value(db)?;
  let index_obj = evaluate_node(db, index_hir, runtime_scope).value(db)?;

  // Bounds check for diagnostics before delegating to protocol
  if let Some(number) = index_obj.as_td_number_obj()
    && let Some(len) = container.len(db)
  {
    let index = number.value(db) as usize;
    if index >= len {
      let node = index_hir.node(db);
      let (trimmed_offset, trimmed_len) = node.trimmed_range();
      diagnostics.push(Diagnostic::IndexOutOfBounds {
        index,
        length: len,
        start_offset: trimmed_offset,
        end_offset: trimmed_offset + trimmed_len,
      });
      return None;
    }
  }
  container.index(db, &index_obj)
}

fn construct_macro<'db>(
  db: &'db TypedownDatabase,
  kind: BuiltinMacroKind,
  args: Vec<HirValue<'db>>,
) -> Option<TdObjEnum<'db>> {
  match kind {
    BuiltinMacroKind::Fref => construct_fref(db, args),
  }
}

fn evaluate_interpolated<'db>(
  db: &'db TypedownDatabase,
  runtime_scope: RuntimeScope<'db>,
  parts: Vec<InterpolatedPart<'db>>,
) -> Option<TdObjEnum<'db>> {
  let mut value = String::new();
  for part in parts {
    match part {
      InterpolatedPart::Literal(lit) => value.push_str(&lit),
      InterpolatedPart::Expr(expr) => {
        let obj = evaluate_node(db, expr, runtime_scope).value(db)?;
        let to_string_func = obj.lookup_method(db, "to_string")?;
        let project = runtime_scope.scope(db).project(db);
        let string_obj = to_string_func.call(db, project, Some(obj), vec![]).ok()?;
        let string_value = string_obj.as_td_string_obj()?;
        value.push_str(&string_value.value(db));
      }
    }
  }
  Some(TdStringObj::new(db, value).into())
}

// Evaluate mapping as an object of type `typ`
fn evaluate_mapping<'db>(
  db: &'db TypedownDatabase,
  typ: &TdTypEnum<'db>,
  entries: Vec<(String, HirValue<'db>)>,
) -> Option<TdObjEnum<'db>> {
  // Schema type
  if typ.is_td_schema_meta_typ() {
    let properties_entries = match entries.iter().find(|(key, _)| key == "properties") {
      Some((_, props_hir)) => match props_hir.kind(db) {
        HirValueKind::Mapping(entries) => entries,
        _ => return None,
      },
      None => vec![],
    };
    let mut fields = BTreeMap::new();
    for (prop_name, prop_hir) in properties_entries {
      if prop_name.starts_with('_') && TdSchemaTyp::builtin_field_typ(db, &prop_name).is_none() {
        fields.insert(
          prop_name,
          PropertyDescriptor {
            field_typ: LazyTyp::eager(get_never_typ(db).into()),
            default_value: None,
            computed_func: None,
          },
        );
        continue;
      }
      if let Some(desc) = resolve_property_descriptor(db, prop_hir, &mut vec![]) {
        fields.insert(prop_name, desc);
      }
    }
    return Some(
      TdSchemaTyp::new(
        db,
        "anonymous".to_string(),
        BTreeMap::new(),
        fields,
        BTreeMap::new(),
        None,
      )
      .into(),
    );
  }

  // Schema type: build product obj from fields, then construct schema instance
  if let TdTypEnum::TdSchemaTyp(schema_typ) = &typ {
    let mut builtins = BTreeMap::new();
    let mut fields = BTreeMap::new();
    for (key, val_hir) in entries {
      if key.starts_with('_') {
        builtins.insert(key, Either::Left(val_hir));
      } else {
        fields.insert(key, Either::Left(val_hir));
      }
    }
    let project = builtins
      .values()
      .chain(fields.values())
      .find_map(|value| match value {
        Either::Left(hir) => Some(hir.project(db)),
        _ => None,
      })?;
    return Some(
      TdSchemaObj::new(db, (*schema_typ).into(), project, None, builtins, fields).into(),
    );
  }

  // Product type
  if let TdTypEnum::TdProductTyp(product_typ) = &typ {
    let mut builtins = BTreeMap::new();
    let mut fields = BTreeMap::new();
    for (key, value_hir) in entries {
      if key.starts_with('_') {
        builtins.insert(key, Either::Left(value_hir));
      } else {
        fields.insert(key, Either::Left(value_hir));
      }
    }
    return Some(TdProductObj::new(db, (*product_typ).into(), None, builtins, fields).into());
  }

  let dict_entries: BTreeMap<_, _> = entries
    .into_iter()
    .map(|(key, value)| (key, Either::Left(value)))
    .collect();
  Some(TdDictObj::new(db, dict_entries).into())
}

// fref("file.td") evaluates to the target resource's object
fn construct_fref<'db>(
  db: &'db TypedownDatabase,
  args: Vec<HirValue<'db>>,
) -> Option<TdObjEnum<'db>> {
  if args.len() != 1 {
    return None;
  }
  let arg = args[0];
  let path_string = match arg.kind(db) {
    HirValueKind::String(value) => value,
    _ => return None,
  };

  let project = arg.project(db);
  let files = project.files(db);
  let root_dir = get_vault_config(db, project).root_dir(db);
  let target_path = root_dir.join(&path_string);

  let target_file = *files.get(&target_path)?;
  let target_symbol = file_symbol(db, project, target_file).value(db)?;

  evaluate_resource(db, target_symbol).value(db)
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::db::derived::get_builtin_typs::{
    get_bool_typ, get_dict_typ, get_list_typ, get_lit_typ, get_never_typ, get_null_typ,
    get_number_typ, get_string_typ, get_sum_typ, get_typ_typ,
  };
  use crate::db::derived::name_resolver::scope::get_file_runtime_scope;
  use crate::db::types::derived::obj_system::TdFuncObj;
  use crate::db::types::{
    File, FileHandle, FileMetadata, FuncKind, FuncSignature, LazyTyp, LitValue, NativeFuncKind,
    PROTOCOL_CALL, PROTOCOL_INDEX, Project, TdProductTyp, TdSchemaTyp, TdTypEnum,
  };
  use crate::db::utils::lower_file;
  use crate::db::{QueryStorage, TypedownDatabase};

  use std::collections::{BTreeMap, HashMap};
  use std::path::PathBuf;

  fn make_db() -> TypedownDatabase {
    TypedownDatabase {
      storage: QueryStorage::default(),
    }
  }

  #[test]
  fn test_runtime_typ_mapping() {
    let db = make_db();

    // Literal types resolve to primitive underlying types
    let lit_string: TdTypEnum<'_> = get_lit_typ(&db, LitValue::String("hello".into())).into();
    let lit_number: TdTypEnum<'_> = get_lit_typ(&db, LitValue::Number("42".into())).into();
    let lit_bool: TdTypEnum<'_> = get_lit_typ(&db, LitValue::Bool(true)).into();

    assert_eq!(
      lit_string.runtime_typ(&db),
      Some(get_string_typ(&db).into())
    );
    assert_eq!(
      lit_number.runtime_typ(&db),
      Some(get_number_typ(&db).into())
    );
    assert_eq!(lit_bool.runtime_typ(&db), Some(get_bool_typ(&db).into()));

    // Primitive constructible types return themselves
    let string_typ: TdTypEnum = get_string_typ(&db).into();
    let number_typ: TdTypEnum = get_number_typ(&db).into();
    let bool_typ: TdTypEnum = get_bool_typ(&db).into();
    let list_typ: TdTypEnum = get_list_typ(&db).into();
    let dict_typ: TdTypEnum = get_dict_typ(&db).into();
    let null_typ: TdTypEnum = get_null_typ(&db).into();

    assert_eq!(string_typ.runtime_typ(&db), Some(string_typ.clone()));
    assert_eq!(number_typ.runtime_typ(&db), Some(number_typ.clone()));
    assert_eq!(bool_typ.runtime_typ(&db), Some(bool_typ.clone()));
    // Uninstantiated generics are not constructible
    assert_eq!(list_typ.runtime_typ(&db), None);
    assert_eq!(dict_typ.runtime_typ(&db), None);
    assert_eq!(null_typ.runtime_typ(&db), Some(null_typ.clone()));

    // Instantiated generics are constructible
    let list_string: TdTypEnum = get_list_typ(&db)
      .instantiate(&db, vec![LazyTyp::eager(get_string_typ(&db).into())])
      .typ(&db);
    let dict_string_number: TdTypEnum = get_dict_typ(&db)
      .instantiate(
        &db,
        vec![
          LazyTyp::eager(get_string_typ(&db).into()),
          LazyTyp::eager(get_number_typ(&db).into()),
        ],
      )
      .typ(&db);
    assert_eq!(list_string.runtime_typ(&db), Some(list_string.clone()));
    assert_eq!(
      dict_string_number.runtime_typ(&db),
      Some(dict_string_number.clone())
    );

    // Non-constructible types return None
    let sum_typ: TdTypEnum = get_sum_typ(
      &db,
      vec![LazyTyp::eager(string_typ), LazyTyp::eager(number_typ)],
    )
    .into();
    let never_typ: TdTypEnum = get_never_typ(&db).into();
    let product_typ: TdTypEnum = TdProductTyp::new(&db, None, BTreeMap::new()).into();

    let typ_typ_value: TdTypEnum = get_typ_typ(&db).into();
    assert_eq!(typ_typ_value.runtime_typ(&db), Some(typ_typ_value.clone()));
    assert_eq!(sum_typ.runtime_typ(&db), None);
    assert_eq!(never_typ.runtime_typ(&db), None);
    assert_eq!(product_typ.runtime_typ(&db), None);
  }

  #[test]
  fn test_not_constructible_diagnostic_emitted() {
    let db = make_db();

    // Verify runtime_typ returns None for non-constructible sum typ
    let sum_typ: TdTypEnum<'_> = get_sum_typ(
      &db,
      vec![
        LazyTyp::eager(get_string_typ(&db).into()),
        LazyTyp::eager(get_number_typ(&db).into()),
      ],
    )
    .into();
    assert_eq!(sum_typ.runtime_typ(&db), None);

    // Verify runtime_typ returns None for never typ
    let never_typ: TdTypEnum = get_never_typ(&db).into();
    assert_eq!(never_typ.runtime_typ(&db), None);

    // Verify construct_from_hir on constructible literal succeeds cleanly
    let file = File::new(
      &db,
      FileHandle::Content(
        PathBuf::from("test.td"),
        "\"hello\"\n".into(),
        FileMetadata::default(),
      ),
    );
    let project = Project::new(&db, PathBuf::from("/vault"), HashMap::new());
    let (hir, _) = lower_file(&db, project, file);
    let string_hir = hir.expect("file should parse");

    let mut diagnostics = vec![];
    let scope = get_file_runtime_scope(&db, project, file);
    let obj = construct_from_hir(&db, string_hir, scope, &mut diagnostics);
    assert!(obj.is_some());
    assert!(diagnostics.is_empty());
  }

  #[test]
  fn test_dunder_methods_call_and_index() {
    let db = make_db();
    let string_typ: TdTypEnum<'_> = get_string_typ(&db).into();
    let signature = FuncSignature::new(&db, vec![string_typ.clone()], string_typ.clone());
    let index_func = TdFuncObj::new(
      &db,
      PROTOCOL_INDEX.to_string(),
      signature,
      FuncKind::Native(NativeFuncKind::ToStringMethod),
    );
    let call_func = TdFuncObj::new(
      &db,
      PROTOCOL_CALL.to_string(),
      signature,
      FuncKind::Native(NativeFuncKind::ToStringMethod),
    );

    let mut vtable = BTreeMap::new();
    vtable.insert(PROTOCOL_INDEX.to_string(), index_func);
    vtable.insert(PROTOCOL_CALL.to_string(), call_func);

    let schema_typ = TdSchemaTyp::new(
      &db,
      "CustomContainer".into(),
      BTreeMap::new(),
      BTreeMap::new(),
      vtable,
      None,
    );
    let schema_enum: TdTypEnum = schema_typ.into();

    // Verify static typechecking detects [[index]] and [[call]] return types
    assert_eq!(
      schema_enum.index_typ(&db, &get_number_typ(&db).into()),
      Some(signature)
    );
    assert_eq!(
      schema_enum.call_typ(&db, vec![get_string_typ(&db).into()]),
      Some(signature)
    );
  }
}
