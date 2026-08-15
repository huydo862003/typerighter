use crate::syntax::diagnostic::Diagnostic;
use typedown_types::stream::{Utf8Result, Utf8Stream};

use super::ctx::{InterpContext, LexCtx, LexResult};
use crate::syntax::syntax_kind::SyntaxKind;

// YAML frontmatter lexing
impl<S: Utf8Stream> LexCtx<S> {
  pub(in crate::syntax::lex) fn lex_yaml_frontmatter(&mut self) -> LexResult {
    // If inside an interpolation context, dispatch accordingly
    match self.yaml_lex_ctx.interp_stack.last() {
      Some(InterpContext::Interpolation) | Some(InterpContext::Brace) => {
        return self.lex_yaml_interpolation();
      }
      Some(InterpContext::DqString) | Some(InterpContext::SqString) => {
        return self.lex_yaml_resume_string();
      }
      Some(InterpContext::MdDqString) | Some(InterpContext::MdSqString) => {
        unreachable!(
          "[LexCtx::lex_yaml_frontmatter] MdDqString/MdSqString context cannot appear in YAML mode"
        )
      }
      None => {}
    }

    // At line start, handle indentation
    if self.yaml_lex_ctx.at_line_start
      && let Some(result) = self.lex_yaml_indent()
    {
      return result;
    }

    self.lex_yaml_token()
  }

  pub(in crate::syntax::lex) fn lex_yaml_token(&mut self) -> LexResult {
    let char = match self.peek() {
      Utf8Result::Char(char) => char,
      Utf8Result::Eof => return self.emit(SyntaxKind::Eof),
      Utf8Result::Invalid { .. } => {
        return self
          .try_consume_invalid_utf8()
          .unwrap_or_else(|| self.emit(SyntaxKind::Eof));
      }
    };

    match char {
      /* Newlines */
      '\n' | '\r' => {
        self.advance_avoid_invalid_utf8();
        if char == '\r' {
          self.consume_avoid_invalid_utf8('\n');
        }
        self.yaml_lex_ctx.at_line_start = true;
        self.emit(SyntaxKind::Newline)
      }

      /* Whitespace */
      _ if char.is_whitespace() => self.lex_yaml_whitespaces(),

      /* Comments */
      '#' => self.lex_yaml_comment(),

      /* Punctuation and delimiters */
      ':' => {
        self.advance_avoid_invalid_utf8();
        self.emit(SyntaxKind::Colon)
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
      '{' => {
        self.advance_avoid_invalid_utf8();
        if !self.yaml_lex_ctx.interp_stack.is_empty() {
          self.yaml_lex_ctx.interp_stack.push(InterpContext::Brace);
        }
        self.emit(SyntaxKind::LBrace)
      }
      '}' => {
        self.advance_avoid_invalid_utf8();
        match self.yaml_lex_ctx.interp_stack.last() {
          Some(InterpContext::Brace) => {
            self.yaml_lex_ctx.interp_stack.pop();
            self.emit(SyntaxKind::RBrace)
          }
          Some(InterpContext::Interpolation) => {
            self.yaml_lex_ctx.interp_stack.pop();
            self.emit(SyntaxKind::InterpEnd)
          }
          _ => self.emit(SyntaxKind::RBrace),
        }
      }
      ',' => {
        self.advance_avoid_invalid_utf8();
        self.emit(SyntaxKind::Comma)
      }

      /* Strings */
      '"' => self.lex_yaml_dq_string(),
      '\'' => self.lex_yaml_sq_string(),

      /* Numbers */
      '0'..='9' => self.lex_yaml_number(),

      /* Identifiers */
      _ if char.is_alphabetic() || char == '_' => self.lex_yaml_ident(),

      /* Operators */
      _ if is_op_char(char) => self.lex_yaml_op(),

      /* Error fallback */
      _ => {
        self.advance_avoid_invalid_utf8();
        self.emit(SyntaxKind::Error)
      }
    }
  }

  /* Indentation */

  pub(in crate::syntax::lex) fn lex_yaml_indent(&mut self) -> Option<LexResult> {
    self.yaml_lex_ctx.at_line_start = false;

    // Count leading whitespace and track tab/space usage
    let start = self.stream.offset();
    let mut indent = 0;
    let mut saw_space = false;
    let mut saw_tab = false;
    loop {
      match self.peek() {
        Utf8Result::Char(char) if char.is_whitespace() && char != '\n' && char != '\r' => {
          if char == '\t' {
            saw_tab = true;
          } else {
            saw_space = true;
          }
          indent += 1;
          self.advance_avoid_invalid_utf8();
        }
        _ => break,
      }
    }

    // Empty line: emit one Whitespace token per char, no indent
    if matches!(
      self.peek(),
      Utf8Result::Char('\n') | Utf8Result::Char('\r') | Utf8Result::Eof
    ) {
      if !self.text_buffer.is_empty() {
        let chars: Vec<char> = self.text_buffer.drain(..).collect();
        for char in &chars[1..] {
          self.pending_tokens.push(LexResult {
            token: self
              .cache
              .borrow_mut()
              .token(SyntaxKind::Whitespace, char.to_string().as_bytes()),
            diagnostic: None,
          });
        }
        return Some(LexResult {
          token: self
            .cache
            .borrow_mut()
            .token(SyntaxKind::Whitespace, chars[0].to_string().as_bytes()),
          diagnostic: None,
        });
      }
      return None;
    }

    // Detect mixed or inconsistent indentation
    let diagnostic = if indent > 0 {
      if saw_space && saw_tab {
        Some(Diagnostic::MixedIndentation {
          start_offset: start,
          end_offset: self.stream.offset(),
        })
      } else {
        let char = if saw_tab { '\t' } else { ' ' };
        match self.yaml_lex_ctx.indent_char {
          None => {
            self.yaml_lex_ctx.indent_char = Some(char);
            None
          }
          Some(established) if established != char => Some(Diagnostic::InconsistentIndentation {
            expected: established,
            encountered: char,
            start_offset: start,
            end_offset: self.stream.offset(),
          }),
          _ => None,
        }
      }
    } else {
      None
    };

    // Emit YamlIndent for any non-empty line (including 0-indented lines)
    if !self.text_buffer.is_empty() {
      return Some(match diagnostic {
        Some(diag) => self.emit_with(SyntaxKind::YamlIndent, diag),
        None => self.emit(SyntaxKind::YamlIndent),
      });
    }

    // Emit a zero-width YamlIndent for 0-indented non-empty lines
    Some(self.emit(SyntaxKind::YamlIndent))
  }

  /* Whitespace */

  pub(in crate::syntax::lex) fn lex_yaml_whitespaces(&mut self) -> LexResult {
    self.advance_avoid_invalid_utf8();
    self.emit(SyntaxKind::Whitespace)
  }

  /* Comments */

  pub(in crate::syntax::lex) fn lex_yaml_comment(&mut self) -> LexResult {
    self.advance_avoid_invalid_utf8(); // consume #
    loop {
      match self.peek() {
        Utf8Result::Char(char) if char != '\n' && char != '\r' => {
          self.advance_avoid_invalid_utf8();
        }
        _ => break,
      }
    }
    self.emit(SyntaxKind::YamlComment)
  }

  /* Operators */

  pub(in crate::syntax::lex) fn lex_yaml_op(&mut self) -> LexResult {
    // Check for `!` tag before consuming op chars:
    if let Utf8Result::Char('!') = self.peek() {
      self.advance_avoid_invalid_utf8();
      if let Utf8Result::Char(char) = self.peek()
        && (char.is_alphabetic() || char == '_')
      {
        // Tag: consume the identifier part
        self.advance_avoid_invalid_utf8();
        loop {
          match self.peek() {
            Utf8Result::Char(char) if char.is_alphanumeric() || char == '_' => {
              self.advance_avoid_invalid_utf8();
            }
            _ => break,
          }
        }
        return self.emit(SyntaxKind::YamlOp);
      }
      // `!` followed by another op char (e.g. `!=`): continue consuming op chars
      if let Utf8Result::Char(char) = self.peek()
        && is_op_char(char)
      {
        self.consume_op_chars();
        return self.emit(SyntaxKind::YamlOp);
      }
      // Standalone `!` not followed by identifier or op char is an error
      return self.emit(SyntaxKind::Error);
    }

    self.consume_op_chars();
    self.emit(SyntaxKind::YamlOp)
  }

  pub(in crate::syntax::lex) fn consume_op_chars(&mut self) {
    loop {
      match self.peek() {
        Utf8Result::Char(char) if is_op_char(char) => {
          self.advance_avoid_invalid_utf8();
        }
        _ => break,
      }
    }
  }

  /* Strings */

  pub(in crate::syntax::lex) fn lex_yaml_dq_string(&mut self) -> LexResult {
    self.advance_avoid_invalid_utf8();
    self.yaml_lex_ctx.interp_stack.push(InterpContext::DqString);
    self.emit(SyntaxKind::DqStrStart)
  }

  pub(in crate::syntax::lex) fn lex_yaml_sq_string(&mut self) -> LexResult {
    self.advance_avoid_invalid_utf8();
    self.yaml_lex_ctx.interp_stack.push(InterpContext::SqString);
    self.emit(SyntaxKind::SqStrStart)
  }

  pub(in crate::syntax::lex) fn lex_yaml_resume_string(&mut self) -> LexResult {
    match self.yaml_lex_ctx.interp_stack.last() {
      Some(InterpContext::DqString) => {
        self.lex_yaml_string_content('"', SyntaxKind::DqStrContent, SyntaxKind::DqStrEnd)
      }
      Some(InterpContext::SqString) => {
        self.lex_yaml_string_content('\'', SyntaxKind::SqStrContent, SyntaxKind::SqStrEnd)
      }
      _ => panic!(
        "[LexCtx::lex_yaml_resume_string] Called without a string context on the interp stack"
      ),
    }
  }

  pub(in crate::syntax::lex) fn lex_yaml_string_content(
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
            self.yaml_lex_ctx.interp_stack.pop();
            return self.emit(end_kind);
          } else {
            let content = self.emit(content_kind);
            self.advance_avoid_invalid_utf8();
            self.yaml_lex_ctx.interp_stack.pop();
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
                .yaml_lex_ctx
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
              // Single $ not followed by {: inline math
              let buf_len = self.text_buffer.len();
              let string_text: String = self.text_buffer.drain(..buf_len - 1).collect();

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
              self.yaml_lex_ctx.interp_stack.pop();
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
          self.yaml_lex_ctx.interp_stack.pop();
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

  /* Interpolation */

  pub(in crate::syntax::lex) fn lex_yaml_interpolation(&mut self) -> LexResult {
    if let Utf8Result::Eof = self.peek() {
      self.yaml_lex_ctx.interp_stack.pop();
      let offset = self.stream.offset();
      return self.emit_with(
        SyntaxKind::Error,
        Diagnostic::UnterminatedInterpolation {
          start_offset: offset,
          end_offset: offset,
        },
      );
    }
    self.lex_yaml_token()
  }

  /* Numbers */

  pub(in crate::syntax::lex) fn lex_yaml_number(&mut self) -> LexResult {
    self.lex_number()
  }

  /* Identifiers */

  pub(in crate::syntax::lex) fn lex_yaml_ident(&mut self) -> LexResult {
    loop {
      match self.peek() {
        Utf8Result::Char(char) if char.is_alphanumeric() || char == '_' => {
          self.advance_avoid_invalid_utf8();
        }
        _ => break,
      }
    }
    self.emit(SyntaxKind::Ident)
  }
}

pub(in crate::syntax::lex) fn is_op_char(char: char) -> bool {
  matches!(
    char,
    '!'
      | '?'
      | '+'
      | '-'
      | '*'
      | '/'
      | '\\'
      | '.'
      | '~'
      | '^'
      | '|'
      | '>'
      | '<'
      | '='
      | '%'
      | '&'
      | '@'
  )
}
