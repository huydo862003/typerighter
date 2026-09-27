use std::{
  collections::VecDeque,
  ops::{Deref, DerefMut},
};

use crate::syntax::syntax_kind::SyntaxKind;
use typedown_types::stream::Utf8Stream;

use crate::syntax::{
  lex::ctx::{LexCtx, LexMode, LexResult},
  parse::constants::{SKIP_COMMENT, SKIP_INDENT, SKIP_NEWLINE, SKIP_WS},
};

pub struct YamlLexResult {
  result: LexResult,
  /// Total text length of all tokens since the last newline (column offset).
  pub token_indent: usize,
  /// The text length of the most recent YamlIndent token (the line's leading indentation).
  pub block_indent: usize,
}

impl YamlLexResult {
  pub(in crate::syntax::parse) fn new(
    result: LexResult,
    token_indent: usize,
    block_indent: usize,
  ) -> Self {
    Self {
      result,
      token_indent,
      block_indent,
    }
  }
}

impl std::ops::Deref for YamlLexResult {
  type Target = LexResult;

  fn deref(&self) -> &Self::Target {
    &self.result
  }
}

impl From<YamlLexResult> for LexResult {
  fn from(val: YamlLexResult) -> Self {
    val.result
  }
}

pub struct MdLexResult {
  result: LexResult,
}

impl MdLexResult {
  pub(in crate::syntax::parse) fn new(result: LexResult) -> Self {
    Self { result }
  }
}

impl std::ops::Deref for MdLexResult {
  type Target = LexResult;

  fn deref(&self) -> &Self::Target {
    &self.result
  }
}

impl From<MdLexResult> for LexResult {
  fn from(val: MdLexResult) -> Self {
    val.result
  }
}

/// A lex context that allows peeking
/// WARNING: By rewinding, it means that the extracted token can be pushed back to the token stream
/// You CANNOT push back the token's characters, switch mode, re-lex then return a new token
pub struct PeekableLexCtx<S: Utf8Stream> {
  pub(in crate::syntax::parse) lex_ctx: LexCtx<S>,
  token_buffer: VecDeque<LexResult>,
  /// Whether the last non-whitespace token was a Newline (or we're at start of input).
  after_newline: bool,
  /// Total text length of all tokens since the last newline (column offset).
  token_indent: usize,
  /// The text length of the most recent YamlIndent token (the line's leading indentation).
  block_indent: usize,
}

impl<S: Utf8Stream> PeekableLexCtx<S> {
  pub fn new(lex_ctx: LexCtx<S>) -> PeekableLexCtx<S> {
    PeekableLexCtx {
      lex_ctx,
      token_buffer: VecDeque::default(),
      after_newline: true,
      token_indent: 0,
      block_indent: 0,
    }
  }

  pub fn token_indent(&self) -> usize {
    self.token_indent
  }

  pub fn block_indent(&self) -> usize {
    self.block_indent
  }

  pub fn lex(&mut self) -> LexResult {
    let front_token = self.token_buffer.pop_front();
    let result = match front_token {
      Some(token) => token,
      None => self.lex_ctx.lex(),
    };

    match result.token.kind() {
      SyntaxKind::Newline => {
        self.after_newline = true;
        self.token_indent = 0;
        self.block_indent = 0;
      }
      SyntaxKind::YamlIndent => {
        self.block_indent = result.token.chars().count();
        self.token_indent += self.block_indent;
      }
      SyntaxKind::Whitespace => {
        self.token_indent += result.token.chars().count();
      }
      _ => {
        self.token_indent += result.token.chars().count();
        self.after_newline = false;
      }
    }

    result
  }

  pub fn peek(&mut self, skip: usize, mode: LexMode) -> LexResult {
    debug_assert!(
      self.lex_ctx.mode() == mode,
      "[PeekableLexCtx::peek] Lex mode must be the same as the `mode` argument"
    );
    match mode {
      LexMode::YamlFrontmatter => self.peek_yaml(skip).into(),
      LexMode::MarkdownBody => self.peek_md(skip).into(),
    }
  }

  /// Peek at the next non-skipped YAML token without consuming.
  pub fn peek_yaml(&mut self, skip: usize) -> YamlLexResult {
    self.peek_yaml_nth(0, skip)
  }

  /// Peek at the nth non-skipped YAML token without consuming (0-indexed).
  pub fn peek_yaml_nth(&mut self, nth: usize, skip: usize) -> YamlLexResult {
    debug_assert!(
      self.lex_ctx.mode() == LexMode::YamlFrontmatter,
      "[PeekableLexCtx::peek_yaml_nth] Lex mode must be YamlFrontmatter"
    );

    let saved_after_newline = self.after_newline;
    let saved_token_indent = self.token_indent;
    let saved_block_indent = self.block_indent;
    let mut prefetch: VecDeque<LexResult> = VecDeque::new();
    let mut found = 0;

    let result = loop {
      let lex_result = self.lex();
      let token = lex_result.token.clone();
      let diagnostic = lex_result.diagnostic.clone();
      prefetch.push_back(lex_result);

      if token.kind() == SyntaxKind::Eof {
        break LexResult { token, diagnostic };
      }
      if token.kind() == SyntaxKind::YamlOp && token.chars().collect::<String>() == "---" {
        break LexResult { token, diagnostic };
      }

      if !self.should_skip(token.kind(), skip) {
        if found == nth {
          break LexResult { token, diagnostic };
        }
        found += 1;
      }
    };

    for token in prefetch.into_iter().rev() {
      self.token_buffer.push_front(token);
    }

    let peeked_token_indent = self.token_indent;
    let peeked_block_indent = self.block_indent;
    self.after_newline = saved_after_newline;
    self.token_indent = saved_token_indent;
    self.block_indent = saved_block_indent;

    YamlLexResult::new(result, peeked_token_indent, peeked_block_indent)
  }

  /// Peek at the next non-skipped Markdown token without consuming.
  pub fn peek_md(&mut self, skip: usize) -> MdLexResult {
    self.peek_md_nth(0, skip)
  }

  /// Peek at the nth non-skipped Markdown token without consuming.
  pub fn peek_md_nth(&mut self, nth: usize, skip: usize) -> MdLexResult {
    debug_assert!(
      self.lex_ctx.mode() == LexMode::MarkdownBody,
      "[PeekableLexCtx::peek_md_nth] Lex mode must be MarkdownBody"
    );
    let saved_after_newline = self.after_newline;
    let mut prefetch: VecDeque<LexResult> = VecDeque::new();
    let mut found = 0;

    let result = loop {
      let lex_result = self.lex();
      let token = lex_result.token.clone();
      let diagnostic = lex_result.diagnostic.clone();
      prefetch.push_back(lex_result);

      if token.kind() == SyntaxKind::Eof {
        break LexResult { token, diagnostic };
      }

      if !self.should_skip(token.kind(), skip) {
        if found == nth {
          break LexResult { token, diagnostic };
        }
        found += 1;
      }
    };

    for token in prefetch.into_iter().rev() {
      self.token_buffer.push_front(token);
    }

    // Restore state to pre-peek
    self.after_newline = saved_after_newline;

    MdLexResult::new(result)
  }

  /// Whether the most recently lexed token should be skipped given the skip flags.
  pub fn should_skip(&self, kind: SyntaxKind, skip: usize) -> bool {
    match kind {
      SyntaxKind::Whitespace => skip & SKIP_WS != 0,
      SyntaxKind::YamlComment => skip & SKIP_COMMENT != 0,
      SyntaxKind::Newline => skip & SKIP_NEWLINE != 0,
      SyntaxKind::YamlIndent => skip & SKIP_INDENT != 0,
      _ => false,
    }
  }
}

impl<S: Utf8Stream> Deref for PeekableLexCtx<S> {
  type Target = LexCtx<S>;

  fn deref(&self) -> &Self::Target {
    &self.lex_ctx
  }
}

impl<S: Utf8Stream> DerefMut for PeekableLexCtx<S> {
  fn deref_mut(&mut self) -> &mut Self::Target {
    &mut self.lex_ctx
  }
}
