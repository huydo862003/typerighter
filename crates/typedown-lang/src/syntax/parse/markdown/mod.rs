//! Markdown body parsing

mod prop;

use crate::syntax::diagnostic::*;
use crate::syntax::syntax_kind::SyntaxKind;
use typedown_types::stream::Utf8Stream;

use super::ctx::ParseCtx;
use super::ctx::expr_ctx::ExprCtx;
use crate::syntax::green::{GreenNode, SyntaxToken};
use crate::syntax::lex::ctx::LexMode;
use crate::syntax::parse::constants::{SKIP_NONE, SKIP_WCN, SKIP_WS};

const MD_PREFIX_EXACT: usize = 0;
// Trailing whitespace prefix tokens are optional at end of line
const MD_PREFIX_LAX: usize = 1 << 0;

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

      let (block, early_exit) = self.parse_md_block_element(0);
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

  /// Parse a block-level element
  /// INVARIANT: Must be at start of line with prefix already consumed
  /// INVARIANT: Block elements do not consume their trailing newline as one newline can end multiple block elements
  pub(in crate::syntax::parse) fn parse_md_block_element(
    &mut self,
    indent: usize,
  ) -> (GreenNode, Option<ExprCtx>) {
    debug_assert!(
      self.lex_ctx.mode() == LexMode::MarkdownBody,
      "[ParseCtx::parse_md_block_element] Lex mode must be MarkdownBody"
    );

    let next = self.lex_ctx.peek_md(SKIP_NONE);
    let (node, early_exit) = match next.token.kind() {
      SyntaxKind::Eof => {
        let mut children = vec![];
        self.advance_md(&mut children, SKIP_NONE);
        (self.emit(SyntaxKind::Error, &children), None)
      }
      SyntaxKind::Newline => {
        // Blank line separator between blocks
        let mut children = vec![];
        self.advance_md(&mut children, SKIP_NONE);
        (self.emit(SyntaxKind::MdBlankLine, &children), None)
      }
      _ if self.is_horizontal_rule_start(SKIP_NONE) => {
        let mut children = vec![];
        self.advance_md(&mut children, SKIP_NONE);
        (self.emit(SyntaxKind::MdHorizontalRule, &children), None)
      }
      _ if self.is_heading_start(SKIP_NONE) => self.parse_heading(),
      _ if self.is_blockquote_start(SKIP_NONE) => self.parse_blockquote(),
      _ if self.is_bullet_list_start(SKIP_NONE) => self.parse_bullet_list(indent),
      _ if self.is_ordered_list_start(SKIP_NONE) => self.parse_ordered_list(indent),
      _ if self.is_table_start(SKIP_NONE) => self.parse_table(),
      _ if self.is_container_start(SKIP_NONE) => self.parse_container_block(indent),
      _ if self.is_container_shorthand_start(SKIP_NONE) => self.parse_container_shorthand(),
      _ if self.is_media_block_start(SKIP_NONE) => self.parse_media(),
      _ if self.is_code_or_math_block_start(SKIP_NONE) => {
        let mut children = vec![];
        let kind = next.token.kind();
        self.advance_md(&mut children, SKIP_NONE);
        (self.emit(kind, &children), None)
      }
      _ => self.parse_paragraph(),
    };

    // Invariant check: block elements must not consume their trailing newline (unless exiting early)
    debug_assert!(
      early_exit.is_some()
        || node.kind() == SyntaxKind::MdBlankLine
        || matches!(
          self.lex_ctx.peek_md(SKIP_NONE).token.kind(),
          SyntaxKind::Newline | SyntaxKind::Eof
        ),
      "[ParseCtx::parse_md_block_element] Block element {:?} consumed its trailing newline at offset {}, next token is {:?}",
      node.kind(),
      self.offset(),
      self.lex_ctx.peek_md(SKIP_NONE).token.kind()
    );

    (node, early_exit)
  }

  /// Parse an inline element
  /// INVARIANT: Must not be at a Newline or EOF
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
          "***" | "___" => self.parse_bold_italic(),
          "**" | "__" => self.parse_bold(),
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

  /// Parse a heading: `# ...`, `## ...`, etc
  /// INVARIANT: The next token should be a hash sequence
  pub(in crate::syntax::parse) fn parse_heading(&mut self) -> (GreenNode, Option<ExprCtx>) {
    fn is_hash(token: &SyntaxToken) -> bool {
      token.kind() == SyntaxKind::MdSymbol && token.chars().all(|char| char == '#')
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
      TdDiagnosticKind::MissingMarkdownHeadingHash {}.into_error(self.offset(), self.offset()),
    );

    let next_token = &self.lex_ctx.peek_md(SKIP_NONE).token;
    if next_token.kind() != SyntaxKind::Whitespace {
      self.emit_diagnostic(
        TdDiagnosticKind::MissingRequiredSpace {}.into_error(self.offset(), self.offset()),
      );
    } else {
      self.advance_md(&mut children, SKIP_NONE);
    }

    // Require at least one inline element
    let has_inline = {
      let next = self.lex_ctx.peek_md(SKIP_WS);
      !matches!(next.token.kind(), SyntaxKind::Newline | SyntaxKind::Eof)
    };
    if !has_inline {
      self.emit_diagnostic(
        TdDiagnosticKind::MissingSyntaxNode {
          expected: SyntaxKind::MdText,
        }
        .into_error(self.offset(), self.offset()),
      );
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

  /// Parse a paragraph: consecutive non-blank text lines
  /// INVARIANT: The current line is not blank (caller must ensure there is content)
  pub(in crate::syntax::parse) fn parse_paragraph(&mut self) -> (GreenNode, Option<ExprCtx>) {
    self.expr_ctx_stack.enter(ExprCtx::MdParagraph);
    let mut children = vec![];

    loop {
      // Parse all inline elements on this line or nested block
      loop {
        let next_kind = self.lex_ctx.peek_md(SKIP_NONE).token.kind();
        if matches!(next_kind, SyntaxKind::Newline | SyntaxKind::Eof) {
          break;
        }

        // Check for nested indented block
        let block_indent = self.peek_md_past_whitespace_at(0);
        // There is an indent, possible nested block
        if block_indent > 0 && self.is_md_block_start_at(block_indent) {
          self.consume_md_indent(&mut children);
          let (item, early_exit) = self.parse_md_block_element(block_indent);
          children.push(item);
          if early_exit.is_some_and(|ctx| !matches!(ctx, ExprCtx::MdParagraph)) {
            self.expr_ctx_stack.exit(ExprCtx::MdParagraph);
            return (self.emit(SyntaxKind::MdParagraph, &children), early_exit);
          }
          break;
        }

        let (inline, early_exit) = self.parse_md_inline_element();
        children.push(inline);
        if early_exit.is_some() {
          self.expr_ctx_stack.exit(ExprCtx::MdParagraph);
          return (self.emit(SyntaxKind::MdParagraph, &children), early_exit);
        }
      }

      // Stop at EOF
      let next_kind = self.lex_ctx.peek_md(SKIP_NONE).token.kind();
      if next_kind == SyntaxKind::Eof {
        break;
      }

      let Some(after_prefix) = self.peek_md_newline_and_prefix(MD_PREFIX_LAX) else {
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

    self.expr_ctx_stack.exit(ExprCtx::MdParagraph);
    (self.emit(SyntaxKind::MdParagraph, &children), None)
  }

  /// Parse a blockquote: `> ...`
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

    // Consume the optional space after `>`
    if self.lex_ctx.peek_md(SKIP_NONE).token.kind() == SyntaxKind::Whitespace {
      self.advance_md(&mut children, SKIP_NONE);
    }

    // Parse block elements until the blockquote ends
    loop {
      let next_kind = self.lex_ctx.peek_md(SKIP_NONE).token.kind();
      if next_kind == SyntaxKind::Eof {
        break;
      }
      if matches!(next_kind, SyntaxKind::Newline) {
        if self.peek_md_newline_and_prefix(MD_PREFIX_LAX).is_none() {
          break;
        }
        self.consume_md_newline_and_prefix(&mut children);
        continue;
      }

      let (block, early_exit) = self.parse_md_block_element(0);
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

  /// Parse a table: `| ... | ... |`
  /// INVARIANT: Next token must be MdSymbol `|`, optionally preceded by whitespace
  pub(in crate::syntax::parse) fn parse_table(&mut self) -> (GreenNode, Option<ExprCtx>) {
    debug_assert!(
      self.is_table_start(SKIP_NONE),
      "[ParseCtx::parse_table] Expected |"
    );

    let mut children = vec![];

    self.expr_ctx_stack.enter(ExprCtx::MdTable);

    self.consume_md_whitespace(&mut children);

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
      self.emit_diagnostic(
        TdDiagnosticKind::MissingTableSeparatorRow {}.into_error(sep_start, self.offset()),
      );
      self.expr_ctx_stack.exit(ExprCtx::MdTable);
      return (self.emit(SyntaxKind::MdTable, &children), None);
    }
    // Verify separator row starts with `|` followed by `-`
    let next = self.lex_ctx.peek_md(SKIP_WS);
    let next2 = self.lex_ctx.peek_md_nth(1, SKIP_WS);
    let is_separator = next.token.kind() == SyntaxKind::MdSymbol
      && next.token.chars().collect::<String>() == "|"
      && next2.token.kind() == SyntaxKind::MdSymbol
      && next2.token.chars().collect::<String>().starts_with('-');
    if !is_separator {
      self.emit_diagnostic(
        TdDiagnosticKind::MissingTableSeparatorRow {}.into_error(sep_start, self.offset()),
      );
      self.expr_ctx_stack.exit(ExprCtx::MdTable);
      return (self.emit(SyntaxKind::MdTable, &children), None);
    }
    // Consume indentation before separator row
    self.consume_md_whitespace(&mut children);
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
      // Peek ahead to check if next line starts a table row before consuming newline
      let Some(after_prefix) = self.peek_md_newline_and_prefix(MD_PREFIX_LAX) else {
        break;
      };
      // Skip whitespace after prefix to find the `|`
      let peek_offset = self.peek_md_past_whitespace_at(after_prefix);
      let next = self.lex_ctx.peek_md_nth(peek_offset, SKIP_NONE);
      if next.token.kind() != SyntaxKind::MdSymbol || next.token.chars().collect::<String>() != "|"
      {
        break;
      }
      // Confirmed next row, consume newline and prefix
      self.consume_md_newline_and_prefix(&mut children);
      // Consume indentation before data row
      self.consume_md_whitespace(&mut children);
      let row_start = self.offset();
      let (row, col_count, early_exit) = self.parse_table_row(true);
      if col_count != expected_cols {
        self.emit_diagnostic(
          TdDiagnosticKind::TableColumnCountMismatch {
            expected: expected_cols,
            found: col_count,
          }
          .into_error(row_start, self.offset()),
        );
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

  /// Parse a table row: `| cell | cell |`
  /// Returns the node, cell count, and early exit context
  /// INVARIANT: Next token must be MdSymbol `|`
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

  /// Parse a table separator row: `| --- | --- |`
  /// INVARIANT: Next token must be MdSymbol `|`
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

  /// Parse a table cell: inline content until `|` or end of line
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

  /// Parse a bullet list: `- ...` or `* ...` or `+ ...`
  /// INVARIANT: Must be after prefix. Next token must be MdSymbol `-`, `*`, or `+`
  pub(in crate::syntax::parse) fn parse_bullet_list(
    &mut self,
    indent: usize,
  ) -> (GreenNode, Option<ExprCtx>) {
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

    let list_ctx = ExprCtx::MdUnorderedList { indent };
    self.expr_ctx_stack.enter(list_ctx);

    // Parse first list item
    let (item, early_exit) = self.parse_next_bullet_item(&bullet);
    children.push(item);
    if early_exit.is_some_and(|ctx| !matches!(ctx, ExprCtx::MdUnorderedList { .. })) {
      self.expr_ctx_stack.exit(list_ctx);
      return (self.emit(SyntaxKind::MdBulletList, &children), early_exit);
    }

    // Parse remaining list items
    while let Some(after_prefix) = self.peek_md_multinewline_and_prefix(MD_PREFIX_EXACT) {
      let next = self.lex_ctx.peek_md_nth(after_prefix, SKIP_NONE);
      if next.token.kind() != SyntaxKind::MdSymbol {
        break;
      }
      let text: String = next.token.chars().collect();
      if text != bullet {
        break;
      }
      // Require whitespace or newline after the bullet to distinguish from e.g. `->` arrows
      let after = self
        .lex_ctx
        .peek_md_nth(after_prefix + 1, SKIP_NONE)
        .token
        .kind();
      if !matches!(
        after,
        SyntaxKind::Whitespace | SyntaxKind::Newline | SyntaxKind::Eof
      ) {
        break;
      }
      // Confirmed sibling: consume the newlines and prefix
      self.consume_md_multinewline_and_prefix(&mut children);

      let (item, early_exit) = self.parse_next_bullet_item(&bullet);
      children.push(item);
      if early_exit.is_some_and(|ctx| !matches!(ctx, ExprCtx::MdUnorderedList { .. })) {
        self.expr_ctx_stack.exit(list_ctx);
        return (self.emit(SyntaxKind::MdBulletList, &children), early_exit);
      }
    }

    self.expr_ctx_stack.exit(list_ctx);
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

  /// Parse a bullet list item: `- content` or `* content` or `+ content`
  /// INVARIANT: Next token must be the bullet marker matching `bullet`
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
    let after_bullet = self.lex_ctx.peek_md(SKIP_NONE).token.kind();
    if after_bullet == SyntaxKind::Whitespace {
      self.advance_md(&mut children, SKIP_NONE);
    } else if !matches!(after_bullet, SyntaxKind::Newline | SyntaxKind::Eof) {
      self.emit_diagnostic(
        TdDiagnosticKind::MissingRequiredSpace {}.into_error(self.offset(), self.offset()),
      );
    }

    // Parse block elements until end of list item
    let mut block_indent: usize = 0;
    loop {
      let next_kind = self.lex_ctx.peek_md(SKIP_NONE).token.kind();
      if next_kind == SyntaxKind::Eof {
        break;
      }
      if next_kind == SyntaxKind::Newline {
        let Some(after_prefix) = self.peek_md_multinewline_and_prefix(MD_PREFIX_EXACT) else {
          break;
        };
        // Peek past prefix to check for sibling bullet
        let after = self.lex_ctx.peek_md_nth(after_prefix, SKIP_NONE);
        if after.token.kind() == SyntaxKind::MdSymbol
          && after.token.chars().collect::<String>() == bullet
        {
          break;
        }
        self.consume_md_multinewline_and_prefix(&mut children);
        block_indent = self.consume_md_indent(&mut children);
        let next = self.lex_ctx.peek_md(SKIP_NONE);
        if matches!(next.token.kind(), SyntaxKind::Newline | SyntaxKind::Eof) {
          break;
        }
        continue;
      }

      let (block, early_exit) = self.parse_md_block_element(block_indent);
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

    // Parse block elements until end of task list item
    let mut block_indent: usize = 0;
    loop {
      let next_kind = self.lex_ctx.peek_md(SKIP_NONE).token.kind();

      // EOF -> end of list
      if next_kind == SyntaxKind::Eof {
        break;
      }

      // Newline -> either end of list, basic continuation, nested or sibling
      if next_kind == SyntaxKind::Newline {
        let Some(after_prefix) = self.peek_md_multinewline_and_prefix(MD_PREFIX_EXACT) else {
          break;
        };

        // Peek past prefix to check for sibling bullet
        let after = self.lex_ctx.peek_md_nth(after_prefix, SKIP_NONE);
        if after.token.kind() == SyntaxKind::MdSymbol
          && after.token.chars().collect::<String>() == bullet
        {
          break;
        }

        self.consume_md_multinewline_and_prefix(&mut children);
        block_indent = self.consume_md_indent(&mut children);

        let next = self.lex_ctx.peek_md(SKIP_NONE);
        if matches!(next.token.kind(), SyntaxKind::Newline | SyntaxKind::Eof) {
          break;
        }
        continue;
      }

      let (block, early_exit) = self.parse_md_block_element(block_indent);
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

  /// Parse an ordered list: `1. ...`
  /// INVARIANT: The next tokens must be MdNumber and MdSymbol dot
  pub(in crate::syntax::parse) fn parse_ordered_list(
    &mut self,
    indent: usize,
  ) -> (GreenNode, Option<ExprCtx>) {
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

    let list_ctx = ExprCtx::MdOrderedList { indent };
    self.expr_ctx_stack.enter(list_ctx);

    // Parse first list item
    let (item, early_exit) = self.parse_ordered_list_item();
    children.push(item);
    if early_exit.is_some_and(|ctx| !matches!(ctx, ExprCtx::MdOrderedList { .. })) {
      self.expr_ctx_stack.exit(list_ctx);
      return (self.emit(SyntaxKind::MdOrderedList, &children), early_exit);
    }

    // Parse remaining list items
    while let Some(after_prefix) = self.peek_md_multinewline_and_prefix(MD_PREFIX_EXACT) {
      let next = self.lex_ctx.peek_md_nth(after_prefix, SKIP_NONE);
      if next.token.kind() != SyntaxKind::MdNumber {
        break;
      }
      // Verify `.` follows the number
      let dot = self.lex_ctx.peek_md_nth(after_prefix + 1, SKIP_NONE);
      if dot.token.kind() != SyntaxKind::MdSymbol || dot.token.chars().collect::<String>() != "." {
        break;
      }
      // Confirmed sibling: consume the newlines and prefix
      self.consume_md_multinewline_and_prefix(&mut children);

      let (item, early_exit) = self.parse_ordered_list_item();
      children.push(item);
      if early_exit.is_some_and(|ctx| !matches!(ctx, ExprCtx::MdOrderedList { .. })) {
        self.expr_ctx_stack.exit(list_ctx);
        return (self.emit(SyntaxKind::MdOrderedList, &children), early_exit);
      }
    }

    self.expr_ctx_stack.exit(list_ctx);
    (self.emit(SyntaxKind::MdOrderedList, &children), None)
  }

  /// Parse an ordered list item: `1. content`
  /// INVARIANT: Next token must be MdNumber followed by `.`
  fn parse_ordered_list_item(&mut self) -> (GreenNode, Option<ExprCtx>) {
    debug_assert!(
      self.lex_ctx.peek_md(SKIP_NONE).token.kind() == SyntaxKind::MdNumber,
      "[ParseCtx::parse_ordered_list_item] Expected MdNumber"
    );

    let mut children = vec![];

    // Marker width = number width + `.` + space (e.g. `1. ` = 3, `10. ` = 4)
    let number_width = self.lex_ctx.peek_md(SKIP_NONE).token.text_len();
    let marker_width = number_width + 2; // +1 for `.`, +1 for space
    let item_ctx = ExprCtx::MdOrderedListItem { marker_width };
    self.expr_ctx_stack.enter(item_ctx);

    // Consume the number
    self.advance_md(&mut children, SKIP_NONE);

    // Consume `.`
    self.consume_md_if(
      &mut children,
      SKIP_NONE,
      |token| token.kind() == SyntaxKind::MdSymbol && token.chars().collect::<String>() == ".",
      TdDiagnosticKind::MissingSyntaxNode {
        expected: SyntaxKind::MdOrderedListItem,
      }
      .into_error(self.offset(), self.offset()),
    );

    // Require a space after `.`
    let after_dot = self.lex_ctx.peek_md(SKIP_NONE).token.kind();
    if after_dot == SyntaxKind::Whitespace {
      self.advance_md(&mut children, SKIP_NONE);
    } else if !matches!(after_dot, SyntaxKind::Newline | SyntaxKind::Eof) {
      self.emit_diagnostic(
        TdDiagnosticKind::MissingRequiredSpace {}.into_error(self.offset(), self.offset()),
      );
    }

    // Parse block elements until end of list item
    let mut block_indent: usize = 0;
    loop {
      let next_kind = self.lex_ctx.peek_md(SKIP_NONE).token.kind();
      if next_kind == SyntaxKind::Eof {
        break;
      }
      if next_kind == SyntaxKind::Newline {
        let Some(after_prefix) = self.peek_md_multinewline_and_prefix(MD_PREFIX_EXACT) else {
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
        self.consume_md_multinewline_and_prefix(&mut children);
        block_indent = self.consume_md_indent(&mut children);
        let next = self.lex_ctx.peek_md(SKIP_NONE);
        if matches!(next.token.kind(), SyntaxKind::Newline | SyntaxKind::Eof) {
          break;
        }
        continue;
      }

      let (block, early_exit) = self.parse_md_block_element(block_indent);
      children.push(block);
      if early_exit.is_some_and(|ctx| ctx != item_ctx) {
        self.expr_ctx_stack.exit(item_ctx);
        return (
          self.emit(SyntaxKind::MdOrderedListItem, &children),
          early_exit,
        );
      }
      if early_exit == Some(item_ctx) {
        break;
      }
    }

    self.expr_ctx_stack.exit(item_ctx);
    (self.emit(SyntaxKind::MdOrderedListItem, &children), None)
  }

  /// Parse a container block: `::: label ... :::`
  /// INVARIANT: Expect ::: to be the next token, all spaces must already be consumed and passed
  pub(in crate::syntax::parse) fn parse_container_block(
    &mut self,
    indent: usize,
  ) -> (GreenNode, Option<ExprCtx>) {
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

    // Create context based on indentation
    let container_ctx = ExprCtx::MdContainerBlock { indent };
    self.expr_ctx_stack.enter(container_ctx);

    // Consume optional space between `:::` and the label
    if self.lex_ctx.peek_md(SKIP_NONE).token.kind() == SyntaxKind::Whitespace {
      self.advance_md(&mut children, SKIP_NONE);
    }

    // Require a label identifier (supports kebab-case like directory-index)
    if self.lex_ctx.peek_md(SKIP_NONE).token.kind() != SyntaxKind::Ident {
      self.emit_diagnostic(
        TdDiagnosticKind::MissingSyntaxNode {
          expected: SyntaxKind::Ident,
        }
        .into_error(self.offset(), self.offset()),
      );
    } else {
      self.consume_kebab_ident(&mut children);
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
      TdDiagnosticKind::MissingSyntaxNode {
        expected: SyntaxKind::Newline,
      }
      .into_error(self.offset(), self.offset()),
    );

    // Parse block elements until closing `:::` or EOF
    // The container creates a new indentation context: inner elements start at indent 0
    loop {
      let next_kind = self.lex_ctx.peek_md(SKIP_NONE).token.kind();
      if next_kind == SyntaxKind::Eof {
        break;
      }

      // Check for closing `:::` at the same indentation level as the opening
      // The closing `:::` should appear right after the full current prefix
      let check_position = if next_kind == SyntaxKind::Newline {
        let result = self.peek_md_multinewline_and_prefix(MD_PREFIX_EXACT);
        match result {
          Some(position) => position,
          None => break,
        }
      } else {
        self.expr_ctx_stack.md_prefix_tokens().len()
      };

      let after_nth_token = self.lex_ctx.peek_md_nth(check_position, SKIP_NONE);

      // Found closing :::, done
      if after_nth_token.token.kind() == SyntaxKind::MdSymbol
        && matches!(
          after_nth_token.token.chars().collect::<String>().as_str(),
          ":::"
        )
      {
        break;
      }

      // Parse slot
      let (slot, early_exit) = self.parse_container_slot();
      children.push(slot);

      // Not handler ctx, delegate
      if early_exit.is_some_and(|ctx| ctx != container_ctx) {
        self.expr_ctx_stack.exit(container_ctx);
        return (
          self.emit(SyntaxKind::MdContainerBlock, &children),
          early_exit,
        );
      } else if early_exit.is_some() {
        let mut error_children = vec![];
        let ctx = self.consume_or_delegate_md(container_ctx, &mut error_children); // Consume at least 1 token
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

      // Eat the newline after slot
      if self.lex_ctx.peek_md(SKIP_WS).token.kind() == SyntaxKind::Newline {
        self.advance_md(&mut children, SKIP_WS);
      }

      let next = self
        .lex_ctx
        .peek_md_nth(self.expr_ctx_stack.md_prefix_tokens().len(), SKIP_NONE);
      // Find slot separator after prefix
      if next.token.kind() == SyntaxKind::MdSymbol
        && matches!(next.token.chars().collect::<String>().as_str(), "===")
      {
        // Consume prefix
        if !self.consume_md_prefix(&mut children) {
          continue;
        }

        let mut separator_children = vec![];
        self.advance_md(&mut separator_children, SKIP_NONE); // Consume ===

        // Consume Ident
        let start_offset = self.offset();
        let end_offset = start_offset + self.lex_ctx.peek_md(SKIP_WS).token.text_len();
        self.consume_md_if(
          &mut separator_children,
          SKIP_WS,
          |token| token.kind() == SyntaxKind::Ident,
          TdDiagnosticKind::MissingSyntaxNode {
            expected: SyntaxKind::Ident,
          }
          .into_error(start_offset, end_offset),
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
          self.emit_diagnostic(
            TdDiagnosticKind::UnexpectedContainerSlotSeparatorToken {}
              .into_error(start_offset, end_offset),
          )
        }

        if !error_children.is_empty() {
          let error_node = self.emit(SyntaxKind::Error, &error_children);
          separator_children.push(error_node);
        }

        // Consume the newline of the separator
        if self.lex_ctx.peek_md(SKIP_WS).token.kind() == SyntaxKind::Newline {
          self.advance_md(&mut separator_children, SKIP_WS);
        }

        let separator = self.emit(SyntaxKind::MdContainerSlotSeparator, &separator_children);
        children.push(separator);
      }
    }

    // Consume newlines and prefix before the closing `:::`
    if self.lex_ctx.peek_md(SKIP_NONE).token.kind() == SyntaxKind::Newline {
      self.consume_md_multinewline_and_prefix(&mut children);
    } else if !self.expr_ctx_stack.md_prefix_tokens().is_empty() {
      // The newline was already consumed by the slot loop; consume just the prefix
      self.consume_md_prefix(&mut children);
    }

    // Consume closing `:::`
    self.consume_md_if(
      &mut children,
      SKIP_WS,
      |token| token.kind() == SyntaxKind::MdSymbol && token.chars().collect::<String>() == ":::",
      TdDiagnosticKind::MissingSyntaxNode {
        expected: SyntaxKind::MdContainerBlock,
      }
      .into_error(open_offset, self.offset()),
    );

    self.expr_ctx_stack.exit(container_ctx);
    (self.emit(SyntaxKind::MdContainerBlock, &children), None)
  }

  /// Parse [[identifier {props}]] as a self-closing container shorthand
  /// INVARIANT: Expect [[ to be the next two tokens
  pub(in crate::syntax::parse) fn parse_container_shorthand(
    &mut self,
  ) -> (GreenNode, Option<ExprCtx>) {
    debug_assert!(
      self.lex_ctx.peek_md(SKIP_NONE).token.kind() == SyntaxKind::LBracket
        && self.lex_ctx.peek_md_nth(1, SKIP_NONE).token.kind() == SyntaxKind::LBracket,
      "[ParseCtx::parse_container_shorthand] Expected [["
    );

    let mut children = vec![];

    // Consume [[
    self.advance_md(&mut children, SKIP_NONE);
    self.advance_md(&mut children, SKIP_NONE);

    // Require a label identifier (supports kebab-case like directory-index)
    if self.lex_ctx.peek_md(SKIP_NONE).token.kind() != SyntaxKind::Ident {
      self.emit_diagnostic(
        TdDiagnosticKind::MissingSyntaxNode {
          expected: SyntaxKind::Ident,
        }
        .into_error(self.offset(), self.offset()),
      );
    } else {
      self.consume_kebab_ident(&mut children);
    }

    // Optional props block {key=value}
    if self.lex_ctx.peek_md(SKIP_WS).token.kind() == SyntaxKind::LBrace {
      let (props, _) = self.parse_container_prop_block();
      children.push(props);
    }

    // Consume ]]
    self.consume_md(
      &mut children,
      SKIP_NONE,
      SyntaxKind::RBracket,
      TdDiagnosticKind::MissingSyntaxNode {
        expected: SyntaxKind::RBracket,
      }
      .into_error(self.offset(), self.offset()),
    );
    self.consume_md(
      &mut children,
      SKIP_NONE,
      SyntaxKind::RBracket,
      TdDiagnosticKind::MissingSyntaxNode {
        expected: SyntaxKind::RBracket,
      }
      .into_error(self.offset(), self.offset()),
    );

    (self.emit(SyntaxKind::MdContainerShorthand, &children), None)
  }

  // Stop on `:::` at matching indent, or EOF
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

  /// Parse a container props block: `{key=value}`
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
        self.emit_diagnostic(
          TdDiagnosticKind::UnexpectedContainerPropItem {}.into_error(start_offset, self.offset()),
        );

        let error_node = self.emit(SyntaxKind::Error, &error_children);

        children.push(error_node);

        if early_exit.is_some() {
          let start_offset = self.offset();
          let end_offset = self.offset() + self.lex_ctx.peek_md(SKIP_WCN).token.text_len();
          self.emit_diagnostic(
            TdDiagnosticKind::UnclosedContainerPropBlock {}.into_error(start_offset, end_offset),
          );

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
        self.emit_diagnostic(
          TdDiagnosticKind::UnclosedContainerPropBlock {}.into_error(start_offset, end_offset),
        );

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
      TdDiagnosticKind::UnclosedContainerPropBlock {}.into_error(start_offset, end_offset),
    );

    self.expr_ctx_stack.exit(ExprCtx::MdContainerPropBlock);

    (self.emit(SyntaxKind::MdContainerPropBlock, &children), None)
  }

  /// Parse a container prop item: `key=value`
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
      || maybe_eq_token.token.text().is_none_or(|text| text != "=")
    {
      self.expr_ctx_stack.exit(ExprCtx::MdContainerPropItem);

      return (self.emit(SyntaxKind::MdContainerPropItem, &children), None); // It's ok to not have eq
    }

    self.advance_md(&mut children, SKIP_NONE); // Consume eq

    // Enter prop_value_mode BEFORE peeking so the formula lexer handles full numbers (2.5, 1e3) as single tokens
    self.lex_ctx.enter_prop_value_mode();
    children.push(self.parse_js_prop_value());
    self.lex_ctx.exit_prop_value_mode();

    self.expr_ctx_stack.exit(ExprCtx::MdContainerPropItem);

    (self.emit(SyntaxKind::MdContainerPropItem, &children), None)
  }

  /// Parse a container slot: `=== <name>\n<content>\n=== <name>`
  /// INVARIANT: Expect content to be next
  /// INVARIANT: It must not consume the ending newline (except if there is an EOF)
  pub(in crate::syntax::parse) fn parse_container_slot(&mut self) -> (GreenNode, Option<ExprCtx>) {
    let mut children = vec![];

    self.expr_ctx_stack.enter(ExprCtx::MdContainerSlot);

    // Parse block elements until closing `:::`, `===` or EOF
    // The container creates a new indentation context: inner elements start at indent 0
    loop {
      let next_kind = self.lex_ctx.peek_md(SKIP_NONE).token.kind();
      if next_kind == SyntaxKind::Eof {
        break;
      }

      // Check for closing `:::`, `===` at the same indentation level as the opening
      // The closing `:::`, `===` should appear right after the full current prefix
      let check_position = if next_kind == SyntaxKind::Newline {
        let result = self.peek_md_multinewline_and_prefix(MD_PREFIX_EXACT);
        match result {
          Some(position) => position,
          None => break,
        }
      } else {
        self.expr_ctx_stack.md_prefix_tokens().len()
      };

      let after_nth = self.lex_ctx.peek_md_nth(check_position, SKIP_NONE);
      if after_nth.token.kind() == SyntaxKind::MdSymbol
        && matches!(
          after_nth.token.chars().collect::<String>().as_str(),
          ":::" | "==="
        )
      {
        break;
      }

      // Consume newline and prefix before parsing the next block element
      if next_kind == SyntaxKind::Newline {
        if !self.consume_md_multinewline_and_prefix(&mut children) {
          break;
        }
      } else if !self.expr_ctx_stack.md_prefix_tokens().is_empty()
        && !self.consume_md_prefix(&mut children)
      {
        break;
      }

      let (block, early_exit) = self.parse_md_block_element(0);
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

  /// Parse a link: `[text](url)`
  /// INVARIANT: The next token must be LBracket
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
      TdDiagnosticKind::MissingSyntaxNode {
        expected: SyntaxKind::MdLink,
      }
      .into_error(open_offset, open_offset),
    );
    if !ok {
      let handler = self
        .expr_ctx_stack
        .find_handler(&self.lex_ctx.peek_md(SKIP_NONE).token);
      return (self.emit(SyntaxKind::MdLink, &children), handler);
    }

    self.expr_ctx_stack.enter(ExprCtx::MdLinkText);

    // Collect alt text tokens until `]`, Newline, or EOF
    // Recognizes `![` as embedded image and inline code/math as distinct nodes
    let mut alt_children = vec![];
    loop {
      let peek = self.lex_ctx.peek_md(SKIP_NONE);
      match peek.token.kind() {
        SyntaxKind::RBracket | SyntaxKind::Newline | SyntaxKind::Eof => break,
        SyntaxKind::MdSymbol
          if peek.token.chars().collect::<String>() == "!"
            && self.lex_ctx.peek_md_nth(1, SKIP_NONE).token.kind() == SyntaxKind::LBracket =>
        {
          // Flush accumulated text before the embedded image
          if !alt_children.is_empty() {
            children.push(self.emit(SyntaxKind::MdText, &alt_children));
            alt_children = vec![];
          }
          let (media, handler) = self.parse_media();
          children.push(media);
          if let Some(ctx) = handler {
            self.expr_ctx_stack.exit(ExprCtx::MdLinkText);
            return (self.emit(SyntaxKind::MdLink, &children), Some(ctx));
          }
        }
        // Inline code and math inside link text get their own nodes
        SyntaxKind::InlineCode | SyntaxKind::InlineMath => {
          if !alt_children.is_empty() {
            children.push(self.emit(SyntaxKind::MdText, &alt_children));
            alt_children = vec![];
          }
          let kind = peek.token.kind();
          let mut code_children = vec![];
          self.advance_md(&mut code_children, SKIP_NONE);
          children.push(self.emit(kind, &code_children));
        }
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
    if !alt_children.is_empty() {
      children.push(self.emit(SyntaxKind::MdText, &alt_children));
    }

    // Hit newline or EOF before ]: treat the whole thing as plain text
    if is_unclosed {
      self.expr_ctx_stack.exit(ExprCtx::MdLinkText);
      return (self.emit(SyntaxKind::MdText, &children), None);
    }

    // Consume `]`
    let ok = self.consume_md(
      &mut children,
      SKIP_NONE,
      SyntaxKind::RBracket,
      TdDiagnosticKind::MissingSyntaxNode {
        expected: SyntaxKind::MdLink,
      }
      .into_error(open_offset, open_offset),
    );
    self.expr_ctx_stack.exit(ExprCtx::MdLinkText);
    if !ok {
      let handler = self
        .expr_ctx_stack
        .find_handler(&self.lex_ctx.peek_md(SKIP_NONE).token);
      return (self.emit(SyntaxKind::MdLink, &children), handler);
    }

    // No `(` after `]` means this is just `[text]`, not a link
    if self.lex_ctx.peek_md(SKIP_NONE).token.kind() != SyntaxKind::LParen {
      return (self.emit(SyntaxKind::MdText, &children), None);
    }

    // Consume `(`
    let ok = self.consume_md(
      &mut children,
      SKIP_NONE,
      SyntaxKind::LParen,
      TdDiagnosticKind::MissingSyntaxNode {
        expected: SyntaxKind::MdLink,
      }
      .into_error(open_offset, open_offset),
    );
    if !ok {
      let handler = self
        .expr_ctx_stack
        .find_handler(&self.lex_ctx.peek_md(SKIP_NONE).token);
      return (self.emit(SyntaxKind::MdLink, &children), handler);
    }

    self.expr_ctx_stack.enter(ExprCtx::MdLinkUrl);

    // Consume URL tokens until `)`, quote (title start), Newline, or EOF
    let mut url_children = vec![];
    loop {
      let peek = self.lex_ctx.peek_md(SKIP_NONE);
      match peek.token.kind() {
        SyntaxKind::RParen | SyntaxKind::Newline | SyntaxKind::Eof => break,
        SyntaxKind::DqStrStart | SyntaxKind::SqStrStart => break, // title starts
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

    // If a quoted title follows, parse it as a separate MdText node
    self.parse_link_title(&mut children);

    // Consume `)`
    let ok = self.consume_md(
      &mut children,
      SKIP_NONE,
      SyntaxKind::RParen,
      TdDiagnosticKind::MissingSyntaxNode {
        expected: SyntaxKind::MdLink,
      }
      .into_error(open_offset, open_offset),
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

  /// Parse a media embed: `![alt](src)`
  /// INVARIANT: The next token must be MdSymbol `!` followed by `[`
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
      TdDiagnosticKind::MissingSyntaxNode {
        expected: SyntaxKind::MdMedia,
      }
      .into_error(open_offset, open_offset),
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
      TdDiagnosticKind::MissingSyntaxNode {
        expected: SyntaxKind::MdMedia,
      }
      .into_error(open_offset, open_offset),
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

    // Hit newline or EOF before ]: treat the whole thing as plain text
    if is_unclosed {
      self.expr_ctx_stack.exit(ExprCtx::MdLinkText);
      return (self.emit(SyntaxKind::MdText, &children), None);
    }

    // Consume `]`
    let ok = self.consume_md(
      &mut children,
      SKIP_NONE,
      SyntaxKind::RBracket,
      TdDiagnosticKind::MissingSyntaxNode {
        expected: SyntaxKind::MdMedia,
      }
      .into_error(open_offset, open_offset),
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
      TdDiagnosticKind::MissingSyntaxNode {
        expected: SyntaxKind::MdMedia,
      }
      .into_error(open_offset, open_offset),
    );
    if !ok {
      let handler = self
        .expr_ctx_stack
        .find_handler(&self.lex_ctx.peek_md(SKIP_NONE).token);
      return (self.emit(SyntaxKind::MdMedia, &children), handler);
    }

    self.expr_ctx_stack.enter(ExprCtx::MdLinkUrl);

    // Consume URL tokens until `)`, quote (title start), Newline, or EOF
    let mut url_children = vec![];
    loop {
      let peek = self.lex_ctx.peek_md(SKIP_NONE);
      match peek.token.kind() {
        SyntaxKind::RParen | SyntaxKind::Newline | SyntaxKind::Eof => break,
        SyntaxKind::DqStrStart | SyntaxKind::SqStrStart => break, // title starts
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

    // If a quoted title follows, parse it as a separate MdText node
    self.parse_link_title(&mut children);

    // Consume `)`
    let ok = self.consume_md(
      &mut children,
      SKIP_NONE,
      SyntaxKind::RParen,
      TdDiagnosticKind::MissingSyntaxNode {
        expected: SyntaxKind::MdMedia,
      }
      .into_error(open_offset, open_offset),
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

  /// Parse an optional quoted title inside a link/media `(url "title")`
  /// Consumes `"title"` or `'title'` and emits the content as an MdText node
  fn parse_link_title(&mut self, children: &mut Vec<GreenNode>) {
    let peek = self.lex_ctx.peek_md(SKIP_NONE);
    let (_, end_kind) = match peek.token.kind() {
      SyntaxKind::DqStrStart => (SyntaxKind::DqStrStart, SyntaxKind::DqStrEnd),
      SyntaxKind::SqStrStart => (SyntaxKind::SqStrStart, SyntaxKind::SqStrEnd),
      _ => return,
    };

    // Consume opening quote
    self.advance_md(children, SKIP_NONE);

    // Collect title content tokens until closing quote, `)`, Newline, or EOF
    let mut title_children = vec![];
    loop {
      let peek = self.lex_ctx.peek_md(SKIP_NONE);
      match peek.token.kind() {
        kind if kind == end_kind => break,
        SyntaxKind::RParen | SyntaxKind::Newline | SyntaxKind::Eof => break,
        _ => {
          self.advance_md(&mut title_children, SKIP_NONE);
        }
      }
    }
    children.push(self.emit(SyntaxKind::MdText, &title_children));

    // Consume closing quote if present
    if self.lex_ctx.peek_md(SKIP_NONE).token.kind() == end_kind {
      self.advance_md(children, SKIP_NONE);
    }
  }

  /// Parse bold text: `**text**`
  /// INVARIANT: The next token must be MdSymbol `**`
  /// Leading whitespace must already be consumed by the caller
  /// Trailing whitespace after the closing delimiter is not consumed
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
        self.emit_diagnostic(
          TdDiagnosticKind::UnclosedBold {}.into_error(open_offset, self.offset()),
        );
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

  // Stop on `**`, EOF, or end of inline element
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

  /// Parse italic text: `*text*` or `_text_`
  /// INVARIANT: The next token must be MdSymbol `*` or `_`
  /// Leading whitespace must already be consumed by the caller
  /// Trailing whitespace after the closing delimiter is not consumed
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
          self.emit_diagnostic(
            TdDiagnosticKind::MismatchedItalicDelimiter {}.into_error(open_offset, self.offset()),
          );
        }
        break;
      }
      if self.should_end_inline_element(&mut children) {
        self.emit_diagnostic(
          TdDiagnosticKind::UnclosedItalic {}.into_error(open_offset, self.offset()),
        );
        break;
      }
      if self.lex_ctx.peek_md(SKIP_NONE).token.kind() == SyntaxKind::Newline {
        self.advance_md(&mut children, SKIP_NONE);
        continue;
      }
      let (inline, early_exit) = self.parse_md_inline_element();
      children.push(inline);
      if early_exit.is_some_and(|handler_ctx| handler_ctx != ctx) {
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

  // Stop on `*`/`_` matching `opening`, EOF, or end of inline element
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

  /// Parse bolditalic text: `***text***`
  /// INVARIANT: The next token must be MdSymbol `***`
  /// Leading whitespace must already be consumed by the caller
  /// Trailing whitespace after the closing delimiter is not consumed
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
        self.emit_diagnostic(
          TdDiagnosticKind::UnclosedBoldItalic {}.into_error(open_offset, self.offset()),
        );
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

  // Stop on `***`, EOF, or end of inline element
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

  /// Parse strikethrough text: `~~text~~`
  /// INVARIANT: The next token must be MdSymbol `~~`
  /// Leading whitespace must already be consumed by the caller
  /// Trailing whitespace after the closing delimiter is not consumed
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
        self.emit_diagnostic(
          TdDiagnosticKind::UnclosedStrikethrough {}.into_error(open_offset, self.offset()),
        );
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

  // Stop on `~~`, EOF, or end of inline element
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

  /// Parse a text run: consecutive plain text, including surrounding whitespace
  /// Consumes leading and trailing spaces
  pub(in crate::syntax::parse) fn parse_text(&mut self) -> (GreenNode, Option<ExprCtx>) {
    let in_table_cell = self.expr_ctx_stack.current() == Some(ExprCtx::MdTableCell);
    let mut children = vec![];

    loop {
      let next = self.lex_ctx.peek_md(SKIP_NONE);
      let next_kind = next.token.kind();
      if matches!(next_kind, SyntaxKind::Newline | SyntaxKind::Eof) {
        break;
      }
      // Inside a table cell, pipe is a cell separator, not text
      // FIXME: Handling `|` here is a bit adhoc, assuming that all block elements eventually delegate to parse_text which can stop at `|`
      if in_table_cell
        && next_kind == SyntaxKind::MdSymbol
        && next.token.chars().collect::<String>() == "|"
      {
        break;
      }
      if self.is_md_inline_start() {
        break;
      }
      self.advance_md(&mut children, SKIP_NONE);
    }

    (self.emit(SyntaxKind::MdText, &children), None)
  }

  // Consume a kebab-case identifier (e.g. directory-index)
  // Advances through Ident, MdSymbol("-"), Ident sequences
  fn consume_kebab_ident(&mut self, children: &mut Vec<GreenNode>) {
    self.advance_md(children, SKIP_NONE);
    loop {
      let peek = self.lex_ctx.peek_md(SKIP_NONE);
      if peek.token.kind() == SyntaxKind::MdSymbol
        && peek.token.chars().collect::<String>() == "-"
        && self.lex_ctx.peek_md_nth(1, SKIP_NONE).token.kind() == SyntaxKind::Ident
      {
        self.advance_md(children, SKIP_NONE); // -
        self.advance_md(children, SKIP_NONE); // ident
      } else {
        break;
      }
    }
  }

  // Consume the expected prefix on the next line
  fn consume_md_prefix(&mut self, children: &mut Vec<GreenNode>) -> bool {
    debug_assert!(
      self.lex_ctx.peek_md(SKIP_NONE).token.kind() != SyntaxKind::Newline,
      "[ParseCtx::consume_md_prefix] Should be at line start, not sitting on a Newline"
    );

    let expected_prefix_tokens = self.expr_ctx_stack.md_prefix_tokens().to_vec();
    if expected_prefix_tokens.is_empty() {
      return true;
    }

    let mut prefix_children = vec![]; // Collect children for MdPrefix node

    for expected_prefix_token in &expected_prefix_tokens {
      let peek = self.lex_ctx.peek_md(SKIP_NONE);

      // Mismatch, only tolerate optional whitespace prefix tokens on blank lines
      if peek.token != *expected_prefix_token {
        // Check if can be skipped
        if expected_prefix_token.kind() == SyntaxKind::Whitespace
          && matches!(peek.token.kind(), SyntaxKind::Newline | SyntaxKind::Eof)
        {
          continue;
        }

        // Error!
        self.emit_diagnostic(
          TdDiagnosticKind::MissingExpectedMdPrefix {
            expected_prefix: format!("{:?}", expected_prefix_tokens),
          }
          .into_error(self.offset(), self.offset()),
        );

        // Consume the prefix into a prefix node
        if !prefix_children.is_empty() {
          children.push(self.emit(SyntaxKind::MdPrefix, &prefix_children));
        }
        return false;
      }

      self.advance_md(&mut prefix_children, SKIP_NONE);
    }

    // Consume the prefix into a prefix node
    if !prefix_children.is_empty() {
      children.push(self.emit(SyntaxKind::MdPrefix, &prefix_children));
    }
    true
  }

  fn consume_md_whitespace(&mut self, children: &mut Vec<GreenNode>) {
    while self.lex_ctx.peek_md(SKIP_NONE).token.kind() == SyntaxKind::Whitespace {
      self.advance_md(children, SKIP_NONE);
    }
  }

  // Consume a newline and the expected prefix on the next line
  fn consume_md_newline_and_prefix(&mut self, children: &mut Vec<GreenNode>) -> bool {
    self.advance_md(children, SKIP_WS);
    self.consume_md_prefix(children)
  }

  // Consume one or more newlines (blank lines) and the expected prefix
  fn consume_md_multinewline_and_prefix(&mut self, children: &mut Vec<GreenNode>) -> bool {
    self.advance_md(children, SKIP_NONE);
    loop {
      let kind = self.lex_ctx.peek_md(SKIP_NONE).token.kind();
      if kind == SyntaxKind::Newline {
        self.advance_md(children, SKIP_NONE);
        continue;
      }
      // Skip prefix tokens on blank lines (e.g. bare `>` in blockquotes)
      if matches!(kind, SyntaxKind::Whitespace | SyntaxKind::MdSymbol)
        && let Some(past_prefix) = self.peek_md_prefix_at(0, MD_PREFIX_LAX)
      {
        // The token after the prefix
        let after_nth_token = self
          .lex_ctx
          .peek_md_nth(past_prefix, SKIP_NONE)
          .token
          .kind();

        // If it's a newline, it's a blank line
        if after_nth_token == SyntaxKind::Newline {
          let mut prefix_children = vec![];
          for _ in 0..past_prefix {
            self.advance_md(&mut prefix_children, SKIP_NONE);
          }
          children.push(self.emit(SyntaxKind::MdPrefix, &prefix_children));
          self.advance_md(children, SKIP_NONE);
          continue;
        }
      }
      break;
    }
    self.consume_md_prefix(children)
  }

  // If the next token should be handled by an outer context, return that context
  // Otherwise consume the token into `error_children` for the caller to wrap as Error
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

  /// Whether the current inline element should end due to EOF or a line boundary
  /// Consumes the newline and prefix if present
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
            .is_inside(|handler_ctx| matches!(handler_ctx, ExprCtx::MdContainerBlock { .. }));
        }
      }
      // Indented table row
      if after.token.kind() == SyntaxKind::Whitespace {
        let after_ws = self.lex_ctx.peek_md(SKIP_WS);
        if after_ws.token.kind() == SyntaxKind::MdSymbol
          && after_ws.token.chars().collect::<String>() == "|"
        {
          return true;
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

  /// Whether the next token starts an inline element
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

  // Consume leading whitespace tokens and return the count
  fn consume_md_indent(&mut self, children: &mut Vec<GreenNode>) -> usize {
    let mut count: usize = 0;
    while self.lex_ctx.peek_md(SKIP_NONE).token.kind() == SyntaxKind::Whitespace {
      self.advance_md(children, SKIP_NONE);
      count += 1;
    }
    count
  }

  // Return the offset of the first non-whitespace token at or after `offset`
  fn peek_md_past_whitespace_at(&mut self, offset: usize) -> usize {
    let mut position = offset;
    while self.lex_ctx.peek_md_nth(position, SKIP_NONE).token.kind() == SyntaxKind::Whitespace {
      position += 1;
    }
    position
  }

  // Check whether tokens at `offset` match the expected prefix
  fn peek_md_prefix_at(&mut self, offset: usize, flags: usize) -> Option<usize> {
    let expected_tokens = self.expr_ctx_stack.md_prefix_tokens().to_vec();
    let mut position = offset;
    for expected_token in &expected_tokens {
      let actual = self.lex_ctx.peek_md_nth(position, SKIP_NONE);
      if actual.token != *expected_token {
        if flags & MD_PREFIX_LAX != 0
          && expected_token.kind() == SyntaxKind::Whitespace
          && matches!(actual.token.kind(), SyntaxKind::Newline | SyntaxKind::Eof)
        {
          // Skip optional whitespace prefix tokens on blank lines,
          // but keep checking remaining non-whitespace tokens
          continue;
        }
        return None;
      }
      position += 1;
    }
    Some(position)
  }

  // Peek whether the next token is a newline followed by the expected prefix
  // Returns the offset after the matched prefix, or None if mismatch
  // INVARIANT: The next token must be a Newline
  fn peek_md_newline_and_prefix(&mut self, flags: usize) -> Option<usize> {
    debug_assert!(
      matches!(
        self.lex_ctx.peek_md(SKIP_NONE).token.kind(),
        SyntaxKind::Newline | SyntaxKind::Eof
      ),
      "[ParseCtx::peek_md_newline_and_prefix] Expected next token to be Newline"
    );
    self.peek_md_prefix_at(1, flags)
  }

  // Like peek_md_newline_and_prefix but skips blank lines first
  // Returns the offset after the matched prefix, or None if mismatch
  // INVARIANT: The next token must be a Newline
  fn peek_md_multinewline_and_prefix(&mut self, flags: usize) -> Option<usize> {
    debug_assert!(
      matches!(
        self.lex_ctx.peek_md(SKIP_NONE).token.kind(),
        SyntaxKind::Newline | SyntaxKind::Eof
      ),
      "[ParseCtx::peek_md_multinewline_and_prefix] Expected next token to be Newline"
    );
    let mut offset = 1;
    loop {
      let kind = self.lex_ctx.peek_md_nth(offset, SKIP_NONE).token.kind();
      if kind == SyntaxKind::Newline {
        offset += 1;
        continue;
      }
      // Skip prefix tokens on blank lines (e.g. bare `>` in blockquotes)
      if (kind == SyntaxKind::Whitespace || kind == SyntaxKind::MdSymbol)
        && let Some(past_prefix) = self.peek_md_prefix_at(offset, MD_PREFIX_LAX)
      {
        let after = self
          .lex_ctx
          .peek_md_nth(past_prefix, SKIP_NONE)
          .token
          .kind();
        if after == SyntaxKind::Newline {
          offset = past_prefix + 1;
          continue;
        }
      }
      break;
    }
    self.peek_md_prefix_at(offset, flags)
  }
}

// Block element start detection helpers
impl<S: Utf8Stream> ParseCtx<S> {
  // `---`, `***`, or `___` (3+ same char) followed by newline/eof
  fn is_horizontal_rule_start(&mut self, skip: usize) -> bool {
    let next = self.lex_ctx.peek_md(skip);
    if next.token.kind() != SyntaxKind::MdSymbol {
      return false;
    }
    let text: String = next.token.chars().collect();
    if text.len() < 3 {
      return false;
    }
    let first = text.chars().next().unwrap();
    if !matches!(first, '-' | '*' | '_') || !text.chars().all(|char| char == first) {
      return false;
    }
    let after = self.lex_ctx.peek_md_nth(1, skip).token.kind();
    matches!(after, SyntaxKind::Newline | SyntaxKind::Eof)
  }

  // WARNING: Prefix must be consumed already
  fn is_heading_start(&mut self, skip: usize) -> bool {
    let next = self.lex_ctx.peek_md(skip);
    next.token.kind() == SyntaxKind::MdSymbol && next.token.chars().all(|char| char == '#')
  }

  fn is_bullet_list_start(&mut self, skip: usize) -> bool {
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

  fn is_ordered_list_start(&mut self, skip: usize) -> bool {
    let next = self.lex_ctx.peek_md(skip);
    if next.token.kind() != SyntaxKind::MdNumber {
      return false;
    }
    let dot = self.lex_ctx.peek_md_nth(1, skip);
    dot.token.kind() == SyntaxKind::MdSymbol && dot.token.chars().collect::<String>() == "."
  }

  fn is_blockquote_start(&mut self, skip: usize) -> bool {
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

  fn is_table_start(&mut self, skip: usize) -> bool {
    // Skip leading whitespace since table rows can be indented
    let offset = self.peek_md_past_whitespace_at(0);
    let next = self.lex_ctx.peek_md_nth(offset, skip);
    next.token.kind() == SyntaxKind::MdSymbol && next.token.chars().collect::<String>() == "|"
  }

  fn is_container_start(&mut self, skip: usize) -> bool {
    let next = self.lex_ctx.peek_md(skip);
    next.token.kind() == SyntaxKind::MdSymbol && next.token.chars().collect::<String>() == ":::"
  }

  fn is_container_shorthand_start(&mut self, skip: usize) -> bool {
    let first = self.lex_ctx.peek_md(skip);
    first.token.kind() == SyntaxKind::LBracket
      && self.lex_ctx.peek_md_nth(1, skip).token.kind() == SyntaxKind::LBracket
  }

  fn is_media_block_start(&mut self, skip: usize) -> bool {
    let next = self.lex_ctx.peek_md(skip);
    if next.token.kind() != SyntaxKind::MdSymbol {
      return false;
    }
    next.token.chars().collect::<String>() == "!"
      && self.lex_ctx.peek_md_nth(1, skip).token.kind() == SyntaxKind::LBracket
  }

  fn is_code_or_math_block_start(&mut self, skip: usize) -> bool {
    let next = self.lex_ctx.peek_md(skip);
    matches!(
      next.token.kind(),
      SyntaxKind::CodeBlock | SyntaxKind::MathBlock
    )
  }

  // Check if token at a specific offset (using SKIP_NONE) starts a block element
  fn is_md_block_start_at(&mut self, offset: usize) -> bool {
    // Skip leading whitespace when inside a list item
    let mut offset = offset;
    if self.expr_ctx_stack.is_inside(|handler_ctx| {
      matches!(
        handler_ctx,
        ExprCtx::MdUnorderedListItem | ExprCtx::MdOrderedListItem { .. } | ExprCtx::MdTaskListItem
      )
    }) {
      while self.lex_ctx.peek_md_nth(offset, SKIP_NONE).token.kind() == SyntaxKind::Whitespace {
        offset += 1;
      }
    }

    let first = self.lex_ctx.peek_md_nth(offset, SKIP_NONE);
    match first.token.kind() {
      SyntaxKind::MdSymbol => {
        let text: String = first.token.chars().collect();
        if text.chars().all(|handler_ctx| handler_ctx == '#') {
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
            .is_inside(|handler_ctx| matches!(handler_ctx, ExprCtx::MdContainerBlock { .. }));
        }
        false
      }
      SyntaxKind::MdNumber => {
        let dot = self.lex_ctx.peek_md_nth(offset + 1, SKIP_NONE);
        dot.token.kind() == SyntaxKind::MdSymbol && dot.token.chars().collect::<String>() == "."
      }
      SyntaxKind::LBracket => {
        let next = self.lex_ctx.peek_md_nth(offset + 1, SKIP_NONE);
        next.token.kind() == SyntaxKind::LBracket
      }
      SyntaxKind::CodeBlock | SyntaxKind::MathBlock => true,
      // Indented table row
      SyntaxKind::Whitespace => {
        let past_ws = self.peek_md_past_whitespace_at(offset);
        let after_ws = self.lex_ctx.peek_md_nth(past_ws, SKIP_NONE);
        after_ws.token.kind() == SyntaxKind::MdSymbol
          && after_ws.token.chars().collect::<String>() == "|"
      }
      _ => false,
    }
  }
}
