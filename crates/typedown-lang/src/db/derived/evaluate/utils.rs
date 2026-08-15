use std::collections::HashMap;

use crate::db::TypedownDatabase;
use crate::db::derived::evaluate::evaluate_node::evaluate_node;
use crate::db::derived::evaluate::evaluate_resource::evaluate_resource;
use crate::db::derived::evaluate::evaluate_type::{evaluate_type, resolve_property_descriptor};
use crate::db::derived::get_builtin_types::get_schema_type;
use crate::db::derived::get_vault_config::get_vault_config;
use crate::db::derived::name_resolver::file_symbol::file_symbol;
use crate::db::derived::name_resolver::referee::referee;
use crate::db::derived::typechecker::actual_node_type_member::actual_node_type_member;
use crate::db::types::{
  BuiltinGlobalKind, BuiltinMacroKind, HirValue, HirValueKind, InterpolatedPart, MemberType,
  SymbolKind, TdBoolObj, TdDictObj, TdListObj, TdMathObj, TdNullObj, TdNumObj, TdObjectEnum,
  TdObjectLike, TdProductObj, TdProductType, TdStrObj, TdTypeEnum, TdTypeLike, TdVaultObj,
  TypeMember, TypeMemberDescriptors,
};
use crate::db::utils::typecheck::lift_type_member_result;
use crate::syntax::diagnostic::Diagnostic;
use typedown_types::either::Either;

pub(crate) fn construct_from_hir(
  db: &TypedownDatabase,
  hir: HirValue,
  diagnostics: &mut Vec<Diagnostic>,
) -> Option<TdObjectEnum> {
  match hir.kind(db) {
    HirValueKind::Null => {
      return Some(TdNullObj::get(db).into());
    }
    // self evaluates to the current file's resource object
    HirValueKind::Ident(name) if name == "self" => {
      let project = hir.project(db);
      let file = hir.file(db);
      let symbol = file_symbol(db, project, file).value(db)?;
      return evaluate_resource(db, symbol).value(db);
    }
    // Builtin globals and schema references resolve to objects
    HirValueKind::Ident(_) => {
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
          _ => {}
        }
      }
    }
    // Tag expressions: the tag is a type hint for the typechecker; evaluation strips it
    HirValueKind::Tag { inner, .. } => {
      return evaluate_node(db, *inner).value(db);
    }
    // Field access: obj.field
    HirValueKind::Binary { op, left, right } if op == "." => {
      if let HirValueKind::Ident(field_name) = right.kind(db) {
        let this = evaluate_node(db, *left).value(db)?;
        return this.lookup_field(db, &field_name);
      }
    }
    // Arithmetic, comparison, and logical binary operators
    HirValueKind::Binary { op, left, right } => {
      return evaluate_binary(db, &op, *left, *right);
    }
    // Prefix operators
    HirValueKind::Prefix { op, operand } => {
      return evaluate_prefix(db, &op, *operand);
    }
    // TODO: evaluate postfix expressions
    HirValueKind::Postfix { .. } => {
      return None;
    }
    // Index access: list[n] or dict["key"]
    HirValueKind::Index { expr, indices } => {
      return evaluate_index(db, *expr, indices, diagnostics);
    }
    HirValueKind::Call { callee, args } => {
      match callee.kind(db) {
        // Method call: obj.method(args)
        HirValueKind::Binary { op, left, right } if op == "." => {
          if let HirValueKind::Ident(method_name) = right.kind(db) {
            let this = evaluate_node(db, *left).value(db)?;
            let func_obj = this.lookup_method(db, &method_name)?;
            let arg_objs: Vec<_> = args
              .into_iter()
              .filter_map(|arg| evaluate_node(db, arg).value(db))
              .collect();
            return func_obj.call(db, this, arg_objs);
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
          // Plain function call: evaluate callee, check if it's a function, call it
          let callee_obj = evaluate_node(db, *callee).value(db)?;
          if let TdObjectEnum::TdFuncObj(func_obj) = &callee_obj {
            let func_obj = *func_obj;
            let arg_objs: Vec<_> = args
              .into_iter()
              .filter_map(|arg| evaluate_node(db, arg).value(db))
              .collect();
            return func_obj.call(db, callee_obj, arg_objs);
          }
        }
      }
    }
    _ => {}
  }

  // Anonymous mappings have no schema, evaluate as a dict
  let type_result = actual_node_type_member(db, hir);
  if let HirValueKind::Mapping(entries) = hir.kind(db)
    && let Some(member) = type_result.member(db)
    && matches!(member.typ(db), MemberType::Structural(_))
  {
    let dict_entries: HashMap<_, _> = entries
      .into_iter()
      .map(|(k, v)| (k, Either::Left(v)))
      .collect();
    return Some(TdDictObj::new(db, dict_entries).into());
  }

  // Normal construction: convert HIR to args, then call construct
  let typ = lift_type_member_result(db, &type_result)?;
  match hir.kind(db) {
    HirValueKind::Str(val) => typ.construct(db, vec![TdStrObj::new(db, val).into()]),
    HirValueKind::Num(val) => {
      let num: f64 = val.parse().unwrap_or(0.0);
      typ.construct(db, vec![TdNumObj::new(db, num).into()])
    }
    HirValueKind::Bool(val) => typ.construct(db, vec![TdBoolObj::new(db, val).into()]),
    HirValueKind::Math(val) => typ.construct(db, vec![TdMathObj::new(db, val).into()]),
    HirValueKind::Interpolated(parts) => {
      let obj = evaluate_interpolated(db, parts)?;
      typ.construct(db, vec![obj])
    }
    HirValueKind::Sequence(items) => {
      if typ.is_td_list_type() {
        let hir_items = items.into_iter().map(Either::Left).collect();
        return Some(TdListObj::new(db, hir_items).into());
      }
      let args: Vec<_> = items
        .into_iter()
        .filter_map(|item| evaluate_node(db, item).value(db))
        .collect();
      typ.construct(db, args)
    }
    HirValueKind::Mapping(entries) => evaluate_mapping(db, &typ, entries),
    HirValueKind::Markdown(parts) => {
      let obj = evaluate_interpolated(db, parts)?;
      typ.construct(db, vec![obj])
    }
    _ => None,
  }
}

fn evaluate_prefix(db: &TypedownDatabase, op: &str, operand: HirValue) -> Option<TdObjectEnum> {
  let operand_obj = evaluate_node(db, operand).value(db)?;
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

fn evaluate_binary(
  db: &TypedownDatabase,
  op: &str,
  left: HirValue,
  right: HirValue,
) -> Option<TdObjectEnum> {
  let left_obj = evaluate_node(db, left).value(db)?;
  let right_obj = evaluate_node(db, right).value(db)?;
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
    "==" => TdObjectLike::eq(left, db, right),
    "!=" => !TdObjectLike::eq(left, db, right),
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
  diagnostics: &mut Vec<Diagnostic>,
) -> Option<TdObjectEnum> {
  if indices.len() != 1 {
    return None;
  }
  let index_hir = indices[0];
  let container = evaluate_node(db, expr).value(db)?;
  let index_obj = evaluate_node(db, index_hir).value(db)?;

  if let TdObjectEnum::TdListObj(list) = &container {
    let num = index_obj.as_td_num_obj()?;
    let idx = num.value(db) as usize;
    let len = list.len(db);
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
    return list.get(db, idx);
  }
  if let TdObjectEnum::TdDictObj(dict) = &container {
    let key = index_obj.as_td_str_obj()?;
    return dict.get_owned_field(db, &key.value(db));
  }
  if let TdObjectEnum::TdStrObj(str_obj) = &container {
    let num = index_obj.as_td_num_obj()?;
    let idx = num.value(db) as usize;
    let chars: Vec<char> = str_obj.value(db).chars().collect();
    if idx >= chars.len() {
      let node = index_hir.node(db);
      let (tr_offset, tr_len) = node.trimmed_range();
      diagnostics.push(Diagnostic::IndexOutOfBounds {
        index: idx,
        length: chars.len(),
        start_offset: tr_offset,
        end_offset: tr_offset + tr_len,
      });
      return None;
    }
    return Some(TdStrObj::new(db, chars[idx].to_string()).into());
  }
  None
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
  parts: Vec<InterpolatedPart>,
) -> Option<TdObjectEnum> {
  let mut val = String::new();
  for part in parts {
    match part {
      InterpolatedPart::Literal(lit) => val.push_str(&lit),
      InterpolatedPart::Expr(expr) => {
        let obj = evaluate_node(db, expr).value(db)?;
        let to_string_fn = obj.lookup_method(db, "to_string")?;
        let str_obj = to_string_fn.call(db, obj, vec![])?;
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
          TypeMember::new(db, MemberType::Never, TypeMemberDescriptors::empty()),
        );
        continue;
      }
      if let Some((member_type, descriptors)) =
        resolve_property_descriptor(db, prop_hir, &mut vec![])
      {
        fields.insert(prop_name, TypeMember::new(db, member_type, descriptors));
      }
    }
    return Some(
      TdProductType::new(
        db,
        None,
        get_schema_type(db).into(),
        None,
        fields,
        HashMap::new(),
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
  let content_dir = get_vault_config(db, project).content_dir(db);
  let target_path = content_dir.join(&path_str);

  let target_file = *files.get(&target_path)?;
  let target_symbol = file_symbol(db, project, target_file).value(db)?;

  evaluate_resource(db, target_symbol).value(db)
}
