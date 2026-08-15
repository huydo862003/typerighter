//! Parser method for parsing many kinds of expressions

use crate::syntax::diagnostic::Diagnostic;
use crate::syntax::syntax_kind::SyntaxKind;
use typedown_types::stream::Utf8Stream;

use super::constants::*;
use crate::syntax::{
  green::GreenNode,
  lex::ctx::LexMode,
  parse::ctx::{ParseCtx, expr_ctx::ExprCtx},
};

impl<S: Utf8Stream> ParseCtx<S> {
  /// General expression, including formula and yaml.
  pub(in crate::syntax::parse) fn parse_expr(
    &mut self,
    block_indent: usize,
  ) -> (GreenNode, Option<ExprCtx>) {
    let mode = self.lex_ctx.mode();

    // Consume all leading whitespaces and comments
    let mut children = vec![];
    loop {
      let peek = self.lex_ctx.peek_yaml(SKIP_NONE);
      match peek.token.kind() {
        SyntaxKind::Whitespace | SyntaxKind::YamlComment => {
          let result = self.lex_ctx.lex();
          children.push(GreenNode::from_token(result.token));
        }
        _ => break,
      }
    }

    let peek = self.lex_ctx.peek_yaml(SKIP_WC);

    match peek.token.kind() {
      SyntaxKind::Newline => {
        let peek_after = self.lex_ctx.peek_yaml(SKIP_WCN);
        if peek_after.token.kind() == SyntaxKind::YamlIndent {
          self.parse_block_seq_or_mapping(children, peek_after.block_indent)
        } else {
          // No indent after newline: parse as formula
          self.parse_formula_expr(children, block_indent)
        }
      }
      // `|`, `>`, or `-` at the start
      SyntaxKind::YamlOp => {
        let text: String = peek.token.chars().collect();
        match text.as_str() {
          "|" => self.parse_literal_block_str_lit(children, block_indent),
          ">" => self.parse_folded_block_str_lit(children, block_indent),
          "-" => {
            let after = self.lex_ctx.peek_yaml_nth(1, SKIP_NONE);
            if after.token.kind() == SyntaxKind::Whitespace {
              let inline_indent = peek.token_indent - peek.token.chars().count();
              self.parse_inline_block_seq_lit(children, inline_indent)
            } else {
              self.parse_formula_expr(children, block_indent)
            }
          }
          _ => self.parse_formula_expr(children, block_indent),
        }
      }
      // Ident followed by colon: inline mapping
      // We need to handle this specially as in the following case:
      // -  key: value
      // #^^
      // # this is not an indent
      //    key2: value2
      // #^^
      // # this is an indent
      SyntaxKind::Ident if mode == LexMode::YamlFrontmatter => {
        let after_ident = self.lex_ctx.peek_yaml_nth(1, SKIP_WC);
        if after_ident.token.kind() == SyntaxKind::Colon {
          let inline_indent = peek.token_indent - peek.token.chars().count();
          self.parse_inline_block_mapping_lit(children, inline_indent)
        } else {
          self.parse_formula_expr(children, block_indent)
        }
      }
      // Everything else: formula expression
      _ => self.parse_formula_expr(children, block_indent),
    }
  }

  // Based on context, determine which trivia we should skip
  fn formula_expr_skip_flags(&self) -> u16 {
    let mut skip = SKIP_WC;
    if self.expr_ctx_stack.should_expr_span_newline() {
      skip |= SKIP_NEWLINE;
    }
    skip
  }

  /// Formula expressions: Pratt-parsed expressions that follow most programming language rules
  pub(in crate::syntax::parse) fn parse_formula_expr(
    &mut self,
    children: Vec<GreenNode>,
    block_indent: usize,
  ) -> (GreenNode, Option<ExprCtx>) {
    self.pratt_parse_expr(0, children, block_indent)
  }

  fn pratt_parse_expr(
    &mut self,
    min_bp: u8,
    children: Vec<GreenNode>,
    block_indent: usize,
  ) -> (GreenNode, Option<ExprCtx>) {
    let mode = self.lex_ctx.mode();

    // Handle children operators
    let peek = self.lex_ctx.peek(self.formula_expr_skip_flags(), mode);
    let (mut lhs, early_exit) = if peek.token.kind() == SyntaxKind::YamlOp {
      let op_text: String = peek.token.chars().collect();
      if let Some(((), right_bp)) = children_binding_power(&op_text) {
        let mut children = children;
        // Consume the children operator
        self.advance(&mut children, self.formula_expr_skip_flags(), mode);
        // Parse operand with the children's right binding power
        let (operand, exit) = self.pratt_parse_expr(right_bp, vec![], block_indent);
        children.push(operand);
        (self.emit(SyntaxKind::PrefixExpr, &children), exit)
      } else {
        // Not a children op, parse as primary
        self.parse_primary_expr(children, block_indent)
      }
    } else {
      self.parse_primary_expr(children, block_indent)
    };

    if early_exit.is_some() {
      return (lhs, early_exit);
    }

    // Infix/postfix loop
    loop {
      let peek = self.lex_ctx.peek(self.formula_expr_skip_flags(), mode);

      if matches!(peek.token.kind(), SyntaxKind::LParen | SyntaxKind::LBracket) {
        let op_text = if peek.token.kind() == SyntaxKind::LParen {
          "("
        } else {
          "["
        };
        let (left_bp, ()) = postfix_binding_power(op_text).expect("( and [ are in the table");
        if left_bp < min_bp {
          break;
        }
        let (node, exit) = if peek.token.kind() == SyntaxKind::LParen {
          self.parse_call_expr(lhs, block_indent)
        } else {
          self.parse_index_expr(lhs, block_indent)
        };
        lhs = node;
        if exit.is_some() {
          return (lhs, exit);
        }
        continue;
      }

      // Check for infix operator
      if peek.token.kind() != SyntaxKind::YamlOp {
        break;
      }

      let op_text: String = peek.token.chars().collect();

      // Check postfix first
      if let Some((left_bp, ())) = postfix_binding_power(&op_text) {
        if left_bp < min_bp {
          break;
        }
        let mut children = vec![lhs];
        self.advance(&mut children, self.formula_expr_skip_flags(), mode);
        lhs = self.emit(SyntaxKind::PostfixExpr, &children);
        continue;
      }

      // Check infix
      if let Some((left_bp, right_bp)) = infix_binding_power(&op_text) {
        if left_bp < min_bp {
          break;
        }
        let mut children = vec![lhs];
        // Consume the infix operator
        self.advance(&mut children, self.formula_expr_skip_flags(), mode);
        // Parse right-hand side
        let (rhs, exit) = self.pratt_parse_expr(right_bp, vec![], block_indent);
        children.push(rhs);
        lhs = self.emit(SyntaxKind::BinaryExpr, &children);
        if exit.is_some() {
          return (lhs, exit);
        }
        continue;
      }

      // Not a recognized operator, stop
      break;
    }

    (lhs, None)
  }

  /// Parse a call expression: `callee(arg1, arg2, ...)`.
  /// `callee` has already been parsed and is passed in.
  fn parse_call_expr(
    &mut self,
    callee: GreenNode,
    block_indent: usize,
  ) -> (GreenNode, Option<ExprCtx>) {
    let mode = self.lex_ctx.mode();
    debug_assert!(
      self
        .lex_ctx
        .peek(self.formula_expr_skip_flags(), mode)
        .token
        .kind()
        == SyntaxKind::LParen,
      "[ParseCtx::parse_call_expr] Expected next token to be LParen"
    );

    let mut children = vec![callee];
    self.expr_ctx_stack.enter(ExprCtx::Call);

    // Consume `(`
    self.advance(&mut children, self.formula_expr_skip_flags(), mode);

    // Check for empty args `()`
    let peek = self.lex_ctx.peek(SKIP_WCN, mode);
    if peek.token.kind() == SyntaxKind::RParen {
      self.advance(&mut children, SKIP_ALL_TRIVIA, mode);
      self.expr_ctx_stack.exit(ExprCtx::Call);
      return (self.emit(SyntaxKind::CallExpr, &children), None);
    }

    // Parse first argument (formula expression, not full YAML expr)
    let (arg, early_exit) = self.parse_formula_expr(vec![], block_indent);
    children.push(arg);
    if early_exit.is_some_and(|ctx| ctx != ExprCtx::Call) {
      self.expr_ctx_stack.exit(ExprCtx::Call);
      return (self.emit(SyntaxKind::CallExpr, &children), early_exit);
    }

    // Parse remaining arguments
    loop {
      let peek = self.lex_ctx.peek(SKIP_ALL_TRIVIA, mode);
      match peek.token.kind() {
        SyntaxKind::RParen => {
          self.advance(&mut children, SKIP_ALL_TRIVIA, mode);
          break;
        }
        SyntaxKind::Comma => {
          self.advance(&mut children, SKIP_ALL_TRIVIA, mode);

          // Trailing comma before `)` is allowed
          let peek = self.lex_ctx.peek(SKIP_ALL_TRIVIA, mode);
          if peek.token.kind() == SyntaxKind::RParen {
            self.advance(&mut children, SKIP_ALL_TRIVIA, mode);
            break;
          }

          let (arg, early_exit) = self.parse_formula_expr(vec![], block_indent);
          children.push(arg);
          if early_exit.is_some_and(|ctx| ctx != ExprCtx::Call) {
            self.expr_ctx_stack.exit(ExprCtx::Call);
            return (self.emit(SyntaxKind::CallExpr, &children), early_exit);
          }
        }
        SyntaxKind::Eof => {
          self.diagnostics.push(Diagnostic::MissingSyntaxNode {
            expected: SyntaxKind::RParen,
            start_offset: self.offset(),
            end_offset: self.offset(),
          });
          break;
        }
        _ => {
          let handler = self.expr_ctx_stack.find_handler(&peek.token);
          if handler.is_some_and(|ctx| ctx != ExprCtx::Call) {
            self.expr_ctx_stack.exit(ExprCtx::Call);
            return (self.emit(SyntaxKind::CallExpr, &children), handler);
          }
          if let Some(ctx) = self.synchronize_call_expr(&mut children) {
            self.expr_ctx_stack.exit(ExprCtx::Call);
            return (self.emit(SyntaxKind::CallExpr, &children), Some(ctx));
          }
        }
      }
    }

    self.expr_ctx_stack.exit(ExprCtx::Call);
    (self.emit(SyntaxKind::CallExpr, &children), None)
  }

  // Stop on Comma and RParen
  fn synchronize_call_expr(&mut self, children: &mut Vec<GreenNode>) -> Option<ExprCtx> {
    let mut error_children = vec![];
    let result = loop {
      let peek = self.lex_ctx.peek(SKIP_NONE, self.lex_ctx.mode());
      match peek.token.kind() {
        SyntaxKind::Comma | SyntaxKind::RParen | SyntaxKind::Eof => break None,
        _ => {
          if let Some(ctx) = self.consume_or_delegate(ExprCtx::Call, &mut error_children) {
            break Some(ctx);
          }
        }
      }
    };
    if !error_children.is_empty() {
      children.push(self.emit(SyntaxKind::Error, &error_children));
    }
    result
  }

  /// Parse an index expression: `expr[expr, ...]`
  /// `target` has already been parsed and is passed in.
  fn parse_index_expr(
    &mut self,
    target: GreenNode,
    block_indent: usize,
  ) -> (GreenNode, Option<ExprCtx>) {
    let mode = self.lex_ctx.mode();
    debug_assert!(
      self
        .lex_ctx
        .peek(self.formula_expr_skip_flags(), mode)
        .token
        .kind()
        == SyntaxKind::LBracket,
      "[ParseCtx::parse_index_expr] Expected next token to be LBracket"
    );

    let mut children = vec![target];
    self.expr_ctx_stack.enter(ExprCtx::Index);

    // Consume `[`
    self.advance(&mut children, self.formula_expr_skip_flags(), mode);

    // Check for empty index `[]`
    let peek = self.lex_ctx.peek(SKIP_WCN, mode);
    if peek.token.kind() == SyntaxKind::RBracket {
      self.advance(&mut children, SKIP_ALL_TRIVIA, mode);
      self.expr_ctx_stack.exit(ExprCtx::Index);
      return (self.emit(SyntaxKind::IndexExpr, &children), None);
    }

    // Parse first index
    let (idx, early_exit) = self.parse_expr(block_indent);
    children.push(idx);
    if early_exit.is_some_and(|ctx| ctx != ExprCtx::Index) {
      self.expr_ctx_stack.exit(ExprCtx::Index);
      return (self.emit(SyntaxKind::IndexExpr, &children), early_exit);
    }

    // Parse remaining indices
    loop {
      let peek = self.lex_ctx.peek(SKIP_ALL_TRIVIA, mode);
      match peek.token.kind() {
        SyntaxKind::RBracket => {
          self.advance(&mut children, SKIP_ALL_TRIVIA, mode);
          break;
        }
        SyntaxKind::Comma => {
          self.advance(&mut children, SKIP_ALL_TRIVIA, mode);

          // Trailing comma before `]` is allowed
          let peek = self.lex_ctx.peek(SKIP_ALL_TRIVIA, mode);
          if peek.token.kind() == SyntaxKind::RBracket {
            self.advance(&mut children, SKIP_ALL_TRIVIA, mode);
            break;
          }

          let (idx, early_exit) = self.parse_expr(block_indent);
          children.push(idx);
          if early_exit.is_some_and(|ctx| ctx != ExprCtx::Index) {
            self.expr_ctx_stack.exit(ExprCtx::Index);
            return (self.emit(SyntaxKind::IndexExpr, &children), early_exit);
          }
        }
        SyntaxKind::Eof => {
          self.diagnostics.push(Diagnostic::MissingSyntaxNode {
            expected: SyntaxKind::RBracket,
            start_offset: self.offset(),
            end_offset: self.offset(),
          });
          break;
        }
        _ => {
          let handler = self.expr_ctx_stack.find_handler(&peek.token);
          if handler.is_some_and(|ctx| ctx != ExprCtx::Index) {
            self.expr_ctx_stack.exit(ExprCtx::Index);
            return (self.emit(SyntaxKind::IndexExpr, &children), handler);
          }
          if let Some(ctx) = self.synchronize_index_expr(&mut children) {
            self.expr_ctx_stack.exit(ExprCtx::Index);
            return (self.emit(SyntaxKind::IndexExpr, &children), Some(ctx));
          }
        }
      }
    }

    self.expr_ctx_stack.exit(ExprCtx::Index);
    (self.emit(SyntaxKind::IndexExpr, &children), None)
  }

  // Stop on Comma and RBracket
  fn synchronize_index_expr(&mut self, children: &mut Vec<GreenNode>) -> Option<ExprCtx> {
    let mut error_children = vec![];
    let result = loop {
      let peek = self.lex_ctx.peek(SKIP_NONE, self.lex_ctx.mode());
      match peek.token.kind() {
        SyntaxKind::Comma | SyntaxKind::RBracket | SyntaxKind::Eof => break None,
        _ => {
          if let Some(ctx) = self.consume_or_delegate(ExprCtx::Index, &mut error_children) {
            break Some(ctx);
          }
        }
      }
    };
    if !error_children.is_empty() {
      children.push(self.emit(SyntaxKind::Error, &error_children));
    }
    result
  }

  /// Parse a primary expression (an operand): literal, ident, paren, etc.
  pub(in crate::syntax::parse) fn parse_primary_expr(
    &mut self,
    children: Vec<GreenNode>,
    block_indent: usize,
  ) -> (GreenNode, Option<ExprCtx>) {
    let mode = self.lex_ctx.mode();
    let peek = self.lex_ctx.peek(self.formula_expr_skip_flags(), mode);

    match peek.token.kind() {
      SyntaxKind::Number => self.parse_number_lit(children, block_indent),
      SyntaxKind::DqStrStart => self.parse_dq_str_lit(children, block_indent),
      SyntaxKind::SqStrStart => self.parse_sq_str_lit(children, block_indent),
      SyntaxKind::InlineCode | SyntaxKind::CodeBlock => self.parse_code_lit(children, block_indent),
      SyntaxKind::InlineMath | SyntaxKind::MathBlock => self.parse_math_lit(children, block_indent),
      SyntaxKind::Ident => self.parse_ident_lit(children, block_indent),
      SyntaxKind::LParen => self.parse_paren_expr(children, block_indent),
      SyntaxKind::LBracket => self.parse_list_lit(children, block_indent),
      SyntaxKind::LBrace => self.parse_dict_lit(children, block_indent),
      _ => {
        // Check if an outer context can handle this token
        let handler = self.expr_ctx_stack.find_handler(&peek.token);
        if handler.is_some() {
          // Don't consume: let the caller handle it
          self.diagnostics.push(Diagnostic::MissingSyntaxNode {
            expected: SyntaxKind::PrimaryExpr,
            start_offset: self.offset(),
            end_offset: self.offset(),
          });
          (self.emit(SyntaxKind::PrimaryExpr, &children), handler)
        } else {
          // No one can handle it: consume as error
          let mut children = children;
          self.advance(&mut children, self.formula_expr_skip_flags(), mode);
          let bad = children.pop().unwrap();
          children.push(self.emit(SyntaxKind::Error, &[bad]));
          self.diagnostics.push(Diagnostic::MissingSyntaxNode {
            expected: SyntaxKind::PrimaryExpr,
            start_offset: self.offset(),
            end_offset: self.offset(),
          });
          (self.emit(SyntaxKind::PrimaryExpr, &children), None)
        }
      }
    }
  }

  /// Parse a parenthesized expression: `(expr)`.
  pub(in crate::syntax::parse) fn parse_paren_expr(
    &mut self,
    children: Vec<GreenNode>,
    block_indent: usize,
  ) -> (GreenNode, Option<ExprCtx>) {
    debug_assert!(
      self
        .lex_ctx
        .peek(SKIP_ALL_TRIVIA, self.lex_ctx.mode())
        .token
        .kind()
        == SyntaxKind::LParen,
      "[ParseCtx::parse_paren_expr] Expected next token to be LParen"
    );
    let mut children = children;
    self.expr_ctx_stack.enter(ExprCtx::Paren);

    // Consume `(`
    let offset = self.offset();
    self.consume(
      &mut children,
      SKIP_ALL_TRIVIA,
      self.lex_ctx.mode(),
      SyntaxKind::LParen,
      Diagnostic::MissingSyntaxNode {
        expected: SyntaxKind::LParen,
        start_offset: offset,
        end_offset: self.offset(),
      },
    );

    // Parse inner expression
    let (inner, early_exit) = self.parse_formula_expr(vec![], block_indent);
    children.push(inner);

    if early_exit.is_some_and(|ctx| ctx != ExprCtx::Paren) {
      self.expr_ctx_stack.exit(ExprCtx::Paren);
      return (self.emit(SyntaxKind::ParenExpr, &children), early_exit);
    }

    // Consume `)`
    let offset = self.offset();
    self.consume(
      &mut children,
      SKIP_ALL_TRIVIA,
      self.lex_ctx.mode(),
      SyntaxKind::RParen,
      Diagnostic::MissingSyntaxNode {
        expected: SyntaxKind::RParen,
        start_offset: offset,
        end_offset: self.offset(),
      },
    );

    self.expr_ctx_stack.exit(ExprCtx::Paren);
    (self.emit(SyntaxKind::ParenExpr, &children), None)
  }

  /// Parse a flow list literal: `[expr, expr, ...]`.
  pub(in crate::syntax::parse) fn parse_list_lit(
    &mut self,
    children: Vec<GreenNode>,
    block_indent: usize,
  ) -> (GreenNode, Option<ExprCtx>) {
    let outer_skip = self.formula_expr_skip_flags()
      | if self.expr_ctx_stack.should_expr_skip_indent() {
        SKIP_INDENT
      } else {
        0
      };
    debug_assert!(
      self
        .lex_ctx
        .peek(outer_skip, self.lex_ctx.mode())
        .token
        .kind()
        == SyntaxKind::LBracket,
      "[ParseCtx::parse_list_lit] Expected next token to be LBracket"
    );

    let mode = self.lex_ctx.mode();
    let mut children = children;
    self.expr_ctx_stack.enter(ExprCtx::List);

    // Consume `[`
    let offset = self.offset();
    self.consume(
      &mut children,
      outer_skip,
      mode,
      SyntaxKind::LBracket,
      Diagnostic::MissingSyntaxNode {
        expected: SyntaxKind::LBracket,
        start_offset: offset,
        end_offset: self.offset(),
      },
    );

    // Check for empty list `[]`
    let peek = self.lex_ctx.peek(SKIP_ALL_TRIVIA, mode);
    if peek.token.kind() == SyntaxKind::RBracket {
      self.advance(&mut children, SKIP_ALL_TRIVIA, mode);
      self.expr_ctx_stack.exit(ExprCtx::List);
      return (self.emit(SyntaxKind::ListLit, &children), None);
    }

    // Parse first item (no leading comma)
    let (item, early_exit) = self.parse_list_item(block_indent);
    children.push(item);
    if early_exit.is_some_and(|ctx| ctx != ExprCtx::List) {
      self.expr_ctx_stack.exit(ExprCtx::List);
      return (self.emit(SyntaxKind::ListLit, &children), early_exit);
    }

    // Parse remaining items
    loop {
      let peek = self.lex_ctx.peek(SKIP_ALL_TRIVIA, mode);
      match peek.token.kind() {
        // End of list
        SyntaxKind::RBracket => {
          self.advance(&mut children, SKIP_ALL_TRIVIA, mode);
          break;
        }
        // Separator: expect another item
        SyntaxKind::Comma => {
          self.advance(&mut children, SKIP_ALL_TRIVIA, mode);

          // Trailing comma before `]` is allowed
          let peek = self.lex_ctx.peek(SKIP_ALL_TRIVIA, mode);
          if peek.token.kind() == SyntaxKind::RBracket {
            self.advance(&mut children, SKIP_ALL_TRIVIA, mode);
            break;
          }

          let (item, early_exit) = self.parse_list_item(block_indent);
          children.push(item);
          if early_exit.is_some_and(|ctx| ctx != ExprCtx::List) {
            self.expr_ctx_stack.exit(ExprCtx::List);
            return (self.emit(SyntaxKind::ListLit, &children), early_exit);
          }
        }
        // EOF
        SyntaxKind::Eof => {
          self.diagnostics.push(Diagnostic::MissingSyntaxNode {
            expected: SyntaxKind::RBracket,
            start_offset: self.offset(),
            end_offset: self.offset(),
          });
          break;
        }
        // Unexpected token: check handler context
        _ => {
          let handler = self.expr_ctx_stack.find_handler(&peek.token);
          if handler.is_some_and(|ctx| ctx != ExprCtx::List) {
            // Outer context should handle this token
            self.expr_ctx_stack.exit(ExprCtx::List);
            return (self.emit(SyntaxKind::ListLit, &children), handler);
          }
          // Current context or no handler: synchronize
          if let Some(ctx) = self.synchronize_list_lit(&mut children) {
            self.expr_ctx_stack.exit(ExprCtx::List);
            return (self.emit(SyntaxKind::ListLit, &children), Some(ctx));
          }
        }
      }
    }

    self.expr_ctx_stack.exit(ExprCtx::List);
    (self.emit(SyntaxKind::ListLit, &children), None)
  }

  fn parse_list_item(&mut self, block_indent: usize) -> (GreenNode, Option<ExprCtx>) {
    let (expr, early_exit) = self.parse_expr(block_indent);
    (self.emit(SyntaxKind::ListItem, &[expr]), early_exit)
  }

  // Stop on Comma and RBracket
  fn synchronize_list_lit(&mut self, children: &mut Vec<GreenNode>) -> Option<ExprCtx> {
    let mut error_children = vec![];
    let result = loop {
      let peek = self.lex_ctx.peek(SKIP_NONE, self.lex_ctx.mode());
      match peek.token.kind() {
        SyntaxKind::Comma | SyntaxKind::RBracket | SyntaxKind::Eof => break None,
        _ => {
          if let Some(ctx) = self.consume_or_delegate(ExprCtx::List, &mut error_children) {
            break Some(ctx);
          }
        }
      }
    };
    if !error_children.is_empty() {
      children.push(self.emit(SyntaxKind::Error, &error_children));
    }
    result
  }

  /// Parse a block expression (sequence or mapping) after a newline.
  pub(in crate::syntax::parse) fn parse_block_seq_or_mapping(
    &mut self,
    children: Vec<GreenNode>,
    block_indent: usize,
  ) -> (GreenNode, Option<ExprCtx>) {
    let mode = self.lex_ctx.mode();
    let mut children = children;

    // Consume indent
    let offset = self.offset();
    self.consume(
      &mut children,
      SKIP_WCN,
      mode,
      SyntaxKind::YamlIndent,
      Diagnostic::MissingSyntaxNode {
        expected: SyntaxKind::YamlIndent,
        start_offset: offset,
        end_offset: self.offset(),
      },
    );

    // Peek to decide: `-` means sequence, `ident` means mapping
    let peek = self.lex_ctx.peek(SKIP_WCN, mode);

    if peek.token.kind() == SyntaxKind::YamlOp && peek.token.chars().collect::<String>() == "-" {
      self.parse_block_seq_lit(children, block_indent)
    } else {
      self.parse_block_mapping_lit(children, block_indent)
    }
  }

  /// Parse a block sequence literal: Start with `-` on the current line, not on a new line with indentation
  /// INVARIANT: Next token must be - followed by a whitespace.
  pub(in crate::syntax::parse) fn parse_inline_block_seq_lit(
    &mut self,
    children: Vec<GreenNode>,
    block_indent: usize,
  ) -> (GreenNode, Option<ExprCtx>) {
    let mode = self.lex_ctx.mode();
    let mut children = children;
    self.expr_ctx_stack.enter(ExprCtx::BlockSeq);

    // Parse first item on the current line
    let (item, early_exit) = self.parse_block_seq_item(block_indent);
    children.push(item);
    if early_exit.is_some_and(|ctx| ctx != ExprCtx::BlockSeq) {
      self.expr_ctx_stack.exit(ExprCtx::BlockSeq);
      return (self.emit(SyntaxKind::YamlSequence, &children), early_exit);
    }

    // Check for continuation items on indented lines
    let peek = self
      .lex_ctx
      .peek(SKIP_NEWLINE | SKIP_WS | SKIP_COMMENT, mode);
    if peek.token.kind() == SyntaxKind::YamlIndent {
      // Consume indent and parse remaining items like block_seq_lit
      self.advance(&mut children, SKIP_NEWLINE | SKIP_WS | SKIP_COMMENT, mode);
      loop {
        let peek = self
          .lex_ctx
          .peek(SKIP_NEWLINE | SKIP_WS | SKIP_COMMENT, mode);

        if self.is_block_dedent(&peek.token, block_indent) {
          break;
        }
        match peek.token.kind() {
          SyntaxKind::YamlIndent => {
            self.advance(&mut children, SKIP_NEWLINE | SKIP_WS | SKIP_COMMENT, mode);
          }
          SyntaxKind::YamlOp if peek.token.chars().collect::<String>() == "-" => {
            let (item, early_exit) = self.parse_block_seq_item(block_indent);
            children.push(item);
            if early_exit.is_some_and(|ctx| ctx != ExprCtx::BlockSeq) {
              self.expr_ctx_stack.exit(ExprCtx::BlockSeq);
              return (self.emit(SyntaxKind::YamlSequence, &children), early_exit);
            }
          }
          _ => {
            let handler = self.expr_ctx_stack.find_handler(&peek.token);
            if handler.is_some_and(|ctx| ctx != ExprCtx::BlockSeq) {
              self.expr_ctx_stack.exit(ExprCtx::BlockSeq);
              return (self.emit(SyntaxKind::YamlSequence, &children), handler);
            }
            if let Some(ctx) = self.synchronize_block_seq(&mut children) {
              self.expr_ctx_stack.exit(ExprCtx::BlockSeq);
              return (self.emit(SyntaxKind::YamlSequence, &children), Some(ctx));
            }
          }
        }
      }
    }

    self.expr_ctx_stack.exit(ExprCtx::BlockSeq);
    (self.emit(SyntaxKind::YamlSequence, &children), None)
  }

  pub(in crate::syntax::parse) fn parse_block_seq_lit(
    &mut self,
    children: Vec<GreenNode>,
    block_indent: usize,
  ) -> (GreenNode, Option<ExprCtx>) {
    let mode = self.lex_ctx.mode();
    let mut children = children;
    self.expr_ctx_stack.enter(ExprCtx::BlockSeq);

    // Parse items
    loop {
      let peek = self
        .lex_ctx
        .peek(SKIP_NEWLINE | SKIP_WS | SKIP_COMMENT, mode);

      if self.is_block_dedent(&peek.token, block_indent) {
        break;
      }
      match peek.token.kind() {
        SyntaxKind::YamlIndent => {
          self.advance(&mut children, SKIP_WCN, mode);
        }
        SyntaxKind::YamlOp if peek.token.chars().collect::<String>() == "-" => {
          let (item, early_exit) = self.parse_block_seq_item(block_indent);
          children.push(item);
          if early_exit.is_some_and(|ctx| ctx != ExprCtx::BlockSeq) {
            self.expr_ctx_stack.exit(ExprCtx::BlockSeq);
            return (self.emit(SyntaxKind::YamlSequence, &children), early_exit);
          }
        }
        _ => {
          let handler = self.expr_ctx_stack.find_handler(&peek.token);
          if handler.is_some_and(|ctx| ctx != ExprCtx::BlockSeq) {
            self.expr_ctx_stack.exit(ExprCtx::BlockSeq);
            return (self.emit(SyntaxKind::YamlSequence, &children), handler);
          }
          if let Some(ctx) = self.synchronize_block_seq(&mut children) {
            self.expr_ctx_stack.exit(ExprCtx::BlockSeq);
            return (self.emit(SyntaxKind::YamlSequence, &children), Some(ctx));
          }
        }
      }
    }

    self.expr_ctx_stack.exit(ExprCtx::BlockSeq);
    (self.emit(SyntaxKind::YamlSequence, &children), None)
  }

  /// Parse a single block sequence item: `- expr`.
  fn parse_block_seq_item(&mut self, block_indent: usize) -> (GreenNode, Option<ExprCtx>) {
    let mode = self.lex_ctx.mode();
    let mut children = vec![];

    // Consume `-`
    self.advance(&mut children, SKIP_WCN, mode);

    // Parse the value expression
    let (value, early_exit) = self.parse_expr(block_indent);
    children.push(value);

    (
      self.emit(SyntaxKind::YamlSequenceItem, &children),
      early_exit,
    )
  }

  /// Parse a flow mapping literal: `{key: value, ...}`.
  pub(in crate::syntax::parse) fn parse_dict_lit(
    &mut self,
    children: Vec<GreenNode>,
    block_indent: usize,
  ) -> (GreenNode, Option<ExprCtx>) {
    let outer_skip = self.formula_expr_skip_flags()
      | if self.expr_ctx_stack.should_expr_skip_indent() {
        SKIP_INDENT
      } else {
        0
      };
    debug_assert!(
      self
        .lex_ctx
        .peek(outer_skip, self.lex_ctx.mode())
        .token
        .kind()
        == SyntaxKind::LBrace,
      "[ParseCtx::parse_dict_lit] Expected next token to be LBrace"
    );

    let mode = self.lex_ctx.mode();
    let mut children = children;
    self.expr_ctx_stack.enter(ExprCtx::Dict);

    // Consume `{`
    let offset = self.offset();
    self.consume(
      &mut children,
      outer_skip,
      mode,
      SyntaxKind::LBrace,
      Diagnostic::MissingSyntaxNode {
        expected: SyntaxKind::LBrace,
        start_offset: offset,
        end_offset: self.offset(),
      },
    );

    // Check for empty mapping `{}`
    let peek = self.lex_ctx.peek(SKIP_ALL_TRIVIA, mode);
    if peek.token.kind() == SyntaxKind::RBrace {
      self.advance(&mut children, SKIP_ALL_TRIVIA, mode);
      self.expr_ctx_stack.exit(ExprCtx::Dict);
      return (self.emit(SyntaxKind::DictLit, &children), None);
    }

    // Parse first entry
    let (entry, early_exit) = self.parse_dict_entry_lit(block_indent);
    children.push(entry);
    if early_exit.is_some_and(|ctx| ctx != ExprCtx::Dict) {
      self.expr_ctx_stack.exit(ExprCtx::Dict);
      return (self.emit(SyntaxKind::DictLit, &children), early_exit);
    }

    // Parse remaining entries
    loop {
      let peek = self.lex_ctx.peek(SKIP_ALL_TRIVIA, mode);
      match peek.token.kind() {
        // End of mapping
        SyntaxKind::RBrace => {
          self.advance(&mut children, SKIP_ALL_TRIVIA, mode);
          break;
        }
        // Separator: expect another entry
        SyntaxKind::Comma => {
          self.advance(&mut children, SKIP_ALL_TRIVIA, mode);

          // Trailing comma before `}` is allowed
          let peek = self.lex_ctx.peek(SKIP_ALL_TRIVIA, mode);
          if peek.token.kind() == SyntaxKind::RBrace {
            self.advance(&mut children, SKIP_ALL_TRIVIA, mode);
            break;
          }

          let (entry, early_exit) = self.parse_dict_entry_lit(block_indent);
          children.push(entry);
          if early_exit.is_some_and(|ctx| ctx != ExprCtx::Dict) {
            self.expr_ctx_stack.exit(ExprCtx::Dict);
            return (self.emit(SyntaxKind::DictLit, &children), early_exit);
          }
        }
        // EOF
        SyntaxKind::Eof => {
          self.diagnostics.push(Diagnostic::MissingSyntaxNode {
            expected: SyntaxKind::RBrace,
            start_offset: self.offset(),
            end_offset: self.offset(),
          });
          break;
        }
        // Unexpected token
        _ => {
          let handler = self.expr_ctx_stack.find_handler(&peek.token);
          if handler.is_some_and(|ctx| ctx != ExprCtx::Dict) {
            self.expr_ctx_stack.exit(ExprCtx::Dict);
            return (self.emit(SyntaxKind::DictLit, &children), handler);
          }
          if let Some(ctx) = self.synchronize_dict_lit(&mut children) {
            self.expr_ctx_stack.exit(ExprCtx::Dict);
            return (self.emit(SyntaxKind::DictLit, &children), Some(ctx));
          }
        }
      }
    }

    self.expr_ctx_stack.exit(ExprCtx::Dict);
    (self.emit(SyntaxKind::DictLit, &children), None)
  }

  /// Parse a single mapping entry: `key: value`.
  fn parse_dict_entry_lit(&mut self, block_indent: usize) -> (GreenNode, Option<ExprCtx>) {
    let mode = self.lex_ctx.mode();
    let mut children = vec![];

    let peek = self.lex_ctx.peek(SKIP_ALL_TRIVIA, mode);

    // Missing key: `:` seen immediately
    if peek.token.kind() == SyntaxKind::Colon {
      self.diagnostics.push(Diagnostic::MissingSyntaxNode {
        expected: SyntaxKind::DictEntryKey,
        start_offset: self.offset(),
        end_offset: self.offset(),
      });
      // Emit empty MappingEntryKey as error
      children.push(self.emit(SyntaxKind::DictEntryKey, &[]));
    } else {
      // Key (required to be an identifier)
      let offset = self.offset();
      self.consume(
        &mut children,
        SKIP_ALL_TRIVIA,
        mode,
        SyntaxKind::Ident,
        Diagnostic::MissingSyntaxNode {
          expected: SyntaxKind::Ident,
          start_offset: offset,
          end_offset: self.offset(),
        },
      );
      let key_token = children.pop().unwrap();
      children.push(self.emit(SyntaxKind::DictEntryKey, &[key_token]));
    }

    // Colon
    let peek = self.lex_ctx.peek(SKIP_WS, mode);
    if peek.token.kind() == SyntaxKind::Colon {
      self.advance(&mut children, SKIP_WS, mode);
    } else {
      // Missing colon: emit diagnostic but continue to parse value
      self.diagnostics.push(Diagnostic::MissingSyntaxNode {
        expected: SyntaxKind::Colon,
        start_offset: self.offset(),
        end_offset: self.offset(),
      });
    }

    // Value
    let peek = self.lex_ctx.peek(SKIP_ALL_TRIVIA, mode);
    match peek.token.kind() {
      SyntaxKind::Comma | SyntaxKind::RBrace | SyntaxKind::Eof => {
        // Missing value
        self.diagnostics.push(Diagnostic::MissingSyntaxNode {
          expected: SyntaxKind::DictEntryValue,
          start_offset: self.offset(),
          end_offset: self.offset(),
        });
        children.push(self.emit(SyntaxKind::DictEntryValue, &[]));
        (self.emit(SyntaxKind::DictEntry, &children), None)
      }
      _ => {
        let (value_expr, early_exit) = self.parse_expr(block_indent);
        children.push(self.emit(SyntaxKind::DictEntryValue, &[value_expr]));
        (self.emit(SyntaxKind::DictEntry, &children), early_exit)
      }
    }
  }

  // Stop on Comma and RBrace
  fn synchronize_dict_lit(&mut self, children: &mut Vec<GreenNode>) -> Option<ExprCtx> {
    let mut error_children = vec![];
    let result = loop {
      let peek = self.lex_ctx.peek(SKIP_NONE, self.lex_ctx.mode());
      match peek.token.kind() {
        SyntaxKind::Comma | SyntaxKind::RBrace | SyntaxKind::Eof => break None,
        _ => {
          if let Some(ctx) = self.consume_or_delegate(ExprCtx::Dict, &mut error_children) {
            break Some(ctx);
          }
        }
      }
    };
    if !error_children.is_empty() {
      children.push(self.emit(SyntaxKind::Error, &error_children));
    }
    result
  }

  // Stop on `-` (YamlOp), dedentation, Newline, Eof
  fn synchronize_block_seq(&mut self, children: &mut Vec<GreenNode>) -> Option<ExprCtx> {
    let mut error_children = vec![];
    let result = loop {
      let peek = self.lex_ctx.peek(SKIP_NONE, self.lex_ctx.mode());
      match peek.token.kind() {
        SyntaxKind::YamlIndent | SyntaxKind::Newline | SyntaxKind::Eof => break None,
        SyntaxKind::YamlOp if peek.token.chars().collect::<String>() == "-" => break None,
        _ => {
          if let Some(ctx) = self.consume_or_delegate(ExprCtx::BlockSeq, &mut error_children) {
            break Some(ctx);
          }
        }
      }
    };
    if !error_children.is_empty() {
      children.push(self.emit(SyntaxKind::Error, &error_children));
    }
    result
  }

  // Stop on Ident, Colon, Dedentation, Newline, Eof
  fn synchronize_block_mapping(&mut self, children: &mut Vec<GreenNode>) -> Option<ExprCtx> {
    let mut error_children = vec![];
    let result = loop {
      let peek = self.lex_ctx.peek(SKIP_NONE, self.lex_ctx.mode());
      match peek.token.kind() {
        SyntaxKind::Ident
        | SyntaxKind::Colon
        | SyntaxKind::YamlIndent
        | SyntaxKind::Newline
        | SyntaxKind::Eof => break None,
        _ => {
          if let Some(ctx) = self.consume_or_delegate(ExprCtx::BlockMap, &mut error_children) {
            break Some(ctx);
          }
        }
      }
    };
    if !error_children.is_empty() {
      children.push(self.emit(SyntaxKind::Error, &error_children));
    }
    result
  }

  /// Parse a literal block string: `|` followed by indented content.
  pub(in crate::syntax::parse) fn parse_literal_block_str_lit(
    &mut self,
    children: Vec<GreenNode>,
    block_indent: usize,
  ) -> (GreenNode, Option<ExprCtx>) {
    let mode = self.lex_ctx.mode();
    debug_assert!(
      {
        let peek = self.lex_ctx.peek(self.formula_expr_skip_flags(), mode);
        peek.token.kind() == SyntaxKind::YamlOp && peek.token.chars().collect::<String>() == "|"
      },
      "[ParseCtx::parse_literal_block_str_lit] Expected next token to be `|`"
    );

    let mut children = children;

    // Consume `|`
    self.advance(&mut children, self.formula_expr_skip_flags(), mode);

    // Expect newline after `|`
    let offset = self.offset();
    self.consume(
      &mut children,
      SKIP_WS | SKIP_COMMENT,
      mode,
      SyntaxKind::Newline,
      Diagnostic::MissingSyntaxNode {
        expected: SyntaxKind::Newline,
        start_offset: offset,
        end_offset: self.offset(),
      },
    );

    // Expect indent
    let offset = self.offset();
    self.consume(
      &mut children,
      SKIP_NONE,
      mode,
      SyntaxKind::YamlIndent,
      Diagnostic::MissingSyntaxNode {
        expected: SyntaxKind::YamlIndent,
        start_offset: offset,
        end_offset: self.offset(),
      },
    );
    let content_indent = self.lex_ctx.token_indent();

    if content_indent <= block_indent {
      self.emit_diagnostic(Diagnostic::InsufficientBlockIndent {
        expected_more_than: block_indent,
        found: content_indent,
        start_offset: offset,
        end_offset: self.offset(),
      });
    }

    // Consume content until dedent or EOF
    loop {
      let peek = self.lex_ctx.peek(SKIP_NONE, mode);
      if self.is_block_dedent(&peek.token, content_indent) {
        break;
      }
      self.advance(&mut children, SKIP_NONE, mode);
    }

    let literal_block_str_lit = self.emit(SyntaxKind::YamlLiteralBlockStrLit, &children);
    (
      self.emit(SyntaxKind::StrLit, &[literal_block_str_lit]),
      None,
    )
  }

  /// Parse a folded block string: `>` followed by indented content.
  pub(in crate::syntax::parse) fn parse_folded_block_str_lit(
    &mut self,
    children: Vec<GreenNode>,
    block_indent: usize,
  ) -> (GreenNode, Option<ExprCtx>) {
    let mode = self.lex_ctx.mode();
    debug_assert!(
      {
        let peek = self.lex_ctx.peek(self.formula_expr_skip_flags(), mode);
        peek.token.kind() == SyntaxKind::YamlOp && peek.token.chars().collect::<String>() == ">"
      },
      "[ParseCtx::parse_folded_block_str_lit] Expected next token to be `>`"
    );

    let mut children = children;

    // Consume `>`
    self.advance(&mut children, self.formula_expr_skip_flags(), mode);

    // Expect newline after `>`
    let offset = self.offset();
    self.consume(
      &mut children,
      SKIP_WS | SKIP_COMMENT,
      mode,
      SyntaxKind::Newline,
      Diagnostic::MissingSyntaxNode {
        expected: SyntaxKind::Newline,
        start_offset: offset,
        end_offset: self.offset(),
      },
    );

    // Expect indent
    let offset = self.offset();
    self.consume(
      &mut children,
      SKIP_NONE,
      mode,
      SyntaxKind::YamlIndent,
      Diagnostic::MissingSyntaxNode {
        expected: SyntaxKind::YamlIndent,
        start_offset: offset,
        end_offset: self.offset(),
      },
    );
    let content_indent = self.lex_ctx.token_indent();

    if content_indent <= block_indent {
      self.emit_diagnostic(Diagnostic::InsufficientBlockIndent {
        expected_more_than: block_indent,
        found: content_indent,
        start_offset: offset,
        end_offset: self.offset(),
      });
    }

    // Consume content until dedent or EOF
    loop {
      let peek = self.lex_ctx.peek(SKIP_NONE, mode);
      if self.is_block_dedent(&peek.token, content_indent) {
        break;
      }
      self.advance(&mut children, SKIP_NONE, mode);
    }

    let folded_block_str_lit = self.emit(SyntaxKind::YamlFoldedBlockStrLit, &children);
    (self.emit(SyntaxKind::StrLit, &[folded_block_str_lit]), None)
  }

  /// Parse an inline block mapping: starts with `key: value` on the current line (not on a newline with indentation).
  fn parse_inline_block_mapping_lit(
    &mut self,
    mut children: Vec<GreenNode>,
    block_indent: usize,
  ) -> (GreenNode, Option<ExprCtx>) {
    let mode = self.lex_ctx.mode();
    self.expr_ctx_stack.enter(ExprCtx::BlockMap);

    let (entry, early_exit) = self.parse_block_mapping_entry(vec![], block_indent);
    children.push(entry);
    if early_exit.is_some_and(|ctx| ctx != ExprCtx::BlockMap) {
      self.expr_ctx_stack.exit(ExprCtx::BlockMap);
      return (self.emit(SyntaxKind::YamlMapping, &children), early_exit);
    }

    // Check for continuation entries on indented lines
    let peek = self
      .lex_ctx
      .peek(SKIP_NEWLINE | SKIP_WS | SKIP_COMMENT, mode);
    if peek.token.kind() == SyntaxKind::YamlIndent {
      self.advance(&mut children, SKIP_NEWLINE | SKIP_WS | SKIP_COMMENT, mode);
      loop {
        let peek = self
          .lex_ctx
          .peek(SKIP_NEWLINE | SKIP_WS | SKIP_COMMENT, mode);

        if self.is_block_dedent(&peek.token, block_indent) {
          break;
        }
        match peek.token.kind() {
          SyntaxKind::YamlIndent => {
            self.advance(&mut children, SKIP_NEWLINE | SKIP_WS | SKIP_COMMENT, mode);
          }
          SyntaxKind::Ident | SyntaxKind::Colon => {
            let (entry, early_exit) = self.parse_block_mapping_entry(vec![], block_indent);
            children.push(entry);
            if early_exit.is_some_and(|ctx| ctx != ExprCtx::BlockMap) {
              self.expr_ctx_stack.exit(ExprCtx::BlockMap);
              return (self.emit(SyntaxKind::YamlMapping, &children), early_exit);
            }
          }
          _ => {
            let handler = self.expr_ctx_stack.find_handler(&peek.token);
            if handler.is_some_and(|ctx| ctx != ExprCtx::BlockMap) {
              self.expr_ctx_stack.exit(ExprCtx::BlockMap);
              return (self.emit(SyntaxKind::YamlMapping, &children), handler);
            }
            if let Some(ctx) = self.synchronize_block_mapping(&mut children) {
              self.expr_ctx_stack.exit(ExprCtx::BlockMap);
              return (self.emit(SyntaxKind::YamlMapping, &children), Some(ctx));
            }
          }
        }
      }
    }

    self.expr_ctx_stack.exit(ExprCtx::BlockMap);
    (self.emit(SyntaxKind::YamlMapping, &children), None)
  }

  /// Parse a block mapping literal (indentation-based `key: value` pairs).
  pub(in crate::syntax::parse) fn parse_block_mapping_lit(
    &mut self,
    children: Vec<GreenNode>,
    block_indent: usize,
  ) -> (GreenNode, Option<ExprCtx>) {
    let mode = self.lex_ctx.mode();
    let mut children = children;
    self.expr_ctx_stack.enter(ExprCtx::BlockMap);

    loop {
      let peek = self.lex_ctx.peek(SKIP_WCN, mode);

      if self.is_block_dedent(&peek.token, block_indent) {
        break;
      }
      match peek.token.kind() {
        SyntaxKind::YamlIndent => {
          self.advance(&mut children, SKIP_WCN, mode);
        }
        SyntaxKind::Ident | SyntaxKind::Colon => {
          let (entry, early_exit) = self.parse_block_mapping_entry(vec![], block_indent);
          children.push(entry);
          if early_exit.is_some_and(|ctx| ctx != ExprCtx::BlockMap) {
            self.expr_ctx_stack.exit(ExprCtx::BlockMap);
            return (self.emit(SyntaxKind::YamlMapping, &children), early_exit);
          }
        }
        _ => {
          let handler = self.expr_ctx_stack.find_handler(&peek.token);
          if handler.is_some_and(|ctx| ctx != ExprCtx::BlockMap) {
            self.expr_ctx_stack.exit(ExprCtx::BlockMap);
            return (self.emit(SyntaxKind::YamlMapping, &children), handler);
          }
          if let Some(ctx) = self.synchronize_block_mapping(&mut children) {
            self.expr_ctx_stack.exit(ExprCtx::BlockMap);
            return (self.emit(SyntaxKind::YamlMapping, &children), Some(ctx));
          }
        }
      }
    }

    self.expr_ctx_stack.exit(ExprCtx::BlockMap);
    (self.emit(SyntaxKind::YamlMapping, &children), None)
  }

  /// Parse a single block mapping entry: `key: value`.
  fn parse_block_mapping_entry(
    &mut self,
    children: Vec<GreenNode>,
    block_indent: usize,
  ) -> (GreenNode, Option<ExprCtx>) {
    let mode = self.lex_ctx.mode();
    let mut children = children;

    let peek = self.lex_ctx.peek(SKIP_WCN, mode);

    // Missing key: `:` seen immediately
    if peek.token.kind() == SyntaxKind::Colon {
      self.diagnostics.push(Diagnostic::MissingSyntaxNode {
        expected: SyntaxKind::YamlMappingEntryKey,
        start_offset: self.offset(),
        end_offset: self.offset(),
      });
      children.push(self.emit(SyntaxKind::YamlMappingEntryKey, &[]));
    } else {
      // Key (identifier)
      let offset = self.offset();
      let mut key_children = vec![];
      self.consume(
        &mut key_children,
        SKIP_WCN,
        mode,
        SyntaxKind::Ident,
        Diagnostic::MissingSyntaxNode {
          expected: SyntaxKind::YamlMappingEntryKey,
          start_offset: offset,
          end_offset: self.offset(),
        },
      );
      children.push(self.emit(SyntaxKind::YamlMappingEntryKey, &key_children));
    }

    // Colon
    let peek = self.lex_ctx.peek(SKIP_WS, mode);
    if peek.token.kind() == SyntaxKind::Colon {
      self.advance(&mut children, SKIP_WS, mode);
    } else {
      self.diagnostics.push(Diagnostic::MissingSyntaxNode {
        expected: SyntaxKind::Colon,
        start_offset: self.offset(),
        end_offset: self.offset(),
      });
    }

    // Value: check for newline + indent (nested block) or inline expression
    let peek = self.lex_ctx.peek(SKIP_WC, mode);
    match peek.token.kind() {
      // Newline: could be a nested block (seq or mapping)
      SyntaxKind::Newline => {
        // Peek past the newline to see if indent follows
        let peek_after = self.lex_ctx.peek_yaml(SKIP_WCN);
        if peek_after.token.kind() == SyntaxKind::YamlIndent
          && peek_after.block_indent > block_indent
        {
          let (nested, early_exit) =
            self.parse_block_seq_or_mapping(vec![], peek_after.block_indent);
          children.push(self.emit(SyntaxKind::YamlMappingEntryValue, &[nested]));
          return (
            self.emit(SyntaxKind::YamlMappingEntry, &children),
            early_exit,
          );
        } else {
          self.advance(&mut children, SKIP_WCN, mode);
          self.diagnostics.push(Diagnostic::MissingSyntaxNode {
            expected: SyntaxKind::YamlMappingEntryValue,
            start_offset: self.offset(),
            end_offset: self.offset(),
          });
          children.push(self.emit(SyntaxKind::YamlMappingEntryValue, &[]));
        }
      }
      // EOF: missing value
      SyntaxKind::Eof => {
        self.diagnostics.push(Diagnostic::MissingSyntaxNode {
          expected: SyntaxKind::YamlMappingEntryValue,
          start_offset: self.offset(),
          end_offset: self.offset(),
        });
        children.push(self.emit(SyntaxKind::YamlMappingEntryValue, &[]));
      }
      // Inline value
      _ => {
        let (value, early_exit) = self.parse_expr(block_indent);
        children.push(self.emit(SyntaxKind::YamlMappingEntryValue, &[value]));
        return (
          self.emit(SyntaxKind::YamlMappingEntry, &children),
          early_exit,
        );
      }
    }

    (self.emit(SyntaxKind::YamlMappingEntry, &children), None)
  }

  /// Parse a double-quoted string literal with interpolation: `"content ${expr} content"`.
  pub(in crate::syntax::parse) fn parse_dq_str_lit(
    &mut self,
    children: Vec<GreenNode>,
    block_indent: usize,
  ) -> (GreenNode, Option<ExprCtx>) {
    let mode = self.lex_ctx.mode();
    debug_assert!(
      self
        .lex_ctx
        .peek(self.formula_expr_skip_flags(), mode)
        .token
        .kind()
        == SyntaxKind::DqStrStart,
      "[ParseCtx::parse_dq_str_lit] Expected next token to be DqStrStart"
    );

    let mut children = children;
    self.expr_ctx_stack.enter(ExprCtx::DqString);
    self.advance(&mut children, self.formula_expr_skip_flags(), mode);

    loop {
      let peek = self.lex_ctx.peek(SKIP_NONE, mode);
      match peek.token.kind() {
        SyntaxKind::DqStrEnd => {
          self.advance(&mut children, SKIP_NONE, mode);
          break;
        }
        SyntaxKind::InterpStart => {
          let (fragment, early_exit) = self.parse_interp_fragment(block_indent);
          children.push(fragment);
          if early_exit.is_some_and(|ctx| ctx != ExprCtx::DqString) {
            self.expr_ctx_stack.exit(ExprCtx::DqString);
            return (self.emit(SyntaxKind::StrLit, &children), early_exit);
          }
        }
        SyntaxKind::InlineMath => {
          let (math, early_exit) = self.parse_math_lit(vec![], block_indent);
          children.push(math);
          if early_exit.is_some_and(|ctx| ctx != ExprCtx::DqString) {
            self.expr_ctx_stack.exit(ExprCtx::DqString);
            return (self.emit(SyntaxKind::StrLit, &children), early_exit);
          }
        }
        SyntaxKind::Eof | SyntaxKind::Error => {
          self.advance(&mut children, SKIP_NONE, mode);
          break;
        }
        _ => {
          self.advance(&mut children, SKIP_NONE, mode);
        }
      }
    }

    self.expr_ctx_stack.exit(ExprCtx::DqString);
    (self.emit(SyntaxKind::StrLit, &children), None)
  }

  /// Parse a single-quoted string literal with interpolation: `'content ${expr} content'`.
  pub(in crate::syntax::parse) fn parse_sq_str_lit(
    &mut self,
    children: Vec<GreenNode>,
    block_indent: usize,
  ) -> (GreenNode, Option<ExprCtx>) {
    let mode = self.lex_ctx.mode();
    debug_assert!(
      self
        .lex_ctx
        .peek(self.formula_expr_skip_flags(), mode)
        .token
        .kind()
        == SyntaxKind::SqStrStart,
      "[ParseCtx::parse_sq_str_lit] Expected next token to be SqStrStart"
    );

    let mut children = children;
    self.expr_ctx_stack.enter(ExprCtx::SqString);
    self.advance(&mut children, self.formula_expr_skip_flags(), mode);

    loop {
      let peek = self.lex_ctx.peek(SKIP_NONE, mode);
      match peek.token.kind() {
        SyntaxKind::SqStrEnd => {
          self.advance(&mut children, SKIP_NONE, mode);
          break;
        }
        SyntaxKind::InterpStart => {
          let (fragment, early_exit) = self.parse_interp_fragment(block_indent);
          children.push(fragment);
          if early_exit.is_some_and(|ctx| ctx != ExprCtx::SqString) {
            self.expr_ctx_stack.exit(ExprCtx::SqString);
            return (self.emit(SyntaxKind::StrLit, &children), early_exit);
          }
        }
        SyntaxKind::InlineMath => {
          let (math, early_exit) = self.parse_math_lit(vec![], block_indent);
          children.push(math);
          if early_exit.is_some_and(|ctx| ctx != ExprCtx::SqString) {
            self.expr_ctx_stack.exit(ExprCtx::SqString);
            return (self.emit(SyntaxKind::StrLit, &children), early_exit);
          }
        }
        SyntaxKind::Eof | SyntaxKind::Error => {
          self.advance(&mut children, SKIP_NONE, mode);
          break;
        }
        _ => {
          self.advance(&mut children, SKIP_NONE, mode);
        }
      }
    }

    self.expr_ctx_stack.exit(ExprCtx::SqString);
    (self.emit(SyntaxKind::StrLit, &children), None)
  }

  /// Parse an interpolation fragment: `${...}` inside a string.
  pub(in crate::syntax::parse) fn parse_interp_fragment(
    &mut self,
    block_indent: usize,
  ) -> (GreenNode, Option<ExprCtx>) {
    let mode = self.lex_ctx.mode();
    debug_assert!(
      self.lex_ctx.peek(SKIP_NONE, mode).token.kind() == SyntaxKind::InterpStart,
      "[ParseCtx::parse_interp_fragment] Expected next token to be InterpStart"
    );

    let mut children = vec![];
    self.expr_ctx_stack.enter(ExprCtx::Interp);

    // Consume `${`
    self.advance(&mut children, SKIP_NONE, mode);

    // Parse the expression inside
    let (inner, early_exit) = self.parse_formula_expr(vec![], block_indent);
    children.push(inner);

    if early_exit.is_some_and(|ctx| ctx != ExprCtx::Interp) {
      self.expr_ctx_stack.exit(ExprCtx::Interp);
      return (self.emit(SyntaxKind::InterpFragment, &children), early_exit);
    }

    // Consume `}`
    let offset = self.offset();
    self.consume(
      &mut children,
      SKIP_ALL_TRIVIA,
      mode,
      SyntaxKind::InterpEnd,
      Diagnostic::MissingSyntaxNode {
        expected: SyntaxKind::InterpEnd,
        start_offset: offset,
        end_offset: self.offset(),
      },
    );

    self.expr_ctx_stack.exit(ExprCtx::Interp);
    (self.emit(SyntaxKind::InterpFragment, &children), None)
  }

  /// Parse a math literal (inline or block math).
  /// Wraps a single InlineMath or MathBlock token.
  pub(in crate::syntax::parse) fn parse_math_lit(
    &mut self,
    children: Vec<GreenNode>,
    _block_indent: usize,
  ) -> (GreenNode, Option<ExprCtx>) {
    let mode = self.lex_ctx.mode();
    debug_assert!(
      matches!(
        self
          .lex_ctx
          .peek(self.formula_expr_skip_flags(), mode)
          .token
          .kind(),
        SyntaxKind::InlineMath | SyntaxKind::MathBlock
      ),
      "[ParseCtx::parse_math_lit] Expected next token to be InlineMath or MathBlock"
    );
    let mut children = children;
    self.advance(&mut children, self.formula_expr_skip_flags(), mode);
    (self.emit(SyntaxKind::MathLit, &children), None)
  }

  /// Parse a code literal (inline or block code).
  /// Wraps a single InlineCode or CodeBlock token.
  pub(in crate::syntax::parse) fn parse_code_lit(
    &mut self,
    children: Vec<GreenNode>,
    _block_indent: usize,
  ) -> (GreenNode, Option<ExprCtx>) {
    let mode = self.lex_ctx.mode();
    debug_assert!(
      matches!(
        self
          .lex_ctx
          .peek(self.formula_expr_skip_flags(), mode)
          .token
          .kind(),
        SyntaxKind::InlineCode | SyntaxKind::CodeBlock
      ),
      "[ParseCtx::parse_code_lit] Expected next token to be InlineCode or CodeBlock"
    );
    let mut children = children;
    self.advance(&mut children, self.formula_expr_skip_flags(), mode);
    (self.emit(SyntaxKind::CodeLit, &children), None)
  }

  /// Parse a number literal.
  /// Wraps a single Number token.
  pub(in crate::syntax::parse) fn parse_number_lit(
    &mut self,
    children: Vec<GreenNode>,
    _block_indent: usize,
  ) -> (GreenNode, Option<ExprCtx>) {
    let mode = self.lex_ctx.mode();
    debug_assert!(
      self
        .lex_ctx
        .peek(self.formula_expr_skip_flags(), mode)
        .token
        .kind()
        == SyntaxKind::Number,
      "[ParseCtx::parse_number_lit] Expected next token to be Number"
    );
    let mut children = children;
    self.advance(&mut children, self.formula_expr_skip_flags(), mode);
    (self.emit(SyntaxKind::NumberLit, &children), None)
  }

  /// Parse an identifier literal.
  /// Wraps a single Ident token.
  pub(in crate::syntax::parse) fn parse_ident_lit(
    &mut self,
    children: Vec<GreenNode>,
    _block_indent: usize,
  ) -> (GreenNode, Option<ExprCtx>) {
    let mode = self.lex_ctx.mode();
    debug_assert!(
      self
        .lex_ctx
        .peek(self.formula_expr_skip_flags(), mode)
        .token
        .kind()
        == SyntaxKind::Ident,
      "[ParseCtx::parse_ident_lit] Expected next token to be Ident"
    );
    let mut children = children;
    self.advance(&mut children, self.formula_expr_skip_flags(), mode);
    (self.emit(SyntaxKind::IdentLit, &children), None)
  }

  /// If the next token should be handled by an outer context, return that context.
  /// Otherwise consume the token into `error_children` for the caller to wrap.
  fn consume_or_delegate(
    &mut self,
    current: ExprCtx,
    error_children: &mut Vec<GreenNode>,
  ) -> Option<ExprCtx> {
    let peek = self.lex_ctx.peek(SKIP_NONE, self.lex_ctx.mode());
    let handler = self.expr_ctx_stack.find_handler(&peek.token);
    if handler.is_some_and(|ctx| ctx != current) {
      return handler;
    }
    let mode = self.lex_ctx.mode();
    self.advance(error_children, SKIP_NONE, mode);
    None
  }
}

pub(in crate::syntax::parse) fn children_binding_power(op: &str) -> Option<((), u8)> {
  let bp = match op {
    _ if op.starts_with('!') => 1,
    "~" | "-" | "+" => 15,
    _ => return None,
  };
  Some(((), bp))
}

pub(in crate::syntax::parse) fn infix_binding_power(op: &str) -> Option<(u8, u8)> {
  let bp = match op {
    "||" => (3, 4),                     // logical OR
    "&&" => (5, 6),                     // logical AND
    "==" | "!=" => (7, 8),              // equality
    "<" | ">" | "<=" | ">=" => (9, 10), // comparison
    "+" | "-" => (11, 12),              // additive
    "*" | "/" | "%" => (13, 14),        // multiplicative
    "**" => (18, 17),                   // exponentiation (right-assoc)
    "." => (19, 20),                    // member access
    _ => return None,
  };
  Some(bp)
}

pub(in crate::syntax::parse) fn postfix_binding_power(op: &str) -> Option<(u8, ())> {
  let bp = match op {
    "?" => 16,
    "(" | "[" => 19,
    _ => return None,
  };
  Some((bp, ()))
}
