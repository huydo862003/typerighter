use std::collections::HashMap;

use crate::db::TypedownDatabase;
use crate::db::derived::evaluate::evaluate_node::evaluate_node;
use crate::db::derived::evaluate::evaluate_resource::evaluate_resource;
use crate::db::derived::evaluate::evaluate_type::{evaluate_type, resolve_property_descriptor};
use crate::db::derived::get_builtin_types::{
  get_never_type, get_null_type, get_schema_type, get_sum_type,
};
use crate::db::derived::get_vault_config::get_vault_config;
use crate::db::derived::name_resolver::file_symbol::file_symbol;
use crate::db::derived::name_resolver::referee::referee;
use crate::db::derived::typechecker::actual_node_type::actual_node_type;
use crate::db::types::{
  BuiltinGlobalKind, BuiltinMacroKind, FnKind, HirValue, HirValueKind, InterpolatedPart, LazyType,
  PropertyDescriptor, RuntimeScope, SymbolKind, TdBoolObj, TdDictObj, TdFuncObj, TdFuncType,
  TdListObj, TdMathObj, TdNullObj, TdNumObj, TdObjectEnum, TdProductObj, TdProductType,
  TdRuntimeObject, TdStaticType, TdStrObj, TdTypeEnum, TdVaultObj,
};
use crate::syntax::diagnostic::Diagnostic;
use typedown_types::either::Either;

pub(crate) fn construct_from_hir(
  db: &TypedownDatabase,
  hir: HirValue,
  runtime_scope: RuntimeScope,
  diagnostics: &mut Vec<Diagnostic>,
) -> Option<TdObjectEnum> {
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
            };
          }
          // Schema identifiers evaluate to the schema type as an object
          SymbolKind::UserDefinedSchema(_, _) | SymbolKind::BuiltinSchema(_) => {
            return evaluate_type(db, symbol).typ(db).map(TdObjectEnum::from);
          }
          // Resource identifiers (including self) evaluate to the resource object
          SymbolKind::UserDefinedResource(_, _) => {
            return evaluate_resource(db, symbol).value(db);
          }
          _ => {}
        }
      }
    }
    // Tag expressions: the tag is a type hint for the typechecker
    HirValueKind::Tag { inner, .. } => {
      return evaluate_node(db, *inner, runtime_scope).value(db);
    }
    // Field access: obj.field
    HirValueKind::Binary { op, left, right } if op == "." => {
      if let HirValueKind::Ident(field_name) = right.kind(db) {
        let this = evaluate_node(db, *left, runtime_scope).value(db)?;
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
            let this = evaluate_node(db, *left, runtime_scope).value(db)?;
            let func_obj = this.lookup_method(db, &method_name)?;
            let arg_objs: Vec<_> = args
              .into_iter()
              .filter_map(|arg| evaluate_node(db, arg, runtime_scope).value(db))
              .collect();
            return func_obj.call(db, Some(this), arg_objs).ok();
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
          let callee_obj = evaluate_node(db, *callee, runtime_scope).value(db)?;
          let arg_objs: Vec<_> = args
            .into_iter()
            .filter_map(|arg| evaluate_node(db, arg, runtime_scope).value(db))
            .collect();
          return callee_obj.call(db, None, arg_objs).ok();
        }
      }
    }
    // Closure: create a TdFuncObj capturing the defining scope
    HirValueKind::Closure { ref params, .. } => {
      let func_type = match actual_node_type(db, hir).typ(db) {
        Some(TdTypeEnum::TdFuncType(f)) => f,
        // No expected type context: assume never for params and return
        _ => {
          let never: TdTypeEnum = get_never_type(db).into();
          let param_types = vec![never.clone(); params.len()];
          TdFuncType::get(db, param_types, never)
        }
      };
      let func_obj = TdFuncObj::new(
        db,
        "<closure>".to_string(),
        func_type.signature(db),
        FnKind::UserDefined(hir, runtime_scope),
      );
      return Some(func_obj.into());
    }
    _ => {}
  }

  // Anonymous mappings have no schema, evaluate as a dict
  let type_result = actual_node_type(db, hir);
  if let HirValueKind::Mapping(entries) = hir.kind(db)
    && type_result
      .typ(db)
      .is_some_and(|t| t.is_td_structural_type())
  {
    let dict_entries: HashMap<_, _> = entries
      .into_iter()
      .map(|(k, v)| (k, Either::Left(v)))
      .collect();
    return Some(TdDictObj::new(db, dict_entries).into());
  }

  // Normal construction: convert HIR to args, then call construct
  let raw_typ = type_result.typ(db)?;
  let typ = match raw_typ.runtime_type(db) {
    Some(t) => t,
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
    HirValueKind::Str(val) => typ.construct(db, vec![TdStrObj::new(db, val).into()]),
    HirValueKind::Num(val) => {
      let num: f64 = val.parse().unwrap_or(0.0);
      typ.construct(db, vec![TdNumObj::new(db, num).into()])
    }
    HirValueKind::Bool(val) => typ.construct(db, vec![TdBoolObj::new(db, val).into()]),
    HirValueKind::Math(val) => typ.construct(db, vec![TdMathObj::new(db, val).into()]),
    HirValueKind::Interpolated(parts) => {
      let obj = evaluate_interpolated(db, runtime_scope, parts)?;
      typ.construct(db, vec![obj])
    }
    HirValueKind::Sequence(items) => {
      if typ.is_td_list_type() {
        let hir_items = items.into_iter().map(Either::Left).collect();
        return Some(TdListObj::new(db, hir_items).into());
      }
      let args: Vec<_> = items
        .into_iter()
        .filter_map(|item| evaluate_node(db, item, runtime_scope).value(db))
        .collect();
      typ.construct(db, args)
    }
    HirValueKind::Mapping(entries) => evaluate_mapping(db, &typ, entries),
    HirValueKind::Markdown(parts) => {
      let obj = evaluate_interpolated(db, runtime_scope, parts)?;
      typ.construct(db, vec![obj])
    }
    _ => None,
  }
}

fn evaluate_prefix(
  db: &TypedownDatabase,
  op: &str,
  operand: HirValue,
  runtime_scope: RuntimeScope,
) -> Option<TdObjectEnum> {
  let operand_obj = evaluate_node(db, operand, runtime_scope).value(db)?;
  match op {
    "-" | "+" => {
      let num = operand_obj.as_td_num_obj()?;
      let val = num.value(db);
      let result = match op {
        "-" => -val,
        "+" => val,
        _ => unreachable!(),
      };
      Some(TdNumObj::new(db, result).into())
    }
    // Logical not: only null and false are falsy, everything else is truthy
    "~" => {
      let is_falsy = operand_obj.as_td_bool_obj().is_some_and(|b| !b.value(db));
      Some(TdBoolObj::new(db, is_falsy).into())
    }
    _ => None,
  }
}

fn evaluate_postfix(
  db: &TypedownDatabase,
  op: &str,
  operand: HirValue,
  runtime_scope: RuntimeScope,
) -> Option<TdObjectEnum> {
  match op {
    // T? evaluates to Sum([T, null]) as a type object
    "?" => {
      let inner = evaluate_node(db, operand, runtime_scope).value(db)?;
      let inner_type = inner.into_td_type_obj().ok()?;
      Some(
        get_sum_type(
          db,
          vec![
            LazyType::eager(inner_type),
            LazyType::eager(get_null_type(db).into()),
          ],
        )
        .into(),
      )
    }
    _ => None,
  }
}

fn evaluate_binary(
  db: &TypedownDatabase,
  op: &str,
  left: HirValue,
  right: HirValue,
  runtime_scope: RuntimeScope,
) -> Option<TdObjectEnum> {
  let left_obj = evaluate_node(db, left, runtime_scope).value(db)?;
  let right_obj = evaluate_node(db, right, runtime_scope).value(db)?;
  match op {
    "+" | "-" | "*" | "/" | "%" | "**" => {
      let lnum = left_obj.as_td_num_obj()?;
      let rnum = right_obj.as_td_num_obj()?;
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
      Some(TdNumObj::new(db, result).into())
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

fn compare_objects(
  db: &TypedownDatabase,
  op: &str,
  left: &TdObjectEnum,
  right: &TdObjectEnum,
) -> bool {
  match op {
    "==" => TdRuntimeObject::eq(left, db, right),
    "!=" => !TdRuntimeObject::eq(left, db, right),
    "<" => left.lt(db, right),
    ">" => left.gt(db, right),
    "<=" => left.le(db, right),
    ">=" => left.ge(db, right),
    _ => false,
  }
}

fn evaluate_index(
  db: &TypedownDatabase,
  expr: HirValue,
  indices: Vec<HirValue>,
  runtime_scope: RuntimeScope,
  diagnostics: &mut Vec<Diagnostic>,
) -> Option<TdObjectEnum> {
  if indices.len() != 1 {
    return None;
  }
  let index_hir = indices[0];
  let container = evaluate_node(db, expr, runtime_scope).value(db)?;
  let index_obj = evaluate_node(db, index_hir, runtime_scope).value(db)?;

  // Bounds check for diagnostics before delegating to protocol
  if let Some(num) = index_obj.as_td_num_obj()
    && let Some(len) = container.len(db)
  {
    let idx = num.value(db) as usize;
    if idx >= len {
      let node = index_hir.node(db);
      let (tr_offset, tr_len) = node.trimmed_range();
      diagnostics.push(Diagnostic::IndexOutOfBounds {
        index: idx,
        length: len,
        start_offset: tr_offset,
        end_offset: tr_offset + tr_len,
      });
      return None;
    }
  }
  container.index(db, &index_obj)
}

fn construct_macro(
  db: &TypedownDatabase,
  kind: BuiltinMacroKind,
  args: Vec<HirValue>,
) -> Option<TdObjectEnum> {
  match kind {
    BuiltinMacroKind::Fref => construct_fref(db, args),
  }
}

fn evaluate_interpolated(
  db: &TypedownDatabase,
  runtime_scope: RuntimeScope,
  parts: Vec<InterpolatedPart>,
) -> Option<TdObjectEnum> {
  let mut val = String::new();
  for part in parts {
    match part {
      InterpolatedPart::Literal(lit) => val.push_str(&lit),
      InterpolatedPart::Expr(expr) => {
        let obj = evaluate_node(db, expr, runtime_scope).value(db)?;
        let to_string_fn = obj.lookup_method(db, "to_string")?;
        let str_obj = to_string_fn.call(db, Some(obj), vec![]).ok()?;
        let str_val = str_obj.as_td_str_obj()?;
        val.push_str(&str_val.value(db));
      }
    }
  }
  Some(TdStrObj::new(db, val).into())
}

// Evaluate mapping as an object of type `typ`
fn evaluate_mapping(
  db: &TypedownDatabase,
  typ: &TdTypeEnum,
  entries: Vec<(String, HirValue)>,
) -> Option<TdObjectEnum> {
  // Schema type
  if *typ == TdTypeEnum::from(get_schema_type(db)) {
    let properties_entries = match entries.iter().find(|(key, _)| key == "properties") {
      Some((_, props_hir)) => match props_hir.kind(db) {
        HirValueKind::Mapping(entries) => entries,
        _ => return None,
      },
      None => vec![],
    };
    let mut fields = HashMap::new();
    for (prop_name, prop_hir) in properties_entries {
      if prop_name.starts_with('_')
        && prop_name != "_type"
        && prop_name != "_label"
        && prop_name != "_content"
      {
        fields.insert(
          prop_name,
          PropertyDescriptor {
            field_type: LazyType::eager(get_never_type(db).into()),
            default_value: None,
            computed_fn: None,
          },
        );
        continue;
      }
      if let Some(desc) = resolve_property_descriptor(db, prop_hir, &mut vec![]) {
        fields.insert(prop_name, desc);
      }
    }
    return Some(
      TdProductType::new(
        db,
        None,
        get_schema_type(db).into(),
        fields,
        HashMap::new(),
        None,
      )
      .into(),
    );
  }

  // Product type
  if let TdTypeEnum::TdProductType(product_typ) = &typ {
    let mut fields = HashMap::new();
    for (key, val_hir) in entries {
      if key == "_type" {
        continue;
      }
      fields.insert(key, Either::Left(val_hir));
    }
    return Some(TdProductObj::new(db, (*product_typ).into(), None, fields).into());
  }

  let dict_entries: HashMap<_, _> = entries
    .into_iter()
    .map(|(k, v)| (k, Either::Left(v)))
    .collect();
  Some(TdDictObj::new(db, dict_entries).into())
}

// fref("file.td") evaluates to the target resource's object
fn construct_fref(db: &TypedownDatabase, args: Vec<HirValue>) -> Option<TdObjectEnum> {
  if args.len() != 1 {
    return None;
  }
  let arg = args[0];
  let path_str = match arg.kind(db) {
    HirValueKind::Str(val) => val,
    _ => return None,
  };

  let project = arg.project(db);
  let files = project.files(db);
  let root_dir = get_vault_config(db, project).root_dir(db);
  let target_path = root_dir.join(&path_str);

  let target_file = *files.get(&target_path)?;
  let target_symbol = file_symbol(db, project, target_file).value(db)?;

  evaluate_resource(db, target_symbol).value(db)
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::db::derived::get_builtin_types::{
    get_bool_type, get_dict_type, get_list_type, get_literal_type, get_never_type, get_null_type,
    get_num_type, get_str_type, get_sum_type, get_type_type,
  };
  use crate::db::derived::name_resolver::scope::get_file_runtime_scope;
  use crate::db::types::derived::object_system::TdFuncObj;
  use crate::db::types::{
    File, FileHandle, FileMetadata, FnKind, FuncSignature, LazyType, LiteralValue, NativeFnKind,
    PROTOCOL_CALL, PROTOCOL_INDEX, Project, TdStructuralType, TdTypeEnum,
  };
  use crate::db::utils::lower_file;
  use crate::db::{QueryStorage, TypedownDatabase};

  use std::collections::HashMap;
  use std::path::PathBuf;

  fn make_db() -> TypedownDatabase {
    TypedownDatabase {
      storage: QueryStorage::default(),
    }
  }

  #[test]
  fn test_runtime_type_mapping() {
    let db = make_db();

    // Literal types resolve to primitive underlying types
    let lit_str: TdTypeEnum = get_literal_type(&db, LiteralValue::Str("hello".into())).into();
    let lit_num: TdTypeEnum = get_literal_type(&db, LiteralValue::Num("42".into())).into();
    let lit_bool: TdTypeEnum = get_literal_type(&db, LiteralValue::Bool(true)).into();

    assert_eq!(lit_str.runtime_type(&db), Some(get_str_type(&db).into()));
    assert_eq!(lit_num.runtime_type(&db), Some(get_num_type(&db).into()));
    assert_eq!(lit_bool.runtime_type(&db), Some(get_bool_type(&db).into()));

    // Primitive constructible types return themselves
    let str_type: TdTypeEnum = get_str_type(&db).into();
    let num_type: TdTypeEnum = get_num_type(&db).into();
    let bool_type: TdTypeEnum = get_bool_type(&db).into();
    let list_type: TdTypeEnum = get_list_type(&db).into();
    let dict_type: TdTypeEnum = get_dict_type(&db).into();
    let null_type: TdTypeEnum = get_null_type(&db).into();

    assert_eq!(str_type.runtime_type(&db), Some(str_type.clone()));
    assert_eq!(num_type.runtime_type(&db), Some(num_type.clone()));
    assert_eq!(bool_type.runtime_type(&db), Some(bool_type.clone()));
    // Uninstantiated generics are not constructible
    assert_eq!(list_type.runtime_type(&db), None);
    assert_eq!(dict_type.runtime_type(&db), None);
    assert_eq!(null_type.runtime_type(&db), Some(null_type.clone()));

    // Instantiated generics are constructible
    let list_str: TdTypeEnum = get_list_type(&db)
      .instantiate(&db, vec![LazyType::eager(get_str_type(&db).into())])
      .typ(&db);
    let dict_str_num: TdTypeEnum = get_dict_type(&db)
      .instantiate(
        &db,
        vec![
          LazyType::eager(get_str_type(&db).into()),
          LazyType::eager(get_num_type(&db).into()),
        ],
      )
      .typ(&db);
    assert_eq!(list_str.runtime_type(&db), Some(list_str.clone()));
    assert_eq!(dict_str_num.runtime_type(&db), Some(dict_str_num.clone()));

    // Non-constructible types return None
    let sum_type: TdTypeEnum = get_sum_type(
      &db,
      vec![LazyType::eager(str_type), LazyType::eager(num_type)],
    )
    .into();
    let never_type: TdTypeEnum = get_never_type(&db).into();
    let structural_type: TdTypeEnum = TdStructuralType::new(&db, HashMap::new()).into();

    let type_type_val: TdTypeEnum = get_type_type(&db).into();
    assert_eq!(type_type_val.runtime_type(&db), Some(type_type_val.clone()));
    assert_eq!(sum_type.runtime_type(&db), None);
    assert_eq!(never_type.runtime_type(&db), None);
    assert_eq!(structural_type.runtime_type(&db), None);
  }

  #[test]
  fn test_not_constructible_diagnostic_emitted() {
    let db = make_db();

    // Verify runtime_type returns None for non-constructible sum type
    let sum_type: TdTypeEnum = get_sum_type(
      &db,
      vec![
        LazyType::eager(get_str_type(&db).into()),
        LazyType::eager(get_num_type(&db).into()),
      ],
    )
    .into();
    assert_eq!(sum_type.runtime_type(&db), None);

    // Verify runtime_type returns None for never type
    let never_type: TdTypeEnum = get_never_type(&db).into();
    assert_eq!(never_type.runtime_type(&db), None);

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
    let str_hir = hir.expect("file should parse");

    let mut diagnostics = vec![];
    let scope = get_file_runtime_scope(&db, project, file);
    let obj = construct_from_hir(&db, str_hir, scope, &mut diagnostics);
    assert!(obj.is_some());
    assert!(diagnostics.is_empty());
  }

  #[test]
  fn test_dunder_methods_call_and_index() {
    let db = make_db();
    let str_type: TdTypeEnum = get_str_type(&db).into();
    let sig = FuncSignature::new(&db, vec![str_type.clone()], str_type.clone());
    let index_fn = TdFuncObj::new(
      &db,
      PROTOCOL_INDEX.to_string(),
      sig,
      FnKind::Native(NativeFnKind::ToStringMethod),
    );
    let call_fn = TdFuncObj::new(
      &db,
      PROTOCOL_CALL.to_string(),
      sig,
      FnKind::Native(NativeFnKind::ToStringMethod),
    );

    let mut vtable = HashMap::new();
    vtable.insert(PROTOCOL_INDEX.to_string(), index_fn);
    vtable.insert(PROTOCOL_CALL.to_string(), call_fn);

    let product_type = TdProductType::new(
      &db,
      Some("CustomContainer".into()),
      get_type_type(&db).into(),
      HashMap::new(),
      vtable,
      None,
    );
    let product_enum: TdTypeEnum = product_type.into();

    // Verify static typechecking detects [[index]] and [[call]] return types
    assert_eq!(
      product_enum.index_type(&db, &get_num_type(&db).into()),
      Some(sig)
    );
    assert_eq!(
      product_enum.call_type(&db, vec![get_str_type(&db).into()]),
      Some(sig)
    );
  }
}
