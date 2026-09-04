//! A recursive descent parser

pub(in crate::syntax::parse) mod expr_ctx;
pub(in crate::syntax::parse) mod peekable_lex_ctx;

use std::{cell::RefCell, rc::Rc};

use crate::syntax::diagnostic::Diagnostic;
use crate::syntax::syntax_kind::SyntaxKind;
use typedown_types::stream::Utf8Stream;

use super::constants::SKIP_NONE;
use crate::syntax::{
  green::{GreenNode, SyntaxToken, cache::Cache},
  lex::ctx::{LexCtx, LexMode, LexResult},
};
use expr_ctx::ExprCtxStack;
use peekable_lex_ctx::{MdLexResult, PeekableLexCtx, YamlLexResult};

pub struct ParseCtx<S: Utf8Stream> {
  pub(in crate::syntax::parse) cache: Rc<RefCell<Cache>>,
  pub(in crate::syntax::parse) lex_ctx: PeekableLexCtx<S>,
  pub(in crate::syntax::parse) diagnostics: Vec<Diagnostic>,
  pub(in crate::syntax::parse) expr_ctx_stack: ExprCtxStack,
}

pub struct ParseResult {
  pub ast: GreenNode,
  pub diagnostics: Vec<Diagnostic>,
}

impl<S: Utf8Stream> ParseCtx<S> {
  pub fn new(stream: S, cache: Rc<RefCell<Cache>>) -> ParseCtx<S> {
    let expr_ctx_stack = ExprCtxStack::new(cache.clone());
    Self {
      cache: cache.clone(),
      lex_ctx: PeekableLexCtx::new(LexCtx::new(stream, cache)),
      diagnostics: Vec::new(),
      expr_ctx_stack,
    }
  }

  // Consumes the parser, returns the source file AST (frontmatter + markdown body)
  pub fn parse(mut self) -> ParseResult {
    let root = self.parse_source_file();
    ParseResult {
      ast: root,
      diagnostics: self.diagnostics,
    }
  }

  // Consumes the parser, returns a bare YAML document AST (no --- delimiters)
  pub fn parse_yaml_document(mut self) -> ParseResult {
    let yaml_document = self.parse_yaml_document_body();
    let root = self.emit(SyntaxKind::SourceFile, &[yaml_document]);
    ParseResult {
      ast: root,
      diagnostics: self.diagnostics,
    }
  }

  pub(in crate::syntax::parse) fn parse_source_file(&mut self) -> GreenNode {
    let yaml_frontmatter = self.parse_yaml_frontmatter();
    self.lex_ctx.set_mode(LexMode::MarkdownBody);
    let markdown_body = self.parse_markdown_body();
    self.emit(SyntaxKind::SourceFile, &[yaml_frontmatter, markdown_body])
  }
}

impl<S: Utf8Stream> ParseCtx<S> {
  /// Consume the next non-skipped YAML token, pushing skipped trivia and result into children.
  pub(in crate::syntax::parse) fn advance_yaml(
    &mut self,
    children: &mut Vec<GreenNode>,
    skip: u16,
  ) -> YamlLexResult {
    loop {
      let mut result = self.lex_ctx.lex();
      if let Some(diagnostic) = result.diagnostic.take() {
        self.diagnostics.push(diagnostic);
      }
      if self.lex_ctx.should_skip(result.token.kind(), skip) {
        children.push(GreenNode::from_token(result.token));
      } else {
        let token_indent = self.lex_ctx.token_indent();
        let block_indent = self.lex_ctx.block_indent();
        children.push(GreenNode::from_token(result.token.clone()));
        return YamlLexResult::new(result, token_indent, block_indent);
      }
    }
  }

  /// Consume the next non-skipped Markdown token, pushing skipped trivia and result into children.
  pub(in crate::syntax::parse) fn advance_md(
    &mut self,
    children: &mut Vec<GreenNode>,
    skip: u16,
  ) -> MdLexResult {
    loop {
      let mut result = self.lex_ctx.lex();
      if let Some(diagnostic) = result.diagnostic.take() {
        self.diagnostics.push(diagnostic);
      }
      if self.lex_ctx.should_skip(result.token.kind(), skip) {
        children.push(GreenNode::from_token(result.token));
      } else {
        children.push(GreenNode::from_token(result.token.clone()));
        return MdLexResult::new(result);
      }
    }
  }

  pub fn advance(&mut self, children: &mut Vec<GreenNode>, skip: u16, mode: LexMode) -> LexResult {
    debug_assert!(
      self.lex_ctx.mode() == mode,
      "[PeekableLexCtx::advance] Lex mode must be the same as the `mode` argument"
    );
    match mode {
      LexMode::YamlFrontmatter => self.advance_yaml(children, skip).into(),
      LexMode::MarkdownBody => self.advance_md(children, skip).into(),
    }
  }

  /// Like advance_yaml(), but expects the token to match `expected`.
  pub(in crate::syntax::parse) fn consume_yaml(
    &mut self,
    children: &mut Vec<GreenNode>,
    skip: u16,
    expected: SyntaxKind,
    diagnostic: Diagnostic,
  ) -> bool {
    let result = self.advance_yaml(children, skip);
    if result.token.kind() != expected {
      let bad_token = children.pop().unwrap();
      children.push(self.emit(SyntaxKind::Error, &[bad_token]));
      self.diagnostics.push(diagnostic);
      false
    } else {
      true
    }
  }

  /// Like advance_md(), but expects the token to match `expected`.
  pub(in crate::syntax::parse) fn consume_md(
    &mut self,
    children: &mut Vec<GreenNode>,
    skip: u16,
    expected: SyntaxKind,
    diagnostic: Diagnostic,
  ) -> bool {
    let result = self.advance_md(children, skip);
    if result.token.kind() != expected {
      let bad_token = children.pop().unwrap();
      children.push(self.emit(SyntaxKind::Error, &[bad_token]));
      self.diagnostics.push(diagnostic);
      false
    } else {
      true
    }
  }

  pub fn consume(
    &mut self,
    children: &mut Vec<GreenNode>,
    skip: u16,
    mode: LexMode,
    expected: SyntaxKind,
    diagnostic: Diagnostic,
  ) -> bool {
    debug_assert!(
      self.lex_ctx.mode() == mode,
      "[PeekableLexCtx::consume] Lex mode must be the same as the `mode` argument"
    );
    match mode {
      LexMode::YamlFrontmatter => self.consume_yaml(children, skip, expected, diagnostic),
      LexMode::MarkdownBody => self.consume_md(children, skip, expected, diagnostic),
    }
  }

  /// Like advance_yaml(), but expects the token to satisfy a predicate.
  pub(in crate::syntax::parse) fn consume_yaml_if(
    &mut self,
    children: &mut Vec<GreenNode>,
    skip: u16,
    predicate: impl Fn(&SyntaxToken) -> bool,
    diagnostic: Diagnostic,
  ) -> bool {
    let result = self.advance_yaml(children, skip);
    if !predicate(&result.token) {
      let bad_token = children.pop().unwrap();
      children.push(self.emit(SyntaxKind::Error, &[bad_token]));
      self.diagnostics.push(diagnostic);
      false
    } else {
      true
    }
  }

  /// Like advance_md(), but expects the token to satisfy a predicate.
  pub(in crate::syntax::parse) fn consume_md_if(
    &mut self,
    children: &mut Vec<GreenNode>,
    skip: u16,
    predicate: impl Fn(&SyntaxToken) -> bool,
    diagnostic: Diagnostic,
  ) -> bool {
    let result = self.advance_md(children, skip);
    if !predicate(&result.token) {
      let bad_token = children.pop().unwrap();
      children.push(self.emit(SyntaxKind::Error, &[bad_token]));
      self.diagnostics.push(diagnostic);
      false
    } else {
      true
    }
  }

  pub fn consume_if(
    &mut self,
    children: &mut Vec<GreenNode>,
    skip: u16,
    mode: LexMode,
    predicate: impl Fn(&SyntaxToken) -> bool,
    diagnostic: Diagnostic,
  ) -> bool {
    debug_assert!(
      self.lex_ctx.mode() == mode,
      "[PeekableLexCtx::consume_if] Lex mode must be the same as the `mode` argument"
    );
    match mode {
      LexMode::YamlFrontmatter => self.consume_yaml_if(children, skip, predicate, diagnostic),
      LexMode::MarkdownBody => self.consume_md_if(children, skip, predicate, diagnostic),
    }
  }

  /// Consume everything until newline or EOF.
  pub(in crate::syntax::parse) fn expect_end_of_line(
    &mut self,
    children: &mut Vec<GreenNode>,
    diagnostic: Diagnostic,
  ) -> bool {
    let mut has_error = false;
    loop {
      let peek = self.lex_ctx.peek_yaml(SKIP_NONE);
      match peek.token.kind() {
        SyntaxKind::Newline => {
          self.advance_yaml(children, SKIP_NONE);
          break;
        }
        SyntaxKind::Eof => break,
        SyntaxKind::Whitespace | SyntaxKind::YamlComment => {
          self.advance_yaml(children, SKIP_NONE);
        }
        _ => {
          self.advance_yaml(children, SKIP_NONE);
          let bad = children.pop().unwrap();
          children.push(self.emit(SyntaxKind::Error, &[bad]));
          if !has_error {
            self.diagnostics.push(diagnostic.clone());
            has_error = true;
          }
        }
      }
    }
    !has_error
  }

  /// Current byte offset in the source stream.
  pub(in crate::syntax::parse) fn offset(&self) -> usize {
    self.lex_ctx.offset()
  }

  /// Push a diagnostic.
  pub(in crate::syntax::parse) fn emit_diagnostic(&mut self, diagnostic: Diagnostic) {
    self.diagnostics.push(diagnostic);
  }

  /// Emit a GreenNode
  pub(in crate::syntax::parse) fn emit(
    &mut self,
    kind: SyntaxKind,
    children: &[GreenNode],
  ) -> GreenNode {
    GreenNode::from_node(self.cache.borrow_mut().node(kind, children))
  }
}
