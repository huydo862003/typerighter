use crate::syntax::diagnostic::Diagnostic;
use typedown_types::stream::{Utf8Result, Utf8Stream};

use super::ctx::{InterpContext, LexCtx, LexResult};
use super::yaml::is_op_char;
use crate::syntax::green::token::SyntaxToken;
use crate::syntax::syntax_kind::SyntaxKind;

// Markdown body lexing
impl<S: Utf8Stream> LexCtx<S> {
  pub(in crate::syntax::lex) fn lex_markdown_body(&mut self) -> LexResult {
    // If inside a formula/string context, dispatch accordingly
    match self.markdown_lex_ctx.interp_stack.last() {
      Some(InterpContext::Interpolation) | Some(InterpContext::Brace) => {
        return self.lex_markdown_formula();
      }
      Some(InterpContext::DqString) | Some(InterpContext::SqString) => {
        return self.lex_markdown_resume_string();
      }
      // MdDqString/MdSqString: content lexes as normal markdown tokens, fall through
      Some(InterpContext::MdDqString) | Some(InterpContext::MdSqString) | None => {}
    }

    let char = match self.peek() {
      Utf8Result::Char(char) => char,
      _ => panic!(
        "[LexCtx::lex_markdown_body] Expected a valid UTF-8 character but got EOF or invalid bytes."
      ),
    };

    match char {
      /* Newlines */
      '\n' | '\r' => {
        self.advance_avoid_invalid_utf8();
        if char == '\r' {
          self.consume_avoid_invalid_utf8('\n');
        }
        self.emit(SyntaxKind::Newline)
      }

      /* Whitespace */
      _ if char.is_whitespace() => {
        self.advance_avoid_invalid_utf8();
        self.emit(SyntaxKind::Whitespace)
      }

      /* Dollar */
      '$' => self.lex_markdown_dollar(),

      /* Brackets and parens and braces */
      '[' => {
        self.advance_avoid_invalid_utf8();
        self.emit(SyntaxKind::LBracket)
      }
      ']' => {
        self.advance_avoid_invalid_utf8();
        self.emit(SyntaxKind::RBracket)
      }
      '(' => {
        self.advance_avoid_invalid_utf8();
        self.emit(SyntaxKind::LParen)
      }
      ')' => {
        self.advance_avoid_invalid_utf8();
        self.emit(SyntaxKind::RParen)
      }
      '{' => {
        self.advance_avoid_invalid_utf8();
        self.emit(SyntaxKind::LBrace)
      }
      '}' => {
        self.advance_avoid_invalid_utf8();
        self.emit(SyntaxKind::RBrace)
      }

      /* Code spans */
      '`' => self.lex_markdown_code(),

      /* Numbers */
      '0'..='9' => self.lex_markdown_number(),

      /* String delimiters (no interpolation in markdown) */
      '"' => {
        self.advance_avoid_invalid_utf8();
        match self.markdown_lex_ctx.interp_stack.last() {
          Some(InterpContext::MdDqString) => {
            self.markdown_lex_ctx.interp_stack.pop();
            self.emit(SyntaxKind::DqStrEnd)
          }
          _ => {
            self
              .markdown_lex_ctx
              .interp_stack
              .push(InterpContext::MdDqString);
            self.emit(SyntaxKind::DqStrStart)
          }
        }
      }
      '\'' => {
        self.advance_avoid_invalid_utf8();
        match self.markdown_lex_ctx.interp_stack.last() {
          Some(InterpContext::MdSqString) => {
            self.markdown_lex_ctx.interp_stack.pop();
            self.emit(SyntaxKind::SqStrEnd)
          }
          _ => {
            self
              .markdown_lex_ctx
              .interp_stack
              .push(InterpContext::MdSqString);
            self.emit(SyntaxKind::SqStrStart)
          }
        }
      }

      /* HTML entities */
      '&' => self.lex_markdown_html_entity(),

      /* Symbols */
      _ if is_md_symbol_char(char) => self.lex_markdown_symbol(),

      /* Everything else is text */
      _ => self.lex_markdown_text(),
    }
  }

  /* Dollar */

  pub(in crate::syntax::lex) fn lex_markdown_dollar(&mut self) -> LexResult {
    // Count opening $ delimiters
    let mut fence_count = 0;
    while let Utf8Result::Char('$') = self.peek() {
      self.advance_avoid_invalid_utf8();
      fence_count += 1;
    }

    // Check for ${ (formula mode, only when exactly one $)
    if fence_count == 1 && self.consume_avoid_invalid_utf8('{') {
      self
        .markdown_lex_ctx
        .interp_stack
        .push(InterpContext::Interpolation);
      return self.emit(SyntaxKind::InterpStart);
    }

    // Check if content starts with a newline (block) or not (inline)
    let is_block = matches!(self.peek(), Utf8Result::Char('\n') | Utf8Result::Char('\r'));

    // Consume math content until matching fence count
    loop {
      match self.peek() {
        Utf8Result::Char('$') => {
          let mut count = 0;
          while let Utf8Result::Char('$') = self.peek() {
            self.advance_avoid_invalid_utf8();
            count += 1;
            if count == fence_count {
              break;
            }
          }
          if count == fence_count {
            let kind = if is_block {
              SyntaxKind::MathBlock
            } else {
              SyntaxKind::InlineMath
            };
            return self.emit(kind);
          }
          // Not enough $, they're part of the content
        }
        Utf8Result::Eof => {
          let start = self.stream.offset() - self.text_buffer.len();
          let end = self.stream.offset();
          return self.emit_with(
            SyntaxKind::Error,
            Diagnostic::UnterminatedMathBlock {
              start_offset: start,
              end_offset: end,
            },
          );
        }
        Utf8Result::Char('\n') | Utf8Result::Char('\r') => {
          let start = self.stream.offset() - self.text_buffer.len();
          let end = self.stream.offset();
          if !is_block {
            return self.emit_with(
              SyntaxKind::Error,
              Diagnostic::UnterminatedMathBlock {
                start_offset: start,
                end_offset: end,
              },
            );
          }
          self.advance_avoid_invalid_utf8();
        }
        _ => {
          self.advance_avoid_invalid_utf8();
        }
      }
    }
  }

  /* HTML entities */

  pub(in crate::syntax::lex) fn lex_markdown_html_entity(&mut self) -> LexResult {
    // Consume `&`
    self.advance_avoid_invalid_utf8();

    match self.peek() {
      // Numeric entity: &#digits; or &#xhex;
      Utf8Result::Char('#') => {
        self.advance_avoid_invalid_utf8();
        match self.peek() {
          Utf8Result::Char('x') | Utf8Result::Char('X') => {
            self.advance_avoid_invalid_utf8();
            let mut count = 0;
            while let Utf8Result::Char(ch) = self.peek() {
              if ch.is_ascii_hexdigit() {
                self.advance_avoid_invalid_utf8();
                count += 1;
              } else {
                break;
              }
            }
            if count > 0
              && let Utf8Result::Char(';') = self.peek()
            {
              self.advance_avoid_invalid_utf8();
              return self.emit(SyntaxKind::MdHtmlEntity);
            }
          }
          _ => {
            let mut count = 0;
            while let Utf8Result::Char(ch) = self.peek() {
              if ch.is_ascii_digit() {
                self.advance_avoid_invalid_utf8();
                count += 1;
              } else {
                break;
              }
            }
            if count > 0
              && let Utf8Result::Char(';') = self.peek()
            {
              self.advance_avoid_invalid_utf8();
              return self.emit(SyntaxKind::MdHtmlEntity);
            }
          }
        }
      }
      // Named entity: &name;
      // Advance name chars directly into a local buffer (not text_buffer) so that on
      // failure we can emit just `&` as MdSymbol and queue the name chars as a separate Ident.
      Utf8Result::Char(ch) if ch.is_ascii_alphabetic() => {
        let mut name = String::new();
        loop {
          match self.peek() {
            Utf8Result::Char(ch) if ch.is_ascii_alphanumeric() => {
              self.stream.advance();
              name.push(ch);
            }
            _ => break,
          }
        }
        if let Utf8Result::Char(';') = self.peek() {
          // Valid named entity: commit name + `;` into text_buffer and emit
          self.text_buffer.push_str(&name);
          self.advance_avoid_invalid_utf8(); // consume `;`
          return self.emit(SyntaxKind::MdHtmlEntity);
        }
        // Invalid entity (no `;`): emit `&` as MdSymbol, queue name as Ident
        let ampersand_token = self.emit(SyntaxKind::MdSymbol);
        if !name.is_empty() {
          let ident_token = SyntaxToken::new(
            &mut self.cache.borrow_mut(),
            SyntaxKind::Ident,
            name.as_bytes(),
          );
          self.pending_tokens.push(LexResult {
            token: ident_token,
            diagnostic: None,
          });
        }
        return ampersand_token;
      }
      _ => {}
    }

    // Not a valid entity: emit `&` as MdSymbol
    self.emit(SyntaxKind::MdSymbol)
  }

  /* Text */

  pub(in crate::syntax::lex) fn lex_markdown_text(&mut self) -> LexResult {
    loop {
      match self.peek() {
        Utf8Result::Char(char)
          if !char.is_whitespace()
            && char != '$'
            && char != '['
            && char != ']'
            && char != '('
            && char != ')'
            && char != '{'
            && char != '}'
            && char != '`'
            && char != '"'
            && char != '\''
            && !is_md_symbol_char(char)
            && !char.is_ascii_digit() =>
        {
          self.advance_avoid_invalid_utf8();
        }
        _ => break,
      }
    }
    self.emit(SyntaxKind::Ident)
  }

  /* Symbols */

  pub(in crate::syntax::lex) fn lex_markdown_symbol(&mut self) -> LexResult {
    let first = match self.peek() {
      Utf8Result::Char(char) => char,
      _ => unreachable!(),
    };
    self.advance_avoid_invalid_utf8();
    // Consume consecutive runs of the same symbol character
    while let Utf8Result::Char(char) = self.peek() {
      if char == first {
        self.advance_avoid_invalid_utf8();
      } else {
        break;
      }
    }
    self.emit(SyntaxKind::MdSymbol)
  }

  /* Code spans */

  pub(in crate::syntax::lex) fn lex_markdown_code(&mut self) -> LexResult {
    // Count opening backticks
    let mut fence_count = 0;
    while let Utf8Result::Char('`') = self.peek() {
      self.advance_avoid_invalid_utf8();
      fence_count += 1;
    }

    // Consume language tag (alphanumeric chars, underscore, hyphen)
    while let Utf8Result::Char(c) = self.peek()
      && (c.is_alphanumeric() || c == '_' || c == '-')
    {
      self.advance_avoid_invalid_utf8();
    }

    // Consume optional line range indicator like {1,3,5-8}
    let mut range_diagnostic: Option<Diagnostic> = None;
    if let Utf8Result::Char('{') = self.peek() {
      let range_start = self.stream.offset();
      self.advance_avoid_invalid_utf8();
      let mut closed = false;

      loop {
        match self.peek() {
          Utf8Result::Char('}') => {
            self.advance_avoid_invalid_utf8();
            closed = true;
            break;
          }
          Utf8Result::Char(c) if c.is_ascii_digit() || c == ',' || c == '-' || c == ' ' => {
            self.advance_avoid_invalid_utf8();
          }
          Utf8Result::Char('\n') | Utf8Result::Char('\r') | Utf8Result::Eof => {
            break;
          }
          _ => {
            // Invalid character in range indicator
            let offset = self.stream.offset();
            range_diagnostic = Some(Diagnostic::InvalidCodeRangeIndicator {
              start_offset: range_start,
              end_offset: offset,
            });
            self.advance_avoid_invalid_utf8();
          }
        }
      }

      if !closed {
        range_diagnostic = Some(Diagnostic::InvalidCodeRangeIndicator {
          start_offset: range_start,
          end_offset: self.stream.offset(),
        });
      }
    }

    // Consume trailing whitespace before newline
    while let Utf8Result::Char(c) = self.peek()
      && c.is_whitespace()
      && c != '\n'
      && c != '\r'
    {
      self.advance_avoid_invalid_utf8();
    }

    // Check if content starts with a newline (block) or not (inline)
    let is_block = matches!(self.peek(), Utf8Result::Char('\n') | Utf8Result::Char('\r'));

    // Consume content until matching fence count
    loop {
      match self.peek() {
        Utf8Result::Char('`') => {
          let mut count = 0;
          while let Utf8Result::Char('`') = self.peek() {
            self.advance_avoid_invalid_utf8();
            count += 1;
            if count == fence_count {
              break;
            }
          }
          if count == fence_count {
            let kind = if is_block {
              SyntaxKind::CodeBlock
            } else {
              SyntaxKind::InlineCode
            };
            return match range_diagnostic {
              Some(diag) => self.emit_with(kind, diag),
              None => self.emit(kind),
            };
          }
        }
        Utf8Result::Eof => {
          let start = self.stream.offset() - self.text_buffer.len();
          let end = self.stream.offset();
          return self.emit_with(
            SyntaxKind::Error,
            Diagnostic::UnterminatedCodeBlock {
              start_offset: start,
              end_offset: end,
            },
          );
        }
        Utf8Result::Char('\n') | Utf8Result::Char('\r') => {
          let start = self.stream.offset() - self.text_buffer.len();
          let end = self.stream.offset();
          if !is_block {
            return self.emit_with(
              SyntaxKind::Error,
              Diagnostic::UnterminatedCodeBlock {
                start_offset: start,
                end_offset: end,
              },
            );
          }
          self.advance_avoid_invalid_utf8();
        }
        _ => {
          self.advance_avoid_invalid_utf8();
        }
      }
    }
  }

  /* Numbers */

  pub(in crate::syntax::lex) fn lex_markdown_number(&mut self) -> LexResult {
    loop {
      match self.peek() {
        Utf8Result::Char(char) if char.is_ascii_digit() => {
          self.advance_avoid_invalid_utf8();
        }
        _ => break,
      }
    }
    self.emit(SyntaxKind::MdNumber)
  }

  /* Formula mode (inside ${...} in markdown) */

  pub(in crate::syntax::lex) fn lex_markdown_formula(&mut self) -> LexResult {
    if let Utf8Result::Eof = self.peek() {
      self.markdown_lex_ctx.interp_stack.pop();
      let offset = self.stream.offset();
      return self.emit_with(
        SyntaxKind::Error,
        Diagnostic::UnterminatedInterpolation {
          start_offset: offset,
          end_offset: offset,
        },
      );
    }

    let char = match self.peek() {
      Utf8Result::Char(char) => char,
      _ => unreachable!(),
    };

    match char {
      '}' => {
        self.advance_avoid_invalid_utf8();
        match self.markdown_lex_ctx.interp_stack.last() {
          Some(InterpContext::Brace) => {
            self.markdown_lex_ctx.interp_stack.pop();
            self.emit(SyntaxKind::RBrace)
          }
          Some(InterpContext::Interpolation) => {
            self.markdown_lex_ctx.interp_stack.pop();
            self.emit(SyntaxKind::InterpEnd)
          }
          _ => self.emit(SyntaxKind::RBrace),
        }
      }
      '{' => {
        self.advance_avoid_invalid_utf8();
        self
          .markdown_lex_ctx
          .interp_stack
          .push(InterpContext::Brace);
        self.emit(SyntaxKind::LBrace)
      }
      '"' => {
        self.advance_avoid_invalid_utf8();
        self
          .markdown_lex_ctx
          .interp_stack
          .push(InterpContext::DqString);
        self.emit(SyntaxKind::DqStrStart)
      }
      '\'' => {
        self.advance_avoid_invalid_utf8();
        self
          .markdown_lex_ctx
          .interp_stack
          .push(InterpContext::SqString);
        self.emit(SyntaxKind::SqStrStart)
      }
      '\n' | '\r' => {
        self.advance_avoid_invalid_utf8();
        if char == '\r' {
          self.consume_avoid_invalid_utf8('\n');
        }
        self.emit(SyntaxKind::Newline)
      }
      _ if char.is_whitespace() => {
        self.advance_avoid_invalid_utf8();
        self.emit(SyntaxKind::Whitespace)
      }
      '(' => {
        self.advance_avoid_invalid_utf8();
        self.emit(SyntaxKind::LParen)
      }
      ')' => {
        self.advance_avoid_invalid_utf8();
        self.emit(SyntaxKind::RParen)
      }
      '[' => {
        self.advance_avoid_invalid_utf8();
        self.emit(SyntaxKind::LBracket)
      }
      ']' => {
        self.advance_avoid_invalid_utf8();
        self.emit(SyntaxKind::RBracket)
      }
      ',' => {
        self.advance_avoid_invalid_utf8();
        self.emit(SyntaxKind::Comma)
      }
      '#' => self.lex_yaml_comment(),
      ':' => {
        self.advance_avoid_invalid_utf8();
        self.emit(SyntaxKind::Colon)
      }
      '0'..='9' => self.lex_number(),
      _ if char.is_alphabetic() || char == '_' => self.lex_yaml_ident(),
      _ if is_op_char(char) => self.lex_yaml_op(),
      _ => {
        self.advance_avoid_invalid_utf8();
        self.emit(SyntaxKind::Error)
      }
    }
  }

  pub(in crate::syntax::lex) fn lex_markdown_resume_string(&mut self) -> LexResult {
    match self.markdown_lex_ctx.interp_stack.last() {
      Some(InterpContext::DqString) => {
        self.lex_markdown_string_content('"', SyntaxKind::DqStrContent, SyntaxKind::DqStrEnd)
      }
      Some(InterpContext::SqString) => {
        self.lex_markdown_string_content('\'', SyntaxKind::SqStrContent, SyntaxKind::SqStrEnd)
      }
      _ => panic!(
        "[LexCtx::lex_markdown_resume_string] Called without a string context on the interp stack"
      ),
    }
  }

  pub(in crate::syntax::lex) fn lex_markdown_string_content(
    &mut self,
    closing: char,
    content_kind: SyntaxKind,
    end_kind: SyntaxKind,
  ) -> LexResult {
    loop {
      match self.peek() {
        Utf8Result::Char(char) if char == closing => {
          if self.text_buffer.is_empty() {
            self.advance_avoid_invalid_utf8();
            self.markdown_lex_ctx.interp_stack.pop();
            return self.emit(end_kind);
          } else {
            let content = self.emit(content_kind);
            self.advance_avoid_invalid_utf8();
            self.markdown_lex_ctx.interp_stack.pop();
            let end = self.emit(end_kind);
            self.pending_tokens.push(end);
            return content;
          }
        }
        Utf8Result::Char('$') => {
          self.advance_avoid_invalid_utf8();
          match self.peek() {
            Utf8Result::Char('{') => {
              self.advance_avoid_invalid_utf8();
              let buf_len = self.text_buffer.len();
              let string_text: String = self.text_buffer.drain(..buf_len - 2).collect();
              self.text_buffer.clear();

              self
                .markdown_lex_ctx
                .interp_stack
                .push(InterpContext::Interpolation);

              let interp_start = LexResult {
                token: self
                  .cache
                  .borrow_mut()
                  .token(SyntaxKind::InterpStart, "${".as_bytes()),
                diagnostic: None,
              };

              if !string_text.is_empty() {
                let content = LexResult {
                  token: self
                    .cache
                    .borrow_mut()
                    .token(content_kind, string_text.as_bytes()),
                  diagnostic: None,
                };
                self.pending_tokens.push(interp_start);
                return content;
              } else {
                return interp_start;
              }
            }
            _ => {
              // Single $ inside string: inline math
              let buf_len = self.text_buffer.len();
              let string_text: String = self.text_buffer.drain(..buf_len - 1).collect();
              self.text_buffer.clear();

              let math_token = self.lex_inline_math_content();

              if !string_text.is_empty() {
                let content = LexResult {
                  token: self
                    .cache
                    .borrow_mut()
                    .token(content_kind, string_text.as_bytes()),
                  diagnostic: None,
                };
                self.pending_tokens.push(math_token);
                return content;
              } else {
                return math_token;
              }
            }
          }
        }
        Utf8Result::Char('\\') => {
          self.advance_avoid_invalid_utf8();
          match self.peek() {
            Utf8Result::Char('\n') | Utf8Result::Char('\r') | Utf8Result::Eof => {
              let start = self.stream.offset() - self.text_buffer.len();
              let end = self.stream.offset();
              self.markdown_lex_ctx.interp_stack.pop();
              return self.emit_with(
                SyntaxKind::Error,
                Diagnostic::UnterminatedString {
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
        Utf8Result::Char('\n') | Utf8Result::Char('\r') | Utf8Result::Eof => {
          let start = self.stream.offset() - self.text_buffer.len();
          let end = self.stream.offset();
          self.markdown_lex_ctx.interp_stack.pop();
          return self.emit_with(
            SyntaxKind::Error,
            Diagnostic::UnterminatedString {
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
}

fn is_md_symbol_char(char: char) -> bool {
  matches!(
    char,
    '#'
      | '!'
      | '*'
      | '_'
      | '~'
      | '^'
      | '-'
      | '>'
      | '<'
      | '|'
      | '@'
      | ':'
      | '&'
      | '\\'
      | '/'
      | '='
      | '+'
      | '%'
      | '.'
  )
}
