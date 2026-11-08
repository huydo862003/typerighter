//! Tracked query to get the actual (bottom-up) type of an AST node
// This is the idea of bidirectional typechecking

use std::collections::BTreeMap;
use std::collections::HashMap;

use crate::db::TypedownDatabase;
use crate::db::derived::evaluate::evaluate_typ::evaluate_typ;
use crate::db::derived::get_builtin_typs::{
  get_bool_typ, get_date_typ, get_datetime_typ, get_func_typ, get_list_typ, get_lit_typ,
  get_math_typ, get_null_typ, get_number_typ, get_obj_typ, get_string_typ, get_sum_typ,
  get_time_typ,
};
use crate::db::derived::get_vault_config::get_vault_config;
use crate::db::derived::name_resolver::file_symbol::file_symbol;
use crate::db::derived::name_resolver::referee::referee;
use crate::db::derived::typechecker::expected_node_typ::expected_node_typ;
use crate::db::derived::typechecker::get_symbol_typ::{get_symbol_typ, infer_param_typ_from_body};
use crate::db::types::derived::obj_system::{
  TdProductTyp, TdStaticTyp, is_valid_iso_date, is_valid_iso_datetime, is_valid_iso_time,
};
use crate::db::types::{
  Binder, BuiltinMacroKind, FuncSignature, LazyTyp, LitValue, ProjectRedNode, Symbol, SymbolKind,
  TdBoundVariableTyp, TdDictTyp, TdFreeVariableTyp, TdListTyp, TdMetatypeInstanceTyp, TdTypEnum,
  TypParams, TypResult, Variance,
};
use crate::db::types::{File, Project};
use crate::db::utils::resolve_fref_path;
use crate::syntax::ast::{
  AstNode, CallExpr, ClosureExpr, DictLit, Expr, IdentLit, IndexExpr, MathLit, NumberLit,
  PostfixExpr, SourceFile, StringLit, YamlFrontmatter, YamlMapping, YamlOpKind,
};
use crate::syntax::diagnostic::*;
use crate::syntax::syntax_kind::SyntaxKind;
use typedown_incremental::QueryDatabase;
use typedown_macros::query_derived;
use typedown_types::either::Either;

// Infer the type of an AST node bottom-up from its structure
// Exception: closures read expected(closure) to get param types
#[query_derived(no_hash)]
pub fn actual_node_typ<'db>(
  db: &'db TypedownDatabase,
  project_node: ProjectRedNode,
) -> TypResult<'db> {
  let node = project_node.node.clone();
  let project = project_node.project;
  let file = project_node.file;

  // SourceFile/YamlFrontmatter: delegate to the frontmatter mapping
  if let Some(source_file) = SourceFile::cast(node.clone()) {
    if let Some(frontmatter) = source_file.frontmatter()
      && let Some(mapping) = frontmatter.mapping()
    {
      let mapping_node = ProjectRedNode::new(project, file, mapping.syntax().clone());
      return actual_node_typ(db, mapping_node);
    }
    return TypResult::new(db, None, vec![]);
  }
  if let Some(frontmatter) = YamlFrontmatter::cast(node.clone()) {
    if let Some(mapping) = frontmatter.mapping() {
      let mapping_node = ProjectRedNode::new(project, file, mapping.syntax().clone());
      return actual_node_typ(db, mapping_node);
    }
    return TypResult::new(db, None, vec![]);
  }

  // IdentLit covers ident references plus YAML null/bool scalars
  if let Some(ident) = IdentLit::cast(node.clone()) {
    return match ident.value().as_deref() {
      Some("null") => TypResult::new(db, Some(get_null_typ(db).into()), vec![]),
      Some("true") => TypResult::new(
        db,
        Some(get_lit_typ(db, LitValue::Bool(true)).into()),
        vec![],
      ),
      Some("false") => TypResult::new(
        db,
        Some(get_lit_typ(db, LitValue::Bool(false)).into()),
        vec![],
      ),
      _ => {
        let resolved = referee(db, project_node);
        match resolved.value(db) {
          Some(symbol) => get_symbol_typ(db, symbol),
          None => TypResult::new(db, None, vec![]),
        }
      }
    };
  }

  if let Some(string_lit) = StringLit::cast(node.clone()) {
    return if string_lit.is_interpolated() {
      TypResult::new(db, Some(get_string_typ(db).into()), vec![])
    } else {
      // A string whose only content is a math literal (e.g. "$x^2$") has math type
      let children: Vec<_> = node.children().collect();
      let has_text = children.iter().any(|child| {
        matches!(
          child.kind(),
          SyntaxKind::DqStrContent | SyntaxKind::SqStrContent
        ) && !child.text().trim().is_empty()
      });
      if !has_text
        && children
          .iter()
          .any(|child| child.kind() == SyntaxKind::MathLit)
      {
        return TypResult::new(db, Some(get_math_typ(db).into()), vec![]);
      }
      let value: String = string_lit
        .fragments()
        .filter_map(|fragment| match fragment {
          Either::Left(string) => Some(string),
          _ => None,
        })
        .collect::<Vec<_>>()
        .concat();
      let typ = if is_valid_iso_datetime(&value) {
        get_datetime_typ(db).into()
      } else if is_valid_iso_date(&value) {
        get_date_typ(db).into()
      } else if is_valid_iso_time(&value) {
        get_time_typ(db).into()
      } else {
        get_lit_typ(db, LitValue::String(value)).into()
      };
      TypResult::new(db, Some(typ), vec![])
    };
  }

  if let Some(number) = NumberLit::cast(node.clone()) {
    let value = number
      .value()
      .map(|value| value.to_string())
      .unwrap_or_default();
    return TypResult::new(
      db,
      Some(get_lit_typ(db, LitValue::Number(value)).into()),
      vec![],
    );
  }

  if MathLit::cast(node.clone()).is_some() {
    return TypResult::new(db, Some(get_math_typ(db).into()), vec![]);
  }

  if let Some(expr) = Expr::cast(node.clone()) {
    return match expr {
      Expr::YamlMapping(_) | Expr::DictLit(_) => get_mapping_typ(db, project_node),
      Expr::YamlSequence(sequence) => get_sequence_typ(
        db,
        sequence
          .values()
          .map(|expr| ProjectRedNode::new(project, file, expr.syntax().clone()))
          .collect(),
      ),
      Expr::ListLit(list) => get_sequence_typ(
        db,
        list
          .items()
          .filter_map(|item| item.value())
          .map(|expr| ProjectRedNode::new(project, file, expr.syntax().clone()))
          .collect(),
      ),
      Expr::CallExpr(call) => get_call_typ(db, call, project, file),
      Expr::IndexExpr(index) => get_index_typ(db, index, project, file),
      Expr::BinaryExpr(binary) => {
        let op = binary.op().and_then(|op| op.kind());
        match op {
          Some(YamlOpKind::Tag(_)) => {
            // Tag: actual type is the tag symbol's type
            let Some(tag_expr) = binary.left() else {
              return TypResult::new(db, None, vec![]);
            };
            let tag_node = ProjectRedNode::new(project, file, tag_expr.syntax().clone());
            get_tag_typ(db, tag_node)
          }
          Some(YamlOpKind::Dot) => {
            let Some(left_expr) = binary.left() else {
              return TypResult::new(db, None, vec![]);
            };
            let Some(right_expr) = binary.right() else {
              return TypResult::new(db, None, vec![]);
            };
            let left_node = ProjectRedNode::new(project, file, left_expr.syntax().clone());
            let right_node = ProjectRedNode::new(project, file, right_expr.syntax().clone());
            get_field_access_typ(db, left_node, right_node)
          }
          Some(op_kind) => get_binary_op_typ(db, op_kind),
          None => TypResult::new(db, None, vec![]),
        }
      }
      Expr::PrefixExpr(prefix) => {
        let op = prefix.op().and_then(|op| op.kind());
        get_prefix_typ(db, op)
      }
      Expr::PostfixExpr(postfix) => {
        let op_text = postfix
          .op()
          .map(|op| op.syntax().text())
          .unwrap_or_default();
        get_postfix_typ(db, &op_text, postfix, project, file)
      }
      Expr::ClosureExpr(_) => get_closure_typ(db, project_node),
      Expr::CodeLit(_) => TypResult::new(db, Some(get_string_typ(db).into()), vec![]),
      Expr::ParenExpr(paren) => {
        let Some(inner) = paren.expr() else {
          return TypResult::new(db, None, vec![]);
        };
        let inner_node = ProjectRedNode::new(project, file, inner.syntax().clone());
        actual_node_typ(db, inner_node)
      }
      // Literals that were matched above but Expr covers them too
      Expr::NumberLit(_) | Expr::StringLit(_) | Expr::MathLit(_) | Expr::IdentLit(_) => {
        unreachable!("handled above before Expr::cast")
      }
    };
  }

  TypResult::new(db, None, vec![])
}

fn get_mapping_typ<'db>(db: &'db TypedownDatabase, project_node: ProjectRedNode) -> TypResult<'db> {
  let node = project_node.node.clone();
  let project = project_node.project;
  let file = project_node.file;

  // Collect entries from YamlMapping or DictLit
  let entries: Vec<(String, ProjectRedNode)> =
    if let Some(mapping) = YamlMapping::cast(node.clone()) {
      mapping
        .entries()
        .map(|(key, value)| {
          (
            key,
            ProjectRedNode::new(project, file, value.syntax().clone()),
          )
        })
        .collect()
    } else if let Some(dict) = DictLit::cast(node.clone()) {
      dict
        .entries()
        .filter_map(|expr| {
          let (key, value) = expr.entry()?;
          Some((
            key,
            ProjectRedNode::new(project, file, value.syntax().clone()),
          ))
        })
        .collect()
    } else {
      return TypResult::new(db, None, vec![]);
    };

  // If _type is present, resolve the schema
  for (key, value_node) in &entries {
    if key == "_type" {
      let resolved = referee(db, value_node.clone());
      if let Some(symbol) = resolved.value(db) {
        return evaluate_typ(db, symbol);
      }
      let red_node = &value_node.node;
      let (trimmed_offset, trimmed_len) = red_node.trimmed_range();
      return TypResult::new(
        db,
        None,
        vec![
          TdDiagnosticKind::UnresolvedSchema {
            name: red_node.text(),
          }
          .into_error(trimmed_offset, trimmed_offset + trimmed_len),
        ],
      );
    }
  }

  // No _type: infer a structural shape from the entries
  let mut diagnostics = vec![];
  let mut fields = BTreeMap::new();
  for (key, value_node) in entries {
    let field_result = actual_node_typ(db, value_node);
    diagnostics.extend(field_result.diagnostics(db).iter().cloned());
    if let Some(typ) = field_result.typ(db) {
      fields.insert(key, LazyTyp::eager(typ));
    }
  }
  TypResult::new(
    db,
    Some(TdProductTyp::new(db, None, fields).into()),
    diagnostics,
  )
}

fn get_tag_typ<'db>(db: &'db TypedownDatabase, tag_node: ProjectRedNode) -> TypResult<'db> {
  let resolved = referee(db, tag_node.clone());
  match resolved.value(db) {
    Some(symbol) => evaluate_typ(db, symbol),
    None => {
      let red_node = &tag_node.node;
      let (trimmed_offset, trimmed_len) = red_node.trimmed_range();
      TypResult::new(
        db,
        None,
        vec![
          TdDiagnosticKind::UnresolvedSchema {
            name: red_node.text(),
          }
          .into_error(trimmed_offset, trimmed_offset + trimmed_len),
        ],
      )
    }
  }
}

fn get_prefix_typ<'db>(db: &'db TypedownDatabase, op: Option<YamlOpKind>) -> TypResult<'db> {
  match op {
    Some(YamlOpKind::Minus) | Some(YamlOpKind::Plus) => {
      TypResult::new(db, Some(get_number_typ(db).into()), vec![])
    }
    Some(YamlOpKind::Tilde) => TypResult::new(db, Some(get_bool_typ(db).into()), vec![]),
    _ => TypResult::new(db, None, vec![]),
  }
}

fn get_postfix_typ<'db>(
  db: &'db TypedownDatabase,
  op_text: &str,
  postfix: PostfixExpr,
  project: Project,
  file: File,
) -> TypResult<'db> {
  match op_text {
    "?" => {
      let Some(operand_expr) = postfix.expr() else {
        return TypResult::new(db, None, vec![]);
      };
      let operand_node = ProjectRedNode::new(project, file, operand_expr.syntax().clone());
      actual_node_typ(db, operand_node)
    }
    _ => TypResult::new(db, None, vec![]),
  }
}

fn get_field_access_typ<'db>(
  db: &'db TypedownDatabase,
  left_node: ProjectRedNode,
  right_node: ProjectRedNode,
) -> TypResult<'db> {
  let left_result = actual_node_typ(db, left_node);
  let mut diagnostics = left_result.diagnostics(db).clone();
  let left_typ = match left_result.typ(db) {
    Some(typ) => typ,
    None => return TypResult::new(db, None, diagnostics),
  };
  let field_name = match IdentLit::cast(right_node.node.clone()).and_then(|ident| ident.value()) {
    Some(name) => name,
    None => return TypResult::new(db, None, diagnostics),
  };
  match left_typ.lookup_field_typ(db, &field_name) {
    Some(typ) => TypResult::new(db, Some(typ), diagnostics),
    None => {
      let red_node = &right_node.node;
      let (trimmed_offset, trimmed_len) = red_node.trimmed_range();
      diagnostics.push(
        TdDiagnosticKind::UnknownField {
          field: field_name,
          on_typ: left_typ.display_name(db),
        }
        .into_error(trimmed_offset, trimmed_offset + trimmed_len),
      );
      TypResult::new(db, None, diagnostics)
    }
  }
}

fn get_binary_op_typ<'db>(db: &'db TypedownDatabase, op: YamlOpKind) -> TypResult<'db> {
  match op {
    YamlOpKind::Plus
    | YamlOpKind::Minus
    | YamlOpKind::Mul
    | YamlOpKind::Div
    | YamlOpKind::Mod
    | YamlOpKind::Pow => TypResult::new(db, Some(get_number_typ(db).into()), vec![]),
    YamlOpKind::Eq
    | YamlOpKind::NotEq
    | YamlOpKind::Lt
    | YamlOpKind::Gt
    | YamlOpKind::LtEq
    | YamlOpKind::GtEq
    | YamlOpKind::And
    | YamlOpKind::Or => TypResult::new(db, Some(get_bool_typ(db).into()), vec![]),
    _ => TypResult::new(db, None, vec![]),
  }
}

fn get_sequence_typ<'db>(db: &'db TypedownDatabase, items: Vec<ProjectRedNode>) -> TypResult<'db> {
  let mut diagnostics = vec![];
  let mut arms = vec![];

  for item in items {
    let item_result = actual_node_typ(db, item);
    diagnostics.extend(item_result.diagnostics(db).iter().cloned());
    if let Some(typ) = item_result.typ(db) {
      arms.push(LazyTyp::eager(typ));
    }
  }

  let element = if arms.len() == 1 {
    arms.into_iter().next().unwrap()
  } else {
    LazyTyp::eager(get_sum_typ(db, arms.into_iter().collect()).into())
  };
  let list_typ = get_list_typ(db).instantiate(db, vec![element]).typ(db);
  TypResult::new(db, Some(list_typ), diagnostics)
}

fn get_call_typ<'db>(
  db: &'db TypedownDatabase,
  call: CallExpr,
  project: crate::db::types::Project,
  file: crate::db::types::File,
) -> TypResult<'db> {
  let Some(callee_expr) = call.callee() else {
    return TypResult::new(db, None, vec![]);
  };
  let callee_node = ProjectRedNode::new(project, file, callee_expr.syntax().clone());

  // Check if callee is a macro
  let resolved = referee(db, callee_node.clone());
  if let Some(symbol) = resolved.value(db)
    && let SymbolKind::BuiltinMacro(kind) = symbol.kind(db)
  {
    let arg_nodes: Vec<ProjectRedNode> = call
      .args()
      .into_iter()
      .map(|expr| ProjectRedNode::new(project, file, expr.syntax().clone()))
      .collect();
    return get_macro_call_typ(db, kind, arg_nodes);
  }

  let callee_result = actual_node_typ(db, callee_node);
  let mut diagnostics = callee_result.diagnostics(db).clone();
  let callee_typ = match callee_result.typ(db) {
    Some(typ) => typ,
    None => return TypResult::new(db, None, diagnostics),
  };

  let arg_typs: Vec<TdTypEnum> = call
    .args()
    .into_iter()
    .filter_map(|arg| {
      let node = ProjectRedNode::new(project, file, arg.syntax().clone());
      actual_node_typ(db, node).typ(db)
    })
    .collect();

  let call_result = callee_typ.call_typ(db, arg_typs);
  let (call_offset, call_len) = call.syntax().trimmed_range();
  for kind in call_result.diagnostics {
    diagnostics.push(kind.into_error(call_offset, call_len));
  }
  if let Some(signature) = call_result.signature {
    return TypResult::new(db, Some(signature.ret(db)), diagnostics);
  }

  TypResult::new(db, None, diagnostics)
}

fn get_macro_call_typ<'db>(
  db: &'db TypedownDatabase,
  kind: BuiltinMacroKind,
  args: Vec<ProjectRedNode>,
) -> TypResult<'db> {
  match kind {
    BuiltinMacroKind::Fref => get_fref_typ(db, args),
  }
}

// fref("file.td") returns link[T] where T is the target file's schema type
fn get_fref_typ<'db>(db: &'db TypedownDatabase, args: Vec<ProjectRedNode>) -> TypResult<'db> {
  if args.len() != 1 {
    let (trimmed_offset, trimmed_len) = args
      .first()
      .map(|arg| arg.node.trimmed_range())
      .unwrap_or((0, 0));
    return TypResult::new(
      db,
      None,
      vec![
        TdDiagnosticKind::WrongArgCount {
          expected: 1,
          got: args.len(),
        }
        .into_error(trimmed_offset, trimmed_offset + trimmed_len),
      ],
    );
  }

  let arg_node = args[0].clone();
  let red_node = &arg_node.node;
  let (trimmed_offset, trimmed_len) = red_node.trimmed_range();

  // Argument must be a plain (non-interpolated) string literal
  let path_string = match StringLit::cast(red_node.clone()) {
    Some(string_lit) if !string_lit.is_interpolated() => string_lit
      .fragments()
      .filter_map(|fragment| match fragment {
        Either::Left(string) => Some(string),
        _ => None,
      })
      .collect::<Vec<_>>()
      .concat(),
    _ => {
      return TypResult::new(
        db,
        None,
        vec![
          TdDiagnosticKind::ArgTypMismatch {
            expected: "string".to_string(),
          }
          .into_error(trimmed_offset, trimmed_offset + trimmed_len),
        ],
      );
    }
  };

  let project = arg_node.project;
  let files = project.files(db);
  let root_dir = get_vault_config(db, project).root_dir(db);
  let file_dir = arg_node
    .file
    .handle(db)
    .path()
    .and_then(|parent| parent.parent().map(|diagnostic| diagnostic.to_path_buf()))
    .unwrap_or_else(|| root_dir.clone());
  let target_path = resolve_fref_path(&path_string, &file_dir, &root_dir);

  let target_file = match target_path.and_then(|path| files.get(&path)) {
    Some(file) => *file,
    None => {
      return TypResult::new(
        db,
        None,
        vec![
          TdDiagnosticKind::UnresolvedFileRef { path: path_string }
            .into_error(trimmed_offset, trimmed_offset + trimmed_len),
        ],
      );
    }
  };
  let target_symbol = file_symbol(db, project, target_file);

  match target_symbol.value(db) {
    Some(symbol) => get_symbol_typ(db, symbol),
    None => TypResult::new(
      db,
      None,
      vec![
        TdDiagnosticKind::UnresolvedSchema { name: path_string }
          .into_error(trimmed_offset, trimmed_offset + trimmed_len),
      ],
    ),
  }
}

fn get_index_typ<'db>(
  db: &'db TypedownDatabase,
  index: IndexExpr,
  project: Project,
  file: File,
) -> TypResult<'db> {
  let Some(base_expr) = index.expr() else {
    return TypResult::new(db, None, vec![]);
  };
  let base_node = ProjectRedNode::new(project, file, base_expr.syntax().clone());
  let expr_result = actual_node_typ(db, base_node.clone());
  let mut diagnostics = expr_result.diagnostics(db).clone();

  let expr_typ = match expr_result.typ(db) {
    Some(typ) => typ,
    None => return TypResult::new(db, None, diagnostics),
  };

  // Generic instantiation: if the base ident resolves to a generic type, use that
  let expr_typ = if expr_typ.arity(db) == 0
    && IdentLit::cast(base_expr.syntax().clone()).is_some()
    && let Some(symbol) = referee(db, base_node).value(db)
    && let Some(typ) = evaluate_typ(db, symbol).typ(db)
    && typ.arity(db) > 0
  {
    typ
  } else {
    expr_typ
  };

  let index_nodes: Vec<ProjectRedNode> = index
    .indices()
    .into_iter()
    .map(|expr| ProjectRedNode::new(project, file, expr.syntax().clone()))
    .collect();

  let first_index_node = index_nodes.first().cloned();

  if expr_typ.arity(db) > 0 {
    let mut arg_typs = vec![];
    for index_node in index_nodes {
      let index_result = actual_node_typ(db, index_node);
      diagnostics.extend(index_result.diagnostics(db).iter().cloned());
      match index_result.typ(db) {
        Some(typ) => arg_typs.push(typ),
        None => return TypResult::new(db, None, diagnostics),
      }
    }
    let instantiate_result =
      expr_typ.instantiate(db, arg_typs.into_iter().map(LazyTyp::eager).collect());
    diagnostics.extend(instantiate_result.diagnostics(db).iter().cloned());
    return TypResult::new(db, Some(instantiate_result.typ(db)), diagnostics);
  }

  let key_typ = first_index_node
    .and_then(|node| actual_node_typ(db, node).typ(db))
    .unwrap_or_else(|| get_number_typ(db).into());

  if let Some(signature) = expr_typ.index_typ(db, &key_typ) {
    return TypResult::new(db, Some(signature.ret(db)), diagnostics);
  }

  TypResult::new(db, None, diagnostics)
}

// Generalize free variables back to bound variables (free -> bound)
fn generalize_free_variables<'db>(
  db: &'db TypedownDatabase,
  typ: &TdTypEnum<'db>,
  mapping: &HashMap<TdFreeVariableTyp<'db>, TdBoundVariableTyp<'db>>,
) -> TdTypEnum<'db> {
  match typ {
    TdTypEnum::TdFreeVariableTyp(free_variable) => {
      if let Some(bound) = mapping.get(free_variable) {
        (*bound).into()
      } else {
        typ.clone()
      }
    }
    TdTypEnum::TdFuncTyp(func) => {
      let signature = func.signature(db);
      let new_params: Vec<TdTypEnum<'db>> = signature
        .params(db)
        .iter()
        .map(|param| generalize_free_variables(db, param, mapping))
        .collect();
      let new_ret = generalize_free_variables(db, &signature.ret(db), mapping);
      get_func_typ(
        db,
        FuncSignature::new(db, signature.typ_params(db), new_params, new_ret),
      )
      .into()
    }
    TdTypEnum::TdListTyp(list) => {
      if let Some(element) = list.element(db).and_then(|element| element.resolve(db)) {
        let substituted = generalize_free_variables(db, &element, mapping);
        return TdListTyp::new(db, Some(LazyTyp::eager(substituted))).into();
      }
      typ.clone()
    }
    TdTypEnum::TdDictTyp(dict) => {
      let new_key = dict
        .key(db)
        .and_then(|key| key.resolve(db))
        .map(|key| LazyTyp::eager(generalize_free_variables(db, &key, mapping)));
      let new_value = dict
        .value(db)
        .and_then(|value| value.resolve(db))
        .map(|value| LazyTyp::eager(generalize_free_variables(db, &value, mapping)));
      if new_key.is_some() || new_value.is_some() {
        return TdDictTyp::new(db, new_key, new_value).into();
      }
      typ.clone()
    }
    TdTypEnum::TdProductTyp(product) => {
      let new_fields: BTreeMap<String, LazyTyp<'db>> = product
        .fields(db)
        .into_iter()
        .map(|(name, lazy)| {
          let substituted = lazy
            .resolve(db)
            .map(|resolved| LazyTyp::eager(generalize_free_variables(db, &resolved, mapping)))
            .unwrap_or(lazy);
          (name, substituted)
        })
        .collect();
      TdProductTyp::new(db, None, new_fields).into()
    }
    TdTypEnum::TdSumTyp(sum) => {
      let new_members: Vec<LazyTyp<'db>> = sum
        .members(db)
        .iter()
        .map(|member| {
          member
            .resolve(db)
            .map(|resolved| LazyTyp::eager(generalize_free_variables(db, &resolved, mapping)))
            .unwrap_or_else(|| member.clone())
        })
        .collect();
      get_sum_typ(db, new_members).into()
    }
    // Existential variables should not appear in closure return types
    TdTypEnum::TdExistentialVariableTyp(_) => typ.clone(),
    // Self-contained, no free variables inside
    TdTypEnum::TdExistentialTyp(_) => typ.clone(),
    TdTypEnum::TdMetatypeInstanceTyp(instance) => {
      let inner = instance.inner(db);
      let substituted = generalize_free_variables(db, &inner, mapping);
      if substituted == inner {
        typ.clone()
      } else {
        TdMetatypeInstanceTyp::new(db, substituted).into()
      }
    }
    // Primitives
    _ => typ.clone(),
  }
}

// actual(closure) = fn(params, return from actual(body))
// When expected type provides param types, use those, otherwise infer from body usage

fn get_closure_typ<'db>(db: &'db TypedownDatabase, project_node: ProjectRedNode) -> TypResult<'db> {
  let closure = match ClosureExpr::cast(project_node.node.clone()) {
    Some(closure) => closure,
    None => return TypResult::new(db, None, vec![]),
  };

  let project = project_node.project;
  let file = project_node.file;

  let params: Vec<String> = match closure.params() {
    Some(Either::Left(param_list)) => param_list
      .params()
      .filter_map(|param| param.value())
      .collect(),
    Some(Either::Right(ident)) => ident.value().map(|node| vec![node]).unwrap_or_default(),
    None => vec![],
  };

  let body_expr = match closure.body() {
    Some(body) => body,
    None => return TypResult::new(db, None, vec![]),
  };

  let expected = expected_node_typ(db, project_node.clone()).typ(db);
  let param_typs: Vec<TdTypEnum> = match expected {
    Some(TdTypEnum::TdFuncTyp(func)) => {
      let typs = func.signature(db).params(db);
      if typs.len() != params.len() {
        let (trimmed_offset, trimmed_len) = project_node.node.trimmed_range();
        return TypResult::new(
          db,
          None,
          vec![
            TdDiagnosticKind::WrongArgCount {
              expected: typs.len(),
              got: params.len(),
            }
            .into_error(trimmed_offset, trimmed_offset + trimmed_len),
          ],
        );
      }
      typs
    }
    _ => {
      // No expected type: infer each param type from body usage
      // Params with no inferred type become generic type variables
      params
        .iter()
        .enumerate()
        .map(|(index, param_name)| {
          let param_symbol = Symbol::new(
            db,
            SymbolKind::FuncParam(project, file, project_node.clone(), param_name.clone()),
          );
          match infer_param_typ_from_body(db, project, file, body_expr.syntax(), param_symbol) {
            Some(typ) => {
              // Instantiate callee's bound variable to free variable with its upper bound
              if let TdTypEnum::TdBoundVariableTyp(bound) = &typ {
                let new_bound = TdBoundVariableTyp::new(
                  db,
                  Binder::Closure(project_node.clone()),
                  index,
                  TdBoundVariableTyp::upper_bound(*bound, db),
                  Variance::Covariant,
                );
                TdFreeVariableTyp::new(db, new_bound, project_node.clone()).into()
              } else {
                typ
              }
            }
            None => {
              let new_bound = TdBoundVariableTyp::new(
                db,
                Binder::Closure(project_node.clone()),
                index,
                LazyTyp::eager(get_obj_typ(db).into()),
                Variance::Covariant,
              );
              TdFreeVariableTyp::new(db, new_bound, project_node.clone()).into()
            }
          }
        })
        .collect()
    }
  };

  let body_node = ProjectRedNode::new(project, file, body_expr.syntax().clone());
  let body_result = actual_node_typ(db, body_node);
  let ret = match body_result.typ(db) {
    Some(typ) => typ,
    None => return TypResult::new(db, None, body_result.diagnostics(db).clone()),
  };

  let mut bound_variables: Vec<TdBoundVariableTyp> = vec![];
  let mut free_to_bound: HashMap<TdFreeVariableTyp, TdBoundVariableTyp> = HashMap::new();
  let param_typs: Vec<TdTypEnum> = param_typs
    .into_iter()
    .enumerate()
    .map(|(index, typ)| {
      if let TdTypEnum::TdFreeVariableTyp(free_variable) = &typ {
        let upper_bound =
          TdBoundVariableTyp::upper_bound(TdFreeVariableTyp::bound_origin(*free_variable, db), db);
        let bound_variable = TdBoundVariableTyp::new(
          db,
          Binder::Closure(project_node.clone()),
          index,
          upper_bound,
          Variance::Covariant,
        );
        bound_variables.push(bound_variable);
        free_to_bound.insert(*free_variable, bound_variable);
        bound_variable.into()
      } else {
        typ
      }
    })
    .collect();
  let ret = generalize_free_variables(db, &ret, &free_to_bound);
  let typ_params = if bound_variables.is_empty() {
    None
  } else {
    Some(TypParams::new(db, bound_variables, vec![]))
  };

  let signature = FuncSignature::new(db, typ_params, param_typs, ret);
  let func_typ = get_func_typ(db, signature);
  TypResult::new(
    db,
    Some(func_typ.into()),
    body_result.diagnostics(db).clone(),
  )
}

#[cfg(test)]
mod tests {
  use super::{actual_node_typ, generalize_free_variables};
  use crate::db::derived::get_builtin_typs::{
    get_func_typ, get_lit_typ, get_number_typ, get_obj_typ, get_schema_meta_typ, get_string_typ,
  };
  use crate::db::derived::parse_file::parse_file;
  use crate::db::fixtures::load_vault_fixture;
  use crate::db::types::derived::obj_system::TdStaticTyp;
  use crate::db::types::{
    Binder, File, FileHandle, FileMetadata, LazyTyp, LitValue, Project, ProjectRedNode,
    TdBoundVariableTyp, TdFreeVariableTyp, TdFuncTyp, TdListTyp, TdProductTyp, Variance,
  };
  use crate::db::types::{FuncSignature, TdTypEnum};
  use crate::db::{QueryStorage, TypedownDatabase};
  use crate::syntax::diagnostic::*;
  use std::collections::BTreeMap;
  use std::{collections::HashMap, path::PathBuf};

  fn is_lit_string(db: &TypedownDatabase, typ: &TdTypEnum, expected: &str) -> bool {
    if let TdTypEnum::TdLitTyp(lit) = typ {
      return lit.value(db) == LitValue::String(expected.to_string());
    }
    false
  }

  fn is_lit_number(db: &TypedownDatabase, typ: &TdTypEnum, expected: &str) -> bool {
    if let TdTypEnum::TdLitTyp(lit) = typ {
      return lit.value(db) == LitValue::Number(expected.to_string());
    }
    false
  }

  fn is_lit_bool(db: &TypedownDatabase, typ: &TdTypEnum<'_>, expected: bool) -> bool {
    if let TdTypEnum::TdLitTyp(lit) = typ {
      return lit.value(db) == LitValue::Bool(expected);
    }
    false
  }

  fn vault_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/evaluate_schema/my_vault")
  }

  #[test]
  fn infer_anonymous_mapping_narrows_lit_fields() {
    let (db, project, file) = load_vault_fixture("typecheck/narrow_vault", "anonymous_mapping.td");
    let root_node = parse_file(&db, project, file).ast(&db);
    let result_typ = actual_node_typ(&db, root_node);
    let typ = result_typ.typ(&db).expect("should infer a type");
    let product = typ
      .as_td_product_typ()
      .expect("anonymous mapping should be Product");
    let fields = product.get_fields(&db);

    // String literal narrows to TdLiteralType(Str)
    let name_lazy = fields.get("name").expect("should have name field");
    let name_typ = name_lazy.resolve(&db).expect("should resolve");
    assert!(
      is_lit_string(&db, &name_typ, "Alice"),
      r#"name should be literal str "Alice""#
    );

    // Num literal narrows to TdLiteralType(Num)
    let age_lazy = fields.get("age").expect("should have age field");
    let age_typ = age_lazy.resolve(&db).expect("should resolve");
    assert!(
      is_lit_number(&db, &age_typ, "30"),
      r#"age should be literal num "30""#
    );

    // Bool literal narrows to TdLiteralType(Bool)
    let active_lazy = fields.get("active").expect("should have active field");
    let active_typ = active_lazy.resolve(&db).expect("should resolve");
    assert!(
      is_lit_bool(&db, &active_typ, true),
      "active should be literal bool true"
    );

    // Sequence ["a", 3] narrows to list[sum]
    let tags_lazy = fields.get("tags").expect("should have tags field");
    let tags_typ = tags_lazy.resolve(&db).expect("should resolve");
    let list = tags_typ
      .as_td_list_typ()
      .expect("tags should be a list type");
    let element = list.element(&db).expect("list should have element");
    let element_typ = element.resolve(&db).expect("element should resolve");
    let sum = element_typ
      .as_td_sum_typ()
      .expect("element should be a sum type");
    let members = sum.members(&db);
    assert_eq!(members.len(), 2, "tags should have 2 arms");
    let has_a = members.iter().any(|member| {
      member
        .resolve(&db)
        .is_some_and(|typ| is_lit_string(&db, &typ, "a"))
    });
    let has_3 = members.iter().any(|member| {
      member
        .resolve(&db)
        .is_some_and(|typ| is_lit_number(&db, &typ, "3"))
    });
    assert!(has_a, "arms should contain literal str 'a'");
    assert!(has_3, "arms should contain literal num '3'");
  }

  #[test]
  fn actual_node_typ_of_schema_file_top_level_mapping_is_schema_typ() {
    let vault = vault_root();
    let schema_file_path = vault.join("_types/Person.td");

    let db = TypedownDatabase {
      storage: QueryStorage::default(),
    };

    let file = File::new(
      &db,
      FileHandle::Path(schema_file_path.clone(), FileMetadata::default()),
    );
    let files = HashMap::from([(schema_file_path, file)]);
    let project = Project::new(&db, vault, files, None);

    let root_node = parse_file(&db, project, file).ast(&db);
    let result = actual_node_typ(&db, root_node);

    let typ = result.typ(&db);
    let expected_typ = Some(TdTypEnum::from(get_schema_meta_typ(&db)));
    assert!(
      typ == expected_typ,
      "top-level mapping of a schema file should have schema type"
    );
    assert!(
      result.diagnostics(&db).is_empty(),
      "expected no diagnostics, got: {:?}",
      result.diagnostics(&db)
    );
  }

  #[test]
  fn actual_node_typ_string_lit_returns_lit() {
    let (db, project, file) = load_vault_fixture("typecheck/my_vault", "valid_person.td");
    let root_node = parse_file(&db, project, file).ast(&db);
    let entries = root_node.mapping_entries().expect("should have entries");
    let name_node = entries
      .into_iter()
      .find(|(key, _)| key == "name")
      .map(|(_, value)| value)
      .expect("should have name field");
    let result_typ = actual_node_typ(&db, name_node);
    let typ = result_typ.typ(&db).expect("should have a type");
    assert!(
      is_lit_string(&db, &typ, "Alice"),
      "string value should be literal str"
    );
  }

  #[test]
  fn actual_node_typ_bool_returns_lit() {
    let (db, project, file) = load_vault_fixture("typecheck/narrow_vault", "anonymous_mapping.td");
    let root_node = parse_file(&db, project, file).ast(&db);
    let entries = root_node.mapping_entries().expect("should have entries");
    let active_node = entries
      .into_iter()
      .find(|(key, _)| key == "active")
      .map(|(_, value)| value)
      .expect("should have active field");
    let result_typ = actual_node_typ(&db, active_node);
    let typ = result_typ.typ(&db).expect("should have a type");
    assert!(
      is_lit_bool(&db, &typ, true),
      "bool value should be literal bool"
    );
  }

  #[test]
  fn actual_node_typ_sequence_returns_list_typ() {
    let (db, project, file) = load_vault_fixture("typecheck/narrow_vault", "anonymous_mapping.td");
    let root_node = parse_file(&db, project, file).ast(&db);
    let entries = root_node.mapping_entries().expect("should have entries");
    let tags_node = entries
      .into_iter()
      .find(|(key, _)| key == "tags")
      .map(|(_, value)| value)
      .expect("should have tags field");
    let result_typ = actual_node_typ(&db, tags_node);
    let typ = result_typ.typ(&db).expect("should have a type");
    assert!(typ.is_td_list_typ(), "sequence should be a list type");
  }

  // Date strings narrow to date type, not Literal
  #[test]
  fn actual_node_typ_date_string_returns_simple_date() {
    let (db, project, file) = load_vault_fixture("typecheck/my_vault", "valid_event.td");
    let root_node = parse_file(&db, project, file).ast(&db);
    let entries = root_node.mapping_entries().expect("should have entries");
    let date_node = entries
      .into_iter()
      .find(|(key, _)| key == "date")
      .map(|(_, value)| value)
      .expect("should have date field");
    let result_typ = actual_node_typ(&db, date_node);
    let typ = result_typ.typ(&db).expect("should have a type");
    assert_eq!(
      typ.display_name(&db),
      "date",
      "ISO date string should resolve to date"
    );
  }

  // Fref returns the resource's schema type, not typ_typ
  #[test]
  fn actual_node_typ_fref_returns_resource_typ() {
    let (db, project, file) =
      load_vault_fixture("typecheck/narrow_vault", "article_fref_status.td");
    let root_node = parse_file(&db, project, file).ast(&db);
    let entries = root_node.mapping_entries().expect("should have entries");
    // status: fref("summary.td").status
    let status_node = entries
      .into_iter()
      .find(|(key, _)| key == "status")
      .map(|(_, value)| value)
      .expect("should have status field");
    let result_typ = actual_node_typ(&db, status_node);
    if let Some(typ) = result_typ.typ(&db) {
      assert_ne!(
        typ.display_name(&db),
        "type",
        "fref field access should not return typ_typ"
      );
    }
  }

  // Num literal returns Literal(Num)
  #[test]
  fn actual_node_typ_number_returns_lit() {
    let (db, project, file) = load_vault_fixture("typecheck/my_vault", "valid_person.td");
    let root_node = parse_file(&db, project, file).ast(&db);
    let entries = root_node.mapping_entries().expect("should have entries");
    let age_node = entries
      .into_iter()
      .find(|(key, _)| key == "age")
      .map(|(_, value)| value)
      .expect("should have age field");
    let result_typ = actual_node_typ(&db, age_node);
    let typ = result_typ.typ(&db).expect("should have a type");
    assert!(
      is_lit_number(&db, &typ, "30"),
      "number value should be literal num"
    );
  }

  #[test]
  fn actual_node_typ_to_string_field_access_returns_func_typ() {
    let (db, _, _) = load_vault_fixture("typecheck/my_vault", "valid_person.td");
    let number_lit: TdTypEnum<'_> = get_lit_typ(&db, LitValue::Number("42".to_string())).into();

    let field_typ = number_lit
      .lookup_field_typ(&db, "to_string")
      .expect("should have to_string method");
    let expected_signature = FuncSignature::new(&db, None, vec![], get_string_typ(&db).into());
    let expected_func_typ: TdTypEnum = get_func_typ(&db, expected_signature).into();

    assert_eq!(field_typ, expected_func_typ);
  }

  // fref("./peer.td") in a file inside subdir/ resolves relative to that file, not vault root
  #[test]
  fn actual_node_typ_relative_fref_resolves_from_file_directory() {
    let (db, project, file) =
      load_vault_fixture("typecheck/narrow_vault", "subdir/with_relative_fref.td");
    let root_node = parse_file(&db, project, file).ast(&db);
    let entries = root_node.mapping_entries().expect("should have entries");
    let related_node = entries
      .into_iter()
      .find(|(key, _)| key == "related")
      .map(|(_, value)| value)
      .expect("should have 'related' field");
    let result = actual_node_typ(&db, related_node);
    let has_unresolved = result.diagnostics(&db).iter().any(|diagnostic| {
      matches!(
        diagnostic,
        TdDiagnostic {
          kind: TdDiagnosticKind::UnresolvedFileRef { .. },
          ..
        }
      )
    });
    assert!(
      !has_unresolved,
      r#"fref("./peer.td") should resolve relative to subdir/, not vault root: {:?}"#,
      result.diagnostics(&db)
    );
    assert!(
      result.typ(&db).is_some(),
      r#"fref("./peer.td") should have a resolved type"#
    );
  }

  #[test]
  fn actual_node_typ_method_call_to_string_returns_string_typ() {
    let (db, project, file) = load_vault_fixture("typecheck/my_vault", "method_call.td");
    let root_node = parse_file(&db, project, file).ast(&db);
    let entries = root_node.mapping_entries().expect("should have entries");
    let result_node = entries
      .into_iter()
      .find(|(key, _)| key == "result")
      .map(|(_, value)| value)
      .expect("should have result field");
    let result = actual_node_typ(&db, result_node);
    let typ = result.typ(&db).expect("should have a type");
    let string_typ: TdTypEnum = get_string_typ(&db).into();
    assert_eq!(typ, string_typ);
  }

  // When a closure has no expected type context, param types are inferred from body usage
  #[test]
  fn actual_node_typ_closure_infers_param_typ_from_body() {
    let (db, project, file) =
      load_vault_fixture("typecheck/my_vault", "closure_infer_from_binary_op.td");
    let root_node = parse_file(&db, project, file).ast(&db);
    let entries = root_node.mapping_entries().expect("should have entries");
    let result_node = entries
      .into_iter()
      .find(|(key, _)| key == "result")
      .map(|(_, value)| value)
      .expect("should have result field");
    let result = actual_node_typ(&db, result_node);
    // The call should type-check: closure infers x: number from x + 1, returns number
    let typ = result.typ(&db).expect("should have a type");
    let number_typ: TdTypEnum = get_number_typ(&db).into();
    assert_eq!(
      typ.display_name(&db),
      number_typ.display_name(&db),
      "((x) -> x + 1)(5) should return number"
    );
    assert!(
      result.diagnostics(&db).is_empty(),
      "should have no diagnostics: {:?}",
      result.diagnostics(&db)
    );
  }

  #[test]
  fn actual_node_typ_generic_identity_closure_instantiates_to_arg_typ() {
    let (db, project, file) =
      load_vault_fixture("typecheck/my_vault", "closure_identity_generic.td");
    let root_node = parse_file(&db, project, file).ast(&db);
    let entries = root_node.mapping_entries().expect("should have entries");
    let result_node = entries
      .into_iter()
      .find(|(key, _)| key == "result")
      .map(|(_, value)| value)
      .expect("should have result field");
    let result = actual_node_typ(&db, result_node);
    // ((x) -> x)("hello") infers T = literal "hello", so return type is the literal type
    let typ = result.typ(&db).expect("should have a type");
    assert_eq!(
      typ.display_name(&db),
      r#""hello""#,
      "expected literal hello type"
    );
    assert!(
      result.diagnostics(&db).is_empty(),
      "should have no diagnostics: {:?}",
      result.diagnostics(&db)
    );
  }

  // generalize_free_variables tests

  fn create_test_mapping<'db>(
    db: &'db TypedownDatabase,
    site: ProjectRedNode,
    count: usize,
  ) -> (
    Vec<TdFreeVariableTyp<'db>>,
    Vec<TdBoundVariableTyp<'db>>,
    HashMap<TdFreeVariableTyp<'db>, TdBoundVariableTyp<'db>>,
  ) {
    let mut free_variables = vec![];
    let mut bound_variables = vec![];
    let mut mapping = HashMap::new();
    for index in 0..count {
      let bound = TdBoundVariableTyp::new(
        db,
        Binder::Closure(site.clone()),
        index,
        LazyTyp::eager(get_obj_typ(db).into()),
        Variance::Covariant,
      );
      let free = TdFreeVariableTyp::new(db, bound, site.clone());
      mapping.insert(free, bound);
      free_variables.push(free);
      bound_variables.push(bound);
    }
    (free_variables, bound_variables, mapping)
  }

  #[test]
  fn check_generalize_product_with_free_variable() {
    let (db, project, file) = load_vault_fixture("evaluate/my_vault", "valid_person.td");
    let site = parse_file(&db, project, file).ast(&db);
    let (free_variables, bound_variables, mapping) = create_test_mapping(&db, site, 1);

    // { name: FreeVariable(0) } -> { name: BoundVariable(0) }
    let product: TdTypEnum = TdProductTyp::new(
      &db,
      None,
      BTreeMap::from([("name".to_string(), LazyTyp::eager(free_variables[0].into()))]),
    )
    .into();

    let result = generalize_free_variables(&db, &product, &mapping);
    if let TdTypEnum::TdProductTyp(result_product) = result {
      let name_typ = result_product
        .fields(&db)
        .get("name")
        .unwrap()
        .resolve(&db)
        .unwrap();
      assert_eq!(name_typ, bound_variables[0].into());
    } else {
      panic!("expected product type");
    }
  }

  #[test]
  fn check_generalize_list_with_free_variable() {
    let (db, project, file) = load_vault_fixture("evaluate/my_vault", "valid_person.td");
    let site = parse_file(&db, project, file).ast(&db);
    let (free_variables, bound_variables, mapping) = create_test_mapping(&db, site, 1);

    // list[FreeVariable(0)] -> list[BoundVariable(0)]
    let list: TdTypEnum =
      TdListTyp::new(&db, Some(LazyTyp::eager(free_variables[0].into()))).into();

    let result = generalize_free_variables(&db, &list, &mapping);
    if let TdTypEnum::TdListTyp(result_list) = result {
      let element = result_list.get_typ_args(&db);
      assert_eq!(element[0], bound_variables[0].into());
    } else {
      panic!("expected list type");
    }
  }

  #[test]
  fn check_generalize_function_with_free_variable() {
    let (db, project, file) = load_vault_fixture("evaluate/my_vault", "valid_person.td");
    let site = parse_file(&db, project, file).ast(&db);
    let (free_variables, bound_variables, mapping) = create_test_mapping(&db, site, 1);

    // (FreeVariable(0)) -> FreeVariable(0) should become (BoundVariable(0)) -> BoundVariable(0)
    let func: TdTypEnum = TdFuncTyp::get(
      &db,
      vec![free_variables[0].into()],
      free_variables[0].into(),
    )
    .into();

    let result = generalize_free_variables(&db, &func, &mapping);
    if let TdTypEnum::TdFuncTyp(result_func) = result {
      let signature = result_func.signature(&db);
      assert_eq!(signature.params(&db)[0], bound_variables[0].into());
      assert_eq!(signature.ret(&db), bound_variables[0].into());
    } else {
      panic!("expected function type");
    }
  }

  #[test]
  fn check_generalize_skips_unmapped_free_variable() {
    let (db, project, file) = load_vault_fixture("evaluate/my_vault", "valid_person.td");
    let site = parse_file(&db, project, file).ast(&db);
    let (free_variables, bound_variables, mapping) = create_test_mapping(&db, site.clone(), 1);

    // A free variable from a different site, not in the mapping
    let other_site = {
      let files: Vec<_> = project.files(&db).values().copied().collect();
      parse_file(&db, project, files[1]).ast(&db)
    };
    let other_bound = TdBoundVariableTyp::new(
      &db,
      Binder::Closure(other_site.clone()),
      0,
      LazyTyp::eager(get_obj_typ(&db).into()),
      Variance::Covariant,
    );
    let other_free = TdFreeVariableTyp::new(&db, other_bound, other_site);

    // { mapped: FreeVariable(0), unmapped: other_free }
    let product: TdTypEnum = TdProductTyp::new(
      &db,
      None,
      BTreeMap::from([
        (
          "mapped".to_string(),
          LazyTyp::eager(free_variables[0].into()),
        ),
        ("unmapped".to_string(), LazyTyp::eager(other_free.into())),
      ]),
    )
    .into();

    let result = generalize_free_variables(&db, &product, &mapping);
    if let TdTypEnum::TdProductTyp(result_product) = result {
      let fields = result_product.fields(&db);
      let mapped = fields.get("mapped").unwrap().resolve(&db).unwrap();
      let unmapped = fields.get("unmapped").unwrap().resolve(&db).unwrap();
      assert_eq!(mapped, bound_variables[0].into());
      // Unmapped free variable passes through unchanged
      assert_eq!(unmapped, other_free.into());
    } else {
      panic!("expected product type");
    }
  }

  #[test]
  fn check_generalize_mixed_concrete_and_free() {
    let (db, project, file) = load_vault_fixture("evaluate/my_vault", "valid_person.td");
    let site = parse_file(&db, project, file).ast(&db);
    let (free_variables, bound_variables, mapping) = create_test_mapping(&db, site, 1);
    let number_typ: TdTypEnum = get_number_typ(&db).into();

    // { name: FreeVariable(0), age: number } -> { name: BoundVariable(0), age: number }
    let product: TdTypEnum = TdProductTyp::new(
      &db,
      None,
      BTreeMap::from([
        ("name".to_string(), LazyTyp::eager(free_variables[0].into())),
        ("age".to_string(), LazyTyp::eager(number_typ.clone())),
      ]),
    )
    .into();

    let result = generalize_free_variables(&db, &product, &mapping);
    if let TdTypEnum::TdProductTyp(result_product) = result {
      let fields = result_product.fields(&db);
      let name = fields.get("name").unwrap().resolve(&db).unwrap();
      let age = fields.get("age").unwrap().resolve(&db).unwrap();
      assert_eq!(name, bound_variables[0].into());
      assert_eq!(age, number_typ);
    } else {
      panic!("expected product type");
    }
  }

  /* vault.from_nodes type inference */

  fn get_result_typ<'a>(
    db: &'a TypedownDatabase,
    project: crate::db::types::Project,
    file: File,
  ) -> crate::db::types::TypResult<'a> {
    let root_node = parse_file(db, project, file).ast(db);
    let entries = root_node.mapping_entries().expect("should have entries");
    let result_node = entries
      .into_iter()
      .find(|(key, _)| key == "result")
      .map(|(_, value)| value)
      .expect("should have result field");
    actual_node_typ(db, result_node)
  }

  #[test]
  fn vault_from_nodes_valid_schema_returns_node_set() {
    let (db, project, file) = load_vault_fixture("typecheck/my_vault", "vault_from_nodes_valid.td");
    let result = get_result_typ(&db, project, file);
    let typ = result
      .typ(&db)
      .expect("from_nodes(Person) should have a return type");
    assert!(
      typ
        .as_td_schema_typ()
        .is_some_and(|schema| schema.name(&db).starts_with("node_set")),
      "from_nodes(Person) should return node_set, got {}",
      typ.display_name(&db)
    );
    assert!(
      result.diagnostics(&db).is_empty(),
      "from_nodes(Person) should have no diagnostics: {:?}",
      result.diagnostics(&db)
    );
  }

  #[test]
  fn vault_from_nodes_wrong_type_has_diagnostic() {
    let (db, project, file) =
      load_vault_fixture("typecheck/my_vault", "vault_from_nodes_wrong_type.td");
    let result = get_result_typ(&db, project, file);
    assert!(
      !result.diagnostics(&db).is_empty(),
      "vault.from_nodes(\"hello\") should have diagnostics"
    );
  }

  #[test]
  fn vault_from_nodes_to_list_returns_parameterized_list() {
    let (db, project, file) =
      load_vault_fixture("typecheck/my_vault", "vault_from_nodes_to_list.td");
    let result = get_result_typ(&db, project, file);
    let typ = result.typ(&db).expect("should have a type");
    let list = typ.as_td_list_typ().unwrap_or_else(|| {
      panic!(
        "to_list() should return list, got {}",
        typ.display_name(&db)
      )
    });
    let element = list
      .element(&db)
      .expect("to_list() should return parameterized list")
      .resolve(&db)
      .expect("element type should resolve");
    // Element type should be metatype_instance<lit<Person>> which is equivalent to Person
    // Not obj (which would mean unparameterized)
    assert!(
      !element.as_td_obj_typ().is_some(),
      "to_list() element should not be obj (should be parameterized), got {}",
      element.display_name(&db)
    );
  }

  #[test]
  fn vault_from_nodes_count_returns_number() {
    let (db, project, file) = load_vault_fixture("typecheck/my_vault", "vault_from_nodes_count.td");
    let result = get_result_typ(&db, project, file);
    let typ = result.typ(&db).expect("should have a type");
    assert!(
      typ.as_td_number_typ().is_some(),
      "vault.from_nodes(Person).count() should return number, got {}",
      typ.display_name(&db)
    );
  }

  #[test]
  fn vault_from_nodes_first_returns_person_or_null() {
    let (db, project, file) = load_vault_fixture("typecheck/my_vault", "vault_from_nodes_first.td");
    let result = get_result_typ(&db, project, file);
    let typ = result.typ(&db).expect("should have a type");
    let sum = typ.as_td_sum_typ().unwrap_or_else(|| {
      panic!(
        "first() should return nullable, got {}",
        typ.display_name(&db)
      )
    });
    // Sum should contain a non-obj type (Person via metatype_instance) and null
    let members: Vec<_> = sum
      .members(&db)
      .iter()
      .filter_map(|member| member.resolve(&db))
      .collect();
    assert!(
      members
        .iter()
        .any(|member| member.as_td_null_typ().is_some()),
      "first() should include null, got {}",
      typ.display_name(&db)
    );
    assert!(
      members
        .iter()
        .any(|member| !member.as_td_null_typ().is_some() && !member.as_td_obj_typ().is_some()),
      "first() should include a parameterized element (not obj), got {}",
      typ.display_name(&db)
    );
  }

  #[test]
  fn vault_from_nodes_filter_returns_node_set() {
    let (db, project, file) =
      load_vault_fixture("typecheck/my_vault", "vault_from_nodes_filter.td");
    let result = get_result_typ(&db, project, file);
    let typ = result.typ(&db).expect("should have a type");
    assert!(
      typ
        .as_td_schema_typ()
        .is_some_and(|s| s.name(&db).starts_with("node_set")),
      "vault.from_nodes(Person).filter(...) should return node_set, got {}",
      typ.display_name(&db)
    );
  }

  #[test]
  fn vault_from_nodes_chain_filter_count_returns_number() {
    let (db, project, file) = load_vault_fixture("typecheck/my_vault", "vault_from_nodes_chain.td");
    let result = get_result_typ(&db, project, file);
    let typ = result.typ(&db).expect("should have a type");
    assert!(
      typ.as_td_number_typ().is_some(),
      "vault.from_nodes(Person).filter(...).count() should return number, got {}",
      typ.display_name(&db)
    );
  }

  #[test]
  fn vault_from_nodes_follow_returns_node_set() {
    let (db, project, file) =
      load_vault_fixture("typecheck/my_vault", "vault_from_nodes_follow.td");
    let result = get_result_typ(&db, project, file);
    let typ = result.typ(&db).expect("should have a type");
    assert!(
      typ
        .as_td_schema_typ()
        .is_some_and(|s| s.name(&db).starts_with("node_set")),
      "follow() should return node_set, got {}",
      typ.display_name(&db)
    );
  }

  #[test]
  fn vault_from_nodes_filter_first_preserves_parameterization() {
    let (db, project, file) =
      load_vault_fixture("typecheck/my_vault", "vault_from_nodes_filter_first.td");
    let result = get_result_typ(&db, project, file);
    let typ = result.typ(&db).expect("should have a type");
    let sum = typ.as_td_sum_typ().unwrap_or_else(|| {
      panic!(
        "filter(...).first() should return nullable, got {}",
        typ.display_name(&db)
      )
    });
    let members: Vec<_> = sum
      .members(&db)
      .iter()
      .filter_map(|member| member.resolve(&db))
      .collect();
    assert!(
      members
        .iter()
        .any(|member| member.as_td_null_typ().is_some()),
      "filter(...).first() should include null, got {}",
      typ.display_name(&db)
    );
    assert!(
      members
        .iter()
        .any(|member| !member.as_td_null_typ().is_some() && !member.as_td_obj_typ().is_some()),
      "filter(...).first() should include parameterized element (not obj), got {}",
      typ.display_name(&db)
    );
  }

  #[test]
  fn vault_from_nodes_filter_to_list_preserves_parameterization() {
    let (db, project, file) =
      load_vault_fixture("typecheck/my_vault", "vault_from_nodes_filter_to_list.td");
    let result = get_result_typ(&db, project, file);
    let typ = result.typ(&db).expect("should have a type");
    let list = typ.as_td_list_typ().unwrap_or_else(|| {
      panic!(
        "filter(...).to_list() should return list, got {}",
        typ.display_name(&db)
      )
    });
    assert!(
      list.element(&db).is_some(),
      "filter(...).to_list() should return parameterized list, got {}",
      typ.display_name(&db)
    );
  }

  #[test]
  fn vault_from_nodes_first_field_access_on_nullable_has_diagnostic() {
    let (db, project, file) =
      load_vault_fixture("typecheck/my_vault", "vault_from_nodes_first_field.td");
    let result = get_result_typ(&db, project, file);
    // first() returns Person | null, accessing .name on nullable produces UnknownField
    assert!(
      result.diagnostics(&db).iter().any(|diagnostic| matches!(
        &diagnostic.kind,
        TdDiagnosticKind::UnknownField { field, on_typ }
          if field == "name" && on_typ.contains("Person") && on_typ.contains("null")
      )),
      "first().name on nullable should produce UnknownField with Person | null, got {:?}",
      result.diagnostics(&db)
    );
  }

  #[test]
  fn vault_from_nodes_to_list_index_field_access() {
    let (db, project, file) = load_vault_fixture(
      "typecheck/my_vault",
      "vault_from_nodes_to_list_index_field.td",
    );
    let result = get_result_typ(&db, project, file);
    let typ = result.typ(&db).unwrap_or_else(|| {
      panic!(
        "to_list()[0].name should resolve. diagnostics: {:?}",
        result.diagnostics(&db)
      )
    });
    // to_list()[0].name should be string
    assert!(
      typ.as_td_string_typ().is_some(),
      "to_list()[0].name should be string, got {}",
      typ.display_name(&db)
    );
  }

  #[test]
  fn vault_from_nodes_builtin_type_has_diagnostic() {
    let (db, project, file) =
      load_vault_fixture("typecheck/my_vault", "vault_from_nodes_number.td");
    let result = get_result_typ(&db, project, file);
    assert!(
      !result.diagnostics(&db).is_empty(),
      "vault.from_nodes(number) should have diagnostics"
    );
  }

  /* vault.from_node type inference */

  #[test]
  fn vault_from_node_returns_parameterized_node_set() {
    let (db, project, file) = load_vault_fixture("typecheck/my_vault", "vault_from_node_valid.td");
    let result = get_result_typ(&db, project, file);
    let typ = result
      .typ(&db)
      .expect("from_node(self) should have a return type");
    assert!(
      typ
        .as_td_schema_typ()
        .is_some_and(|schema| schema.name(&db).starts_with("node_set")),
      "from_node(self) should return node_set, got {}",
      typ.display_name(&db)
    );
    assert!(
      result.diagnostics(&db).is_empty(),
      "from_node(self) should have no diagnostics: {:?}",
      result.diagnostics(&db)
    );
  }

  #[test]
  fn vault_from_node_first_returns_person_or_null() {
    let (db, project, file) = load_vault_fixture("typecheck/my_vault", "vault_from_node_first.td");
    let result = get_result_typ(&db, project, file);
    let typ = result.typ(&db).expect("should have a type");
    let sum = typ.as_td_sum_typ().unwrap_or_else(|| {
      panic!(
        "from_node(self).first() should return nullable, got {}",
        typ.display_name(&db)
      )
    });
    let members: Vec<_> = sum
      .members(&db)
      .iter()
      .filter_map(|member| member.resolve(&db))
      .collect();
    assert!(
      members
        .iter()
        .any(|member| member.as_td_null_typ().is_some()),
      "from_node(self).first() should include null, got {}",
      typ.display_name(&db)
    );
    assert!(
      members
        .iter()
        .any(|member| !member.as_td_null_typ().is_some() && !member.as_td_obj_typ().is_some()),
      "from_node(self).first() should include Person (not obj), got {}",
      typ.display_name(&db)
    );
  }

  #[test]
  fn vault_from_node_to_list_index_field_access() {
    let (db, project, file) = load_vault_fixture(
      "typecheck/my_vault",
      "vault_from_node_to_list_index_field.td",
    );
    let result = get_result_typ(&db, project, file);
    let typ = result.typ(&db).unwrap_or_else(|| {
      panic!(
        "from_node(self).to_list()[0].name should resolve. diagnostics: {:?}",
        result.diagnostics(&db)
      )
    });
    assert!(
      typ.as_td_string_typ().is_some(),
      "from_node(self).to_list()[0].name should be string, got {}",
      typ.display_name(&db)
    );
  }

  // Closure inference with vault queries

  #[test]
  fn check_closure_from_node_infers_field_type() {
    // ((x) -> vault.from_node(x))(self).first().name
    // x inferred as metatype_instance<schema_meta> from from_node's param bound
    // first() returns Person | null, .name on nullable produces UnknownField
    let (db, project, file) =
      load_vault_fixture("typecheck/my_vault", "vault_closure_from_node.td");
    let result = get_result_typ(&db, project, file);
    let diagnostics = result.diagnostics(&db);
    // No bound violation (free variable inherits callee's bound)
    assert!(
      !diagnostics.iter().any(|diagnostic| matches!(
        &diagnostic.kind,
        TdDiagnosticKind::TypArgBoundViolation { .. }
      )),
      "should not have bound violation, got {:?}",
      diagnostics
    );
    // .name on nullable Person | null produces UnknownField mentioning both Person and null
    assert!(
      diagnostics.iter().any(|diagnostic| matches!(
        &diagnostic.kind,
        TdDiagnosticKind::UnknownField { field, on_typ }
          if field == "name" && on_typ.contains("null")
      )),
      "first().name on nullable should warn about null, got {:?}",
      diagnostics
    );
    // .name on nullable returns None (can't resolve field on union with null)
    assert!(
      result.typ(&db).is_none(),
      "field access on nullable should not resolve"
    );
  }

  #[test]
  fn check_closure_from_nodes_has_unresolved_field() {
    // ((x) -> vault.from_nodes(x).first())(Person).name
    // Type doesn't fully resolve through closure, metatype_instance stays unresolved
    let (db, project, file) =
      load_vault_fixture("typecheck/my_vault", "vault_closure_from_nodes.td");
    let result = get_result_typ(&db, project, file);
    let diagnostics = result.diagnostics(&db);
    // No bound violation
    assert!(
      !diagnostics.iter().any(|diagnostic| matches!(
        &diagnostic.kind,
        TdDiagnosticKind::TypArgBoundViolation { .. }
      )),
      "should not have bound violation, got {:?}",
      diagnostics
    );
    // .name on unresolved metatype_instance produces UnknownField
    assert!(
      diagnostics
        .iter()
        .any(|diagnostic| matches!(&diagnostic.kind, TdDiagnosticKind::UnknownField { .. })),
      "expected unresolved field from closure inference, got {:?}",
      result.diagnostics(&db)
    );
  }

  /* follow type inference */

  #[test]
  fn vault_follow_owner_first_returns_person_or_null() {
    let (db, project, file) = load_vault_fixture("typecheck/my_vault", "vault_follow_owner.td");
    let result = get_result_typ(&db, project, file);
    let typ = result.typ(&db).unwrap_or_else(|| {
      panic!(
        "follow(\"owner\").first() should typecheck. diagnostics: {:?}",
        result.diagnostics(&db)
      )
    });
    let sum = typ.as_td_sum_typ().unwrap_or_else(|| {
      panic!(
        "follow(\"owner\").first() should return nullable, got {}",
        typ.display_name(&db)
      )
    });
    let members: Vec<_> = sum
      .members(&db)
      .iter()
      .filter_map(|member| member.resolve(&db))
      .collect();
    assert!(
      members
        .iter()
        .any(|member| member.as_td_null_typ().is_some()),
      "should include null, got {}",
      typ.display_name(&db)
    );
    // Blocked: BoundVariable(0) scope collision causes Unwrap<Projection<...>> to not resolve
    // When fixed, follow("owner") on WithBareRef should return Person via Unwrap<Person | null>
    assert!(
      members.iter().any(|member| member
        .as_td_schema_typ()
        .is_some_and(|schema| schema.name(&db) == "Person")),
      "should include Person, got {}",
      typ.display_name(&db)
    );
  }

  /* follow_back type inference */

  #[test]
  fn vault_follow_back_first_returns_with_bare_ref_or_null() {
    let (db, project, file) =
      load_vault_fixture("typecheck/my_vault", "vault_follow_back_owner.td");
    let result = get_result_typ(&db, project, file);
    let typ = result.typ(&db).unwrap_or_else(|| {
      panic!(
        "follow_back().first() should typecheck. diagnostics: {:?}",
        result.diagnostics(&db)
      )
    });
    let sum = typ.as_td_sum_typ().unwrap_or_else(|| {
      panic!(
        "follow_back().first() should return nullable, got {}",
        typ.display_name(&db)
      )
    });
    let members: Vec<_> = sum
      .members(&db)
      .iter()
      .filter_map(|member| member.resolve(&db))
      .collect();
    assert!(
      members
        .iter()
        .any(|member| member.as_td_null_typ().is_some()),
      "should include null, got {}",
      typ.display_name(&db)
    );
    // Blocked: metatype_instance<schema> doesn't resolve to WithBareRef
    // When fixed, follow_back(WithBareRef, "owner") should return WithBareRef
    assert!(
      members.iter().any(|member| member
        .as_td_schema_typ()
        .is_some_and(|schema| schema.name(&db) == "WithBareRef")),
      "should include WithBareRef, got {}",
      typ.display_name(&db)
    );
  }

  /* Bound variable scope collision bugs */

  #[test]
  fn vault_follow_owner_field_access_resolves() {
    // follow("owner") on WithBareRef gives node_set<Person>
    let (db, project, file) =
      load_vault_fixture("typecheck/my_vault", "vault_follow_owner_field.td");
    let result = get_result_typ(&db, project, file);
    let typ = result
      .typ(&db)
      .unwrap_or_else(|| panic!("should resolve. diagnostics: {:?}", result.diagnostics(&db)));
    assert!(
      typ.as_td_string_typ().is_some(),
      "should be string, got {}",
      typ.display_name(&db)
    );
  }

  #[test]
  fn vault_follow_back_owner_field_access_resolves() {
    // follow_back(WithBareRef, "owner") gives node_set<WithBareRef>
    // to_list()[0].name resolves to string
    let (db, project, file) =
      load_vault_fixture("typecheck/my_vault", "vault_follow_back_owner_field.td");
    let result = get_result_typ(&db, project, file);
    let typ = result
      .typ(&db)
      .unwrap_or_else(|| panic!("should resolve. diagnostics: {:?}", result.diagnostics(&db)));
    assert!(
      typ.as_td_string_typ().is_some(),
      "should be string, got {}",
      typ.display_name(&db)
    );
  }

  #[test]
  fn nested_closure_bound_variable_scope_collision() {
    // ((x) -> (y) -> x)(42)("hello") should return number
    let (db, project, file) =
      load_vault_fixture("typecheck/my_vault", "nested_closure_identity.td");
    let result = get_result_typ(&db, project, file);
    let typ = result
      .typ(&db)
      .unwrap_or_else(|| panic!("should resolve. diagnostics: {:?}", result.diagnostics(&db)));
    // the outer closure captures x, so calling with 42 produces literal type 42
    assert_eq!(typ.display_name(&db), "42");
  }
}
