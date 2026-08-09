//! Markdown body parsing

use crate::syntax::diagnostic::Diagnostic;
use crate::syntax::syntax_kind::SyntaxKind;
use typedown_types::stream::Utf8Stream;

use super::ctx::ParseCtx;
use super::ctx::expr_ctx::ExprCtx;
use crate::syntax::green::{GreenNode, SyntaxToken};
use crate::syntax::lex::ctx::LexMode;
use crate::syntax::parse::constants::{SKIP_NONE, SKIP_WCN, SKIP_WS};

// Markdown body parsing
// We distinguish between block elements and inline elements
// Inline elements (like links) must always be nested in a block element, such as paragraphs
impl<S: Utf8Stream> ParseCtx<S> {
  pub(in crate::syntax::parse) fn parse_markdown_body(&mut self) -> GreenNode {
    debug_assert!(
      self.lex_ctx.mode() == LexMode::MarkdownBody,
      "[ParseCtx::parse_markdown_body] Lex mode must be MarkdownBody"
    );
    let mut children = vec![];
    self.expr_ctx_stack.enter(ExprCtx::MarkdownBody);

    loop {
      // Skip blank lines
      while self.lex_ctx.peek_md(SKIP_NONE).token.kind() == SyntaxKind::Newline {
        self.advance_md(&mut children, SKIP_NONE);
      }

      if self.lex_ctx.peek_md(SKIP_NONE).token.kind() == SyntaxKind::Eof {
        break;
      }

      let (block, early_exit) = self.parse_md_block_element();
      children.push(block);

      if early_exit == Some(ExprCtx::MarkdownBody) {
        // Consume erroneous tokens until EOF
        let mut error_children = vec![];
        loop {
          let next = self.lex_ctx.peek_md(SKIP_NONE);
          if next.token.kind() == SyntaxKind::Eof {
            break;
          }
          self.advance_md(&mut error_children, SKIP_NONE);
        }
        if !error_children.is_empty() {
          children.push(self.emit(SyntaxKind::Error, &error_children));
        }
        break;
      }
      if early_exit.is_some() {
        // Unexpected early exit from a child: consume as error
        let mut error_children = vec![];
        loop {
          let next = self.lex_ctx.peek_md(SKIP_NONE);
          if next.token.kind() == SyntaxKind::Eof {
            break;
          }
          self.advance_md(&mut error_children, SKIP_NONE);
        }
        if !error_children.is_empty() {
          children.push(self.emit(SyntaxKind::Error, &error_children));
        }
        break;
      }
    }

    self.expr_ctx_stack.exit(ExprCtx::MarkdownBody);
    self.emit(SyntaxKind::MdBody, &children)
  }

  /// Parse a block-level element.
  /// INVARIANT: Must be at start of line with prefix already consumed.
  /// INVARIANT: Block elements do not consume their trailing newline as one newline can end multiple block elements
  pub(in crate::syntax::parse) fn parse_md_block_element(
    &mut self,
  ) -> (GreenNode, Option<ExprCtx>) {
    debug_assert!(
      self.lex_ctx.mode() == LexMode::MarkdownBody,
      "[ParseCtx::parse_md_block_element] Lex mode must be MarkdownBody"
    );

    let next = self.lex_ctx.peek_md(SKIP_NONE);
    match next.token.kind() {
      SyntaxKind::Eof => {
        let mut children = vec![];
        self.advance_md(&mut children, SKIP_NONE);
        (self.emit(SyntaxKind::Error, &children), None)
      }
      SyntaxKind::Newline => {
        // Blank line: consume and return empty
        let mut children = vec![];
        self.advance_md(&mut children, SKIP_NONE);
        (self.emit(SyntaxKind::MdText, &children), None)
      }
      _ if self.is_heading_start(SKIP_NONE) => self.parse_heading(),
      _ if self.is_toggle_list_start(SKIP_NONE) => self.parse_toggle_list(),
      _ if self.is_blockquote_start(SKIP_NONE) => self.parse_blockquote(),
      _ if self.is_bullet_list_start(SKIP_NONE) => self.parse_bullet_list(),
      _ if self.is_ordered_list_start(SKIP_NONE) => self.parse_ordered_list(),
      _ if self.is_table_start(SKIP_NONE) => self.parse_table(),
      _ if self.is_container_start(SKIP_NONE) => self.parse_container_block(),
      _ if self.is_media_block_start(SKIP_NONE) => self.parse_media(),
      _ if self.is_code_or_math_block_start(SKIP_NONE) => {
        let mut children = vec![];
        let kind = next.token.kind();
        self.advance_md(&mut children, SKIP_NONE);
        (self.emit(kind, &children), None)
      }
      _ => self.parse_paragraph(),
    }
  }

  /// Parse an inline element.
  /// INVARIANT: Must not be at a Newline or EOF.
  pub(in crate::syntax::parse) fn parse_md_inline_element(
    &mut self,
  ) -> (GreenNode, Option<ExprCtx>) {
    debug_assert!(
      !matches!(
        self.lex_ctx.peek_md(SKIP_NONE).token.kind(),
        SyntaxKind::Newline | SyntaxKind::Eof
      ),
      "[ParseCtx::parse_md_inline_element] Must not be at Newline or EOF"
    );

    let next = self.lex_ctx.peek_md(SKIP_NONE);
    match next.token.kind() {
      SyntaxKind::LBracket => self.parse_link(),
      SyntaxKind::MdSymbol => {
        let text: String = next.token.chars().collect();
        match text.as_str() {
          "***" => self.parse_bold_italic(),
          "**" => self.parse_bold(),
          "*" | "_" => self.parse_italic(),
          "~~" => self.parse_strikethrough(),
          "!" => {
            let second = self.lex_ctx.peek_md_nth(1, SKIP_NONE);
            if second.token.kind() == SyntaxKind::LBracket {
              self.parse_media()
            } else {
              self.parse_text()
            }
          }
          _ => self.parse_text(),
        }
      }
      SyntaxKind::InterpStart => {
        let (fragment, early_exit) = self.parse_interp_fragment(0);
        (fragment, early_exit)
      }
      SyntaxKind::InlineMath | SyntaxKind::InlineCode => {
        let kind = next.token.kind();
        let mut children = vec![];
        self.advance_md(&mut children, SKIP_NONE);
        (self.emit(kind, &children), None)
      }
      _ => self.parse_text(),
    }
  }

  /// Parse a heading: `# ...`, `## ...`, etc.
  /// INVARIANT: The next token should be a hash sequence
  pub(in crate::syntax::parse) fn parse_heading(&mut self) -> (GreenNode, Option<ExprCtx>) {
    fn is_hash(token: &SyntaxToken) -> bool {
      token.kind() == SyntaxKind::MdSymbol && token.chars().all(|c| c == '#')
    }
    debug_assert!(
      is_hash(&self.lex_ctx.peek_md(SKIP_NONE).token),
      "[ParseCtx::parse_heading] Expect the next immediate token to be a hash"
    );
    let mut children = vec![];

    self.consume_md_if(
      &mut children,
      SKIP_NONE,
      is_hash,
      Diagnostic::MissingMarkdownHeadingHash {
        start_offset: self.offset(),
        end_offset: self.offset(),
      },
    );

    let next_token = &self.lex_ctx.peek_md(SKIP_NONE).token;
    if next_token.kind() != SyntaxKind::Whitespace {
      self.emit_diagnostic(Diagnostic::MissingRequiredSpacesBetweenHashAndHeading {
        start_offset: self.offset(),
        end_offset: self.offset(),
      });
    } else {
      self.advance_md(&mut children, SKIP_NONE);
    }

    // Require at least one inline element
    let has_inline = {
      let next = self.lex_ctx.peek_md(SKIP_WS);
      !matches!(next.token.kind(), SyntaxKind::Newline | SyntaxKind::Eof)
    };
    if !has_inline {
      self.emit_diagnostic(Diagnostic::MissingSyntaxNode {
        expected: SyntaxKind::MdText,
        start_offset: self.offset(),
        end_offset: self.offset(),
      });
    } else {
      // Parse inline elements until newline or EOF
      loop {
        let next_kind = self.lex_ctx.peek_md(SKIP_NONE).token.kind();
        if matches!(next_kind, SyntaxKind::Newline | SyntaxKind::Eof) {
          break;
        }
        let (inline, early_exit) = self.parse_md_inline_element();
        children.push(inline);
        if early_exit.is_some() {
          return (self.emit(SyntaxKind::MdHeading, &children), early_exit);
        }
      }
    }

    (self.emit(SyntaxKind::MdHeading, &children), None)
  }

  /// Parse a paragraph: consecutive non-blank text lines.
  /// INVARIANT: The current line is not blank (caller must ensure there is content).
  pub(in crate::syntax::parse) fn parse_paragraph(&mut self) -> (GreenNode, Option<ExprCtx>) {
    let mut children = vec![];

    loop {
      // Parse all inline elements on this line
      loop {
        let next_kind = self.lex_ctx.peek_md(SKIP_NONE).token.kind();
        if matches!(next_kind, SyntaxKind::Newline | SyntaxKind::Eof) {
          break;
        }
        let (inline, early_exit) = self.parse_md_inline_element();
        children.push(inline);
        if early_exit.is_some() {
          return (self.emit(SyntaxKind::MdParagraph, &children), early_exit);
        }
      }

      // Stop at EOF
      let next_kind = self.lex_ctx.peek_md(SKIP_NONE).token.kind();
      if next_kind == SyntaxKind::Eof {
        break;
      }

      let Some(after_prefix) = self.peek_md_newline_and_prefix() else {
        break;
      };
      let after = self.lex_ctx.peek_md_nth(after_prefix, SKIP_NONE);
      if matches!(after.token.kind(), SyntaxKind::Newline | SyntaxKind::Eof) {
        break;
      }
      if self.is_md_block_start_at(after_prefix) {
        break;
      }

      // Paragraph continues onto the next line, consume the newline
      self.advance_md(&mut children, SKIP_NONE);
    }

    (self.emit(SyntaxKind::MdParagraph, &children), None)
  }

  /// Parse a blockquote: `> ...`.
  /// INVARIANT: Expect the next token to be `>`
  pub(in crate::syntax::parse) fn parse_blockquote(&mut self) -> (GreenNode, Option<ExprCtx>) {
    debug_assert!(
      self.lex_ctx.peek_md(SKIP_NONE).token.kind() == SyntaxKind::MdSymbol
        && self
          .lex_ctx
          .peek_md(SKIP_NONE)
          .token
          .chars()
          .collect::<String>()
          == ">",
      "[ParseCtx::parse_blockquote] Expected >"
    );

    let mut children = vec![];

    self.expr_ctx_stack.enter(ExprCtx::MdBlockQuote);

    // Consume `>`
    self.advance_md(&mut children, SKIP_NONE);

    // Require a space after `>`
    if self.lex_ctx.peek_md(SKIP_NONE).token.kind() != SyntaxKind::Whitespace {
      self.emit_diagnostic(Diagnostic::MissingRequiredSpacesBetweenHashAndHeading {
        start_offset: self.offset(),
        end_offset: self.offset(),
      });
    } else {
      self.advance_md(&mut children, SKIP_NONE);
    }

    // Parse block elements until the blockquote ends
    loop {
      let next_kind = self.lex_ctx.peek_md(SKIP_NONE).token.kind();
      if next_kind == SyntaxKind::Eof {
        break;
      }
      if matches!(next_kind, SyntaxKind::Newline) {
        if self.peek_md_newline_and_prefix().is_none() {
          break;
        }
        self.consume_md_newline_and_prefix(&mut children);
        continue;
      }

      let (block, early_exit) = self.parse_md_block_element();
      children.push(block);
      if early_exit.is_some_and(|ctx| ctx != ExprCtx::MdBlockQuote) {
        self.expr_ctx_stack.exit(ExprCtx::MdBlockQuote);
        return (self.emit(SyntaxKind::MdBlockquote, &children), early_exit);
      }
      if early_exit == Some(ExprCtx::MdBlockQuote) {
        break;
      }
    }

    self.expr_ctx_stack.exit(ExprCtx::MdBlockQuote);
    (self.emit(SyntaxKind::MdBlockquote, &children), None)
  }

  /// Parse a table: `| ... | ... |`.
  /// INVARIANT: Next token must be MdSymbol `|`.
  pub(in crate::syntax::parse) fn parse_table(&mut self) -> (GreenNode, Option<ExprCtx>) {
    debug_assert!(
      self.lex_ctx.peek_md(SKIP_NONE).token.kind() == SyntaxKind::MdSymbol
        && self
          .lex_ctx
          .peek_md(SKIP_NONE)
          .token
          .chars()
          .collect::<String>()
          == "|",
      "[ParseCtx::parse_table] Expected |"
    );

    let mut children = vec![];

    self.expr_ctx_stack.enter(ExprCtx::MdTable);

    // Parse header row
    let (row, col_count, early_exit) = self.parse_table_row(false);
    let expected_cols = col_count;
    children.push(row);
    if early_exit.is_some_and(|ctx| ctx != ExprCtx::MdTable) {
      self.expr_ctx_stack.exit(ExprCtx::MdTable);
      return (self.emit(SyntaxKind::MdTable, &children), early_exit);
    }

    // Parse required separator row
    let sep_start = self.offset();
    if self.lex_ctx.peek_md(SKIP_NONE).token.kind() != SyntaxKind::Newline
      || !self.consume_md_newline_and_prefix(&mut children)
    {
      self.emit_diagnostic(Diagnostic::MissingTableSeparatorRow {
        start_offset: sep_start,
        end_offset: self.offset(),
      });
      self.expr_ctx_stack.exit(ExprCtx::MdTable);
      return (self.emit(SyntaxKind::MdTable, &children), None);
    }
    // Verify separator row starts with `|` followed by `-`
    let next = self.lex_ctx.peek_md(SKIP_NONE);
    let next2 = self.lex_ctx.peek_md_nth(1, SKIP_WS);
    let is_separator = next.token.kind() == SyntaxKind::MdSymbol
      && next.token.chars().collect::<String>() == "|"
      && next2.token.kind() == SyntaxKind::MdSymbol
      && next2.token.chars().collect::<String>().starts_with('-');
    if !is_separator {
      self.emit_diagnostic(Diagnostic::MissingTableSeparatorRow {
        start_offset: sep_start,
        end_offset: self.offset(),
      });
      self.expr_ctx_stack.exit(ExprCtx::MdTable);
      return (self.emit(SyntaxKind::MdTable, &children), None);
    }
    let (sep, early_exit) = self.parse_table_separator_row();
    children.push(sep);
    if early_exit.is_some_and(|ctx| ctx != ExprCtx::MdTable) {
      self.expr_ctx_stack.exit(ExprCtx::MdTable);
      return (self.emit(SyntaxKind::MdTable, &children), early_exit);
    }

    // Parse body rows
    loop {
      if self.lex_ctx.peek_md(SKIP_NONE).token.kind() != SyntaxKind::Newline {
        break;
      }
      if !self.consume_md_newline_and_prefix(&mut children) {
        break;
      }
      let next = self.lex_ctx.peek_md(SKIP_NONE);
      if next.token.kind() != SyntaxKind::MdSymbol || next.token.chars().collect::<String>() != "|"
      {
        break;
      }

      let row_start = self.offset();
      let (row, col_count, early_exit) = self.parse_table_row(true);
      if col_count != expected_cols {
        self.emit_diagnostic(Diagnostic::TableColumnCountMismatch {
          expected: expected_cols,
          found: col_count,
          start_offset: row_start,
          end_offset: self.offset(),
        });
      }
      children.push(row);
      if early_exit.is_some_and(|ctx| ctx != ExprCtx::MdTable) {
        self.expr_ctx_stack.exit(ExprCtx::MdTable);
        return (self.emit(SyntaxKind::MdTable, &children), early_exit);
      }
    }

    self.expr_ctx_stack.exit(ExprCtx::MdTable);
    (self.emit(SyntaxKind::MdTable, &children), None)
  }

  /// Parse a table row: `| cell | cell |`.
  /// Returns the node, cell count, and early exit context.
  /// INVARIANT: Next token must be MdSymbol `|`.
  fn parse_table_row(&mut self, is_data_row: bool) -> (GreenNode, usize, Option<ExprCtx>) {
    debug_assert!(
      self.lex_ctx.peek_md(SKIP_NONE).token.kind() == SyntaxKind::MdSymbol
        && self
          .lex_ctx
          .peek_md(SKIP_NONE)
          .token
          .chars()
          .collect::<String>()
          == "|",
      "[ParseCtx::parse_table_row] Expected |"
    );

    let row_kind = if is_data_row {
      SyntaxKind::MdTableDataRow
    } else {
      SyntaxKind::MdTableHeaderRow
    };

    let mut children = vec![];
    let mut cell_count = 0;

    self.expr_ctx_stack.enter(ExprCtx::MdTableRow);

    // Consume leading `|`
    self.advance_md(&mut children, SKIP_NONE);

    loop {
      // Check for end of row
      let next = self.lex_ctx.peek_md(SKIP_NONE);
      if matches!(next.token.kind(), SyntaxKind::Newline | SyntaxKind::Eof) {
        break;
      }

      // Parse a cell
      let (cell, early_exit) = self.parse_table_cell();
      children.push(cell);
      cell_count += 1;
      if early_exit.is_some_and(|ctx| ctx != ExprCtx::MdTableRow) {
        self.expr_ctx_stack.exit(ExprCtx::MdTableRow);
        return (self.emit(row_kind, &children), cell_count, early_exit);
      }

      // Consume `|` separator
      let next = self.lex_ctx.peek_md(SKIP_NONE);
      if next.token.kind() == SyntaxKind::MdSymbol && next.token.chars().collect::<String>() == "|"
      {
        self.advance_md(&mut children, SKIP_NONE);
      } else {
        break;
      }
    }

    self.expr_ctx_stack.exit(ExprCtx::MdTableRow);
    (self.emit(row_kind, &children), cell_count, None)
  }

  /// Parse a table separator row: `| --- | --- |`.
  /// INVARIANT: Next token must be MdSymbol `|`.
  fn parse_table_separator_row(&mut self) -> (GreenNode, Option<ExprCtx>) {
    let mut children = vec![];

    self.expr_ctx_stack.enter(ExprCtx::MdTableRow);

    // Consume everything until Newline or EOF
    loop {
      let next = self.lex_ctx.peek_md(SKIP_NONE);
      if matches!(next.token.kind(), SyntaxKind::Newline | SyntaxKind::Eof) {
        break;
      }
      self.advance_md(&mut children, SKIP_NONE);
    }

    self.expr_ctx_stack.exit(ExprCtx::MdTableRow);
    (self.emit(SyntaxKind::MdTableSeparatorRow, &children), None)
  }

  /// Parse a table cell: inline content until `|` or end of line.
  fn parse_table_cell(&mut self) -> (GreenNode, Option<ExprCtx>) {
    let mut children = vec![];

    self.expr_ctx_stack.enter(ExprCtx::MdTableCell);

    // Skip leading whitespace
    if self.lex_ctx.peek_md(SKIP_NONE).token.kind() == SyntaxKind::Whitespace {
      self.advance_md(&mut children, SKIP_NONE);
    }

    loop {
      let next = self.lex_ctx.peek_md(SKIP_NONE);
      // End on `|`, Newline, or EOF
      if matches!(next.token.kind(), SyntaxKind::Newline | SyntaxKind::Eof) {
        break;
      }
      if next.token.kind() == SyntaxKind::MdSymbol && next.token.chars().collect::<String>() == "|"
      {
        break;
      }

      let (inline, early_exit) = self.parse_md_inline_element();
      children.push(inline);
      if early_exit.is_some_and(|ctx| ctx != ExprCtx::MdTableCell) {
        self.expr_ctx_stack.exit(ExprCtx::MdTableCell);
        return (self.emit(SyntaxKind::MdTableCell, &children), early_exit);
      }
      if early_exit == Some(ExprCtx::MdTableCell) {
        break;
      }
    }

    self.expr_ctx_stack.exit(ExprCtx::MdTableCell);
    (self.emit(SyntaxKind::MdTableCell, &children), None)
  }

  /// Parse a bullet list: `- ...` or `* ...` or `+ ...`.
  /// INVARIANT: Must be after prefix. Next token must be MdSymbol `-`, `*`, or `+`.
  pub(in crate::syntax::parse) fn parse_bullet_list(&mut self) -> (GreenNode, Option<ExprCtx>) {
    debug_assert!(
      {
        let peek = self.lex_ctx.peek_md(SKIP_NONE);
        peek.token.kind() == SyntaxKind::MdSymbol && {
          let text: String = peek.token.chars().collect();
          text == "-" || text == "*" || text == "+"
        }
      },
      "[ParseCtx::parse_bullet_list] Expected -, *, or +"
    );

    let mut children = vec![];
    let bullet: String = self.lex_ctx.peek_md(SKIP_NONE).token.chars().collect();

    self.expr_ctx_stack.enter(ExprCtx::MdUnorderedList);

    // Parse first list item
    let (item, early_exit) = self.parse_next_bullet_item(&bullet);
    children.push(item);
    if early_exit.is_some_and(|ctx| ctx != ExprCtx::MdUnorderedList) {
      self.expr_ctx_stack.exit(ExprCtx::MdUnorderedList);
      return (self.emit(SyntaxKind::MdBulletList, &children), early_exit);
    }

    // Parse remaining list items
    loop {
      // Consume newline + prefix, then check if next line starts another item with the same bullet
      if !self.consume_md_newline_and_prefix(&mut children) {
        break;
      }
      let next = self.lex_ctx.peek_md(SKIP_NONE);
      if next.token.kind() != SyntaxKind::MdSymbol {
        break;
      }
      let text: String = next.token.chars().collect();
      if text != bullet {
        break;
      }

      let (item, early_exit) = self.parse_next_bullet_item(&bullet);
      children.push(item);
      if early_exit.is_some_and(|ctx| ctx != ExprCtx::MdUnorderedList) {
        self.expr_ctx_stack.exit(ExprCtx::MdUnorderedList);
        return (self.emit(SyntaxKind::MdBulletList, &children), early_exit);
      }
    }

    self.expr_ctx_stack.exit(ExprCtx::MdUnorderedList);
    (self.emit(SyntaxKind::MdBulletList, &children), None)
  }

  fn parse_next_bullet_item(&mut self, bullet: &str) -> (GreenNode, Option<ExprCtx>) {
    // Check if a checkbox follows the bullet and space
    if self.lex_ctx.peek_md_nth(2, SKIP_NONE).token.kind() == SyntaxKind::LBracket {
      let inner: String = self
        .lex_ctx
        .peek_md_nth(3, SKIP_NONE)
        .token
        .chars()
        .collect();
      if (inner == " " || inner.to_lowercase() == "x")
        && self.lex_ctx.peek_md_nth(4, SKIP_NONE).token.kind() == SyntaxKind::RBracket
      {
        return self.parse_task_list_item(bullet);
      }
    }
    self.parse_bullet_list_item(bullet)
  }

  /// Parse a bullet list item: `- content` or `* content` or `+ content`.
  /// INVARIANT: Next token must be the bullet marker matching `bullet`.
  fn parse_bullet_list_item(&mut self, bullet: &str) -> (GreenNode, Option<ExprCtx>) {
    debug_assert!(
      {
        let peek = self.lex_ctx.peek_md(SKIP_NONE);
        peek.token.kind() == SyntaxKind::MdSymbol
          && peek.token.chars().collect::<String>() == bullet
      },
      "[ParseCtx::parse_bullet_list_item] Expected bullet marker"
    );

    let mut children = vec![];

    self.expr_ctx_stack.enter(ExprCtx::MdUnorderedListItem);

    // Consume the bullet marker
    self.advance_md(&mut children, SKIP_NONE);

    // Require a space after the bullet
    if self.lex_ctx.peek_md(SKIP_NONE).token.kind() != SyntaxKind::Whitespace {
      self.emit_diagnostic(Diagnostic::MissingRequiredSpacesBetweenHashAndHeading {
        start_offset: self.offset(),
        end_offset: self.offset(),
      });
    } else {
      self.advance_md(&mut children, SKIP_NONE);
    }

    // Parse block elements until end of list item
    loop {
      let next_kind = self.lex_ctx.peek_md(SKIP_NONE).token.kind();
      if next_kind == SyntaxKind::Eof {
        break;
      }
      if next_kind == SyntaxKind::Newline {
        let Some(after_prefix) = self.peek_md_newline_and_prefix() else {
          break;
        };
        // Peek past prefix to check for sibling bullet
        let after = self.lex_ctx.peek_md_nth(after_prefix, SKIP_NONE);
        if after.token.kind() == SyntaxKind::MdSymbol
          && after.token.chars().collect::<String>() == bullet
        {
          break;
        }
        self.consume_md_newline_and_prefix(&mut children);
        let next = self.lex_ctx.peek_md(SKIP_NONE);
        if matches!(next.token.kind(), SyntaxKind::Newline | SyntaxKind::Eof) {
          break;
        }
        continue;
      }

      let (block, early_exit) = self.parse_md_block_element();
      children.push(block);
      if early_exit.is_some_and(|ctx| ctx != ExprCtx::MdUnorderedListItem) {
        self.expr_ctx_stack.exit(ExprCtx::MdUnorderedListItem);
        return (
          self.emit(SyntaxKind::MdBulletListItem, &children),
          early_exit,
        );
      }
      if early_exit == Some(ExprCtx::MdUnorderedListItem) {
        break;
      }
    }

    self.expr_ctx_stack.exit(ExprCtx::MdUnorderedListItem);
    (self.emit(SyntaxKind::MdBulletListItem, &children), None)
  }

  fn parse_task_list_item(&mut self, bullet: &str) -> (GreenNode, Option<ExprCtx>) {
    let mut children = vec![];

    self.expr_ctx_stack.enter(ExprCtx::MdTaskListItem);

    // Consume the bullet marker
    self.advance_md(&mut children, SKIP_NONE);

    // Consume the space after the bullet
    if self.lex_ctx.peek_md(SKIP_NONE).token.kind() == SyntaxKind::Whitespace {
      self.advance_md(&mut children, SKIP_NONE);
    }

    // Consume the checkbox: `[` inner `]`
    let mut checkbox_children = vec![];
    self.advance_md(&mut checkbox_children, SKIP_NONE); // `[`
    self.advance_md(&mut checkbox_children, SKIP_NONE); // ` ` or `x`
    self.advance_md(&mut checkbox_children, SKIP_NONE); // `]`
    children.push(self.emit(SyntaxKind::MdCheckbox, &checkbox_children));

    // Parse block elements until end of task list item (same logic as bullet list item)
    loop {
      let next_kind = self.lex_ctx.peek_md(SKIP_NONE).token.kind();
      if next_kind == SyntaxKind::Eof {
        break;
      }
      if next_kind == SyntaxKind::Newline {
        let Some(after_prefix) = self.peek_md_newline_and_prefix() else {
          break;
        };
        // Peek past prefix to check for sibling bullet
        let after = self.lex_ctx.peek_md_nth(after_prefix, SKIP_NONE);
        if after.token.kind() == SyntaxKind::MdSymbol
          && after.token.chars().collect::<String>() == bullet
        {
          break;
        }
        self.consume_md_newline_and_prefix(&mut children);
        let next = self.lex_ctx.peek_md(SKIP_NONE);
        if matches!(next.token.kind(), SyntaxKind::Newline | SyntaxKind::Eof) {
          break;
        }
        continue;
      }

      let (block, early_exit) = self.parse_md_block_element();
      children.push(block);
      if early_exit.is_some_and(|ctx| ctx != ExprCtx::MdTaskListItem) {
        self.expr_ctx_stack.exit(ExprCtx::MdTaskListItem);
        return (self.emit(SyntaxKind::MdTaskListItem, &children), early_exit);
      }
      if early_exit == Some(ExprCtx::MdTaskListItem) {
        break;
      }
    }

    self.expr_ctx_stack.exit(ExprCtx::MdTaskListItem);
    (self.emit(SyntaxKind::MdTaskListItem, &children), None)
  }

  /// Parse an ordered list: `1. ...`.
  /// INVARIANT: The next tokens must be MdNumber and MdSymbol dot.
  pub(in crate::syntax::parse) fn parse_ordered_list(&mut self) -> (GreenNode, Option<ExprCtx>) {
    debug_assert!(
      self.lex_ctx.peek_md(SKIP_NONE).token.kind() == SyntaxKind::MdNumber,
      "[ParseCtx::parse_ordered_list] Expected MdNumber"
    );
    debug_assert!(
      self.lex_ctx.peek_md_nth(1, SKIP_NONE).token.kind() == SyntaxKind::MdSymbol
        && self
          .lex_ctx
          .peek_md_nth(1, SKIP_NONE)
          .token
          .chars()
          .collect::<String>()
          == ".",
      "[ParseCtx::parse_ordered_list] Expected . after MdNumber"
    );

    let mut children = vec![];

    self.expr_ctx_stack.enter(ExprCtx::MdOrderedList);

    // Parse first list item
    let (item, early_exit) = self.parse_ordered_list_item();
    children.push(item);
    if early_exit.is_some_and(|ctx| ctx != ExprCtx::MdOrderedList) {
      self.expr_ctx_stack.exit(ExprCtx::MdOrderedList);
      return (self.emit(SyntaxKind::MdOrderedList, &children), early_exit);
    }

    // Parse remaining list items
    loop {
      // Consume newline + prefix, then check for next item
      if !self.consume_md_newline_and_prefix(&mut children) {
        break;
      }
      let next = self.lex_ctx.peek_md(SKIP_NONE);
      if next.token.kind() != SyntaxKind::MdNumber {
        break;
      }
      // Verify `.` follows the number
      let dot = self.lex_ctx.peek_md_nth(1, SKIP_NONE);
      if dot.token.kind() != SyntaxKind::MdSymbol || dot.token.chars().collect::<String>() != "." {
        break;
      }

      let (item, early_exit) = self.parse_ordered_list_item();
      children.push(item);
      if early_exit.is_some_and(|ctx| ctx != ExprCtx::MdOrderedList) {
        self.expr_ctx_stack.exit(ExprCtx::MdOrderedList);
        return (self.emit(SyntaxKind::MdOrderedList, &children), early_exit);
      }
    }

    self.expr_ctx_stack.exit(ExprCtx::MdOrderedList);
    (self.emit(SyntaxKind::MdOrderedList, &children), None)
  }

  /// Parse an ordered list item: `1. content`.
  /// INVARIANT: Next token must be MdNumber followed by `.`.
  fn parse_ordered_list_item(&mut self) -> (GreenNode, Option<ExprCtx>) {
    debug_assert!(
      self.lex_ctx.peek_md(SKIP_NONE).token.kind() == SyntaxKind::MdNumber,
      "[ParseCtx::parse_ordered_list_item] Expected MdNumber"
    );

    let mut children = vec![];

    self.expr_ctx_stack.enter(ExprCtx::MdOrderedListItem);

    // Consume the number
    self.advance_md(&mut children, SKIP_NONE);

    // Consume `.`
    self.consume_md_if(
      &mut children,
      SKIP_NONE,
      |token| token.kind() == SyntaxKind::MdSymbol && token.chars().collect::<String>() == ".",
      Diagnostic::MissingSyntaxNode {
        expected: SyntaxKind::MdOrderedListItem,
        start_offset: self.offset(),
        end_offset: self.offset(),
      },
    );

    // Require a space after `.`
    if self.lex_ctx.peek_md(SKIP_NONE).token.kind() != SyntaxKind::Whitespace {
      self.emit_diagnostic(Diagnostic::MissingRequiredSpacesBetweenHashAndHeading {
        start_offset: self.offset(),
        end_offset: self.offset(),
      });
    } else {
      self.advance_md(&mut children, SKIP_NONE);
    }

    // Parse block elements until end of list item
    loop {
      let next_kind = self.lex_ctx.peek_md(SKIP_NONE).token.kind();
      if next_kind == SyntaxKind::Eof {
        break;
      }
      if next_kind == SyntaxKind::Newline {
        let Some(after_prefix) = self.peek_md_newline_and_prefix() else {
          break;
        };
        // Peek past prefix to check for sibling ordered item
        let after = self.lex_ctx.peek_md_nth(after_prefix, SKIP_NONE);
        if after.token.kind() == SyntaxKind::MdNumber {
          let dot = self.lex_ctx.peek_md_nth(after_prefix + 1, SKIP_NONE);
          if dot.token.kind() == SyntaxKind::MdSymbol
            && dot.token.chars().collect::<String>() == "."
          {
            break;
          }
        }
        self.consume_md_newline_and_prefix(&mut children);
        let next = self.lex_ctx.peek_md(SKIP_NONE);
        if matches!(next.token.kind(), SyntaxKind::Newline | SyntaxKind::Eof) {
          break;
        }
        continue;
      }

      let (block, early_exit) = self.parse_md_block_element();
      children.push(block);
      if early_exit.is_some_and(|ctx| ctx != ExprCtx::MdOrderedListItem) {
        self.expr_ctx_stack.exit(ExprCtx::MdOrderedListItem);
        return (
          self.emit(SyntaxKind::MdOrderedListItem, &children),
          early_exit,
        );
      }
      if early_exit == Some(ExprCtx::MdOrderedListItem) {
        break;
      }
    }

    self.expr_ctx_stack.exit(ExprCtx::MdOrderedListItem);
    (self.emit(SyntaxKind::MdOrderedListItem, &children), None)
  }

  /// Parse a toggle list: `>- ...`.
  /// INVARIANT: Next tokens must be MdSymbol `>` followed by MdSymbol `-`.
  pub(in crate::syntax::parse) fn parse_toggle_list(&mut self) -> (GreenNode, Option<ExprCtx>) {
    debug_assert!(
      self.lex_ctx.peek_md(SKIP_NONE).token.kind() == SyntaxKind::MdSymbol
        && self
          .lex_ctx
          .peek_md(SKIP_NONE)
          .token
          .chars()
          .collect::<String>()
          == ">"
        && self.lex_ctx.peek_md_nth(1, SKIP_NONE).token.kind() == SyntaxKind::MdSymbol
        && self
          .lex_ctx
          .peek_md_nth(1, SKIP_NONE)
          .token
          .chars()
          .collect::<String>()
          == "-",
      "[ParseCtx::parse_toggle_list] Expected > followed by -"
    );

    let mut children = vec![];

    self.expr_ctx_stack.enter(ExprCtx::MdToggleList);

    // Parse first toggle item
    let (item, early_exit) = self.parse_toggle_list_item();
    children.push(item);
    if early_exit.is_some_and(|ctx| ctx != ExprCtx::MdToggleList) {
      self.expr_ctx_stack.exit(ExprCtx::MdToggleList);
      return (self.emit(SyntaxKind::MdToggleList, &children), early_exit);
    }

    // Parse remaining toggle items
    loop {
      if !self.consume_md_newline_and_prefix(&mut children) {
        break;
      }
      let next = self.lex_ctx.peek_md(SKIP_NONE);
      let next_next = self.lex_ctx.peek_md_nth(1, SKIP_NONE);
      if next.token.kind() != SyntaxKind::MdSymbol
        || next.token.chars().collect::<String>() != ">"
        || next_next.token.kind() != SyntaxKind::MdSymbol
        || next_next.token.chars().collect::<String>() != "-"
      {
        break;
      }

      let (item, early_exit) = self.parse_toggle_list_item();
      children.push(item);
      if early_exit.is_some_and(|ctx| ctx != ExprCtx::MdToggleList) {
        self.expr_ctx_stack.exit(ExprCtx::MdToggleList);
        return (self.emit(SyntaxKind::MdToggleList, &children), early_exit);
      }
    }

    self.expr_ctx_stack.exit(ExprCtx::MdToggleList);
    (self.emit(SyntaxKind::MdToggleList, &children), None)
  }

  /// Parse a toggle list item: `>- summary\n\n   details`.
  /// INVARIANT: Next token must be MdSymbol `>-`.
  fn parse_toggle_list_item(&mut self) -> (GreenNode, Option<ExprCtx>) {
    debug_assert!(
      self.lex_ctx.peek_md(SKIP_NONE).token.kind() == SyntaxKind::MdSymbol
        && self
          .lex_ctx
          .peek_md(SKIP_NONE)
          .token
          .chars()
          .collect::<String>()
          == ">"
        && self.lex_ctx.peek_md_nth(1, SKIP_NONE).token.kind() == SyntaxKind::MdSymbol
        && self
          .lex_ctx
          .peek_md_nth(1, SKIP_NONE)
          .token
          .chars()
          .collect::<String>()
          == "-",
      "[ParseCtx::parse_toggle_list_item] Expected > followed by -"
    );

    let mut children = vec![];

    self.expr_ctx_stack.enter(ExprCtx::MdToggleListItem);

    // Consume `>` and `-`
    self.advance_md(&mut children, SKIP_NONE);
    self.advance_md(&mut children, SKIP_NONE);

    // Require a space after `>-`
    if self.lex_ctx.peek_md(SKIP_NONE).token.kind() != SyntaxKind::Whitespace {
      self.emit_diagnostic(Diagnostic::MissingRequiredSpacesBetweenHashAndHeading {
        start_offset: self.offset(),
        end_offset: self.offset(),
      });
    } else {
      self.advance_md(&mut children, SKIP_NONE);
    }

    // Parse summary: inline elements on this line
    let mut summary_children = vec![];
    loop {
      let next_kind = self.lex_ctx.peek_md(SKIP_NONE).token.kind();
      if matches!(next_kind, SyntaxKind::Newline | SyntaxKind::Eof) {
        break;
      }
      let (inline, early_exit) = self.parse_md_inline_element();
      summary_children.push(inline);
      if early_exit.is_some() {
        children.push(self.emit(SyntaxKind::MdToggleListSummary, &summary_children));
        self.expr_ctx_stack.exit(ExprCtx::MdToggleListItem);
        return (
          self.emit(SyntaxKind::MdToggleListItem, &children),
          early_exit,
        );
      }
    }
    children.push(self.emit(SyntaxKind::MdToggleListSummary, &summary_children));

    // Check for blank line separating summary from details
    let next_kind = self.lex_ctx.peek_md(SKIP_NONE).token.kind();
    if next_kind == SyntaxKind::Eof {
      self.expr_ctx_stack.exit(ExprCtx::MdToggleListItem);
      return (self.emit(SyntaxKind::MdToggleListItem, &children), None);
    }

    // Consume the newline after summary
    self.advance_md(&mut children, SKIP_NONE);

    if !self.consume_md_blank_line(&mut children) {
      self.expr_ctx_stack.exit(ExprCtx::MdToggleListItem);
      return (self.emit(SyntaxKind::MdToggleListItem, &children), None);
    }

    // Parse details: block elements until end of toggle item
    let mut details_children = vec![];
    loop {
      if !self.consume_md_newline_and_prefix(&mut children) {
        break;
      }
      let next = self.lex_ctx.peek_md(SKIP_NONE);
      if matches!(next.token.kind(), SyntaxKind::Newline | SyntaxKind::Eof) {
        break;
      }

      let (block, early_exit) = self.parse_md_block_element();
      details_children.push(block);
      if early_exit.is_some_and(|ctx| ctx != ExprCtx::MdToggleListItem) {
        children.push(self.emit(SyntaxKind::MdToggleListDetails, &details_children));
        self.expr_ctx_stack.exit(ExprCtx::MdToggleListItem);
        return (
          self.emit(SyntaxKind::MdToggleListItem, &children),
          early_exit,
        );
      }
      if early_exit == Some(ExprCtx::MdToggleListItem) {
        break;
      }
    }
    if !details_children.is_empty() {
      children.push(self.emit(SyntaxKind::MdToggleListDetails, &details_children));
    }

    self.expr_ctx_stack.exit(ExprCtx::MdToggleListItem);
    (self.emit(SyntaxKind::MdToggleListItem, &children), None)
  }

  /// Parse a container block: `::: label ... :::`.
  /// INVARIANT: Expect ::: to be the next token, all spaces must already be consumed and passed
  pub(in crate::syntax::parse) fn parse_container_block(&mut self) -> (GreenNode, Option<ExprCtx>) {
    debug_assert!(
      self.lex_ctx.peek_md(SKIP_NONE).token.kind() == SyntaxKind::MdSymbol
        && self
          .lex_ctx
          .peek_md(SKIP_NONE)
          .token
          .chars()
          .collect::<String>()
          == ":::",
      "[ParseCtx::parse_container_block] Expected :::"
    );

    let mut children = vec![];
    let open_offset = self.offset();

    // Consume `:::`
    self.advance_md(&mut children, SKIP_NONE);

    let parent_prefix_count = self.expr_ctx_stack.md_prefix_tokens().len();
    let container_ctx = ExprCtx::MdContainerBlock(parent_prefix_count as u16);
    self.expr_ctx_stack.enter(container_ctx);

    // Require a space between `:::` and the label
    let next = self.lex_ctx.peek_md(SKIP_NONE);
    if next.token.kind() != SyntaxKind::Whitespace {
      self.emit_diagnostic(Diagnostic::MissingRequiredSpacesBetweenHashAndHeading {
        start_offset: self.offset(),
        end_offset: self.offset(),
      });
    } else {
      self.advance_md(&mut children, SKIP_NONE);
    }

    // Require a label identifier
    if self.lex_ctx.peek_md(SKIP_NONE).token.kind() != SyntaxKind::Ident {
      self.emit_diagnostic(Diagnostic::MissingSyntaxNode {
        expected: SyntaxKind::Ident,
        start_offset: self.offset(),
        end_offset: self.offset(),
      });
    } else {
      self.advance_md(&mut children, SKIP_NONE);
    }

    // Container props (`{key=value key=value}`)
    if self.lex_ctx.peek_md(SKIP_WS).token.kind() == SyntaxKind::LBrace {
      let (props, early_exit) = self.parse_container_prop_block();
      children.push(props);
      if early_exit.is_some_and(|ctx| ctx != container_ctx) {
        self.expr_ctx_stack.exit(container_ctx);
        return (
          self.emit(SyntaxKind::MdContainerBlock, &children),
          early_exit,
        );
      }
      if early_exit == Some(container_ctx)
        && let Some(ctx) = self.synchronize_container_block(&mut children)
      {
        self.expr_ctx_stack.exit(container_ctx);
        return (
          self.emit(SyntaxKind::MdContainerBlock, &children),
          Some(ctx),
        );
      }
    }

    // Consume optional title text after the label until newline
    loop {
      let next = self.lex_ctx.peek_md(SKIP_NONE);

      if next.token.kind() == SyntaxKind::Newline || next.token.kind() == SyntaxKind::Eof {
        break;
      }
      self.advance_md(&mut children, SKIP_NONE);
    }

    // Consume the newline after the label/title
    self.consume_md(
      &mut children,
      SKIP_NONE,
      SyntaxKind::Newline,
      Diagnostic::MissingSyntaxNode {
        expected: SyntaxKind::Newline,
        start_offset: self.offset(),
        end_offset: self.offset(),
      },
    );

    // Parse block elements until closing `:::` or EOF
    // The container creates a new indentation context: inner elements start at indent 0
    loop {
      let next_kind = self.lex_ctx.peek_md(SKIP_NONE).token.kind();
      if next_kind == SyntaxKind::Eof {
        break;
      }

      // Check for closing `:::` at the same indentation level as the opening
      // The closing `:::` should appear right after the parent prefix
      let check_pos = if next_kind == SyntaxKind::Newline {
        match self.peek_md_newline_and_prefix() {
          Some(pos) => pos,
          None => break,
        }
      } else {
        parent_prefix_count
      };

      let after = self.lex_ctx.peek_md_nth(check_pos, SKIP_NONE);
      if after.token.kind() == SyntaxKind::MdSymbol
        && matches!(after.token.chars().collect::<String>().as_str(), ":::")
      {
        break;
      }

      let (slot, early_exit) = self.parse_container_slot();
      children.push(slot);

      if early_exit.is_some_and(|ctx| ctx != container_ctx) {
        self.expr_ctx_stack.exit(container_ctx);
        return (
          self.emit(SyntaxKind::MdContainerBlock, &children),
          early_exit,
        );
      } else if early_exit.is_some() {
        let mut error_children = vec![];
        let ctx = self.consume_or_delegate_md(container_ctx, &mut error_children);
        if !error_children.is_empty() {
          children.push(self.emit(SyntaxKind::Error, &error_children));
        }
        if ctx.is_some() {
          self.expr_ctx_stack.exit(container_ctx);
          return (
            self.emit(SyntaxKind::MdContainerBlock, &children),
            early_exit,
          );
        }
      }

      // Eat the newline
      if self.lex_ctx.peek_md(SKIP_WS).token.kind() == SyntaxKind::Newline {
        self.advance_md(&mut children, SKIP_WS);
      }

      let next = self.lex_ctx.peek_md_nth(parent_prefix_count, SKIP_NONE);
      if next.token.kind() == SyntaxKind::MdSymbol
        && matches!(next.token.chars().collect::<String>().as_str(), "===")
      {
        if !self.consume_md_prefix(&mut children) {
          continue;
        }

        let mut sep_children = vec![];
        self.advance_md(&mut sep_children, SKIP_NONE); // Consume ===

        // Consume Ident
        let start_offset = self.offset();
        let end_offset = start_offset + self.lex_ctx.peek_md(SKIP_WS).token.text_len();
        self.consume_md_if(
          &mut sep_children,
          SKIP_WS,
          |token| token.kind() == SyntaxKind::Ident,
          Diagnostic::MissingSyntaxNode {
            expected: SyntaxKind::Ident,
            start_offset,
            end_offset,
          },
        );

        // Consume redundant tokens till end of line
        let mut error_children = vec![];
        while !matches!(
          self.lex_ctx.peek_md(SKIP_WS).token.kind(),
          SyntaxKind::Newline | SyntaxKind::Eof
        ) {
          let start_offset = self.offset();
          let end_offset = start_offset + self.lex_ctx.peek_md(SKIP_WS).token.text_len();
          self.advance_md(&mut error_children, SKIP_WS);
          self.emit_diagnostic(Diagnostic::UnexpectedContainerSlotSeparatorToken {
            start_offset,
            end_offset,
          })
        }
        if !error_children.is_empty() {
          let error_node = self.emit(SyntaxKind::Error, &error_children);
          sep_children.push(error_node);
        }

        // Consume the newline of the separator
        if self.lex_ctx.peek_md(SKIP_WS).token.kind() == SyntaxKind::Newline {
          self.advance_md(&mut sep_children, SKIP_WS);
        }

        let separator = self.emit(SyntaxKind::MdContainerSlotSeparator, &sep_children);
        children.push(separator);
      }
    }

    // Consume closing `:::`
    self.consume_md_if(
      &mut children,
      SKIP_WS,
      |token| token.kind() == SyntaxKind::MdSymbol && token.chars().collect::<String>() == ":::",
      Diagnostic::MissingSyntaxNode {
        expected: SyntaxKind::MdContainerBlock,
        start_offset: open_offset,
        end_offset: self.offset(),
      },
    );

    self.expr_ctx_stack.exit(container_ctx);
    (self.emit(SyntaxKind::MdContainerBlock, &children), None)
  }

  // Stop on `:::` at matching indent, or EOF.
  fn synchronize_container_block(&mut self, children: &mut Vec<GreenNode>) -> Option<ExprCtx> {
    let current = self.expr_ctx_stack.current().unwrap();
    let mut error_children = vec![];
    let result = loop {
      let peek = self.lex_ctx.peek_md(SKIP_WS);
      let is_closing = peek.token.kind() == SyntaxKind::MdSymbol
        && peek.token.chars().collect::<String>() == ":::";
      if is_closing || peek.token.kind() == SyntaxKind::Eof {
        break None;
      }
      if let Some(ctx) = self.consume_or_delegate_md(current, &mut error_children) {
        break Some(ctx);
      }
    };
    if !error_children.is_empty() {
      children.push(self.emit(SyntaxKind::Error, &error_children));
    }
    result
  }

  /// Parse a container props block: `{key=value}`.
  /// INVARIANT: Expect { to be the next NON-WHITESPACE token
  pub(in crate::syntax::parse) fn parse_container_prop_block(
    &mut self,
  ) -> (GreenNode, Option<ExprCtx>) {
    debug_assert!(
      self.lex_ctx.peek_md(SKIP_WS).token.kind() == SyntaxKind::LBrace,
      "[ParseCtx::parse_container_prop_block] Expected {{",
    );

    let mut children = vec![];

    self.expr_ctx_stack.enter(ExprCtx::MdContainerPropBlock);

    // Consume `{`
    self.advance_md(&mut children, SKIP_WS);

    loop {
      // Consume all trivia
      let next_token = loop {
        let next = self.lex_ctx.peek_md(SKIP_NONE);
        if !matches!(
          next.token.kind(),
          SyntaxKind::Newline | SyntaxKind::Whitespace
        ) {
          break next;
        }
        self.advance_md(&mut children, SKIP_NONE);
      };

      if matches!(
        next_token.token.kind(),
        SyntaxKind::Eof | SyntaxKind::RBrace
      ) {
        break;
      }

      // Look for possible key
      if !matches!(next_token.token.kind(), SyntaxKind::Ident) {
        let mut error_children = vec![];

        let start_offset = self.offset();

        let early_exit =
          self.consume_or_delegate_md(ExprCtx::MdContainerPropBlock, &mut error_children);

        // No match, emit diagnostic
        self.emit_diagnostic(Diagnostic::UnexpectedContainerPropItem {
          start_offset,
          end_offset: self.offset(),
        });

        let error_node = self.emit(SyntaxKind::Error, &error_children);

        children.push(error_node);

        if early_exit.is_some() {
          let start_offset = self.offset();
          let end_offset = self.offset() + self.lex_ctx.peek_md(SKIP_WCN).token.text_len();
          self.emit_diagnostic(Diagnostic::UnclosedContainerPropBlock {
            start_offset,
            end_offset,
          });

          self.expr_ctx_stack.exit(ExprCtx::MdContainerPropBlock);
          return (
            self.emit(SyntaxKind::MdContainerPropBlock, &children),
            early_exit,
          );
        }
        continue;
      }

      let (prop_item, early_exit) = self.parse_container_prop_item();
      children.push(prop_item);

      if early_exit.is_some_and(|ctx| ctx != ExprCtx::MdContainerPropBlock) {
        self.expr_ctx_stack.exit(ExprCtx::MdContainerPropBlock);
        return (
          self.emit(SyntaxKind::MdContainerPropBlock, &children),
          early_exit,
        );
      } else if early_exit.is_some()
        && let Some(ctx) = self.consume_or_delegate_md(ExprCtx::MdContainerPropBlock, &mut children)
      {
        let start_offset = self.offset();
        let end_offset = self.offset() + self.lex_ctx.peek_md(SKIP_WCN).token.text_len();
        self.emit_diagnostic(Diagnostic::UnclosedContainerPropBlock {
          start_offset,
          end_offset,
        });

        self.expr_ctx_stack.exit(ExprCtx::MdContainerPropBlock);
        return (
          self.emit(SyntaxKind::MdContainerPropBlock, &children),
          Some(ctx),
        );
      }
    }

    let start_offset = self.offset();
    let end_offset = self.offset() + self.lex_ctx.peek_md(SKIP_WCN).token.text_len();
    self.consume_md(
      &mut children,
      SKIP_WCN,
      SyntaxKind::RBrace,
      Diagnostic::UnclosedContainerPropBlock {
        start_offset,
        end_offset,
      },
    );

    self.expr_ctx_stack.exit(ExprCtx::MdContainerPropBlock);

    (self.emit(SyntaxKind::MdContainerPropBlock, &children), None)
  }

  /// Parse a container prop item: `key=value`.
  /// INVARIANT: Expect ident to be the next NON-WHITESPACE token
  pub(in crate::syntax::parse) fn parse_container_prop_item(
    &mut self,
  ) -> (GreenNode, Option<ExprCtx>) {
    debug_assert!(
      self.lex_ctx.peek_md(SKIP_WCN).token.kind() == SyntaxKind::Ident,
      "[ParseCtx::parse_container_prop_item] Expected prop item",
    );

    let mut children = vec![];

    self.expr_ctx_stack.enter(ExprCtx::MdContainerPropItem);

    // Consume identifier
    self.advance_md(&mut children, SKIP_WCN);

    let maybe_eq_token = self.lex_ctx.peek_md(SKIP_NONE);
    if maybe_eq_token.token.kind() != SyntaxKind::MdSymbol
      || !maybe_eq_token.token.text().is_some_and(|t| t == "=")
    {
      self.expr_ctx_stack.exit(ExprCtx::MdContainerPropItem);

      return (self.emit(SyntaxKind::MdContainerPropItem, &children), None); // It's ok to not have eq
    }

    self.advance_md(&mut children, SKIP_NONE); // Consume eq

    // Number value
    if self.lex_ctx.peek_md(SKIP_NONE).token.kind() == SyntaxKind::MdNumber {
      self.advance_md(&mut children, SKIP_NONE);

      self.expr_ctx_stack.exit(ExprCtx::MdContainerPropItem);

      return (self.emit(SyntaxKind::MdContainerPropItem, &children), None);
    }

    // Double quoted and single quoted string
    let quote = self.lex_ctx.peek_md(SKIP_NONE).token.kind();
    if matches!(quote, SyntaxKind::DqStrStart | SyntaxKind::SqStrStart) {
      let closing = if quote == SyntaxKind::DqStrStart {
        SyntaxKind::DqStrEnd
      } else {
        SyntaxKind::SqStrEnd
      };
      self.advance_md(&mut children, SKIP_NONE); // opening quote

      // Consume everything up to the matching closing quote
      let value_start = self.offset();
      loop {
        let kind = self.lex_ctx.peek_md(SKIP_NONE).token.kind();
        if kind == closing {
          self.advance_md(&mut children, SKIP_NONE);
          break;
        }
        if matches!(
          kind,
          SyntaxKind::Eof | SyntaxKind::Newline | SyntaxKind::RBrace
        ) {
          self.emit_diagnostic(Diagnostic::UnexpectedContainerPropValue {
            start_offset: value_start,
            end_offset: self.offset(),
          });
          break;
        }
        self.advance_md(&mut children, SKIP_NONE);
      }

      self.expr_ctx_stack.exit(ExprCtx::MdContainerPropItem);
      return (self.emit(SyntaxKind::MdContainerPropItem, &children), None);
    }

    self.emit_diagnostic(Diagnostic::MissingContainerPropValueAfterEq {
      offset: self.offset(),
    });
    self.expr_ctx_stack.exit(ExprCtx::MdContainerPropItem);

    (self.emit(SyntaxKind::MdContainerPropItem, &children), None)
  }

  /// Parse a container slot: `=== <name>\n<content>\n=== <name>`.
  /// INVARIANT: Expect content to be next
  /// INVARIANT: It must not consume the ending newline (except if there is an EOF)
  pub(in crate::syntax::parse) fn parse_container_slot(&mut self) -> (GreenNode, Option<ExprCtx>) {
    let mut children = vec![];

    let parent_prefix_count = self.expr_ctx_stack.md_prefix_tokens().len();
    self.expr_ctx_stack.enter(ExprCtx::MdContainerSlot);

    // Parse block elements until closing `:::`, `===` or EOF
    // The container creates a new indentation context: inner elements start at indent 0
    loop {
      let next_kind = self.lex_ctx.peek_md(SKIP_NONE).token.kind();
      if next_kind == SyntaxKind::Eof {
        break;
      }

      // Check for closing `:::`, `===` at the same indentation level as the opening
      // The closing `:::`, `===` should appear right after the parent prefix
      let check_pos = if next_kind == SyntaxKind::Newline {
        match self.peek_md_newline_and_prefix() {
          Some(pos) => pos,
          None => break,
        }
      } else {
        parent_prefix_count
      };

      let after = self.lex_ctx.peek_md_nth(check_pos, SKIP_NONE);
      if after.token.kind() == SyntaxKind::MdSymbol
        && matches!(
          after.token.chars().collect::<String>().as_str(),
          ":::" | "==="
        )
      {
        break;
      }

      let (block, early_exit) = self.parse_md_block_element();
      children.push(block);
      if early_exit.is_some_and(|ctx| ctx != ExprCtx::MdContainerSlot) {
        self.expr_ctx_stack.exit(ExprCtx::MdContainerSlot);
        return (
          self.emit(SyntaxKind::MdContainerSlot, &children),
          early_exit,
        );
      } else if early_exit.is_some()
        && let Some(ctx) = self.consume_or_delegate_md(ExprCtx::MdContainerSlot, &mut children)
      {
        self.expr_ctx_stack.exit(ExprCtx::MdContainerSlot);
        return (self.emit(SyntaxKind::MdContainerSlot, &children), Some(ctx));
      }
    }

    self.expr_ctx_stack.exit(ExprCtx::MdContainerSlot);
    (self.emit(SyntaxKind::MdContainerSlot, &children), None)
  }

  /// Parse a link: `[text](url)`.
  /// INVARIANT: The next token must be LBracket.
  pub(in crate::syntax::parse) fn parse_link(&mut self) -> (GreenNode, Option<ExprCtx>) {
    debug_assert!(
      self.lex_ctx.peek_md(SKIP_NONE).token.kind() == SyntaxKind::LBracket,
      "[ParseCtx::parse_link] Expected ["
    );

    let mut children = vec![];
    let open_offset = self.offset();

    // Consume `[`
    let ok = self.consume_md(
      &mut children,
      SKIP_NONE,
      SyntaxKind::LBracket,
      Diagnostic::MissingSyntaxNode {
        expected: SyntaxKind::MdLink,
        start_offset: open_offset,
        end_offset: open_offset,
      },
    );
    if !ok {
      let handler = self
        .expr_ctx_stack
        .find_handler(&self.lex_ctx.peek_md(SKIP_NONE).token);
      return (self.emit(SyntaxKind::MdLink, &children), handler);
    }

    self.expr_ctx_stack.enter(ExprCtx::MdLinkText);

    // Collect alt text tokens until `]`, Newline, or EOF
    let mut alt_children = vec![];
    loop {
      let peek = self.lex_ctx.peek_md(SKIP_NONE);
      match peek.token.kind() {
        SyntaxKind::RBracket | SyntaxKind::Newline | SyntaxKind::Eof => break,
        _ => {
          if let Some(ctx) = self.consume_or_delegate_md(ExprCtx::MdLinkText, &mut alt_children) {
            children.push(self.emit(SyntaxKind::MdText, &alt_children));
            self.expr_ctx_stack.exit(ExprCtx::MdLinkText);
            return (self.emit(SyntaxKind::MdLink, &children), Some(ctx));
          }
        }
      }
    }
    let is_unclosed = !matches!(
      self.lex_ctx.peek_md(SKIP_NONE).token.kind(),
      SyntaxKind::RBracket
    );
    children.push(self.emit(SyntaxKind::MdText, &alt_children));

    if is_unclosed {
      self.emit_diagnostic(Diagnostic::UnclosedLink {
        start_offset: open_offset,
        end_offset: self.offset(),
      });
      self.expr_ctx_stack.exit(ExprCtx::MdLinkText);
      return (self.emit(SyntaxKind::MdLink, &children), None);
    }

    // Consume `]`
    let ok = self.consume_md(
      &mut children,
      SKIP_NONE,
      SyntaxKind::RBracket,
      Diagnostic::MissingSyntaxNode {
        expected: SyntaxKind::MdLink,
        start_offset: open_offset,
        end_offset: open_offset,
      },
    );
    self.expr_ctx_stack.exit(ExprCtx::MdLinkText);
    if !ok {
      let handler = self
        .expr_ctx_stack
        .find_handler(&self.lex_ctx.peek_md(SKIP_NONE).token);
      return (self.emit(SyntaxKind::MdLink, &children), handler);
    }

    // Consume `(`
    let ok = self.consume_md(
      &mut children,
      SKIP_NONE,
      SyntaxKind::LParen,
      Diagnostic::MissingSyntaxNode {
        expected: SyntaxKind::MdLink,
        start_offset: open_offset,
        end_offset: open_offset,
      },
    );
    if !ok {
      let handler = self
        .expr_ctx_stack
        .find_handler(&self.lex_ctx.peek_md(SKIP_NONE).token);
      return (self.emit(SyntaxKind::MdLink, &children), handler);
    }

    self.expr_ctx_stack.enter(ExprCtx::MdLinkUrl);

    // Consume plain text tokens until `)`, Newline, or EOF
    let mut url_children = vec![];
    loop {
      let peek = self.lex_ctx.peek_md(SKIP_NONE);
      match peek.token.kind() {
        SyntaxKind::RParen | SyntaxKind::Newline | SyntaxKind::Eof => break,
        _ => {
          if let Some(ctx) = self.consume_or_delegate_md(ExprCtx::MdLinkUrl, &mut url_children) {
            children.push(self.emit(SyntaxKind::MdText, &url_children));
            self.expr_ctx_stack.exit(ExprCtx::MdLinkUrl);
            return (self.emit(SyntaxKind::MdLink, &children), Some(ctx));
          }
        }
      }
    }
    children.push(self.emit(SyntaxKind::MdText, &url_children));

    // Consume `)`
    let ok = self.consume_md(
      &mut children,
      SKIP_NONE,
      SyntaxKind::RParen,
      Diagnostic::MissingSyntaxNode {
        expected: SyntaxKind::MdLink,
        start_offset: open_offset,
        end_offset: open_offset,
      },
    );
    self.expr_ctx_stack.exit(ExprCtx::MdLinkUrl);
    if !ok {
      let handler = self
        .expr_ctx_stack
        .find_handler(&self.lex_ctx.peek_md(SKIP_NONE).token);
      return (self.emit(SyntaxKind::MdLink, &children), handler);
    }

    (self.emit(SyntaxKind::MdLink, &children), None)
  }

  /// Parse a media embed: `![alt](src)`.
  /// INVARIANT: The next token must be MdSymbol `!` followed by `[`.
  pub(in crate::syntax::parse) fn parse_media(&mut self) -> (GreenNode, Option<ExprCtx>) {
    debug_assert!(
      self.lex_ctx.peek_md(SKIP_NONE).token.kind() == SyntaxKind::MdSymbol
        && self
          .lex_ctx
          .peek_md(SKIP_NONE)
          .token
          .chars()
          .collect::<String>()
          == "!",
      "[ParseCtx::parse_media] Expected !"
    );
    debug_assert!(
      self.lex_ctx.peek_md_nth(1, SKIP_NONE).token.kind() == SyntaxKind::LBracket,
      "[ParseCtx::parse_media] Expected [ after !"
    );

    let mut children = vec![];
    let open_offset = self.offset();

    // Consume `!`
    let ok = self.consume_md_if(
      &mut children,
      SKIP_NONE,
      |token| token.kind() == SyntaxKind::MdSymbol && token.chars().collect::<String>() == "!",
      Diagnostic::MissingSyntaxNode {
        expected: SyntaxKind::MdMedia,
        start_offset: open_offset,
        end_offset: open_offset,
      },
    );
    if !ok {
      let handler = self
        .expr_ctx_stack
        .find_handler(&self.lex_ctx.peek_md(SKIP_NONE).token);
      return (self.emit(SyntaxKind::MdMedia, &children), handler);
    }

    // Consume `[`
    let ok = self.consume_md(
      &mut children,
      SKIP_NONE,
      SyntaxKind::LBracket,
      Diagnostic::MissingSyntaxNode {
        expected: SyntaxKind::MdMedia,
        start_offset: open_offset,
        end_offset: open_offset,
      },
    );
    if !ok {
      let handler = self
        .expr_ctx_stack
        .find_handler(&self.lex_ctx.peek_md(SKIP_NONE).token);
      return (self.emit(SyntaxKind::MdMedia, &children), handler);
    }

    self.expr_ctx_stack.enter(ExprCtx::MdLinkText);

    // Collect alt text tokens until `]`, Newline, or EOF
    let mut alt_children = vec![];
    loop {
      let peek = self.lex_ctx.peek_md(SKIP_NONE);
      match peek.token.kind() {
        SyntaxKind::RBracket | SyntaxKind::Newline | SyntaxKind::Eof => break,
        _ => {
          if let Some(ctx) = self.consume_or_delegate_md(ExprCtx::MdLinkText, &mut alt_children) {
            children.push(self.emit(SyntaxKind::MdText, &alt_children));
            self.expr_ctx_stack.exit(ExprCtx::MdLinkText);
            return (self.emit(SyntaxKind::MdMedia, &children), Some(ctx));
          }
        }
      }
    }
    let is_unclosed = !matches!(
      self.lex_ctx.peek_md(SKIP_NONE).token.kind(),
      SyntaxKind::RBracket
    );
    children.push(self.emit(SyntaxKind::MdText, &alt_children));

    if is_unclosed {
      self.emit_diagnostic(Diagnostic::UnclosedLink {
        start_offset: open_offset,
        end_offset: self.offset(),
      });
      self.expr_ctx_stack.exit(ExprCtx::MdLinkText);
      return (self.emit(SyntaxKind::MdMedia, &children), None);
    }

    // Consume `]`
    let ok = self.consume_md(
      &mut children,
      SKIP_NONE,
      SyntaxKind::RBracket,
      Diagnostic::MissingSyntaxNode {
        expected: SyntaxKind::MdMedia,
        start_offset: open_offset,
        end_offset: open_offset,
      },
    );
    self.expr_ctx_stack.exit(ExprCtx::MdLinkText);
    if !ok {
      let handler = self
        .expr_ctx_stack
        .find_handler(&self.lex_ctx.peek_md(SKIP_NONE).token);
      return (self.emit(SyntaxKind::MdMedia, &children), handler);
    }

    // Consume `(`
    let ok = self.consume_md(
      &mut children,
      SKIP_NONE,
      SyntaxKind::LParen,
      Diagnostic::MissingSyntaxNode {
        expected: SyntaxKind::MdMedia,
        start_offset: open_offset,
        end_offset: open_offset,
      },
    );
    if !ok {
      let handler = self
        .expr_ctx_stack
        .find_handler(&self.lex_ctx.peek_md(SKIP_NONE).token);
      return (self.emit(SyntaxKind::MdMedia, &children), handler);
    }

    self.expr_ctx_stack.enter(ExprCtx::MdLinkUrl);

    // Consume plain text tokens until `)`, Newline, or EOF
    let mut url_children = vec![];
    loop {
      let peek = self.lex_ctx.peek_md(SKIP_NONE);
      match peek.token.kind() {
        SyntaxKind::RParen | SyntaxKind::Newline | SyntaxKind::Eof => break,
        _ => {
          if let Some(ctx) = self.consume_or_delegate_md(ExprCtx::MdLinkUrl, &mut url_children) {
            children.push(self.emit(SyntaxKind::MdText, &url_children));
            self.expr_ctx_stack.exit(ExprCtx::MdLinkUrl);
            return (self.emit(SyntaxKind::MdMedia, &children), Some(ctx));
          }
        }
      }
    }
    children.push(self.emit(SyntaxKind::MdText, &url_children));

    // Consume `)`
    let ok = self.consume_md(
      &mut children,
      SKIP_NONE,
      SyntaxKind::RParen,
      Diagnostic::MissingSyntaxNode {
        expected: SyntaxKind::MdMedia,
        start_offset: open_offset,
        end_offset: open_offset,
      },
    );
    self.expr_ctx_stack.exit(ExprCtx::MdLinkUrl);
    if !ok {
      let handler = self
        .expr_ctx_stack
        .find_handler(&self.lex_ctx.peek_md(SKIP_NONE).token);
      return (self.emit(SyntaxKind::MdMedia, &children), handler);
    }

    (self.emit(SyntaxKind::MdMedia, &children), None)
  }

  /// Parse bold text: `**text**`.
  /// INVARIANT: The next token must be MdSymbol `**`.
  /// Leading whitespace must already be consumed by the caller.
  /// Trailing whitespace after the closing delimiter is not consumed.
  pub(in crate::syntax::parse) fn parse_bold(&mut self) -> (GreenNode, Option<ExprCtx>) {
    debug_assert!(
      self.lex_ctx.peek_md(SKIP_NONE).token.kind() == SyntaxKind::MdSymbol
        && self
          .lex_ctx
          .peek_md(SKIP_NONE)
          .token
          .chars()
          .collect::<String>()
          == "**",
      "[ParseCtx::parse_bold] Expected opening **"
    );

    let mut children = vec![];
    let open_offset = self.offset();

    self.expr_ctx_stack.enter(ExprCtx::MdBold);
    self.advance_md(&mut children, SKIP_NONE);

    loop {
      let text: String = self.lex_ctx.peek_md(SKIP_NONE).token.chars().collect();
      if self.lex_ctx.peek_md(SKIP_NONE).token.kind() == SyntaxKind::MdSymbol && text == "**" {
        self.advance_md(&mut children, SKIP_NONE);
        break;
      }
      if self.should_end_inline_element(&mut children) {
        self.emit_diagnostic(Diagnostic::UnclosedBold {
          start_offset: open_offset,
          end_offset: self.offset(),
        });
        break;
      }
      if self.lex_ctx.peek_md(SKIP_NONE).token.kind() == SyntaxKind::Newline {
        self.advance_md(&mut children, SKIP_NONE);
        continue;
      }
      let (inline, early_exit) = self.parse_md_inline_element();
      children.push(inline);
      if early_exit.is_some_and(|ctx| ctx != ExprCtx::MdBold) {
        self.expr_ctx_stack.exit(ExprCtx::MdBold);
        return (self.emit(SyntaxKind::MdBold, &children), early_exit);
      }
      if early_exit == Some(ExprCtx::MdBold)
        && let Some(ctx) = self.synchronize_bold(&mut children)
      {
        self.expr_ctx_stack.exit(ExprCtx::MdBold);
        return (self.emit(SyntaxKind::MdBold, &children), Some(ctx));
      }
    }

    self.expr_ctx_stack.exit(ExprCtx::MdBold);
    (self.emit(SyntaxKind::MdBold, &children), None)
  }

  // Stop on `**`, EOF, or end of inline element.
  fn synchronize_bold(&mut self, children: &mut Vec<GreenNode>) -> Option<ExprCtx> {
    let mut error_children = vec![];
    let result = loop {
      let peek = self.lex_ctx.peek_md(SKIP_NONE);
      let is_closing =
        peek.token.kind() == SyntaxKind::MdSymbol && peek.token.chars().collect::<String>() == "**";
      if is_closing
        || peek.token.kind() == SyntaxKind::Eof
        || self.should_end_inline_element(children)
      {
        break None;
      }
      if let Some(ctx) = self.consume_or_delegate_md(ExprCtx::MdBold, &mut error_children) {
        break Some(ctx);
      }
    };
    if !error_children.is_empty() {
      children.push(self.emit(SyntaxKind::Error, &error_children));
    }
    result
  }

  /// Parse italic text: `*text*` or `_text_`.
  /// INVARIANT: The next token must be MdSymbol `*` or `_`.
  /// Leading whitespace must already be consumed by the caller.
  /// Trailing whitespace after the closing delimiter is not consumed.
  pub(in crate::syntax::parse) fn parse_italic(&mut self) -> (GreenNode, Option<ExprCtx>) {
    let opening: String = self.lex_ctx.peek_md(SKIP_NONE).token.chars().collect();
    debug_assert!(
      self.lex_ctx.peek_md(SKIP_NONE).token.kind() == SyntaxKind::MdSymbol
        && (opening == "*" || opening == "_"),
      "[ParseCtx::parse_italic] Expected opening * or _"
    );

    let ctx = if opening == "*" {
      ExprCtx::MdItalicStar
    } else {
      ExprCtx::MdItalicUnderscore
    };
    let mut children = vec![];
    let open_offset = self.offset();

    self.expr_ctx_stack.enter(ctx);
    self.advance_md(&mut children, SKIP_NONE);

    loop {
      let text: String = self.lex_ctx.peek_md(SKIP_NONE).token.chars().collect();
      if self.lex_ctx.peek_md(SKIP_NONE).token.kind() == SyntaxKind::MdSymbol
        && (text == "*" || text == "_")
      {
        self.advance_md(&mut children, SKIP_NONE);
        if text != opening {
          self.emit_diagnostic(Diagnostic::MismatchedItalicDelimiter {
            start_offset: open_offset,
            end_offset: self.offset(),
          });
        }
        break;
      }
      if self.should_end_inline_element(&mut children) {
        self.emit_diagnostic(Diagnostic::UnclosedItalic {
          start_offset: open_offset,
          end_offset: self.offset(),
        });
        break;
      }
      if self.lex_ctx.peek_md(SKIP_NONE).token.kind() == SyntaxKind::Newline {
        self.advance_md(&mut children, SKIP_NONE);
        continue;
      }
      let (inline, early_exit) = self.parse_md_inline_element();
      children.push(inline);
      if early_exit.is_some_and(|c| c != ctx) {
        self.expr_ctx_stack.exit(ctx);
        return (self.emit(SyntaxKind::MdItalic, &children), early_exit);
      }
      if early_exit == Some(ctx)
        && let Some(propagate) = self.synchronize_italic(&opening, &mut children)
      {
        self.expr_ctx_stack.exit(ctx);
        return (self.emit(SyntaxKind::MdItalic, &children), Some(propagate));
      }
    }

    self.expr_ctx_stack.exit(ctx);
    (self.emit(SyntaxKind::MdItalic, &children), None)
  }

  // Stop on `*`/`_` matching `opening`, EOF, or end of inline element.
  fn synchronize_italic(
    &mut self,
    opening: &str,
    children: &mut Vec<GreenNode>,
  ) -> Option<ExprCtx> {
    let ctx = if opening == "*" {
      ExprCtx::MdItalicStar
    } else {
      ExprCtx::MdItalicUnderscore
    };
    let mut error_children = vec![];
    let result = loop {
      let peek = self.lex_ctx.peek_md(SKIP_NONE);
      let text: String = peek.token.chars().collect();
      let is_closing = peek.token.kind() == SyntaxKind::MdSymbol && (text == "*" || text == "_");
      if is_closing
        || peek.token.kind() == SyntaxKind::Eof
        || self.should_end_inline_element(children)
      {
        break None;
      }
      if let Some(propagate) = self.consume_or_delegate_md(ctx, &mut error_children) {
        break Some(propagate);
      }
    };
    if !error_children.is_empty() {
      children.push(self.emit(SyntaxKind::Error, &error_children));
    }
    result
  }

  /// Parse bolditalic text: `***text***`.
  /// INVARIANT: The next token must be MdSymbol `***`.
  /// Leading whitespace must already be consumed by the caller.
  /// Trailing whitespace after the closing delimiter is not consumed.
  pub(in crate::syntax::parse) fn parse_bold_italic(&mut self) -> (GreenNode, Option<ExprCtx>) {
    debug_assert!(
      self.lex_ctx.peek_md(SKIP_NONE).token.kind() == SyntaxKind::MdSymbol
        && self
          .lex_ctx
          .peek_md(SKIP_NONE)
          .token
          .chars()
          .collect::<String>()
          == "***",
      "[ParseCtx::parse_bold_italic] Expected opening ***"
    );

    let mut children = vec![];
    let open_offset = self.offset();

    self.expr_ctx_stack.enter(ExprCtx::MdBoldItalic);
    self.advance_md(&mut children, SKIP_NONE);

    loop {
      let text: String = self.lex_ctx.peek_md(SKIP_NONE).token.chars().collect();
      if self.lex_ctx.peek_md(SKIP_NONE).token.kind() == SyntaxKind::MdSymbol && text == "***" {
        self.advance_md(&mut children, SKIP_NONE);
        break;
      }
      if self.should_end_inline_element(&mut children) {
        self.emit_diagnostic(Diagnostic::UnclosedBoldItalic {
          start_offset: open_offset,
          end_offset: self.offset(),
        });
        break;
      }
      if self.lex_ctx.peek_md(SKIP_NONE).token.kind() == SyntaxKind::Newline {
        self.advance_md(&mut children, SKIP_NONE);
        continue;
      }
      let (inline, early_exit) = self.parse_md_inline_element();
      children.push(inline);
      if early_exit.is_some_and(|ctx| ctx != ExprCtx::MdBoldItalic) {
        self.expr_ctx_stack.exit(ExprCtx::MdBoldItalic);
        return (self.emit(SyntaxKind::MdBoldItalic, &children), early_exit);
      }
      if early_exit == Some(ExprCtx::MdBoldItalic)
        && let Some(ctx) = self.synchronize_bold_italic(&mut children)
      {
        self.expr_ctx_stack.exit(ExprCtx::MdBoldItalic);
        return (self.emit(SyntaxKind::MdBoldItalic, &children), Some(ctx));
      }
    }

    self.expr_ctx_stack.exit(ExprCtx::MdBoldItalic);
    (self.emit(SyntaxKind::MdBoldItalic, &children), None)
  }

  // Stop on `***`, EOF, or end of inline element.
  fn synchronize_bold_italic(&mut self, children: &mut Vec<GreenNode>) -> Option<ExprCtx> {
    let mut error_children = vec![];
    let result = loop {
      let peek = self.lex_ctx.peek_md(SKIP_NONE);
      let is_closing = peek.token.kind() == SyntaxKind::MdSymbol
        && peek.token.chars().collect::<String>() == "***";
      if is_closing
        || peek.token.kind() == SyntaxKind::Eof
        || self.should_end_inline_element(children)
      {
        break None;
      }
      if let Some(ctx) = self.consume_or_delegate_md(ExprCtx::MdBoldItalic, &mut error_children) {
        break Some(ctx);
      }
    };
    if !error_children.is_empty() {
      children.push(self.emit(SyntaxKind::Error, &error_children));
    }
    result
  }

  /// Parse strikethrough text: `~~text~~`.
  /// INVARIANT: The next token must be MdSymbol `~~`.
  /// Leading whitespace must already be consumed by the caller.
  /// Trailing whitespace after the closing delimiter is not consumed.
  pub(in crate::syntax::parse) fn parse_strikethrough(&mut self) -> (GreenNode, Option<ExprCtx>) {
    debug_assert!(
      self.lex_ctx.peek_md(SKIP_NONE).token.kind() == SyntaxKind::MdSymbol
        && self
          .lex_ctx
          .peek_md(SKIP_NONE)
          .token
          .chars()
          .collect::<String>()
          == "~~",
      "[ParseCtx::parse_strikethrough] Expected opening ~~"
    );

    let mut children = vec![];
    let open_offset = self.offset();

    self.expr_ctx_stack.enter(ExprCtx::MdStrikethrough);
    self.advance_md(&mut children, SKIP_NONE);

    loop {
      let text: String = self.lex_ctx.peek_md(SKIP_NONE).token.chars().collect();
      if self.lex_ctx.peek_md(SKIP_NONE).token.kind() == SyntaxKind::MdSymbol && text == "~~" {
        self.advance_md(&mut children, SKIP_NONE);
        break;
      }
      if self.should_end_inline_element(&mut children) {
        self.emit_diagnostic(Diagnostic::UnclosedStrikethrough {
          start_offset: open_offset,
          end_offset: self.offset(),
        });
        break;
      }
      if self.lex_ctx.peek_md(SKIP_NONE).token.kind() == SyntaxKind::Newline {
        self.advance_md(&mut children, SKIP_NONE);
        continue;
      }
      let (inline, early_exit) = self.parse_md_inline_element();
      children.push(inline);
      if early_exit.is_some_and(|ctx| ctx != ExprCtx::MdStrikethrough) {
        self.expr_ctx_stack.exit(ExprCtx::MdStrikethrough);
        return (
          self.emit(SyntaxKind::MdStrikethrough, &children),
          early_exit,
        );
      }
      if early_exit == Some(ExprCtx::MdStrikethrough)
        && let Some(ctx) = self.synchronize_strikethrough(&mut children)
      {
        self.expr_ctx_stack.exit(ExprCtx::MdStrikethrough);
        return (self.emit(SyntaxKind::MdStrikethrough, &children), Some(ctx));
      }
    }

    self.expr_ctx_stack.exit(ExprCtx::MdStrikethrough);
    (self.emit(SyntaxKind::MdStrikethrough, &children), None)
  }

  // Stop on `~~`, EOF, or end of inline element.
  fn synchronize_strikethrough(&mut self, children: &mut Vec<GreenNode>) -> Option<ExprCtx> {
    let mut error_children = vec![];
    let result = loop {
      let peek = self.lex_ctx.peek_md(SKIP_NONE);
      let is_closing =
        peek.token.kind() == SyntaxKind::MdSymbol && peek.token.chars().collect::<String>() == "~~";
      if is_closing
        || peek.token.kind() == SyntaxKind::Eof
        || self.should_end_inline_element(children)
      {
        break None;
      }
      if let Some(ctx) = self.consume_or_delegate_md(ExprCtx::MdStrikethrough, &mut error_children)
      {
        break Some(ctx);
      }
    };
    if !error_children.is_empty() {
      children.push(self.emit(SyntaxKind::Error, &error_children));
    }
    result
  }

  /// Parse a text run: consecutive plain text, including surrounding whitespace.
  /// Consumes leading and trailing spaces.
  pub(in crate::syntax::parse) fn parse_text(&mut self) -> (GreenNode, Option<ExprCtx>) {
    let mut children = vec![];

    loop {
      let next_kind = self.lex_ctx.peek_md(SKIP_NONE).token.kind();
      if matches!(next_kind, SyntaxKind::Newline | SyntaxKind::Eof) {
        break;
      }
      if self.is_md_inline_start() {
        break;
      }
      self.advance_md(&mut children, SKIP_NONE);
    }

    (self.emit(SyntaxKind::MdText, &children), None)
  }

  // Consume the expected prefix on the next line
  fn consume_md_prefix(&mut self, children: &mut Vec<GreenNode>) -> bool {
    let expected_tokens = self.expr_ctx_stack.md_prefix_tokens().to_vec();
    for expected_token in &expected_tokens {
      let peek = self.lex_ctx.peek_md(SKIP_NONE);
      if peek.token != *expected_token {
        self.emit_diagnostic(Diagnostic::MissingExpectMdPrefix {
          expected_prefix: format!("{:?}", expected_tokens),
          start_offset: self.offset(),
          end_offset: self.offset(),
        });
        return false;
      }
      self.advance_md(children, SKIP_NONE);
    }

    true
  }

  // Consume a newline and the expected prefix on the next line
  fn consume_md_newline_and_prefix(&mut self, children: &mut Vec<GreenNode>) -> bool {
    self.advance_md(children, SKIP_WS);
    self.consume_md_prefix(children)
  }

  // If the next token should be handled by an outer context, return that context.
  // Otherwise consume the token into `error_children` for the caller to wrap as Error.
  fn consume_or_delegate_md(
    &mut self,
    current: ExprCtx,
    error_children: &mut Vec<GreenNode>,
  ) -> Option<ExprCtx> {
    let handler = self
      .expr_ctx_stack
      .find_handler(&self.lex_ctx.peek_md(SKIP_NONE).token);
    if handler.is_some_and(|ctx| ctx != current) {
      return handler;
    }
    self.advance_md(error_children, SKIP_NONE);
    None
  }

  /// Whether the current inline element should end due to EOF or a line boundary.
  /// Consumes the newline and prefix if present.
  fn should_end_inline_element(&mut self, children: &mut Vec<GreenNode>) -> bool {
    let next_kind = self.lex_ctx.peek_md(SKIP_NONE).token.kind();
    if next_kind == SyntaxKind::Eof {
      return true;
    }
    if next_kind == SyntaxKind::Newline {
      self.consume_md_newline_and_prefix(children);
      let after = self.lex_ctx.peek_md(SKIP_NONE);
      if matches!(after.token.kind(), SyntaxKind::Newline | SyntaxKind::Eof) {
        return true;
      }
      if after.token.kind() == SyntaxKind::MdSymbol {
        let text: String = after.token.chars().collect();
        let first = text.chars().next().unwrap_or('\0');
        if matches!(first, '#' | '-' | '*' | '+' | '>' | '|' | ':') {
          return true;
        }
        if text == "===" {
          return self
            .expr_ctx_stack
            .is_inside(|c| matches!(c, ExprCtx::MdContainerBlock(_)));
        }
      }
      if after.token.kind() == SyntaxKind::MdNumber {
        return true;
      }
      if matches!(
        after.token.kind(),
        SyntaxKind::CodeBlock | SyntaxKind::MathBlock
      ) {
        return true;
      }
    }
    false
  }

  /// Whether the next token starts an inline element.
  fn is_md_inline_start(&mut self) -> bool {
    let next = self.lex_ctx.peek_md(SKIP_NONE);
    match next.token.kind() {
      SyntaxKind::LBracket => true,
      SyntaxKind::InterpStart => true,
      SyntaxKind::InlineMath | SyntaxKind::InlineCode => true,
      SyntaxKind::MdSymbol => {
        let text: String = next.token.chars().collect();
        if matches!(text.as_str(), "*" | "_" | "**" | "***" | "~~") {
          return true;
        }
        // `![` starts a media embed
        if text == "!" {
          let second = self.lex_ctx.peek_md_nth(1, SKIP_NONE);
          return second.token.kind() == SyntaxKind::LBracket;
        }
        false
      }
      _ => false,
    }
  }

  // Peek whether the next token is a newline followed by the expected prefix
  // Returns the offset after the matched prefix, or None if mismatch
  // INVARIANT: The next token must be a Newline
  fn peek_md_newline_and_prefix(&mut self) -> Option<usize> {
    debug_assert!(
      self.lex_ctx.peek_md(SKIP_NONE).token.kind() == SyntaxKind::Newline,
      "[ParseCtx::peek_md_newline_and_prefix] Expected next token to be Newline"
    );
    let expected_tokens = self.expr_ctx_stack.md_prefix_tokens().to_vec();
    for (idx, expected_token) in expected_tokens.iter().enumerate() {
      let peek = self.lex_ctx.peek_md_nth(idx + 1, SKIP_NONE);
      if peek.token != *expected_token {
        return None;
      }
    }
    Some(expected_tokens.len() + 1)
  }

  // Blank line: parent prefix without trailing spaces, optional whitespace, then newline
  // Consumes everything except the newline
  fn consume_md_blank_line(&mut self, children: &mut Vec<GreenNode>) -> bool {
    let parent_prefix = self.expr_ctx_stack.md_parent_prefix_tokens().to_vec();
    let mut trim_end = parent_prefix.len();
    while trim_end > 0 && parent_prefix[trim_end - 1].kind() == SyntaxKind::Whitespace {
      trim_end -= 1;
    }
    for (idx, expected_token) in parent_prefix[..trim_end].iter().enumerate() {
      if self.lex_ctx.peek_md_nth(idx, SKIP_NONE).token != *expected_token {
        return false;
      }
    }
    let mut offset = trim_end;
    while self.lex_ctx.peek_md_nth(offset, SKIP_NONE).token.kind() == SyntaxKind::Whitespace {
      offset += 1;
    }
    if self.lex_ctx.peek_md_nth(offset, SKIP_NONE).token.kind() != SyntaxKind::Newline {
      return false;
    }
    for _ in 0..offset {
      self.advance_md(children, SKIP_NONE);
    }
    true
  }
}

// Block element start detection helpers
impl<S: Utf8Stream> ParseCtx<S> {
  // WARNING: Prefix must be consumed already
  fn is_heading_start(&mut self, skip: u16) -> bool {
    let next = self.lex_ctx.peek_md(skip);
    next.token.kind() == SyntaxKind::MdSymbol && next.token.chars().all(|c| c == '#')
  }

  fn is_bullet_list_start(&mut self, skip: u16) -> bool {
    let next = self.lex_ctx.peek_md(skip);
    if next.token.kind() != SyntaxKind::MdSymbol {
      return false;
    }
    let text: String = next.token.chars().collect();
    // Must not skip whitespace here since we need to detect the space after the bullet
    let no_ws = skip & !SKIP_WS;
    matches!(text.as_str(), "-" | "*" | "+")
      && self.lex_ctx.peek_md_nth(1, no_ws).token.kind() == SyntaxKind::Whitespace
  }

  fn is_ordered_list_start(&mut self, skip: u16) -> bool {
    let next = self.lex_ctx.peek_md(skip);
    if next.token.kind() != SyntaxKind::MdNumber {
      return false;
    }
    let dot = self.lex_ctx.peek_md_nth(1, skip);
    dot.token.kind() == SyntaxKind::MdSymbol && dot.token.chars().collect::<String>() == "."
  }

  fn is_blockquote_start(&mut self, skip: u16) -> bool {
    let next = self.lex_ctx.peek_md(skip);
    if next.token.kind() != SyntaxKind::MdSymbol {
      return false;
    }
    let text: String = next.token.chars().collect();
    text == ">"
      && !(self.lex_ctx.peek_md_nth(1, skip).token.kind() == SyntaxKind::MdSymbol
        && self
          .lex_ctx
          .peek_md_nth(1, skip)
          .token
          .chars()
          .collect::<String>()
          == "-")
  }

  fn is_toggle_list_start(&mut self, skip: u16) -> bool {
    let next = self.lex_ctx.peek_md(skip);
    if next.token.kind() != SyntaxKind::MdSymbol {
      return false;
    }
    let text: String = next.token.chars().collect();
    text == ">"
      && self.lex_ctx.peek_md_nth(1, skip).token.kind() == SyntaxKind::MdSymbol
      && self
        .lex_ctx
        .peek_md_nth(1, skip)
        .token
        .chars()
        .collect::<String>()
        == "-"
  }

  fn is_table_start(&mut self, skip: u16) -> bool {
    let next = self.lex_ctx.peek_md(skip);
    next.token.kind() == SyntaxKind::MdSymbol && next.token.chars().collect::<String>() == "|"
  }

  fn is_container_start(&mut self, skip: u16) -> bool {
    let next = self.lex_ctx.peek_md(skip);
    next.token.kind() == SyntaxKind::MdSymbol && next.token.chars().collect::<String>() == ":::"
  }

  fn is_media_block_start(&mut self, skip: u16) -> bool {
    let next = self.lex_ctx.peek_md(skip);
    if next.token.kind() != SyntaxKind::MdSymbol {
      return false;
    }
    next.token.chars().collect::<String>() == "!"
      && self.lex_ctx.peek_md_nth(1, skip).token.kind() == SyntaxKind::LBracket
  }

  fn is_code_or_math_block_start(&mut self, skip: u16) -> bool {
    let next = self.lex_ctx.peek_md(skip);
    matches!(
      next.token.kind(),
      SyntaxKind::CodeBlock | SyntaxKind::MathBlock
    )
  }

  // Check if token at a specific offset (using SKIP_NONE) starts a block element
  fn is_md_block_start_at(&mut self, offset: usize) -> bool {
    let first = self.lex_ctx.peek_md_nth(offset, SKIP_NONE);
    match first.token.kind() {
      SyntaxKind::MdSymbol => {
        let text: String = first.token.chars().collect();
        if text.chars().all(|c| c == '#') {
          return true;
        }
        if matches!(text.as_str(), "-" | "*" | "+") {
          let next = self.lex_ctx.peek_md_nth(offset + 1, SKIP_NONE);
          return next.token.kind() == SyntaxKind::Whitespace;
        }
        if text == ">" {
          return true;
        }
        if text == "|" || text == ":::" {
          return true;
        }
        if text == "!" {
          let next = self.lex_ctx.peek_md_nth(offset + 1, SKIP_NONE);
          return next.token.kind() == SyntaxKind::LBracket;
        }
        if text == "===" {
          return self
            .expr_ctx_stack
            .is_inside(|c| matches!(c, ExprCtx::MdContainerBlock(_)));
        }
        false
      }
      SyntaxKind::MdNumber => {
        let dot = self.lex_ctx.peek_md_nth(offset + 1, SKIP_NONE);
        dot.token.kind() == SyntaxKind::MdSymbol && dot.token.chars().collect::<String>() == "."
      }
      SyntaxKind::CodeBlock | SyntaxKind::MathBlock => true,
      _ => false,
    }
  }
}
