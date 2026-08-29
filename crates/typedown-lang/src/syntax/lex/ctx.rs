//! An on-demand lexer supporting 2 lex modes (YAML frontmatter and Markdown).
//!
//! The parser triggers lex and switches mode where sensible,
//! producing one green token at a time.

use std::cell::RefCell;
use std::rc::Rc;

use crate::syntax::diagnostic::Diagnostic;
use typedown_types::stream::{Utf8Result, Utf8Stream};

use crate::syntax::green::cache::Cache;
use crate::syntax::green::token::SyntaxToken;
use crate::syntax::syntax_kind::SyntaxKind;

pub struct LexResult {
  pub token: SyntaxToken,
  pub diagnostic: Option<Diagnostic>,
}

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum LexMode {
  YamlFrontmatter,
  MarkdownBody,
}

/// An on-demand lexer supporting 2 lex modes
pub struct LexCtx<S: Utf8Stream> {
  pub(super) stream: S,
  pub(super) cache: Rc<RefCell<Cache>>,
  // Current mode
  mode: LexMode,
  // Text buffer to accumulate the read utf-8
  pub(super) text_buffer: String,
  // Pending tokens to emit before lexing more input
  pub(super) pending_tokens: Vec<LexResult>,

  // State for YAML lexing
  pub(super) yaml_lex_ctx: YamlLexCtx,
  // State for Markdown lexing
  pub(super) markdown_lex_ctx: MarkdownLexCtx,
}

/* YAML state */
pub(super) struct YamlLexCtx {
  // Whether we're just after a newline (linux), CRLF (Windows), carriage return (Mac)
  pub(super) at_line_start: bool,
  // We allow nested interpolations
  // We need to distinguish between nested strings, interpolations, etc
  pub(super) interp_stack: Vec<InterpContext>,
  // The indent character established by the first indented line (None = not yet determined)
  pub(super) indent_char: Option<char>,
}

pub(super) enum InterpContext {
  // Inside ${...}
  Interpolation,
  // Inside a nested {...} within an interpolation
  Brace,
  // Inside '...' in formula/YAML mode
  SqString,
  // Inside "..." in formula/YAML mode
  DqString,
  // Inside "..." in markdown body (no interpolation, content lexed as normal markdown tokens)
  MdDqString,
  // Inside '...' in markdown body (no interpolation, content lexed as normal markdown tokens)
  MdSqString,
}

/* Markdown state */
pub(super) struct MarkdownLexCtx {
  // Context stack for formula mode interpolation inside markdown
  pub(super) interp_stack: Vec<InterpContext>,
}

impl<S: Utf8Stream> LexCtx<S> {
  pub fn new(stream: S, cache: Rc<RefCell<Cache>>) -> Self {
    Self {
      stream,
      cache,
      mode: LexMode::YamlFrontmatter,
      text_buffer: String::from(""),
      pending_tokens: Vec::new(),
      yaml_lex_ctx: YamlLexCtx {
        at_line_start: true,
        interp_stack: vec![],
        indent_char: None,
      },
      markdown_lex_ctx: MarkdownLexCtx {
        interp_stack: vec![],
      },
    }
  }

  pub fn set_mode(&mut self, mode: LexMode) {
    debug_assert!(
      self.pending_tokens.is_empty(),
      "[LexCtx::set_mode] Cannot switch mode while pending tokens are queued"
    );
    self.mode = mode;
  }

  pub fn mode(&self) -> LexMode {
    self.mode
  }

  /// Check if the stream starts with --- at byte 0
  pub fn has_frontmatter_start(&mut self) -> bool {
    use crate::syntax::lex::yaml::is_op_char;
    matches!(self.stream.peek_nth(0), Utf8Result::Char('-'))
      && matches!(self.stream.peek_nth(1), Utf8Result::Char('-'))
      && matches!(self.stream.peek_nth(2), Utf8Result::Char('-'))
      && !matches!(self.stream.peek_nth(3), Utf8Result::Char(c) if is_op_char(c))
  }

  /// Current byte offset in the source stream.
  pub fn offset(&self) -> usize {
    self.stream.offset()
  }

  pub fn lex(&mut self) -> LexResult {
    // Drain pending tokens first (FIFO)
    if !self.pending_tokens.is_empty() {
      return self.pending_tokens.remove(0);
    }

    if self.is_eof() {
      self.emit(SyntaxKind::Eof)
    } else {
      let maybe_invalid_utf8 = self.try_consume_invalid_utf8();
      if let Some(result) = maybe_invalid_utf8 {
        return result;
      }
      match self.mode {
        LexMode::YamlFrontmatter => self.lex_yaml_frontmatter(),
        LexMode::MarkdownBody => self.lex_markdown_body(),
      }
    }
  }
}

/* Shared helpers */
impl<S: Utf8Stream> LexCtx<S> {
  /// Look at the next character without consuming it.
  pub(super) fn peek(&mut self) -> Utf8Result {
    self.stream.peek()
  }

  /// Consume the next character, appending it to the current token text.
  /// If invalid UTF-8 is encountered, do not consume it and return the result.
  pub(super) fn advance_avoid_invalid_utf8(&mut self) -> Utf8Result {
    match self.stream.peek() {
      Utf8Result::Char(_) => {
        let result = self.stream.advance();
        if let Utf8Result::Char(char) = result {
          self.text_buffer.push(char);
        }
        result
      }
      other => other,
    }
  }

  /// Consume the next character if it matches `expected`.
  pub(super) fn consume_avoid_invalid_utf8(&mut self, expected: char) -> bool {
    if let Utf8Result::Char(encountered) = self.peek()
      && encountered == expected
    {
      self.advance_avoid_invalid_utf8();
      true
    } else {
      false
    }
  }

  /// Look for an invalid utf-8 character right ahead and return if any.
  /// INVARIANT: Always call before any other advance()/consume().
  pub(super) fn try_consume_invalid_utf8(&mut self) -> Option<LexResult> {
    debug_assert!(
      self.text_buffer.is_empty(),
      "Do not call advance()/consume() before try_consume_invalid_utf8()"
    );
    if let Utf8Result::Invalid { len, bytes } = self.peek() {
      Some(LexResult {
        token: self
          .cache
          .borrow_mut()
          .token(SyntaxKind::Error, &bytes[..len]),
        diagnostic: Some(Diagnostic::InvalidUtf8 {
          start_offset: self.stream.offset() - len,
          end_offset: self.stream.offset(),
        }),
      })
    } else {
      None
    }
  }

  /// Finalize the current token with no diagnostic.
  pub(super) fn emit(&mut self, kind: SyntaxKind) -> LexResult {
    let text = std::mem::take(&mut self.text_buffer);
    LexResult {
      token: self.cache.borrow_mut().token(kind, text.as_bytes()),
      diagnostic: None,
    }
  }

  /// Finalize the current token with a diagnostic.
  pub(super) fn emit_with(&mut self, kind: SyntaxKind, diagnostic: Diagnostic) -> LexResult {
    let text = std::mem::take(&mut self.text_buffer);
    LexResult {
      token: self.cache.borrow_mut().token(kind, text.as_bytes()),
      diagnostic: Some(diagnostic),
    }
  }

  // Number lexer: integer, decimal, scientific notation
  pub(super) fn lex_number(&mut self) -> LexResult {
    // Integer part
    loop {
      match self.peek() {
        Utf8Result::Char(char) if char.is_ascii_digit() => {
          self.advance_avoid_invalid_utf8();
        }
        _ => break,
      }
    }
    // Decimal part
    if let Utf8Result::Char('.') = self.peek() {
      self.advance_avoid_invalid_utf8();
      loop {
        match self.peek() {
          Utf8Result::Char(char) if char.is_ascii_digit() => {
            self.advance_avoid_invalid_utf8();
          }
          _ => break,
        }
      }
    }
    // Scientific notation
    if let Utf8Result::Char('e' | 'E') = self.peek() {
      self.advance_avoid_invalid_utf8();
      if let Utf8Result::Char('+' | '-') = self.peek() {
        self.advance_avoid_invalid_utf8();
      }
      let has_digits = matches!(self.peek(), Utf8Result::Char(char) if char.is_ascii_digit());
      if !has_digits {
        let start = self.stream.offset() - self.text_buffer.len();
        let end = self.stream.offset();
        return self.emit_with(
          SyntaxKind::Error,
          Diagnostic::MissingExponentDigits {
            start_offset: start,
            end_offset: end,
          },
        );
      }
      loop {
        match self.peek() {
          Utf8Result::Char(char) if char.is_ascii_digit() => {
            self.advance_avoid_invalid_utf8();
          }
          _ => break,
        }
      }
    }
    self.emit(SyntaxKind::Number)
  }

  // Consume inline math content until closing $
  // INVARIANT: The opening $ must be already consumed and in the text buffer
  pub(super) fn lex_inline_math_content(&mut self) -> LexResult {
    // text_buffer already contains the opening $
    loop {
      match self.peek() {
        Utf8Result::Char('$') => {
          self.advance_avoid_invalid_utf8();
          return self.emit(SyntaxKind::InlineMath);
        }
        Utf8Result::Char('\n') | Utf8Result::Char('\r') | Utf8Result::Eof => {
          let start = self.stream.offset() - self.text_buffer.len();
          let end = self.stream.offset();
          return self.emit_with(
            SyntaxKind::Error,
            Diagnostic::UnterminatedInlineMath {
              start_offset: start,
              end_offset: end,
            },
          );
        }
        _ => {
          self.advance_avoid_invalid_utf8();
        }
      }
    }
  }

  /// Whether the stream is exhausted.
  fn is_eof(&mut self) -> bool {
    self.stream.exhausted()
  }
}
