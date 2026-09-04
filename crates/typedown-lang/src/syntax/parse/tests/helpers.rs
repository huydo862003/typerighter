use std::cell::RefCell;
use std::rc::Rc;

use crate::syntax::diagnostic::Diagnostic;
use typedown_types::string_stream::StringStream;

use crate::syntax::green::{GreenNode, cache::Cache};
use crate::syntax::parse::ctx::{ParseCtx, ParseResult};

pub(crate) fn parse(input: &str) -> (GreenNode, Vec<Diagnostic>) {
  let stream = StringStream::new(input);
  let cache = Rc::new(RefCell::new(Cache::new()));
  let ctx = ParseCtx::new(stream, cache);
  let ParseResult { diagnostics, ast } = ctx.parse();
  (ast, diagnostics)
}

pub(crate) fn parse_yaml_document(input: &str) -> (GreenNode, Vec<Diagnostic>) {
  let stream = StringStream::new(input);
  let cache = Rc::new(RefCell::new(Cache::new()));
  let ctx = ParseCtx::new(stream, cache);
  let ParseResult { diagnostics, ast } = ctx.parse_yaml_document();
  (ast, diagnostics)
}

/// Render a green tree in multiline lisp format
pub(crate) fn render_tree(node: &GreenNode) -> String {
  fn render_tree_inner(node: &GreenNode, indent: usize) -> String {
    let pad = "  ".repeat(indent);
    if node.is_token() {
      let token = node.as_token().unwrap();
      let text: String = token.chars().collect();
      format!("{}{:?}", pad, text)
    } else {
      let node = node.as_node().unwrap();
      let children = node.children();
      if children.is_empty() {
        format!("{}({:?})", pad, node.kind())
      } else {
        let inner: Vec<String> = children
          .iter()
          .map(|c| render_tree_inner(c, indent + 1))
          .collect();
        format!("{}({:?}\n{})", pad, node.kind(), inner.join("\n"))
      }
    }
  }

  render_tree_inner(node, 0)
}
