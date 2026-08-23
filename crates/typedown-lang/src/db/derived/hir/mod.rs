//! Tracked query to lower an expression node into a HIR value.

use typedown_macros::query_derived;

use crate::syntax::ast::{
  AstNode, BinaryExpr, CallExpr, ClosureExpr, CodeBlock, CodeLit, DictEntry, DictLit, Expr,
  IdentLit, IndexExpr, InlineCode, InlineMath, InterpFragment, ListItem, ListLit, MathBlock,
  MathLit, MdBody, NumberLit, ParenExpr, PostfixExpr, PrefixExpr, SourceFile, StrLit,
  YamlFrontmatter, YamlMapping, YamlSequence,
};
use crate::syntax::diagnostic::Diagnostic;
use crate::syntax::red::RedNode;
use crate::syntax::syntax_kind::SyntaxKind;
use typedown_types::either::Either;

use crate::db::TypedownDatabase;
use crate::db::types::{File, HirValue, HirValueKind, InterpolatedPart, Project};
use typedown_incremental::QueryDatabase;

// Normalize expressions to a hir form
#[query_derived]
pub fn lower_node(db: &TypedownDatabase, project: Project, file: File, node: RedNode) -> HirValue {
  if SourceFile::cast(node.clone()).is_some() {
    return lower_source_file(db, project, file, node);
  }
  if YamlFrontmatter::cast(node.clone()).is_some() {
    return lower_frontmatter(db, project, file, node);
  }
  if MdBody::cast(node.clone()).is_some() {
    return lower_markdown(db, project, file, node);
  }
  let expr =
    Expr::cast(node.clone()).expect("node must be an Expr, MdBody, YamlFrontmatter, or SourceFile");
  let mut diagnostics = vec![];
  let kind = lower_expr_kind(db, project, file, &expr, &mut diagnostics);
  HirValue::new(db, project, file, node, kind, diagnostics)
}

fn lower_markdown(db: &TypedownDatabase, project: Project, file: File, node: RedNode) -> HirValue {
  fn collect_interpolated_parts(
    db: &TypedownDatabase,
    project: Project,
    file: File,
    node: RedNode,
    parts: &mut Vec<InterpolatedPart>,
  ) {
    // If node is an interp fragment, lower the expression inside it
    if node.kind() == SyntaxKind::InterpFragment
      && let Some(expr) = InterpFragment::cast(node.clone()).and_then(|f| f.expr())
    {
      let hir = lower_node(db, project, file, expr.syntax().clone());
      parts.push(InterpolatedPart::Expr(hir));
      return;
    }
    if MathBlock::cast(node.clone()).is_some()
      || CodeBlock::cast(node.clone()).is_some()
      || InlineMath::cast(node.clone()).is_some()
      || InlineCode::cast(node.clone()).is_some()
    {
      let text = node.text();
      match parts.last_mut() {
        Some(InterpolatedPart::Literal(existing)) => existing.push_str(&text),
        _ => parts.push(InterpolatedPart::Literal(text)),
      }
      return;
    }
    // If node is a token, it must be a string token
    if node.is_token() {
      let text = node.text();
      if !text.is_empty() {
        match parts.last_mut() {
          Some(InterpolatedPart::Literal(existing)) => existing.push_str(&text),
          _ => parts.push(InterpolatedPart::Literal(text)),
        }
      }
      return;
    }
    for child in node.children() {
      collect_interpolated_parts(db, project, file, child, parts);
    }
  }

  let mut parts: Vec<InterpolatedPart> = vec![];
  collect_interpolated_parts(db, project, file, node.clone(), &mut parts);
  HirValue::new(
    db,
    project,
    file,
    node,
    HirValueKind::Markdown(parts),
    vec![],
  )
}

fn lower_expr_kind(
  db: &TypedownDatabase,
  project: Project,
  file: File,
  expr: &Expr,
  diagnostics: &mut Vec<Diagnostic>,
) -> HirValueKind {
  let inner = unwrap_parens(expr.clone());

  // Handle block mapping
  if let Some(mapping) = YamlMapping::cast(inner.syntax().clone()) {
    let entries = mapping.entries().collect::<Vec<_>>();
    let mut seen_keys = std::collections::HashSet::new();
    let hir_entries = entries
      .into_iter()
      .map(|(key, val_expr)| {
        if !seen_keys.insert(key.clone()) {
          let node = val_expr.syntax();
          let (tr_offset, tr_len) = node.trimmed_range();
          diagnostics.push(Diagnostic::DuplicateKey {
            key: key.clone(),
            start_offset: tr_offset,
            end_offset: tr_offset + tr_len,
          });
        }
        let child = lower_node(db, project, file, val_expr.syntax().clone());
        (key, child)
      })
      .collect();
    return HirValueKind::Mapping(hir_entries);
  }

  // Handle flow mapping
  if let Some(dict) = DictLit::cast(inner.syntax().clone()) {
    let entries = dict
      .entries()
      .filter_map(|entry: DictEntry| entry.entry())
      .collect::<Vec<_>>();
    let mut seen_keys = std::collections::HashSet::new();
    let hir_entries = entries
      .into_iter()
      .map(|(key, val_expr)| {
        if !seen_keys.insert(key.clone()) {
          let node = val_expr.syntax();
          let (tr_offset, tr_len) = node.trimmed_range();
          diagnostics.push(Diagnostic::DuplicateKey {
            key: key.clone(),
            start_offset: tr_offset,
            end_offset: tr_offset + tr_len,
          });
        }
        let child = lower_node(db, project, file, val_expr.syntax().clone());
        (key, child)
      })
      .collect();
    return HirValueKind::Mapping(hir_entries);
  }

  // Handle block sequence
  if let Some(seq) = YamlSequence::cast(inner.syntax().clone()) {
    let items = seq.values().collect::<Vec<_>>();
    let hir_items = items
      .into_iter()
      .map(|item_expr| lower_node(db, project, file, item_expr.syntax().clone()))
      .collect();
    return HirValueKind::Sequence(hir_items);
  }

  // Handle flow sequence
  if let Some(list) = ListLit::cast(inner.syntax().clone()) {
    let items = list
      .items()
      .filter_map(|item: ListItem| item.value())
      .collect::<Vec<_>>();
    let hir_items = items
      .into_iter()
      .map(|item_expr| lower_node(db, project, file, item_expr.syntax().clone()))
      .collect();
    return HirValueKind::Sequence(hir_items);
  }

  // Handle string literals (single/double quoted)
  if let Some(lit) = StrLit::cast(inner.syntax().clone()) {
    let children: Vec<_> = inner.syntax().children().collect();
    let has_text = children.iter().any(|c| {
      matches!(
        c.kind(),
        SyntaxKind::SqStrContent | SyntaxKind::DqStrContent
      ) && !c.text().trim().is_empty()
    });
    let has_embedded = lit.is_interpolated()
      || children
        .iter()
        .any(|c| matches!(c.kind(), SyntaxKind::MathLit | SyntaxKind::InlineCode));

    // '$x$' (math only, no text) -> Math
    if !has_text {
      if let Some(math) = children.iter().find_map(|c| MathLit::cast(c.clone()))
        && let Some(val) = math.value()
      {
        return HirValueKind::Math(val);
      }
      if let Some(code) = children.iter().find_map(|c| CodeLit::cast(c.clone()))
        && let Some(val) = code.value()
      {
        return HirValueKind::Str(val);
      }
    }

    // 'text $x$ text' or 'text ${expr} text' -> Interpolated
    if has_embedded {
      let hir_parts = children
        .into_iter()
        .filter_map(|child| match child.kind() {
          SyntaxKind::SqStrContent | SyntaxKind::DqStrContent => {
            let text = child.text();
            (!text.is_empty()).then(|| InterpolatedPart::Literal(text.to_string()))
          }
          SyntaxKind::InterpFragment => {
            let frag = InterpFragment::cast(child)?;
            let child_expr = frag.expr()?;
            Some(InterpolatedPart::Expr(lower_node(
              db,
              project,
              file,
              child_expr.syntax().clone(),
            )))
          }
          SyntaxKind::MathLit => {
            let math = MathLit::cast(child.clone())?;
            let val = math.value()?;
            let hir = HirValue::new(db, project, file, child, HirValueKind::Math(val), vec![]);
            Some(InterpolatedPart::Expr(hir))
          }
          _ => None,
        })
        .collect();
      return HirValueKind::Interpolated(hir_parts);
    }

    // 'plain text' -> Str
    let text = lit
      .fragments()
      .filter_map(|frag| match frag {
        Either::Left(s) => Some(s),
        Either::Right(_) => None,
      })
      .collect::<Vec<_>>()
      .join("");
    return HirValueKind::Str(text);
  }

  // Handle number lit
  if let Some(lit) = NumberLit::cast(inner.syntax().clone())
    && let Some(val) = lit.value()
  {
    return HirValueKind::Num(val.to_string());
  }

  // Handle math lit
  if let Some(lit) = MathLit::cast(inner.syntax().clone())
    && let Some(val) = lit.value()
  {
    return HirValueKind::Math(val);
  }

  // Handle code lit
  if let Some(lit) = CodeLit::cast(inner.syntax().clone())
    && let Some(val) = lit.value()
  {
    return HirValueKind::Str(val);
  }

  // Handle identifier
  if let Some(lit) = IdentLit::cast(inner.syntax().clone())
    && let Some(val) = lit.value()
  {
    return match val.as_str() {
      "null" => HirValueKind::Null,
      "true" => HirValueKind::Bool(true),
      "false" => HirValueKind::Bool(false),
      _ => HirValueKind::Ident(val),
    };
  }

  // Handle prefix
  if let Some(prefix) = PrefixExpr::cast(inner.syntax().clone())
    && let Some(operand) = prefix.expr()
  {
    let op = prefix
      .op()
      .and_then(|o| o.syntax().as_token())
      .and_then(|t| t.text().map(|s| s.to_string()))
      .unwrap_or_default();
    let operand = lower_node(db, project, file, operand.syntax().clone());
    // This is a tag expression
    if op.starts_with('!') && op.len() > 1 {
      let tag_name = op[1..].to_string();
      let op_node = prefix.op().unwrap().syntax().clone();
      let tag_hir = HirValue::new(
        db,
        project,
        file,
        op_node,
        HirValueKind::Ident(tag_name),
        vec![],
      );
      return HirValueKind::Tag {
        tag: tag_hir.into(),
        inner: operand.into(),
      };
    }
    return HirValueKind::Prefix {
      op,
      operand: operand.into(),
    };
  }

  // Handle postfix
  if let Some(postfix) = PostfixExpr::cast(inner.syntax().clone())
    && let Some(operand) = postfix.expr()
  {
    let op = postfix
      .op()
      .and_then(|o| o.syntax().as_token())
      .and_then(|t| t.text().map(|s| s.to_string()))
      .unwrap_or_default();
    let operand = lower_node(db, project, file, operand.syntax().clone());
    return HirValueKind::Postfix {
      op,
      operand: operand.into(),
    };
  }

  // Handle binary
  if let Some(binary) = BinaryExpr::cast(inner.syntax().clone())
    && let (Some(lhs), Some(rhs)) = (binary.left(), binary.right())
  {
    let op = binary
      .op()
      .and_then(|o| o.syntax().as_token())
      .and_then(|t| t.text().map(|s| s.to_string()))
      .unwrap_or_default();
    let left = lower_node(db, project, file, lhs.syntax().clone());
    let right = lower_node(db, project, file, rhs.syntax().clone());
    return HirValueKind::Binary {
      op,
      left: left.into(),
      right: right.into(),
    };
  }

  // Handle call expression
  if let Some(call) = CallExpr::cast(inner.syntax().clone())
    && let Some(callee) = call.callee()
  {
    let callee = lower_node(db, project, file, callee.syntax().clone());
    let args = call
      .args()
      .into_iter()
      .map(|arg| lower_node(db, project, file, arg.syntax().clone()))
      .collect();
    return HirValueKind::Call {
      callee: callee.into(),
      args,
    };
  }

  // Handle index expression
  if let Some(index) = IndexExpr::cast(inner.syntax().clone())
    && let Some(expr) = index.expr()
  {
    let expr = lower_node(db, project, file, expr.syntax().clone());
    let indices = index
      .indices()
      .into_iter()
      .map(|idx| lower_node(db, project, file, idx.syntax().clone()))
      .collect();
    return HirValueKind::Index {
      expr: expr.into(),
      indices,
    };
  }

  // Handle closure expression
  if let Some(closure) = ClosureExpr::cast(inner.syntax().clone()) {
    let param_idents: Vec<IdentLit> = match closure.params() {
      Some(Either::Left(param_list)) => param_list.params().collect(),
      Some(Either::Right(ident)) => vec![ident],
      None => vec![],
    };
    let mut params = Vec::new();
    for id in param_idents {
      if let Some(name) = id.value() {
        if name == "self" {
          let node = id.syntax();
          let (tr_offset, tr_len) = node.trimmed_range();
          diagnostics.push(Diagnostic::InvalidSelfClosureParams {
            start_offset: tr_offset,
            end_offset: tr_offset + tr_len,
          });
        } else {
          params.push(name);
        }
      }
    }
    let body = closure
      .body()
      .map(|b| lower_node(db, project, file, b.syntax().clone()))
      .unwrap_or_else(|| {
        HirValue::new(
          db,
          project,
          file,
          inner.syntax().clone(),
          HirValueKind::Null,
          vec![],
        )
      });
    return HirValueKind::Closure {
      params,
      body: body.into(),
    };
  }

  HirValueKind::Str(inner.syntax().text().trim().to_string())
}

// Remove unnecessary parens
fn unwrap_parens(expr: Expr) -> Expr {
  if let Some(paren) = ParenExpr::cast(expr.syntax().clone())
    && let Some(inner) = paren.expr()
  {
    return unwrap_parens(inner);
  }
  expr
}

fn lower_frontmatter(
  db: &TypedownDatabase,
  project: Project,
  file: File,
  node: RedNode,
) -> HirValue {
  let fm = YamlFrontmatter::cast(node.clone()).expect("node must be a YamlFrontmatter");
  match fm.mapping() {
    Some(mapping) => lower_node(db, project, file, mapping.syntax().clone()),
    None => HirValue::new(db, project, file, node, HirValueKind::Null, vec![]),
  }
}

fn lower_source_file(
  db: &TypedownDatabase,
  project: Project,
  file: File,
  node: RedNode,
) -> HirValue {
  let source_file = SourceFile::cast(node.clone()).expect("node must be a SourceFile");
  let fm_node = match source_file.frontmatter() {
    Some(fm) => fm,
    None => return HirValue::new(db, project, file, node, HirValueKind::Null, vec![]),
  };
  let mapping_hir = lower_node(db, project, file, fm_node.syntax().clone());
  let Some(body) = source_file.body() else {
    return mapping_hir;
  };
  let content_hir = lower_node(db, project, file, body.syntax().clone());
  let mut entries = match mapping_hir.kind(db) {
    HirValueKind::Mapping(entries) => entries,
    _ => vec![],
  };
  entries.push(("_content".to_string(), content_hir));
  let mapping_diagnostics = mapping_hir.diagnostics(db);
  HirValue::new(
    db,
    project,
    file,
    node,
    HirValueKind::Mapping(entries),
    mapping_diagnostics,
  )
}

#[cfg(test)]
mod tests {
  use super::lower_node;
  use crate::db::fixtures::load_vault_fixture;
  use crate::db::types::{HirValueKind, InterpolatedPart};
  use crate::db::utils::lower_file;
  use crate::syntax::diagnostic::Diagnostic;
  use crate::syntax::parse::tests::helpers::parse;
  use crate::syntax::red::RedNode;

  #[test]
  fn markdown_body_plain_text() {
    let (db, project, file) = load_vault_fixture("evaluate/my_vault", "md_plain.td");
    let (hir, _) = lower_file(&db, project, file);
    let hir = hir.expect("should have HIR");
    let entries = match hir.kind(&db) {
      HirValueKind::Mapping(e) => e,
      _ => panic!("expected mapping"),
    };
    let content = entries
      .iter()
      .find(|(k, _)| k == "_content")
      .expect("_content missing");
    match content.1.kind(&db) {
      HirValueKind::Markdown(parts) => {
        assert!(
          parts
            .iter()
            .all(|p| matches!(p, InterpolatedPart::Literal(_)))
        );
      }
      _ => panic!("expected Markdown kind"),
    }
  }

  #[test]
  fn markdown_body_inline_math() {
    let (db, project, file) = load_vault_fixture("evaluate/my_vault", "md_inline_math.td");
    let (hir, _) = lower_file(&db, project, file);
    let hir = hir.expect("should have HIR");
    let entries = match hir.kind(&db) {
      HirValueKind::Mapping(e) => e,
      _ => panic!("expected mapping"),
    };
    let content = entries
      .iter()
      .find(|(k, _)| k == "_content")
      .expect("_content missing");
    let parts = match content.1.kind(&db) {
      HirValueKind::Markdown(parts) => parts,
      _ => panic!("expected Markdown kind"),
    };
    let has_math = parts
      .iter()
      .any(|p| matches!(p, InterpolatedPart::Literal(s) if s.contains('$')));
    assert!(has_math, "expected inline math as literal with $ delimiter");
  }

  #[test]
  fn markdown_body_inline_code() {
    let (db, project, file) = load_vault_fixture("evaluate/my_vault", "md_inline_code.td");
    let (hir, _) = lower_file(&db, project, file);
    let hir = hir.expect("should have HIR");
    let entries = match hir.kind(&db) {
      HirValueKind::Mapping(e) => e,
      _ => panic!("expected mapping"),
    };
    let content = entries
      .iter()
      .find(|(k, _)| k == "_content")
      .expect("_content missing");
    let parts = match content.1.kind(&db) {
      HirValueKind::Markdown(parts) => parts,
      _ => panic!("expected Markdown kind"),
    };
    let has_code = parts
      .iter()
      .any(|p| matches!(p, InterpolatedPart::Literal(s) if s.contains('`')));
    assert!(
      has_code,
      "expected inline code as literal with backtick delimiter"
    );
  }

  #[test]
  fn markdown_body_interpolation() {
    let (db, project, file) = load_vault_fixture("evaluate/my_vault", "md_interp.td");
    let (hir, _) = lower_file(&db, project, file);
    let hir = hir.expect("should have HIR");
    let entries = match hir.kind(&db) {
      HirValueKind::Mapping(e) => e,
      _ => panic!("expected mapping"),
    };
    let content = entries
      .iter()
      .find(|(k, _)| k == "_content")
      .expect("_content missing");
    let parts = match content.1.kind(&db) {
      HirValueKind::Markdown(parts) => parts,
      _ => panic!("expected Markdown kind"),
    };
    let has_expr = parts.iter().any(|p| matches!(p, InterpolatedPart::Expr(_)));
    assert!(has_expr, "expected interpolated expr part");
  }

  #[test]
  fn markdown_body_math_block() {
    let (db, project, file) = load_vault_fixture("evaluate/my_vault", "md_math_block.td");
    let (hir, _) = lower_file(&db, project, file);
    let hir = hir.expect("should have HIR");
    let entries = match hir.kind(&db) {
      HirValueKind::Mapping(e) => e,
      _ => panic!("expected mapping"),
    };
    let content = entries
      .iter()
      .find(|(k, _)| k == "_content")
      .expect("_content missing");
    let parts = match content.1.kind(&db) {
      HirValueKind::Markdown(parts) => parts,
      _ => panic!("expected Markdown kind"),
    };
    let has_math = parts
      .iter()
      .any(|p| matches!(p, InterpolatedPart::Literal(s) if s.contains("$$")));
    assert!(has_math, "expected math block as literal with $$ delimiter");
  }

  #[test]
  fn markdown_body_code_block() {
    let (db, project, file) = load_vault_fixture("evaluate/my_vault", "md_code_block.td");
    let (hir, _) = lower_file(&db, project, file);
    let hir = hir.expect("should have HIR");
    let entries = match hir.kind(&db) {
      HirValueKind::Mapping(e) => e,
      _ => panic!("expected mapping"),
    };
    let content = entries
      .iter()
      .find(|(k, _)| k == "_content")
      .expect("_content missing");
    let parts = match content.1.kind(&db) {
      HirValueKind::Markdown(parts) => parts,
      _ => panic!("expected Markdown kind"),
    };
    let has_code = parts
      .iter()
      .any(|p| matches!(p, InterpolatedPart::Literal(s) if s.contains("```")));
    assert!(
      has_code,
      "expected code block as literal with ``` delimiter"
    );
  }

  #[test]
  fn lower_closure_expression() {
    let (db, project, file) = load_vault_fixture("evaluate/my_vault", "simple.td");
    let (root, _) = parse(
      r#"---
fn: (a, b) -> a + b
---"#,
    );
    let red_root = RedNode::new_root(root.as_node().unwrap().clone());
    let hir = lower_node(&db, project, file, red_root);

    let entries = match hir.kind(&db) {
      HirValueKind::Mapping(e) => e,
      _ => panic!("expected mapping"),
    };
    let fn_entry = entries
      .iter()
      .find(|(k, _)| k == "fn")
      .expect("fn field missing");
    match fn_entry.1.kind(&db) {
      HirValueKind::Closure { params, body } => {
        assert_eq!(params, vec!["a", "b"]);
        assert!(matches!(body.kind(&db), HirValueKind::Binary { .. }));
      }
      _ => panic!("expected Closure HIR kind"),
    }
  }

  #[test]
  fn lower_bare_ident_closure_expression() {
    let (db, project, file) = load_vault_fixture("evaluate/my_vault", "simple.td");
    let (root, _) = parse(
      r#"---
fn: x -> x + 1
---"#,
    );
    let red_root = RedNode::new_root(root.as_node().unwrap().clone());
    let hir = lower_node(&db, project, file, red_root);

    let entries = match hir.kind(&db) {
      HirValueKind::Mapping(e) => e,
      _ => panic!("expected mapping"),
    };
    let fn_entry = entries
      .iter()
      .find(|(k, _)| k == "fn")
      .expect("fn field missing");
    match fn_entry.1.kind(&db) {
      HirValueKind::Closure { params, body } => {
        assert_eq!(params, vec!["x"]);
        assert!(matches!(body.kind(&db), HirValueKind::Binary { .. }));
      }
      _ => panic!("expected Closure HIR kind"),
    }
  }

  #[test]
  fn lower_closure_forbids_self_param() {
    let (db, project, file) = load_vault_fixture("evaluate/my_vault", "simple.td");
    let (root, _) = parse(
      r#"---
fn: (self, x) -> self + x
---"#,
    );
    let red_root = RedNode::new_root(root.as_node().unwrap().clone());
    let hir = lower_node(&db, project, file, red_root);

    let entries = match hir.kind(&db) {
      HirValueKind::Mapping(e) => e,
      _ => panic!("expected mapping"),
    };
    let fn_entry = entries
      .iter()
      .find(|(k, _)| k == "fn")
      .expect("fn field missing");
    assert!(
      fn_entry
        .1
        .diagnostics(&db)
        .iter()
        .any(|d| matches!(d, Diagnostic::InvalidSelfClosureParams { .. })),
      "expected InvalidSelfClosureParams diagnostic for 'self' parameter"
    );
    match fn_entry.1.kind(&db) {
      HirValueKind::Closure { params, .. } => {
        assert_eq!(params, vec!["x"]);
      }
      _ => panic!("expected Closure HIR kind"),
    }
  }
}
