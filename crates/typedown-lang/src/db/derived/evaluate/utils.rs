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
  BuiltinGlobalKind, BuiltinMacroKind, FuncKind, LazyTyp, ProjectRedNode, PropertyDescriptor,
  RuntimeScope, SymbolKind, TdBoolObj, TdDictObj, TdFuncObj, TdFuncTyp, TdIconObj, TdListObj,
  TdMathObj, TdNullObj, TdNumberObj, TdObjEnum, TdProductObj, TdProductTyp, TdRuntimeObj, TdSchemaObj,
  TdSchemaTyp, TdStaticTyp, TdStringObj, TdTypEnum, TdVaultObj,
};
use crate::syntax::ast::{
  AstNode, BinaryExpr, CallExpr, ClosureExpr, CodeBlock, DictLit, Expr, IdentLit, IndexExpr,
  InlineCode, InlineMath, InterpFragment, ListLit, MathBlock, MathLit, MdBody, NumberLit,
  PostfixExpr, PrefixExpr, SourceFile, StrLit, YamlFrontmatter, YamlMapping, YamlOpKind,
  YamlSequence,
};
use crate::syntax::diagnostic::Diagnostic;
use crate::syntax::syntax_kind::SyntaxKind;
use typedown_types::either::Either;

pub(crate) fn construct_from_node<'db>(
  db: &'db TypedownDatabase,
  project_node: ProjectRedNode,
  runtime_scope: RuntimeScope<'db>,
  diagnostics: &mut Vec<Diagnostic>,
) -> Option<TdObjEnum<'db>> {
  let project = project_node.project;
  let file = project_node.file;

  let expr = match Expr::cast(project_node.node.clone()) {
    Some(e) => e,
    None => {
      // Non-Expr root nodes (SourceFile/YamlFrontmatter) - fall through to mapping/construct
      return construct_from_non_expr(db, project_node, runtime_scope, diagnostics);
    }
  };

  match expr {
    Expr::IdentLit(ident) => construct_from_ident(db, ident, project_node, runtime_scope),
    // Tag: the tag is a type hint, evaluate the inner value
    // Tags are parsed as PrefixExpr (not BinaryExpr) because `!tag` is a prefix op
    Expr::PrefixExpr(ref prefix)
      if prefix
        .op()
        .and_then(|op| op.kind())
        .is_some_and(|k| matches!(k, crate::syntax::ast::YamlOpKind::Tag(_))) =>
    {
      if let Some(inner) = prefix.expr() {
        let inner_node = ProjectRedNode::new(project, file, inner.syntax().clone());
        let result = evaluate_node(db, inner_node, runtime_scope);
        return result.value(db);
      }
      None
    }
    // Field access: obj.field
    Expr::BinaryExpr(ref binary)
      if binary
        .op()
        .and_then(|op| op.kind())
        .is_some_and(|k| matches!(k, crate::syntax::ast::YamlOpKind::Dot)) =>
    {
      if let Some(right) = binary.right()
        && let Some(field_ident) = IdentLit::cast(right.syntax().clone())
        && let Some(field_name) = field_ident.value()
      {
        let left = binary.left()?;
        let left_node = ProjectRedNode::new(project, file, left.syntax().clone());
        let left_result = evaluate_node(db, left_node, runtime_scope);
        let this = left_result.value(db)?;
        return this.lookup_field(db, &field_name);
      }
      None
    }
    Expr::BinaryExpr(binary) => evaluate_binary(db, binary, project, file, runtime_scope),
    Expr::PrefixExpr(prefix) => evaluate_prefix(db, prefix, project, file, runtime_scope),
    Expr::PostfixExpr(postfix) => evaluate_postfix(db, postfix, project, file, runtime_scope),
    Expr::IndexExpr(index) => evaluate_index(db, index, project, file, runtime_scope, diagnostics),
    Expr::CallExpr(call) => construct_from_call(db, call, project_node, runtime_scope),
    Expr::ClosureExpr(ref closure) => {
      construct_from_closure(db, closure, project_node, runtime_scope)
    }
    // Mapping nodes
    Expr::YamlMapping(mapping) => construct_from_mapping_expr(
      db,
      project_node,
      mapping_entries_from_yaml_mapping(mapping, project, file),
      diagnostics,
    ),
    Expr::DictLit(dict) => construct_from_mapping_expr(
      db,
      project_node,
      mapping_entries_from_dict_lit(dict, project, file),
      diagnostics,
    ),
    // Sequence nodes
    Expr::YamlSequence(seq) => construct_from_sequence(
      db,
      seq,
      project,
      file,
      project_node,
      runtime_scope,
      diagnostics,
    ),
    Expr::ListLit(list) => construct_from_list(
      db,
      list,
      project,
      file,
      project_node,
      runtime_scope,
      diagnostics,
    ),
    // String / interpolated
    Expr::StrLit(str_lit) => construct_from_str_lit(
      db,
      str_lit,
      project,
      file,
      project_node,
      runtime_scope,
      diagnostics,
    ),
    Expr::NumberLit(num) => construct_from_number(db, num, project_node, diagnostics),
    Expr::MathLit(math) => construct_from_math(db, math, project_node, diagnostics),
    Expr::ParenExpr(paren) => {
      let inner = paren.expr()?;
      let inner_node = ProjectRedNode::new(project, file, inner.syntax().clone());
      construct_from_node(db, inner_node, runtime_scope, diagnostics)
    }
    Expr::CodeLit(_) => None,
  }
}

fn construct_from_non_expr<'db>(
  db: &'db TypedownDatabase,
  project_node: ProjectRedNode,
  runtime_scope: RuntimeScope<'db>,
  diagnostics: &mut Vec<Diagnostic>,
) -> Option<TdObjEnum<'db>> {
  let project = project_node.project;
  let file = project_node.file;
  let node = project_node.node.clone();

  // SourceFile: descend to the frontmatter mapping, or return empty product for body-only files
  if let Some(source_file) = SourceFile::cast(node.clone()) {
    if let Some(fm) = source_file.frontmatter()
      && let Some(mapping) = fm.mapping()
    {
      let mapping_node = ProjectRedNode::new(project, file, mapping.syntax().clone());
      let mut entries = mapping_entries_from_yaml_mapping(mapping, project, file);
      if let Some(body) = source_file.body() {
        let content_node = ProjectRedNode::new(project, file, body.syntax().clone());
        entries.push(("_content".to_string(), content_node));
      }
      return construct_from_mapping_expr(db, mapping_node, entries, diagnostics);
    }
    return Some(TdProductObj::new(db, TdProductTyp::new(db, None, BTreeMap::new()).into(), None, BTreeMap::new(), BTreeMap::new()).into());
  }
  // MdBody: evaluate as a plain string (markdown body text)
  if MdBody::cast(node.clone()).is_some() {
    let text = evaluate_markdown_text(db, project_node.clone(), runtime_scope);
    return Some(TdStringObj::new(db, text).into());
  }
  // YamlFrontmatter: descend to the mapping
  if let Some(fm) = YamlFrontmatter::cast(node.clone()) {
    if let Some(mapping) = fm.mapping() {
      let mapping_node = ProjectRedNode::new(project, file, mapping.syntax().clone());
      return construct_from_mapping_expr(
        db,
        mapping_node,
        mapping_entries_from_yaml_mapping(mapping, project, file),
        diagnostics,
      );
    }
    return None;
  }
  None
}

fn construct_from_ident<'db>(
  db: &'db TypedownDatabase,
  ident: IdentLit,
  project_node: ProjectRedNode,
  runtime_scope: RuntimeScope<'db>,
) -> Option<TdObjEnum<'db>> {
  let name = ident.value().unwrap_or_default();
  if name == "null" {
    return Some(TdNullObj::get(db).into());
  }
  if name == "true" {
    return Some(TdBoolObj::new(db, true).into());
  }
  if name == "false" {
    return Some(TdBoolObj::new(db, false).into());
  }
  // Check runtime scope first (for closure params)
  if let Some(obj) = runtime_scope.lookup(db, &name) {
    return Some(obj);
  }
  let project = project_node.project;
  let resolved = referee(db, project_node);
  if let Some(symbol) = resolved.value(db) {
    match symbol.kind(db) {
      SymbolKind::BuiltinGlobal(kind) => {
        return match kind {
          BuiltinGlobalKind::Vault => Some(TdVaultObj::new(db, project).into()),
          BuiltinGlobalKind::Icon => {
            let mut fields = BTreeMap::new();
            for entry in ICON_ENTRIES {
              let obj = TdIconObj::new(db, entry.name.to_string(), entry.lucide_name.to_string());
              fields.insert(entry.name.to_string(), Either::Right(obj.into()));
            }
            let module_typ = get_icon_module_typ(db).into();
            Some(TdProductObj::new(db, module_typ, None, BTreeMap::new(), fields).into())
          }
        };
      }
      SymbolKind::UserDefinedSchema(_, _) | SymbolKind::BuiltinSchema(_) => {
        return evaluate_typ(db, symbol).typ(db).map(TdObjEnum::from);
      }
      SymbolKind::UserDefinedResource(_, _) => {
        let resource = evaluate_resource(db, symbol);
        return resource.value(db);
      }
      _ => {}
    }
  }
  None
}

fn construct_from_call<'db>(
  db: &'db TypedownDatabase,
  call: CallExpr,
  call_project_node: ProjectRedNode,
  runtime_scope: RuntimeScope<'db>,
) -> Option<TdObjEnum<'db>> {
  let project = call_project_node.project;
  let file = call_project_node.file;
  let callee_expr = call.callee()?;

  // Method call: obj.method(args) - callee is a BinaryExpr with `.` op
  if let Expr::BinaryExpr(ref callee_binary) = callee_expr
    && callee_binary
      .op()
      .and_then(|op| op.kind())
      .is_some_and(|k| matches!(k, YamlOpKind::Dot))
    && let Some(right) = callee_binary.right()
    && let Some(method_ident) = IdentLit::cast(right.syntax().clone())
    && let Some(method_name) = method_ident.value()
  {
    let left = callee_binary.left()?;
    let left_node = ProjectRedNode::new(project, file, left.syntax().clone());
    let left_result = evaluate_node(db, left_node, runtime_scope);
    let this = left_result.value(db)?;
    let func_obj = this.lookup_method(db, &method_name)?;
    let arg_objs: Vec<_> = call
      .args()
      .into_iter()
      .filter_map(|arg| {
        let arg_node = ProjectRedNode::new(project, file, arg.syntax().clone());
        evaluate_node(db, arg_node, runtime_scope).value(db)
      })
      .collect();
    let scope = runtime_scope.scope(db);
    let call_project = scope.project(db);
    return func_obj.call(db, call_project, Some(this), arg_objs).ok();
  }

  // Macro calls: fref and similar
  let callee_node = ProjectRedNode::new(project, file, callee_expr.syntax().clone());
  let resolved = referee(db, callee_node.clone());
  if let Some(symbol) = resolved.value(db)
    && let SymbolKind::BuiltinMacro(kind) = symbol.kind(db)
  {
    let args: Vec<_> = call
      .args()
      .into_iter()
      .map(|arg| ProjectRedNode::new(project, file, arg.syntax().clone()))
      .collect();
    return construct_macro(db, kind, args);
  }

  // Plain function call: evaluate callee, call via protocol
  let callee_obj = evaluate_node(db, callee_node, runtime_scope).value(db)?;
  let arg_objs: Vec<_> = call
    .args()
    .into_iter()
    .filter_map(|arg| {
      let arg_node = ProjectRedNode::new(project, file, arg.syntax().clone());
      evaluate_node(db, arg_node, runtime_scope).value(db)
    })
    .collect();
  let scope = runtime_scope.scope(db);
  let call_project = scope.project(db);
  callee_obj.call(db, call_project, None, arg_objs).ok()
}

fn construct_from_closure<'db>(
  db: &'db TypedownDatabase,
  closure: &ClosureExpr,
  project_node: ProjectRedNode,
  runtime_scope: RuntimeScope<'db>,
) -> Option<TdObjEnum<'db>> {
  let param_count = match closure.params() {
    Some(Either::Left(param_list)) => param_list.params().count(),
    Some(Either::Right(_)) => 1,
    None => 0,
  };
  let func_typ = match actual_node_typ(db, project_node.clone()).typ(db) {
    Some(TdTypEnum::TdFuncTyp(func)) => func,
    _ => {
      let never: TdTypEnum = get_never_typ(db).into();
      let param_typs = vec![never.clone(); param_count];
      TdFuncTyp::get(db, param_typs, never)
    }
  };
  let func_obj = TdFuncObj::new(
    db,
    "<closure>".to_string(),
    func_typ.signature(db),
    FuncKind::UserDefined(project_node, runtime_scope),
  );
  Some(func_obj.into())
}

fn construct_from_mapping_expr<'db>(
  db: &'db TypedownDatabase,
  mapping_project_node: ProjectRedNode,
  entries: Vec<(String, ProjectRedNode)>,
  diagnostics: &mut Vec<Diagnostic>,
) -> Option<TdObjEnum<'db>> {
  let typ_result = actual_node_typ(db, mapping_project_node.clone());
  if typ_result
    .typ(db)
    .is_some_and(|typ| typ.is_td_product_typ())
  {
    let mut builtins = BTreeMap::new();
    let mut fields = BTreeMap::new();
    for (key, value_node) in entries {
      if key.starts_with('_') {
        builtins.insert(key, Either::Left(value_node));
      } else {
        fields.insert(key, Either::Left(value_node));
      }
    }
    let product_typ = typ_result.typ(db).unwrap();
    return Some(TdProductObj::new(db, product_typ, None, builtins, fields).into());
  }

  let raw_typ = typ_result.typ(db)?;
  let typ = match raw_typ.runtime_typ(db) {
    Some(typ) => typ,
    None => {
      let (start, len) = mapping_project_node.node.trimmed_range();
      diagnostics.push(Diagnostic::NotConstructible {
        type_name: raw_typ.display_name(db),
        start_offset: start,
        end_offset: start + len,
      });
      return None;
    }
  };
  evaluate_mapping(db, &typ, entries)
}

fn construct_from_sequence<'db>(
  db: &'db TypedownDatabase,
  seq: YamlSequence,
  project: crate::db::types::Project,
  file: crate::db::types::File,
  project_node: ProjectRedNode,
  runtime_scope: RuntimeScope<'db>,
  diagnostics: &mut Vec<Diagnostic>,
) -> Option<TdObjEnum<'db>> {
  let items: Vec<_> = seq
    .values()
    .map(|value| ProjectRedNode::new(project, file, value.syntax().clone()))
    .collect();
  construct_sequence_items(db, items, project_node, runtime_scope, diagnostics)
}

fn construct_from_list<'db>(
  db: &'db TypedownDatabase,
  list: ListLit,
  project: crate::db::types::Project,
  file: crate::db::types::File,
  project_node: ProjectRedNode,
  runtime_scope: RuntimeScope<'db>,
  diagnostics: &mut Vec<Diagnostic>,
) -> Option<TdObjEnum<'db>> {
  let items: Vec<_> = list
    .items()
    .filter_map(|item| item.value())
    .map(|v| ProjectRedNode::new(project, file, v.syntax().clone()))
    .collect();
  construct_sequence_items(db, items, project_node, runtime_scope, diagnostics)
}

fn construct_sequence_items<'db>(
  db: &'db TypedownDatabase,
  items: Vec<ProjectRedNode>,
  project_node: ProjectRedNode,
  runtime_scope: RuntimeScope<'db>,
  diagnostics: &mut Vec<Diagnostic>,
) -> Option<TdObjEnum<'db>> {
  let typ_result = actual_node_typ(db, project_node.clone());
  let raw_typ = typ_result.typ(db)?;
  let typ = match raw_typ.runtime_typ(db) {
    Some(t) => t,
    None => {
      let (start, len) = project_node.node.trimmed_range();
      diagnostics.push(Diagnostic::NotConstructible {
        type_name: raw_typ.display_name(db),
        start_offset: start,
        end_offset: start + len,
      });
      return None;
    }
  };
  if typ.is_td_list_typ() {
    let list_items = items.into_iter().map(Either::Left).collect();
    return Some(TdListObj::new(db, list_items).into());
  }
  let args: Vec<_> = items
    .into_iter()
    .filter_map(|item| evaluate_node(db, item, runtime_scope).value(db))
    .collect();
  typ.construct(db, project_node.project, args)
}

fn construct_from_str_lit<'db>(
  db: &'db TypedownDatabase,
  str_lit: StrLit,
  project: crate::db::types::Project,
  file: crate::db::types::File,
  project_node: ProjectRedNode,
  runtime_scope: RuntimeScope<'db>,
  diagnostics: &mut Vec<Diagnostic>,
) -> Option<TdObjEnum<'db>> {
  let typ_result = actual_node_typ(db, project_node.clone());
  let raw_typ = typ_result.typ(db)?;
  let typ = match raw_typ.runtime_typ(db) {
    Some(t) => t,
    None => {
      let (start, len) = project_node.node.trimmed_range();
      diagnostics.push(Diagnostic::NotConstructible {
        type_name: raw_typ.display_name(db),
        start_offset: start,
        end_offset: start + len,
      });
      return None;
    }
  };
  if str_lit.is_interpolated() {
    let obj = evaluate_interpolated(db, runtime_scope, str_lit, project, file)?;
    typ.construct(db, project, vec![obj])
  } else {
    // A string containing only a math literal (e.g. "$E = mc^2$") evaluates as TdMathObj
    let children: Vec<_> = project_node.node.children().collect();
    let has_text = children.iter().any(|c| {
      use crate::syntax::syntax_kind::SyntaxKind;
      matches!(
        c.kind(),
        SyntaxKind::DqStrContent | SyntaxKind::SqStrContent
      ) && !c.text().trim().is_empty()
    });
    if !has_text
      && let Some(math_node) = children.iter().find_map(|c| MathLit::cast(c.clone()))
      && let Some(val) = math_node.value()
    {
      return typ.construct(db, project, vec![TdMathObj::new(db, val).into()]);
    }
    let text: String = str_lit
      .fragments()
      .filter_map(|f| match f {
        Either::Left(s) => Some(s),
        _ => None,
      })
      .collect::<String>();
    typ.construct(db, project, vec![TdStringObj::new(db, text).into()])
  }
}

fn construct_from_number<'db>(
  db: &'db TypedownDatabase,
  num: NumberLit,
  project_node: ProjectRedNode,
  diagnostics: &mut Vec<Diagnostic>,
) -> Option<TdObjEnum<'db>> {
  let val = num.value()?;
  let typ_result = actual_node_typ(db, project_node.clone());
  let raw_typ = typ_result.typ(db)?;
  let typ = match raw_typ.runtime_typ(db) {
    Some(t) => t,
    None => {
      let (start, len) = project_node.node.trimmed_range();
      diagnostics.push(Diagnostic::NotConstructible {
        type_name: raw_typ.display_name(db),
        start_offset: start,
        end_offset: start + len,
      });
      return None;
    }
  };
  typ.construct(
    db,
    project_node.project,
    vec![TdNumberObj::new(db, val).into()],
  )
}

fn construct_from_math<'db>(
  db: &'db TypedownDatabase,
  math: MathLit,
  project_node: ProjectRedNode,
  diagnostics: &mut Vec<Diagnostic>,
) -> Option<TdObjEnum<'db>> {
  let val = math.value()?;
  let typ_result = actual_node_typ(db, project_node.clone());
  let raw_typ = typ_result.typ(db)?;
  let typ = match raw_typ.runtime_typ(db) {
    Some(t) => t,
    None => {
      let (start, len) = project_node.node.trimmed_range();
      diagnostics.push(Diagnostic::NotConstructible {
        type_name: raw_typ.display_name(db),
        start_offset: start,
        end_offset: start + len,
      });
      return None;
    }
  };
  typ.construct(
    db,
    project_node.project,
    vec![TdMathObj::new(db, val).into()],
  )
}

fn evaluate_prefix<'db>(
  db: &'db TypedownDatabase,
  prefix: PrefixExpr,
  project: crate::db::types::Project,
  file: crate::db::types::File,
  runtime_scope: RuntimeScope<'db>,
) -> Option<TdObjEnum<'db>> {
  let op = prefix.op()?.kind()?;
  let op_str = match &op {
    crate::syntax::ast::YamlOpKind::Minus => "-",
    crate::syntax::ast::YamlOpKind::Plus => "+",
    crate::syntax::ast::YamlOpKind::Tilde => "~",
    _ => return None,
  };
  let operand_expr = prefix.expr()?;
  let operand_node = ProjectRedNode::new(project, file, operand_expr.syntax().clone());
  let operand_obj = evaluate_node(db, operand_node, runtime_scope).value(db)?;
  match op_str {
    "-" | "+" => {
      let num = operand_obj.as_td_number_obj()?;
      let val = num.value(db);
      let result = if op_str == "-" { -val } else { val };
      Some(TdNumberObj::new(db, result).into())
    }
    "~" => {
      let is_falsy = operand_obj.as_td_bool_obj().is_some_and(|b| !b.value(db));
      Some(TdBoolObj::new(db, is_falsy).into())
    }
    _ => None,
  }
}

fn evaluate_postfix<'db>(
  db: &'db TypedownDatabase,
  postfix: PostfixExpr,
  project: crate::db::types::Project,
  file: crate::db::types::File,
  runtime_scope: RuntimeScope<'db>,
) -> Option<TdObjEnum<'db>> {
  let op = postfix.op()?.kind()?;
  if !matches!(op, crate::syntax::ast::YamlOpKind::Not) {
    // `?` is represented by YamlOp::Not in the AST? Let me handle it by op text
    // Actually check what ? postfix maps to
    return None;
  }
  let operand_expr = postfix.expr()?;
  let operand_node = ProjectRedNode::new(project, file, operand_expr.syntax().clone());
  let inner = evaluate_node(db, operand_node, runtime_scope).value(db)?;
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

fn evaluate_binary<'db>(
  db: &'db TypedownDatabase,
  binary: BinaryExpr,
  project: crate::db::types::Project,
  file: crate::db::types::File,
  runtime_scope: RuntimeScope<'db>,
) -> Option<TdObjEnum<'db>> {
  let op_kind = binary.op()?.kind()?;
  let op_str = yaml_op_to_str(&op_kind)?;
  let left_expr = binary.left()?;
  let right_expr = binary.right()?;
  let left_node = ProjectRedNode::new(project, file, left_expr.syntax().clone());
  let right_node = ProjectRedNode::new(project, file, right_expr.syntax().clone());
  let left_obj = evaluate_node(db, left_node, runtime_scope).value(db)?;
  let right_obj = evaluate_node(db, right_node, runtime_scope).value(db)?;
  match op_str {
    "+" | "-" | "*" | "/" | "%" | "**" => {
      let lnum = left_obj.as_td_number_obj()?;
      let rnum = right_obj.as_td_number_obj()?;
      let lval = lnum.value(db);
      let rval = rnum.value(db);
      let result = match op_str {
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
      let result = compare_objects(db, op_str, &left_obj, &right_obj);
      Some(TdBoolObj::new(db, result).into())
    }
    "&&" | "||" => {
      let lbool = left_obj.as_td_bool_obj()?;
      let rbool = right_obj.as_td_bool_obj()?;
      let result = match op_str {
        "&&" => lbool.value(db) && rbool.value(db),
        "||" => lbool.value(db) || rbool.value(db),
        _ => unreachable!(),
      };
      Some(TdBoolObj::new(db, result).into())
    }
    _ => None,
  }
}

fn yaml_op_to_str(op: &crate::syntax::ast::YamlOpKind) -> Option<&'static str> {
  use crate::syntax::ast::YamlOpKind;
  Some(match op {
    YamlOpKind::Plus => "+",
    YamlOpKind::Minus => "-",
    YamlOpKind::Mul => "*",
    YamlOpKind::Div => "/",
    YamlOpKind::Mod => "%",
    YamlOpKind::Pow => "**",
    YamlOpKind::Eq => "==",
    YamlOpKind::NotEq => "!=",
    YamlOpKind::Lt => "<",
    YamlOpKind::Gt => ">",
    YamlOpKind::LtEq => "<=",
    YamlOpKind::GtEq => ">=",
    YamlOpKind::And => "&&",
    YamlOpKind::Or => "||",
    _ => return None,
  })
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
  index: IndexExpr,
  project: crate::db::types::Project,
  file: crate::db::types::File,
  runtime_scope: RuntimeScope<'db>,
  diagnostics: &mut Vec<Diagnostic>,
) -> Option<TdObjEnum<'db>> {
  let indices = index.indices();
  if indices.len() != 1 {
    return None;
  }
  let index_expr_node = &indices[0];
  let expr = index.expr()?;
  let container_node = ProjectRedNode::new(project, file, expr.syntax().clone());
  let index_node = ProjectRedNode::new(project, file, index_expr_node.syntax().clone());
  let container = evaluate_node(db, container_node, runtime_scope).value(db)?;
  let index_obj = evaluate_node(db, index_node.clone(), runtime_scope).value(db)?;

  if let Some(number) = index_obj.as_td_number_obj()
    && let Some(len) = container.len(db)
  {
    let idx = number.value(db) as usize;
    if idx >= len {
      let (trimmed_offset, trimmed_len) = index_node.node.trimmed_range();
      diagnostics.push(Diagnostic::IndexOutOfBounds {
        index: idx,
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
  args: Vec<ProjectRedNode>,
) -> Option<TdObjEnum<'db>> {
  match kind {
    BuiltinMacroKind::Fref => construct_fref(db, args),
  }
}

fn evaluate_interpolated<'db>(
  db: &'db TypedownDatabase,
  runtime_scope: RuntimeScope<'db>,
  str_lit: StrLit,
  project: crate::db::types::Project,
  file: crate::db::types::File,
) -> Option<TdObjEnum<'db>> {
  let mut value = String::new();
  for fragment in str_lit.fragments() {
    match fragment {
      Either::Left(text) => value.push_str(&text),
      Either::Right(interp) => {
        let inner_expr = interp.expr()?;
        let inner_node = ProjectRedNode::new(project, file, inner_expr.syntax().clone());
        let obj = evaluate_node(db, inner_node, runtime_scope).value(db)?;
        let to_string_func = obj.lookup_method(db, "to_string")?;
        let call_project = runtime_scope.scope(db).project(db);
        let string_obj = to_string_func
          .call(db, call_project, Some(obj), vec![])
          .ok()?;
        let string_value = string_obj.as_td_string_obj()?;
        value.push_str(&string_value.value(db));
      }
    }
  }
  Some(TdStringObj::new(db, value).into())
}

// Walk an MdBody tree, collecting text and evaluating interpolations
fn evaluate_markdown_text<'db>(
  db: &'db TypedownDatabase,
  project_node: ProjectRedNode,
  runtime_scope: RuntimeScope<'db>,
) -> String {
  fn collect(
    db: &TypedownDatabase,
    project: crate::db::types::Project,
    file: crate::db::types::File,
    node: crate::syntax::red::RedNode,
    runtime_scope: RuntimeScope<'_>,
    out: &mut String,
  ) {
    // Interp fragment: evaluate the inner expression and stringify
    if node.kind() == SyntaxKind::InterpFragment {
      if let Some(frag) = InterpFragment::cast(node.clone())
        && let Some(expr) = frag.expr()
      {
        let inner_node = ProjectRedNode::new(project, file, expr.syntax().clone());
        if let Some(obj) = evaluate_node(db, inner_node, runtime_scope).value(db)
          && let Some(to_string) = obj.lookup_method(db, "to_string")
        {
          let call_project = runtime_scope.scope(db).project(db);
          if let Ok(s) = to_string.call(db, call_project, Some(obj), vec![])
            && let Some(sv) = s.as_td_string_obj()
          {
            out.push_str(&sv.value(db));
            return;
          }
        }
      }
      return;
    }
    // Math/code blocks/inlines: preserve as raw text
    if MathBlock::cast(node.clone()).is_some()
      || CodeBlock::cast(node.clone()).is_some()
      || InlineMath::cast(node.clone()).is_some()
      || InlineCode::cast(node.clone()).is_some()
    {
      out.push_str(&node.text());
      return;
    }
    // Leaf token: append text
    if node.is_token() {
      let text = node.text();
      if !text.is_empty() {
        out.push_str(&text);
      }
      return;
    }
    for child in node.children() {
      collect(db, project, file, child, runtime_scope, out);
    }
  }

  let mut result = String::new();
  collect(
    db,
    project_node.project,
    project_node.file,
    project_node.node.clone(),
    runtime_scope,
    &mut result,
  );
  result
}

fn mapping_entries_from_yaml_mapping(
  mapping: YamlMapping,
  project: crate::db::types::Project,
  file: crate::db::types::File,
) -> Vec<(String, ProjectRedNode)> {
  mapping
    .entries()
    .map(|(k, v)| (k, ProjectRedNode::new(project, file, v.syntax().clone())))
    .collect()
}

fn mapping_entries_from_dict_lit(
  dict: DictLit,
  project: crate::db::types::Project,
  file: crate::db::types::File,
) -> Vec<(String, ProjectRedNode)> {
  dict
    .entries()
    .filter_map(|e| {
      let k = e.key()?;
      let v = e.value()?;
      Some((k, ProjectRedNode::new(project, file, v.syntax().clone())))
    })
    .collect()
}

fn evaluate_mapping<'db>(
  db: &'db TypedownDatabase,
  typ: &TdTypEnum<'db>,
  entries: Vec<(String, ProjectRedNode)>,
) -> Option<TdObjEnum<'db>> {
  // Schema meta type: build schema type
  if typ.is_td_schema_meta_typ() {
    let properties_entries = entries
      .iter()
      .find(|(key, _)| key == "properties")
      .and_then(|(_, props_node)| {
        let node = props_node.node.clone();
        let mapping = crate::syntax::ast::YamlMapping::cast(node.clone()).or_else(|| {
          crate::syntax::ast::DictLit::cast(node.clone()).and_then(|_| {
            // DictLit can also act as a mapping
            None // handle below
          })
        });
        mapping.map(|m| mapping_entries_from_yaml_mapping(m, props_node.project, props_node.file))
      })
      .unwrap_or_default();
    let mut fields = BTreeMap::new();
    for (prop_name, prop_node) in properties_entries {
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
      if let Some(desc) = resolve_property_descriptor(db, prop_node, &mut vec![]) {
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

  // Schema type: build schema object
  if let TdTypEnum::TdSchemaTyp(schema_typ) = &typ {
    let mut builtins = BTreeMap::new();
    let mut fields = BTreeMap::new();
    for (key, val_node) in &entries {
      if key.starts_with('_') {
        builtins.insert(key.clone(), Either::Left(val_node.clone()));
      } else {
        fields.insert(key.clone(), Either::Left(val_node.clone()));
      }
    }
    let project = entries.first().map(|(_, n)| n.project)?;
    return Some(
      TdSchemaObj::new(db, (*schema_typ).into(), project, None, builtins, fields).into(),
    );
  }

  // Product type
  if let TdTypEnum::TdProductTyp(product_typ) = &typ {
    let mut builtins = BTreeMap::new();
    let mut fields = BTreeMap::new();
    for (key, val_node) in entries {
      if key.starts_with('_') {
        builtins.insert(key, Either::Left(val_node));
      } else {
        fields.insert(key, Either::Left(val_node));
      }
    }
    return Some(TdProductObj::new(db, (*product_typ).into(), None, builtins, fields).into());
  }

  let dict_entries: BTreeMap<_, _> = entries
    .into_iter()
    .map(|(key, val_node)| (key, Either::Left(val_node)))
    .collect();
  Some(TdDictObj::new(db, dict_entries).into())
}

fn construct_fref<'db>(
  db: &'db TypedownDatabase,
  args: Vec<ProjectRedNode>,
) -> Option<TdObjEnum<'db>> {
  let arg = args.first()?;
  let path_string = StrLit::cast(arg.node.clone()).and_then(|s| s.plain_value())?;

  let project = arg.project;
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
  use crate::db::derived::parse_file::parse_file;
  use crate::db::types::derived::obj_system::TdFuncObj;
  use crate::db::types::{
    File, FileHandle, FileMetadata, FuncKind, FuncSignature, LazyTyp, LitValue, NativeFuncKind,
    PROTOCOL_CALL, PROTOCOL_INDEX, Project, TdProductTyp, TdSchemaTyp, TdTypEnum,
  };
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

    let string_typ: TdTypEnum = get_string_typ(&db).into();
    let number_typ: TdTypEnum = get_number_typ(&db).into();
    let bool_typ: TdTypEnum = get_bool_typ(&db).into();
    let list_typ: TdTypEnum = get_list_typ(&db).into();
    let dict_typ: TdTypEnum = get_dict_typ(&db).into();
    let null_typ: TdTypEnum = get_null_typ(&db).into();

    assert_eq!(string_typ.runtime_typ(&db), Some(string_typ.clone()));
    assert_eq!(number_typ.runtime_typ(&db), Some(number_typ.clone()));
    assert_eq!(bool_typ.runtime_typ(&db), Some(bool_typ.clone()));
    assert_eq!(list_typ.runtime_typ(&db), None);
    assert_eq!(dict_typ.runtime_typ(&db), None);
    assert_eq!(null_typ.runtime_typ(&db), Some(null_typ.clone()));

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

    let sum_typ: TdTypEnum<'_> = get_sum_typ(
      &db,
      vec![
        LazyTyp::eager(get_string_typ(&db).into()),
        LazyTyp::eager(get_number_typ(&db).into()),
      ],
    )
    .into();
    assert_eq!(sum_typ.runtime_typ(&db), None);

    let never_typ: TdTypEnum = get_never_typ(&db).into();
    assert_eq!(never_typ.runtime_typ(&db), None);

    let file = File::new(
      &db,
      FileHandle::Content(
        PathBuf::from("test.td"),
        "\"hello\"\n".into(),
        FileMetadata::default(),
      ),
    );
    let project = Project::new(&db, PathBuf::from("/vault"), HashMap::new());
    let root_node = parse_file(&db, project, file).ast(&db);

    let mut diagnostics = vec![];
    let scope = get_file_runtime_scope(&db, project, file);
    let obj = construct_from_node(&db, root_node, scope, &mut diagnostics);
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
