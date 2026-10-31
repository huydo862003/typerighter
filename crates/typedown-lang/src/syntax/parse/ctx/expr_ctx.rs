//! Expression context for error recovery during parsing

use std::{cell::RefCell, rc::Rc};

use crate::syntax::syntax_kind::SyntaxKind;

use crate::syntax::green::{SyntaxToken, cache::Cache};

/// Expression context stack entry
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::syntax::parse) enum ExprCtx {
  /// Top-level YAML frontmatter context
  YamlFrontmatter,
  /// Inside `${...}` interpolation, closed by `}`
  Interp,
  /// Inside `[...]` list, closed by `]`
  List,
  /// Inside `{...}` dict, closed by `}`
  Dict,
  /// Inside `"..."` double-quoted string, closed by `"`
  DqString,
  /// Inside `'...'` single-quoted string, closed by `'`
  SqString,
  /// Inside `(...)` parenthesized expression, closed by `)`
  Paren,
  /// Inside `func(...)` call expression, closed by `)`
  Call,
  /// Inside `expr[...]` index expression, closed by `]`
  Index,
  /// Inside a block sequence
  BlockSeq,
  /// Inside a block mapping
  BlockMap,

  /// Top-level Markdown body context
  MarkdownBody,
  /// Inside a `:::` container block, closed by `:::`
  MdContainerBlock {
    /// Extra indentation beyond the parent prefix (spaces before `:::`)
    indent: usize,
  },
  MdContainerPropBlock,
  MdContainerPropItem,
  MdContainerSlot,
  /// Inside a `>` blockquote
  MdBlockQuote,
  /// Inside an ordered list
  MdOrderedList {
    /// Extra leading indentation (spaces before the number)
    indent: usize,
  },
  /// Inside an ordered list item
  MdOrderedListItem {
    /// Marker width including trailing space (e.g. 3 for `1. `, 4 for `10. `)
    marker_width: usize,
  },
  /// Inside an unordered list
  MdUnorderedList {
    /// Extra leading indentation (spaces before `-`)
    indent: usize,
  },
  /// Inside an unordered list item
  MdUnorderedListItem,
  /// Inside a task list item
  MdTaskListItem,
  /// Inside a table
  MdTable,
  /// Inside a table row, closed by Newline
  MdTableRow,
  /// Inside a table cell, closed by `|`
  MdTableCell,

  /// Inside prose
  MdParagraph,

  /// Inside the text part of a markdown link or media: `[text]`
  MdLinkText,
  /// Inside the url/src part of a markdown link or media: `(url)`
  MdLinkUrl,
  /// Inside `**text**`, closed by `**`
  MdBold,
  /// Inside `*text*`, closed by `*`
  MdItalicStar,
  /// Inside `_text_`, closed by `_`
  MdItalicUnderscore,
  /// Inside `***text***`, closed by `***`
  MdBoldItalic,
  /// Inside `~~text~~`, closed by `~~`
  MdStrikethrough,
}

struct ExprStackEntry {
  ctx: ExprCtx,
  /// Number of prefix tokens this context contributed
  prefix_token_count: usize,
}

/// Stack of expression contexts for error recovery in expressions
pub(in crate::syntax::parse) struct ExprCtxStack {
  stack: Vec<ExprStackEntry>,
  cache: Rc<RefCell<Cache>>,
  /// Accumulated MD line prefix as a sequence of expected tokens
  md_prefix_tokens: Vec<SyntaxToken>,
}

impl ExprCtxStack {
  pub(in crate::syntax::parse) fn new(cache: Rc<RefCell<Cache>>) -> Self {
    Self {
      stack: Vec::new(),
      cache,
      md_prefix_tokens: Vec::new(),
    }
  }

  /// Push a context onto the stack
  pub(in crate::syntax::parse) fn enter(&mut self, ctx: ExprCtx) {
    let before = self.md_prefix_tokens.len();
    self.push_md_prefix_tokens(ctx);
    let prefix_token_count = self.md_prefix_tokens.len() - before;
    self.stack.push(ExprStackEntry {
      ctx,
      prefix_token_count,
    });
  }

  /// Pop the current context
  pub(in crate::syntax::parse) fn exit(&mut self, expected: ExprCtx) {
    let entry = self.stack.pop();
    debug_assert!(
      entry.as_ref().unwrap().ctx == expected,
      "[ExprCtxStack::exit] Expected {:?} but got {:?}",
      expected,
      entry.as_ref().unwrap().ctx,
    );
    if let Some(entry) = entry {
      let new_len = self.md_prefix_tokens.len() - entry.prefix_token_count;
      self.md_prefix_tokens.truncate(new_len);
    }
  }

  pub(in crate::syntax::parse) fn current(&self) -> Option<ExprCtx> {
    self.stack.last().map(|entry| entry.ctx)
  }

  pub(in crate::syntax::parse) fn is_inside(&self, pred: impl Fn(ExprCtx) -> bool) -> bool {
    self
      .stack
      .iter()
      .find(|context| pred(context.ctx))
      .is_some()
  }

  // The accumulated expected MD prefix tokens
  pub(in crate::syntax::parse) fn md_prefix_tokens(&self) -> &[SyntaxToken] {
    &self.md_prefix_tokens
  }

  /// Whether expressions should skip indent/dedent tokens
  pub(in crate::syntax::parse) fn should_expr_skip_indent(&self) -> bool {
    self
      .stack
      .iter()
      .any(|entry| entry.ctx.should_expr_skip_indent())
  }

  /// Whether expressions can span across newlines
  pub(in crate::syntax::parse) fn should_expr_span_newline(&self) -> bool {
    self
      .stack
      .iter()
      .any(|entry| entry.ctx.should_expr_span_newline())
  }

  /// Push expected prefix tokens for the given context
  fn push_md_prefix_tokens(&mut self, ctx: ExprCtx) {
    let mut cache = self.cache.borrow_mut();
    match ctx {
      ExprCtx::MdBlockQuote => {
        // `>` followed by optional space (blank lines may omit the space)
        self
          .md_prefix_tokens
          .push(cache.token(SyntaxKind::MdSymbol, b">"));
        self
          .md_prefix_tokens
          .push(cache.token(SyntaxKind::Whitespace, b" "));
      }
      ExprCtx::MdUnorderedList { indent }
      | ExprCtx::MdOrderedList { indent }
      | ExprCtx::MdContainerBlock { indent } => {
        for _ in 0..indent {
          self
            .md_prefix_tokens
            .push(cache.token(SyntaxKind::Whitespace, b" "));
        }
      }
      // Per CommonMark spec, continuation lines must be indented to the column where content starts after the list marker
      // For `- ` that's 2, for `1. ` that's 3, etc
      ExprCtx::MdUnorderedListItem | ExprCtx::MdTaskListItem => {
        for _ in 0..2 {
          self
            .md_prefix_tokens
            .push(cache.token(SyntaxKind::Whitespace, b" "));
        }
      }
      ExprCtx::MdOrderedListItem { marker_width } => {
        for _ in 0..marker_width {
          self
            .md_prefix_tokens
            .push(cache.token(SyntaxKind::Whitespace, b" "));
        }
      }
      _ => {}
    }
  }

  /// Find the innermost context that can handle the given token
  /// Falls back to the current (innermost) context if none matches
  pub(in crate::syntax::parse) fn find_handler(&self, token: &SyntaxToken) -> Option<ExprCtx> {
    self
      .stack
      .iter()
      .rev()
      .map(|entry| entry.ctx)
      .find(|ctx| ctx.can_handle(token))
      .or_else(|| self.current())
  }
}

impl ExprCtx {
  /// Whether expressions in this context should skip indent/dedent tokens
  pub(in crate::syntax::parse) fn should_expr_skip_indent(self) -> bool {
    matches!(
      self,
      ExprCtx::List | ExprCtx::Dict | ExprCtx::Paren | ExprCtx::Call | ExprCtx::Index
    )
  }

  /// Whether expressions in this context can span across newlines
  pub(in crate::syntax::parse) fn should_expr_span_newline(self) -> bool {
    matches!(
      self,
      ExprCtx::List | ExprCtx::Dict | ExprCtx::Paren | ExprCtx::Call | ExprCtx::Index
    )
  }

  /// Whether this context can handle the given token
  pub(in crate::syntax::parse) fn can_handle(self, token: &SyntaxToken) -> bool {
    if matches!(token.kind(), SyntaxKind::YamlOp | SyntaxKind::MdSymbol) {
      let text: String = token.chars().collect();
      return matches!(
        (self, text.as_str()),
        (ExprCtx::YamlFrontmatter, "---")
          | (ExprCtx::MdBold, "**")
          | (ExprCtx::MdBold, "__")
          | (ExprCtx::MdItalicStar, "*")
          | (ExprCtx::MdItalicUnderscore, "_")
          | (ExprCtx::MdBoldItalic, "***")
          | (ExprCtx::MdBoldItalic, "___")
          | (ExprCtx::MdStrikethrough, "~~")
          | (ExprCtx::MdContainerBlock { .. }, ":::")
          | (ExprCtx::MdTableCell, "|")
      );
    }

    matches!(
      (self, token.kind()),
      (ExprCtx::YamlFrontmatter, SyntaxKind::YamlIndent)
        | (ExprCtx::YamlFrontmatter, SyntaxKind::Eof)
        | (ExprCtx::MarkdownBody, SyntaxKind::Eof)
        | (ExprCtx::Interp, SyntaxKind::InterpEnd)
        | (ExprCtx::List, SyntaxKind::RBracket)
        | (ExprCtx::List, SyntaxKind::Comma)
        | (ExprCtx::Dict, SyntaxKind::RBrace)
        | (ExprCtx::Dict, SyntaxKind::Comma)
        | (ExprCtx::Dict, SyntaxKind::Colon)
        | (ExprCtx::DqString, SyntaxKind::DqStrEnd)
        | (ExprCtx::SqString, SyntaxKind::SqStrEnd)
        | (ExprCtx::Paren, SyntaxKind::RParen)
        | (ExprCtx::Call, SyntaxKind::RParen)
        | (ExprCtx::Call, SyntaxKind::Comma)
        | (ExprCtx::Index, SyntaxKind::RBracket)
        | (ExprCtx::Index, SyntaxKind::Comma)
        | (ExprCtx::BlockSeq, SyntaxKind::Newline)
        | (ExprCtx::BlockMap, SyntaxKind::Newline)
        | (ExprCtx::MdBlockQuote, SyntaxKind::Newline)
        | (ExprCtx::MdBlockQuote, SyntaxKind::Eof)
        | (ExprCtx::MdOrderedList { .. }, SyntaxKind::Eof)
        | (ExprCtx::MdOrderedListItem { .. }, SyntaxKind::Newline)
        | (ExprCtx::MdOrderedListItem { .. }, SyntaxKind::Eof)
        | (ExprCtx::MdUnorderedList { .. }, SyntaxKind::Eof)
        | (ExprCtx::MdUnorderedListItem, SyntaxKind::Newline)
        | (ExprCtx::MdUnorderedListItem, SyntaxKind::Eof)
        | (ExprCtx::MdTaskListItem, SyntaxKind::Newline)
        | (ExprCtx::MdTaskListItem, SyntaxKind::Eof)
        | (ExprCtx::MdTable, SyntaxKind::Eof)
        | (ExprCtx::MdTableRow, SyntaxKind::Newline)
        | (ExprCtx::MdTableRow, SyntaxKind::Eof)
        | (ExprCtx::MdTableCell, SyntaxKind::Newline)
        | (ExprCtx::MdTableCell, SyntaxKind::Eof)
        | (ExprCtx::MdLinkText, SyntaxKind::RBracket)
        | (ExprCtx::MdLinkText, SyntaxKind::Newline)
        | (ExprCtx::MdLinkText, SyntaxKind::Eof)
        | (ExprCtx::MdLinkUrl, SyntaxKind::RParen)
        | (ExprCtx::MdLinkUrl, SyntaxKind::Newline)
        | (ExprCtx::MdLinkUrl, SyntaxKind::Eof)
        | (ExprCtx::MdBold, SyntaxKind::Newline)
        | (ExprCtx::MdBold, SyntaxKind::Eof)
        | (ExprCtx::MdItalicStar, SyntaxKind::Newline)
        | (ExprCtx::MdItalicStar, SyntaxKind::Eof)
        | (ExprCtx::MdItalicUnderscore, SyntaxKind::Newline)
        | (ExprCtx::MdItalicUnderscore, SyntaxKind::Eof)
        | (ExprCtx::MdBoldItalic, SyntaxKind::Newline)
        | (ExprCtx::MdBoldItalic, SyntaxKind::Eof)
        | (ExprCtx::MdStrikethrough, SyntaxKind::Newline)
        | (ExprCtx::MdStrikethrough, SyntaxKind::Eof)
        | (ExprCtx::MdContainerBlock { .. }, SyntaxKind::Eof)
        | (ExprCtx::MdContainerPropBlock, SyntaxKind::RBrace)
    )
  }
}
