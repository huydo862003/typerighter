//! Tracked query to get the actual (bottom-up) type of an AST node
// This is the idea of bidirectional typechecking

use std::collections::BTreeMap;

use crate::db::TypedownDatabase;
use crate::db::derived::evaluate::evaluate_typ::evaluate_typ;
use crate::db::derived::get_builtin_typs::{
  get_bool_typ, get_date_typ, get_datetime_typ, get_func_typ, get_list_typ, get_lit_typ,
  get_math_typ, get_null_typ, get_number_typ, get_string_typ, get_sum_typ, get_time_typ,
};
use crate::db::derived::get_vault_config::get_vault_config;
use crate::db::derived::name_resolver::file_symbol::file_symbol;
use crate::db::derived::name_resolver::referee::referee;
use crate::db::derived::typechecker::expected_node_typ::expected_node_typ;
use crate::db::derived::typechecker::get_symbol_typ::get_symbol_typ;
use crate::db::types::derived::obj_system::{
  TdProductTyp, TdStaticTyp, is_valid_iso_date, is_valid_iso_datetime, is_valid_iso_time,
};
use crate::db::types::{
  BuiltinMacroKind, FuncSignature, LazyTyp, LitValue, ProjectRedNode, SymbolKind, TdTypEnum,
  TypResult,
};
use crate::db::utils::resolve_fref_path;
use crate::syntax::ast::{
  AstNode, CallExpr, ClosureExpr, DictLit, Expr, IdentLit, IndexExpr, MathLit, NumberLit,
  PostfixExpr, SourceFile, StrLit, YamlFrontmatter, YamlMapping, YamlOpKind,
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

  if let Some(string_lit) = StrLit::cast(node.clone()) {
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
          Either::Left(s) => Some(s),
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
      Expr::YamlSequence(seq) => get_sequence_typ(
        db,
        seq
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
      Expr::NumberLit(_) | Expr::StrLit(_) | Expr::MathLit(_) | Expr::IdentLit(_) => {
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
  project: crate::db::types::Project,
  file: crate::db::types::File,
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
  let diagnostics = callee_result.diagnostics(db).clone();
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

  if let Some(signature) = callee_typ.call_typ(db, arg_typs) {
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
  let path_string = match StrLit::cast(red_node.clone()) {
    Some(string_lit) if !string_lit.is_interpolated() => string_lit
      .fragments()
      .filter_map(|fragment| match fragment {
        Either::Left(s) => Some(s),
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

  let target_file = match files.get(&target_path) {
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
  project: crate::db::types::Project,
  file: crate::db::types::File,
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

// actual(closure) = fn(params from expected, return from actual(body))
fn get_closure_typ<'db>(db: &'db TypedownDatabase, project_node: ProjectRedNode) -> TypResult<'db> {
  let closure = match ClosureExpr::cast(project_node.node.clone()) {
    Some(c) => c,
    None => return TypResult::new(db, None, vec![]),
  };

  let project = project_node.project;
  let file = project_node.file;

  let expected = expected_node_typ(db, project_node.clone()).typ(db);
  let param_typs = match expected {
    Some(TdTypEnum::TdFuncTyp(func)) => func.signature(db).params(db),
    _ => return TypResult::new(db, None, vec![]),
  };

  let params: Vec<String> = match closure.params() {
    Some(Either::Left(param_list)) => param_list
      .params()
      .filter_map(|param| param.value())
      .collect(),
    Some(Either::Right(ident)) => ident.value().map(|node| vec![node]).unwrap_or_default(),
    None => vec![],
  };

  if param_typs.len() != params.len() {
    let (trimmed_offset, trimmed_len) = project_node.node.trimmed_range();
    return TypResult::new(
      db,
      None,
      vec![
        TdDiagnosticKind::WrongArgCount {
          expected: param_typs.len(),
          got: params.len(),
        }
        .into_error(trimmed_offset, trimmed_offset + trimmed_len),
      ],
    );
  }

  let body_expr = match closure.body() {
    Some(b) => b,
    None => return TypResult::new(db, None, vec![]),
  };
  let body_node = ProjectRedNode::new(project, file, body_expr.syntax().clone());
  let body_result = actual_node_typ(db, body_node);
  let ret = match body_result.typ(db) {
    Some(typ) => typ,
    None => return TypResult::new(db, None, body_result.diagnostics(db).clone()),
  };

  let signature = FuncSignature::new(db, param_typs, ret);
  let func_typ = get_func_typ(db, signature);
  TypResult::new(
    db,
    Some(func_typ.into()),
    body_result.diagnostics(db).clone(),
  )
}

#[cfg(test)]
mod tests {
  use super::actual_node_typ;
  use crate::db::derived::get_builtin_typs::{
    get_func_typ, get_lit_typ, get_schema_meta_typ, get_string_typ,
  };
  use crate::db::derived::parse_file::parse_file;
  use crate::db::fixtures::load_vault_fixture;
  use crate::db::types::derived::obj_system::TdStaticTyp;
  use crate::db::types::{File, FileHandle, FileMetadata, LitValue, Project};
  use crate::db::types::{FuncSignature, TdTypEnum};
  use crate::db::{QueryStorage, TypedownDatabase};
  use crate::syntax::diagnostic::*;
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
  fn actual_node_type_of_schema_file_top_level_mapping_is_schema_type() {
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
      .find(|(k, _)| k == "name")
      .map(|(_, v)| v)
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
      .find(|(k, _)| k == "active")
      .map(|(_, v)| v)
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
      .find(|(k, _)| k == "tags")
      .map(|(_, v)| v)
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
      .find(|(k, _)| k == "date")
      .map(|(_, v)| v)
      .expect("should have date field");
    let result_typ = actual_node_typ(&db, date_node);
    let typ = result_typ.typ(&db).expect("should have a type");
    assert_eq!(
      typ.display_name(&db),
      "date",
      "ISO date string should resolve to date"
    );
  }

  // Fref returns the resource's schema type, not type_type
  #[test]
  fn actual_node_typ_fref_returns_resource_typ() {
    let (db, project, file) =
      load_vault_fixture("typecheck/narrow_vault", "article_fref_status.td");
    let root_node = parse_file(&db, project, file).ast(&db);
    let entries = root_node.mapping_entries().expect("should have entries");
    // status: fref("summary.td").status
    let status_node = entries
      .into_iter()
      .find(|(k, _)| k == "status")
      .map(|(_, v)| v)
      .expect("should have status field");
    let result_typ = actual_node_typ(&db, status_node);
    if let Some(typ) = result_typ.typ(&db) {
      assert_ne!(
        typ.display_name(&db),
        "type",
        "fref field access should not return type_type"
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
  fn actual_node_typ_to_string_field_access_returns_func_type() {
    let (db, _, _) = load_vault_fixture("typecheck/my_vault", "valid_person.td");
    let number_lit: TdTypEnum<'_> = get_lit_typ(&db, LitValue::Number("42".to_string())).into();

    let field_typ = number_lit
      .lookup_field_typ(&db, "to_string")
      .expect("should have to_string method");
    let expected_signature = FuncSignature::new(&db, vec![], get_string_typ(&db).into());
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
      .find(|(k, _)| k == "related")
      .map(|(_, v)| v)
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
  fn actual_node_type_method_call_to_string_returns_string_type() {
    let (db, project, file) = load_vault_fixture("typecheck/my_vault", "method_call.td");
    let root_node = parse_file(&db, project, file).ast(&db);
    let entries = root_node.mapping_entries().expect("should have entries");
    let result_node = entries
      .into_iter()
      .find(|(k, _)| k == "result")
      .map(|(_, v)| v)
      .expect("should have result field");
    let result = actual_node_typ(&db, result_node);
    let typ = result.typ(&db).expect("should have a type");
    let string_typ: TdTypEnum = get_string_typ(&db).into();
    assert_eq!(typ, string_typ);
  }
}
