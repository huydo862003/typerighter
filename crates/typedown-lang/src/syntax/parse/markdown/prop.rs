//! Parser methods for JS literal prop values in container blocks

use crate::syntax::diagnostic::*;
use crate::syntax::green::{GreenNode, SyntaxToken};
use crate::syntax::syntax_kind::SyntaxKind;
use typedown_types::stream::Utf8Stream;

use super::super::constants::SKIP_NONE;
use super::super::ctx::ParseCtx;
use super::super::ctx::expr_ctx::ExprCtx;

impl<S: Utf8Stream> ParseCtx<S> {
  // Parse a JS literal value for container props
  // Allowed: number, string, true/false/null/undefined/NaN/Infinity, list, dict, unary minus
  pub(super) fn parse_js_prop_value(&mut self) -> GreenNode {
    let mode = self.lex_ctx.mode();
    let peek = self.lex_ctx.peek(SKIP_NONE, mode);

    match peek.token.kind() {
      SyntaxKind::Number | SyntaxKind::MdNumber => {
        let mut children = vec![];
        self.advance(&mut children, SKIP_NONE, mode);
        self.emit(SyntaxKind::NumberLit, &children)
      }
      SyntaxKind::DqStrStart | SyntaxKind::SqStrStart => {
        let closing = if peek.token.kind() == SyntaxKind::DqStrStart {
          SyntaxKind::DqStrEnd
        } else {
          SyntaxKind::SqStrEnd
        };
        let mut children = vec![];
        self.advance(&mut children, SKIP_NONE, mode); // opening quote
        loop {
          let kind = self.lex_ctx.peek(SKIP_NONE, mode).token.kind();
          if kind == closing {
            self.advance(&mut children, SKIP_NONE, mode);
            break;
          }
          if matches!(kind, SyntaxKind::Eof | SyntaxKind::Newline) {
            break;
          }
          self.advance(&mut children, SKIP_NONE, mode);
        }
        self.emit(SyntaxKind::StrLit, &children)
      }
      SyntaxKind::Ident if Self::is_js_keyword_literal(&peek.token) => {
        let mut children = vec![];
        self.advance(&mut children, SKIP_NONE, mode);
        self.emit(SyntaxKind::IdentLit, &children)
      }
      SyntaxKind::LBracket => self.parse_js_prop_list(),
      SyntaxKind::LBrace => self.parse_js_prop_dict(),
      // Unary minus
      _ if matches!(peek.token.kind(), SyntaxKind::MdSymbol | SyntaxKind::YamlOp)
        && peek.token.text().is_some_and(|text| text == "-") =>
      {
        let mut children = vec![];
        self.advance(&mut children, SKIP_NONE, mode);
        children.push(self.parse_js_prop_value());
        self.emit(SyntaxKind::PrefixExpr, &children)
      }
      _ => {
        self.diagnostics.push(
          TdDiagnosticKind::MissingContainerPropValueAfterEq {}
            .into_error(self.offset(), self.offset()),
        );
        self.emit(SyntaxKind::Error, &[])
      }
    }
  }

  // Parse a list literal in prop value context: [value, value, ...]
  fn parse_js_prop_list(&mut self) -> GreenNode {
    let mode = self.lex_ctx.mode();
    let mut children = vec![];
    self.expr_ctx_stack.enter(ExprCtx::List);

    // Consume `[`
    self.advance(&mut children, SKIP_NONE, mode);

    loop {
      let peek = self.lex_ctx.peek(SKIP_NONE, mode);
      match peek.token.kind() {
        SyntaxKind::RBracket => {
          self.advance(&mut children, SKIP_NONE, mode);
          break;
        }
        SyntaxKind::Eof | SyntaxKind::RBrace | SyntaxKind::Newline => break,
        SyntaxKind::Whitespace | SyntaxKind::Comma => {
          self.advance(&mut children, SKIP_NONE, mode);
        }
        _ => {
          let item = self.parse_js_prop_value();
          if item.kind() == SyntaxKind::Error {
            self.synchronize_js_prop_list(&mut children);
            break;
          }
          children.push(item);
        }
      }
    }

    self.expr_ctx_stack.exit(ExprCtx::List);
    self.emit(SyntaxKind::ListLit, &children)
  }

  fn synchronize_js_prop_list(&mut self, children: &mut Vec<GreenNode>) {
    let mode = self.lex_ctx.mode();
    let mut bad = vec![];
    loop {
      match self.lex_ctx.peek(SKIP_NONE, mode).token.kind() {
        SyntaxKind::RBracket
        | SyntaxKind::RBrace
        | SyntaxKind::Comma
        | SyntaxKind::Newline
        | SyntaxKind::Eof => break,
        _ => {
          self.advance(&mut bad, SKIP_NONE, mode);
        }
      }
    }
    if !bad.is_empty() {
      children.push(self.emit(SyntaxKind::Error, &bad));
    }
  }

  // Parse a dict literal in prop value context: {key: value, ...}
  fn parse_js_prop_dict(&mut self) -> GreenNode {
    let mode = self.lex_ctx.mode();
    let mut children = vec![];
    self.expr_ctx_stack.enter(ExprCtx::Dict);

    // Consume `{`
    self.advance(&mut children, SKIP_NONE, mode);

    loop {
      let peek = self.lex_ctx.peek(SKIP_NONE, mode);
      match peek.token.kind() {
        SyntaxKind::RBrace => {
          self.advance(&mut children, SKIP_NONE, mode);
          break;
        }
        SyntaxKind::Eof | SyntaxKind::Newline => break,
        SyntaxKind::Whitespace | SyntaxKind::Comma => {
          self.advance(&mut children, SKIP_NONE, mode);
        }
        // Key: any ident or string or keyword literal, then colon, then value
        _ => {
          let is_ident = peek.token.kind() == SyntaxKind::Ident;
          let key = if is_ident {
            // Accept any identifier as a dict key
            let mut children = vec![];
            self.advance(&mut children, SKIP_NONE, mode);
            self.emit(SyntaxKind::IdentLit, &children)
          } else {
            self.parse_js_prop_value()
          };
          if key.kind() == SyntaxKind::Error {
            self.synchronize_js_prop_dict(&mut children);
            break;
          }
          children.push(key);
          // Colon
          let peek = self.lex_ctx.peek(SKIP_NONE, mode);
          if peek.token.kind() == SyntaxKind::Colon
            || (peek.token.kind() == SyntaxKind::MdSymbol
              && peek.token.text().is_some_and(|text| text == ":"))
          {
            self.advance(&mut children, SKIP_NONE, mode);
            // Skip whitespace after colon
            while self.lex_ctx.peek(SKIP_NONE, mode).token.kind() == SyntaxKind::Whitespace {
              self.advance(&mut children, SKIP_NONE, mode);
            }
            let value = self.parse_js_prop_value();
            if value.kind() == SyntaxKind::Error {
              self.synchronize_js_prop_dict(&mut children);
              break;
            }
            children.push(value);
          } else {
            // No colon found: synchronize to avoid looping on the same token
            self.synchronize_js_prop_dict(&mut children);
            break;
          }
        }
      }
    }

    self.expr_ctx_stack.exit(ExprCtx::Dict);
    self.emit(SyntaxKind::DictLit, &children)
  }

  fn synchronize_js_prop_dict(&mut self, children: &mut Vec<GreenNode>) {
    let mode = self.lex_ctx.mode();
    let mut bad = vec![];
    loop {
      match self.lex_ctx.peek(SKIP_NONE, mode).token.kind() {
        SyntaxKind::RBrace | SyntaxKind::Comma | SyntaxKind::Newline | SyntaxKind::Eof => break,
        _ => {
          self.advance(&mut bad, SKIP_NONE, mode);
        }
      }
    }
    if !bad.is_empty() {
      children.push(self.emit(SyntaxKind::Error, &bad));
    }
  }

  fn is_js_keyword_literal(token: &SyntaxToken) -> bool {
    matches!(
      token.text(),
      Some("true" | "false" | "null" | "undefined" | "NaN" | "Infinity")
    )
  }
}
