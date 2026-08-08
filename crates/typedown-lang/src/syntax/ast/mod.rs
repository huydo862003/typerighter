//! Layer 3: Typed AST nodes wrapping untyped RedNodes.
//! Each AST type checks the SyntaxKind on cast, providing a type-safe API
//! over the generic tree structure.

use std::hash::Hash;

use crate::syntax::syntax_kind::SyntaxKind;
use typedown_macros::{AstNode, wrapper_ast_node};
use typedown_types::either::Either;
use typedown_types::unescape::{unescape, unescape_html_entity};

use crate::syntax::red::RedNode;

/// All AST nodes implement this trait.
pub trait AstNode: Sized + Clone + Eq + Hash + Send + Sync {
  /// Try to cast a RedNode into this AST type.
  /// Returns None if the SyntaxKind doesn't match.
  fn cast(syntax: RedNode) -> Option<Self>;

  /// Access the underlying RedNode.
  fn syntax(&self) -> &RedNode;
}

fn child<T: AstNode>(parent: &RedNode) -> Option<T> {
  parent.children().find_map(T::cast)
}

fn children<T: AstNode>(parent: &RedNode) -> impl Iterator<Item = T> {
  parent.children().filter_map(T::cast)
}

/* Top-level nodes */

/// The root of a Typedown file: frontmatter + body.
#[derive(Clone, PartialEq, Eq, Hash, AstNode)]
pub struct SourceFile(RedNode);

impl SourceFile {
  /// Whether the file has a non-empty frontmatter section (opened with ---)
  pub fn has_nonempty_frontmatter(&self) -> bool {
    self
      .frontmatter()
      .is_some_and(|fm| fm.syntax().children().count() > 0)
  }

  /// Return the frontmatter of the source file
  pub fn frontmatter(&self) -> Option<YamlFrontmatter> {
    child::<YamlFrontmatter>(&self.0)
  }

  /// Return the body of the source file
  pub fn body(&self) -> Option<MdBody> {
    child::<MdBody>(&self.0)
  }
}

/// The YAML frontmatter
#[derive(Clone, PartialEq, Eq, Hash, AstNode)]
pub struct YamlFrontmatter(RedNode);

impl YamlFrontmatter {
  /// Return the top-level mapping in the frontmatter
  pub fn mapping(&self) -> Option<YamlMapping> {
    child::<YamlMapping>(&self.0)
  }
}

/// The YAML mapping
#[derive(Clone, PartialEq, Eq, Hash, AstNode)]
pub struct YamlMapping(RedNode);

impl YamlMapping {
  /// Return an iterator over the mapping keys
  pub fn keys(&self) -> impl Iterator<Item = String> {
    children::<YamlMappingEntry>(&self.0).filter_map(|e| e.key())
  }

  /// Return an iterator over the mapping values
  pub fn values(&self) -> impl Iterator<Item = Expr> {
    children::<YamlMappingEntry>(&self.0).filter_map(|e| e.value())
  }

  /// Return an iterator over the entries
  pub fn entries(&self) -> impl Iterator<Item = (String, Expr)> {
    children::<YamlMappingEntry>(&self.0).filter_map(|e| e.entry())
  }
}

/// The YAML mapping's key-value pair
#[derive(Clone, PartialEq, Eq, Hash, AstNode)]
pub struct YamlMappingEntry(RedNode);

impl YamlMappingEntry {
  /// Return the key of this mapping entry
  pub fn key(&self) -> Option<String> {
    self
      .0
      .children()
      .find(|c| c.kind() == SyntaxKind::YamlMappingEntryKey)
      .map(|v| v.chars().collect::<String>())
  }

  /// Return the value of this mapping entry
  pub fn value(&self) -> Option<Expr> {
    let entry_value = self
      .0
      .children()
      .find(|c| c.kind() == SyntaxKind::YamlMappingEntryValue)?;
    entry_value.children().find_map(Expr::cast)
  }

  /// Return the entry of this mapping entry
  pub fn entry(&self) -> Option<(String, Expr)> {
    Some((self.key()?, self.value()?))
  }
}

/// The YAML sequence
#[derive(Clone, PartialEq, Eq, Hash, AstNode)]
pub struct YamlSequence(RedNode);
impl YamlSequence {
  /// Return the items of this sequence
  pub fn values(&self) -> impl Iterator<Item = Expr> {
    children::<YamlSequenceItem>(&self.0).filter_map(|e| e.value())
  }
}

/// The YAML sequence item
#[derive(Clone, PartialEq, Eq, Hash, AstNode)]
pub struct YamlSequenceItem(RedNode);

impl YamlSequenceItem {
  /// Return the value of this sequence item
  pub fn value(&self) -> Option<Expr> {
    self.0.children().find_map(Expr::cast)
  }
}

/// The Markdown body
#[derive(Clone, PartialEq, Eq, Hash, AstNode)]
pub struct MdBody(RedNode);

impl MdBody {
  pub fn block_elements(&self) -> impl Iterator<Item = MdBlockElement> {
    children::<MdBlockElement>(&self.0)
  }
}

#[wrapper_ast_node(SyntaxKind = [
  MdHeading, MdParagraph, MdBlockquote, MdTable,
  MdBulletList, MdOrderedList, MdToggleList, MdContainerBlock,
  MdLink, MdMedia,
  MdBold, MdItalic, MdBoldItalic, MdStrikethrough,
  MdText, MdHtmlEntity,
])]
pub struct MdNode(RedNode);

#[wrapper_ast_node(SyntaxKind = [
  MdHeading, MdParagraph, MdBlockquote, MdTable,
  MdBulletList, MdOrderedList, MdToggleList, MdContainerBlock,
])]
pub struct MdBlockElement(RedNode);

#[wrapper_ast_node(SyntaxKind = [
  MdLink, MdMedia,
  MdBold, MdItalic, MdBoldItalic, MdStrikethrough,
  MdText,
])]
pub struct MdInlineElement(RedNode);

/// The Markdown heading
/// Represented by: ## Heading
#[derive(Clone, PartialEq, Eq, Hash, AstNode)]
pub struct MdHeading(RedNode);

impl MdHeading {
  /// Returns the heading level (1 for `#`, 2 for `##`, etc.).
  pub fn level(&self) -> usize {
    self
      .0
      .children()
      .next()
      .and_then(|child| {
        child.as_token().map(|token| {
          token
            .text()
            .unwrap_or("")
            .chars()
            .filter(|ch| *ch == '#')
            .count()
        })
      })
      .unwrap_or(0)
  }
}

/// The Markdown paragraph
/// Represented by: Paragraph ...
#[derive(Clone, PartialEq, Eq, Hash, AstNode)]
pub struct MdParagraph(RedNode);

impl MdParagraph {
  pub fn inline_elements(&self) -> impl Iterator<Item = MdInlineElement> {
    children::<MdInlineElement>(&self.0)
  }
}

/// The Markdown blockquote
/// Represented by: > Blockquote
#[derive(Clone, PartialEq, Eq, Hash, AstNode)]
pub struct MdBlockquote(RedNode);

impl MdBlockquote {
  pub fn block_elements(&self) -> impl Iterator<Item = MdBlockElement> {
    children::<MdBlockElement>(&self.0)
  }
}

/// The Markdown table
/// Represented by:
/// | header 1 | header 2 |
/// | -------- | -------- |
/// | data 1   | data 2   |
#[derive(Clone, PartialEq, Eq, Hash, AstNode)]
pub struct MdTable(RedNode);

impl MdTable {
  pub fn header(&self) -> Option<MdTableHeaderRow> {
    self.0.children().find_map(MdTableHeaderRow::cast)
  }

  pub fn rows(&self) -> impl Iterator<Item = MdTableDataRow> {
    children::<MdTableDataRow>(&self.0)
  }
}

/// The Markdown data row in a table
#[derive(Clone, PartialEq, Eq, Hash, AstNode)]
pub struct MdTableDataRow(RedNode);

impl MdTableDataRow {
  pub fn cells(&self) -> impl Iterator<Item = MdTableCell> {
    children::<MdTableCell>(&self.0)
  }
}

/// The Markdown header row in a table
#[derive(Clone, PartialEq, Eq, Hash, AstNode)]
pub struct MdTableHeaderRow(RedNode);

impl MdTableHeaderRow {
  pub fn cells(&self) -> impl Iterator<Item = MdTableCell> {
    children::<MdTableCell>(&self.0)
  }
}

/// The Markdown cell in a table
#[derive(Clone, PartialEq, Eq, Hash, AstNode)]
pub struct MdTableCell(RedNode);

impl MdTableCell {
  pub fn inline_elements(&self) -> impl Iterator<Item = MdInlineElement> {
    children::<MdInlineElement>(&self.0)
  }
}

/// The Markdown bullet list
/// Represented by:
/// - item 1
/// - item 2
#[derive(Clone, PartialEq, Eq, Hash, AstNode)]
pub struct MdBulletList(RedNode);

impl MdBulletList {
  pub fn items(&self) -> impl Iterator<Item = MdBulletListItem> {
    children::<MdBulletListItem>(&self.0)
  }
}

/// The Markdown bullet list item
#[derive(Clone, PartialEq, Eq, Hash, AstNode)]
pub struct MdBulletListItem(RedNode);

impl MdBulletListItem {
  pub fn block_elements(&self) -> impl Iterator<Item = MdBlockElement> {
    children::<MdBlockElement>(&self.0)
  }
}

/// A task list item: `- [ ] ...` or `- [x] ...`
#[derive(Clone, PartialEq, Eq, Hash, AstNode)]
pub struct MdTaskListItem(RedNode);

impl MdTaskListItem {
  pub fn checkbox(&self) -> Option<MdCheckbox> {
    child::<MdCheckbox>(&self.0)
  }

  pub fn block_elements(&self) -> impl Iterator<Item = MdBlockElement> {
    children::<MdBlockElement>(&self.0)
  }
}

/// A checkbox marker inside a task list item: `[ ]` or `[x]`
#[derive(Clone, PartialEq, Eq, Hash, AstNode)]
pub struct MdCheckbox(RedNode);

impl MdCheckbox {
  pub fn is_checked(&self) -> bool {
    self.0.children().any(|child| {
      let text: String = child.chars().collect();
      text.to_lowercase() == "x"
    })
  }
}

/// The Markdown ordered list
/// Represented by:
/// 1. item 1
/// 2. item 2
#[derive(Clone, PartialEq, Eq, Hash, AstNode)]
pub struct MdOrderedList(RedNode);

impl MdOrderedList {
  pub fn items(&self) -> impl Iterator<Item = MdOrderedListItem> {
    children::<MdOrderedListItem>(&self.0)
  }
}

/// The Markdown ordered list item
#[derive(Clone, PartialEq, Eq, Hash, AstNode)]
pub struct MdOrderedListItem(RedNode);

impl MdOrderedListItem {
  pub fn block_elements(&self) -> impl Iterator<Item = MdBlockElement> {
    children::<MdBlockElement>(&self.0)
  }

  /// Returns the numeric index of this list item (e.g. 1, 2, 3).
  pub fn index(&self) -> Option<usize> {
    self.0.children().next().and_then(|child| {
      child
        .as_token()
        .and_then(|token| token.text().and_then(|text| text.parse().ok()))
    })
  }
}

/// The Markdown toggle list
/// >- summary 1
///
///    details 1
#[derive(Clone, PartialEq, Eq, Hash, AstNode)]
pub struct MdToggleList(RedNode);

impl MdToggleList {
  pub fn items(&self) -> impl Iterator<Item = MdToggleListItem> {
    children::<MdToggleListItem>(&self.0)
  }
}

/// The Markdown toggle list item
/// >- summary
///
///    details
#[derive(Clone, PartialEq, Eq, Hash, AstNode)]
pub struct MdToggleListItem(RedNode);

impl MdToggleListItem {
  pub fn summary(&self) -> Option<MdToggleListSummary> {
    child::<MdToggleListSummary>(&self.0)
  }

  pub fn details(&self) -> Option<MdToggleListDetails> {
    child::<MdToggleListDetails>(&self.0)
  }
}

/// The Markdown toggle list item summary
#[derive(Clone, PartialEq, Eq, Hash, AstNode)]
pub struct MdToggleListSummary(RedNode);

impl MdToggleListSummary {
  pub fn inline_elements(&self) -> impl Iterator<Item = MdInlineElement> {
    children::<MdInlineElement>(&self.0)
  }
}

/// The Markdown toggle list item details
#[derive(Clone, PartialEq, Eq, Hash, AstNode)]
pub struct MdToggleListDetails(RedNode);

impl MdToggleListDetails {
  pub fn block_elements(&self) -> impl Iterator<Item = MdBlockElement> {
    children::<MdBlockElement>(&self.0)
  }
}

/// The Markdown container block
/// Represented by:
/// ::: label
///  content
/// :::
#[derive(Clone, PartialEq, Eq, Hash, AstNode)]
pub struct MdContainerBlock(RedNode);

impl MdContainerBlock {
  pub fn label(&self) -> Option<String> {
    self
      .0
      .children()
      .find(|c| c.kind() == SyntaxKind::Ident)?
      .as_token()?
      .text()
      .map(str::to_string)
  }

  // Title text after the label on the opening line
  pub fn title(&self) -> Option<String> {
    let mut found_label = false;
    let mut title_parts = Vec::new();

    for child in self.0.children() {
      // Skip until we find the label identifier
      if !found_label {
        if child.kind() == SyntaxKind::Ident {
          found_label = true;
        }
        continue;
      }
      // Stop at the first newline after the label
      if child.kind() == SyntaxKind::Newline {
        break;
      }
      // Collect text tokens as part of the title
      if let Some(token) = child.as_token()
        && let Some(text) = token.text()
      {
        title_parts.push(text.to_string());
      }
    }

    let title = title_parts.join("").trim().to_string();

    if title.is_empty() { None } else { Some(title) }
  }

  pub fn value(&self) -> impl Iterator<Item = MdNode> {
    self.0.children().filter_map(MdNode::cast)
  }
}

/// The Markdown link
/// Represented by: [alt](link)
#[derive(Clone, PartialEq, Eq, Hash, AstNode)]
pub struct MdLink(RedNode);

impl MdLink {
  pub fn alt(&self) -> Option<MdText> {
    self.0.children().find_map(MdText::cast)
  }

  pub fn url(&self) -> Option<MdText> {
    self.0.children().filter_map(MdText::cast).nth(1)
  }
}

/// The Markdown media
/// Represented by: ![alt](link)
#[derive(Clone, PartialEq, Eq, Hash, AstNode)]
pub struct MdMedia(RedNode);

impl MdMedia {
  pub fn alt(&self) -> Option<MdText> {
    self.0.children().find_map(MdText::cast)
  }

  pub fn url(&self) -> Option<MdText> {
    self.0.children().filter_map(MdText::cast).nth(1)
  }
}

/// The Markdown bold text
/// Represented by: **bold**
#[derive(Clone, PartialEq, Eq, Hash, AstNode)]
pub struct MdBold(RedNode);

impl MdBold {
  pub fn inline_elements(&self) -> impl Iterator<Item = MdInlineElement> {
    children::<MdInlineElement>(&self.0)
  }
}

/// The Markdown italic text
/// Represented by: _italic_ or *italic*
#[derive(Clone, PartialEq, Eq, Hash, AstNode)]
pub struct MdItalic(RedNode);

impl MdItalic {
  pub fn inline_elements(&self) -> impl Iterator<Item = MdInlineElement> {
    children::<MdInlineElement>(&self.0)
  }
}

/// The Markdown bolditalic text
/// Represented by: ***italic***
#[derive(Clone, PartialEq, Eq, Hash, AstNode)]
pub struct MdBoldItalic(RedNode);

impl MdBoldItalic {
  pub fn inline_elements(&self) -> impl Iterator<Item = MdInlineElement> {
    children::<MdInlineElement>(&self.0)
  }
}

/// The Markdown strikethrough text
/// Represented by: ~strikethrough~
#[derive(Clone, PartialEq, Eq, Hash, AstNode)]
pub struct MdStrikethrough(RedNode);

impl MdStrikethrough {
  pub fn inline_elements(&self) -> impl Iterator<Item = MdInlineElement> {
    children::<MdInlineElement>(&self.0)
  }
}

/// The Markdown plaintext
/// Represented by: text
#[derive(Clone, PartialEq, Eq, Hash, AstNode)]
pub struct MdText(RedNode);

impl MdText {
  pub fn value(&self) -> String {
    self.0.text()
  }
}

/// An HTML entity in markdown text, e.g. &amp; &#42; &#x2A;
#[derive(Clone, PartialEq, Eq, Hash, AstNode)]
pub struct MdHtmlEntity(RedNode);

impl MdHtmlEntity {
  pub fn decode(&self) -> Option<String> {
    let raw = self.0.as_token()?.text()?.to_string();
    Some(unescape_html_entity(&raw))
  }
}

// Expression nodes
#[wrapper_ast_node(SyntaxKind = [
  NumberLit, StrLit, CodeLit, MathLit, IdentLit,
  ListLit, DictLit,
  ParenExpr, CallExpr, UnaryExpr, BinaryExpr, IndexExpr,
  YamlMapping, YamlSequence,
])]
pub struct Expr(RedNode);

pub enum LitKind {
  NumberLit(NumberLit),
  StrLit(StrLit),
  CodeLit(CodeLit),
  MathLit(MathLit),
  IdentLit(IdentLit),
  ListLit(ListLit),
  DictLit(DictLit),
}

pub enum LitValue {
  Number(f64),
  Str(Vec<Either<String, InterpFragment>>),
  Code(String),
  Math(String),
  Ident(String),
  List(Vec<ListItem>),
  Dict(Vec<DictEntry>),
}

#[wrapper_ast_node(SyntaxKind = [
  NumberLit, StrLit, CodeLit, MathLit, IdentLit,
  ListLit, DictLit,
])]
pub struct Lit(RedNode);

impl Lit {
  pub fn kind(&self) -> Option<LitKind> {
    match self.0.kind() {
      SyntaxKind::NumberLit => Some(LitKind::NumberLit(NumberLit::cast(self.0.clone())?)),
      SyntaxKind::StrLit => Some(LitKind::StrLit(StrLit::cast(self.0.clone())?)),
      SyntaxKind::CodeLit => Some(LitKind::CodeLit(CodeLit::cast(self.0.clone())?)),
      SyntaxKind::MathLit => Some(LitKind::MathLit(MathLit::cast(self.0.clone())?)),
      SyntaxKind::IdentLit => Some(LitKind::IdentLit(IdentLit::cast(self.0.clone())?)),
      SyntaxKind::ListLit => Some(LitKind::ListLit(ListLit::cast(self.0.clone())?)),
      SyntaxKind::DictLit => Some(LitKind::DictLit(DictLit::cast(self.0.clone())?)),
      _ => None,
    }
  }

  pub fn value(&self) -> Option<LitValue> {
    match self.0.kind() {
      SyntaxKind::NumberLit => Some(LitValue::Number(NumberLit::cast(self.0.clone())?.value()?)),
      SyntaxKind::StrLit => {
        let fragments = StrLit::cast(self.0.clone())?.fragments().collect();
        Some(LitValue::Str(fragments))
      }
      SyntaxKind::CodeLit => Some(LitValue::Code(CodeLit::cast(self.0.clone())?.value()?)),
      SyntaxKind::MathLit => Some(LitValue::Math(MathLit::cast(self.0.clone())?.value()?)),
      SyntaxKind::IdentLit => Some(LitValue::Ident(IdentLit::cast(self.0.clone())?.value()?)),
      SyntaxKind::ListLit => Some(LitValue::List(
        ListLit::cast(self.0.clone())?.items().collect(),
      )),
      SyntaxKind::DictLit => Some(LitValue::Dict(
        DictLit::cast(self.0.clone())?.entries().collect(),
      )),
      _ => None,
    }
  }
}

#[derive(Clone, PartialEq, Eq, Hash, AstNode)]
pub struct ParenExpr(RedNode);

impl ParenExpr {
  pub fn expr(&self) -> Option<Expr> {
    self.0.children().find_map(Expr::cast)
  }
}

#[derive(Clone, PartialEq, Eq, Hash, AstNode)]
pub struct YamlOp(RedNode);

pub enum YamlOpKind {
  Plus,
  Minus,
  Tilde,
  Not,
  And,
  Or,
  Eq,
  NotEq,
  Lt,
  Gt,
  LtEq,
  GtEq,
  Mul,
  Div,
  Mod,
  Pow,
  Dot,
  Tag(String),
}

impl YamlOp {
  pub fn kind(&self) -> Option<YamlOpKind> {
    let text = self.0.as_token()?.text()?.to_string();
    let kind = match text.as_str() {
      "+" => YamlOpKind::Plus,
      "-" => YamlOpKind::Minus,
      "~" => YamlOpKind::Tilde,
      "!" => YamlOpKind::Not,
      "&&" => YamlOpKind::And,
      "||" => YamlOpKind::Or,
      "==" => YamlOpKind::Eq,
      "!=" => YamlOpKind::NotEq,
      "<" => YamlOpKind::Lt,
      ">" => YamlOpKind::Gt,
      "<=" => YamlOpKind::LtEq,
      ">=" => YamlOpKind::GtEq,
      "*" => YamlOpKind::Mul,
      "/" => YamlOpKind::Div,
      "%" => YamlOpKind::Mod,
      "**" => YamlOpKind::Pow,
      "." => YamlOpKind::Dot,
      op if op.starts_with('!') => YamlOpKind::Tag(op[1..].to_string()),
      _ => None?,
    };
    Some(kind)
  }
}

#[derive(Clone, PartialEq, Eq, Hash, AstNode)]
pub struct CallExpr(RedNode);

impl CallExpr {
  /// Return the callee expression
  pub fn callee(&self) -> Option<Expr> {
    children::<Expr>(&self.0).next()
  }

  /// Return all argument expressions
  pub fn args(&self) -> Vec<Expr> {
    children::<Expr>(&self.0).skip(1).collect()
  }

  /// Return the nth argument (0-indexed)
  pub fn arg(&self, n: usize) -> Option<Expr> {
    children::<Expr>(&self.0).skip(1).nth(n)
  }
}

#[derive(Clone, PartialEq, Eq, Hash, AstNode)]
pub struct UnaryExpr(RedNode);

impl UnaryExpr {
  /// Return the operand expression
  pub fn expr(&self) -> Option<Expr> {
    child::<Expr>(&self.0)
  }

  /// Return the operator
  pub fn op(&self) -> Option<YamlOp> {
    self.0.children().find_map(YamlOp::cast)
  }
}

#[derive(Clone, PartialEq, Eq, Hash, AstNode)]
pub struct BinaryExpr(RedNode);

impl BinaryExpr {
  /// Return the left operand expression
  pub fn left(&self) -> Option<Expr> {
    children::<Expr>(&self.0).next()
  }

  /// Return the operator
  pub fn op(&self) -> Option<YamlOp> {
    self.0.children().find_map(YamlOp::cast)
  }

  /// Return the right operand expression
  pub fn right(&self) -> Option<Expr> {
    children::<Expr>(&self.0).nth(1)
  }
}

#[derive(Clone, PartialEq, Eq, Hash, AstNode)]
pub struct IndexExpr(RedNode);

impl IndexExpr {
  /// Return the indexed expression
  pub fn expr(&self) -> Option<Expr> {
    children::<Expr>(&self.0).next()
  }

  /// Return all index argument expressions
  pub fn indices(&self) -> Vec<Expr> {
    children::<Expr>(&self.0).skip(1).collect()
  }
}

// Literals
#[derive(Clone, PartialEq, Eq, Hash, AstNode)]
pub struct ListLit(RedNode);

impl ListLit {
  pub fn items(&self) -> impl Iterator<Item = ListItem> {
    children::<ListItem>(&self.0)
  }
}

#[derive(Clone, PartialEq, Eq, Hash, AstNode)]
pub struct ListItem(RedNode);

impl ListItem {
  pub fn value(&self) -> Option<Expr> {
    child::<Expr>(&self.0)
  }
}

#[derive(Clone, PartialEq, Eq, Hash, AstNode)]
pub struct DictLit(RedNode);

impl DictLit {
  pub fn entries(&self) -> impl Iterator<Item = DictEntry> {
    children::<DictEntry>(&self.0)
  }

  pub fn keys(&self) -> impl Iterator<Item = String> {
    children::<DictEntry>(&self.0).filter_map(|e| e.key())
  }

  pub fn values(&self) -> impl Iterator<Item = Expr> {
    children::<DictEntry>(&self.0).filter_map(|e| e.value())
  }
}

#[derive(Clone, PartialEq, Eq, Hash, AstNode)]
pub struct DictEntry(RedNode);

impl DictEntry {
  pub fn key(&self) -> Option<String> {
    self
      .0
      .children()
      .find(|c| c.kind() == SyntaxKind::DictEntryKey)
      .map(|n| n.text())
  }

  pub fn value(&self) -> Option<Expr> {
    let red_node = self
      .0
      .children()
      .find(|c| c.kind() == SyntaxKind::DictEntryValue)?;
    child::<Expr>(&red_node)
  }

  pub fn entry(&self) -> Option<(String, Expr)> {
    Some((self.key()?, self.value()?))
  }
}

#[derive(Clone, PartialEq, Eq, Hash, AstNode)]
pub struct StrLit(RedNode);

impl StrLit {
  pub fn is_interpolated(&self) -> bool {
    self
      .0
      .children()
      .any(|c| c.kind() == SyntaxKind::InterpFragment)
  }

  pub fn fragments(&self) -> impl Iterator<Item = Either<String, InterpFragment>> {
    self.0.children().filter_map(|child| match child.kind() {
      SyntaxKind::DqStrContent | SyntaxKind::SqStrContent => {
        let raw = child.as_token()?.text()?.to_string();
        Some(Either::Left(unescape(&raw).unwrap_or(raw)))
      }
      SyntaxKind::YamlLiteralBlockStrLit => Some(Either::Left(extract_literal_block(&child))),
      SyntaxKind::YamlFoldedBlockStrLit => Some(Either::Left(extract_folded_block(&child))),
      SyntaxKind::InterpFragment => Some(Either::Right(InterpFragment(child))),
      _ => None,
    })
  }
}

#[derive(Clone, PartialEq, Eq, Hash, AstNode)]
pub struct InterpFragment(RedNode);

impl InterpFragment {
  pub fn expr(&self) -> Option<Expr> {
    child::<Expr>(&self.0)
  }
}

#[derive(Clone, PartialEq, Eq, Hash, AstNode)]
pub struct MathLit(RedNode);

impl MathLit {
  pub fn value(&self) -> Option<String> {
    child::<InlineMath>(&self.0)
      .and_then(|n| n.value())
      .or_else(|| child::<MathBlock>(&self.0).and_then(|n| n.value()))
  }
}

#[derive(Clone, PartialEq, Eq, Hash, AstNode)]
pub struct CodeLit(RedNode);

impl CodeLit {
  pub fn label(&self) -> Option<String> {
    child::<CodeBlock>(&self.0).and_then(|n| n.label())
  }

  pub fn value(&self) -> Option<String> {
    child::<InlineCode>(&self.0)
      .and_then(|n| n.value())
      .or_else(|| child::<CodeBlock>(&self.0).and_then(|n| n.value()))
  }
}

#[derive(Clone, PartialEq, Eq, Hash, AstNode)]
pub struct NumberLit(RedNode);

impl NumberLit {
  pub fn value(&self) -> Option<f64> {
    self
      .0
      .children()
      .find(|c| c.kind() == SyntaxKind::Number)?
      .as_token()?
      .text()?
      .parse()
      .ok()
  }
}

#[derive(Clone, PartialEq, Eq, Hash, AstNode)]
pub struct IdentLit(RedNode);

impl IdentLit {
  pub fn value(&self) -> Option<String> {
    self
      .0
      .children()
      .find(|c| c.kind() == SyntaxKind::Ident)?
      .as_token()?
      .text()
      .map(str::to_string)
  }
}

#[derive(Clone, PartialEq, Eq, Hash, AstNode)]
pub struct InlineMath(RedNode);

impl InlineMath {
  pub fn value(&self) -> Option<String> {
    let text = self.0.as_token()?.text()?.to_string();
    let fence_count = text.chars().take_while(|c| *c == '$').count();
    // Strip opening and closing fence
    let content = text.get(fence_count..text.len().checked_sub(fence_count)?)?;
    Some(content.to_string())
  }
}

#[derive(Clone, PartialEq, Eq, Hash, AstNode)]
pub struct MathBlock(RedNode);

impl MathBlock {
  pub fn value(&self) -> Option<String> {
    let text = self.0.as_token()?.text()?.to_string();
    let fence_count = text.chars().take_while(|c| *c == '$').count();
    // Skip opening fence then the newline
    let after_fence = text.get(fence_count..)?;
    let content_start = after_fence.find('\n')? + 1;
    let content = after_fence.get(content_start..)?;
    // Strip closing fence
    let content = content.get(..content.len().checked_sub(fence_count)?)?;
    Some(content.to_string())
  }
}

#[derive(Clone, PartialEq, Eq, Hash, AstNode)]
pub struct InlineCode(RedNode);

impl InlineCode {
  pub fn value(&self) -> Option<String> {
    let text = self.0.as_token()?.text()?.to_string();
    let fence_count = text.chars().take_while(|c| *c == '`').count();
    // Strip opening and closing fence
    let content = text.get(fence_count..text.len().checked_sub(fence_count)?)?;
    Some(content.to_string())
  }
}

#[derive(Clone, PartialEq, Eq, Hash, AstNode)]
pub struct CodeBlock(RedNode);

pub struct CodeLineRange {
  pub start: usize,
  pub end: usize,
}

impl CodeBlock {
  // Full label text after the opening fence (e.g. "js{1,3,5-8}")
  pub fn label(&self) -> Option<String> {
    let text = self.0.as_token()?.text()?.to_string();
    let fence_count = text.chars().take_while(|c| *c == '`').count();
    let after_fence = text.get(fence_count..)?;
    let label_end = after_fence.find('\n')?;
    let label = after_fence.get(..label_end)?.trim();
    if label.is_empty() {
      None
    } else {
      Some(label.to_string())
    }
  }

  // Language identifier without the range indicator (e.g. "js" from "js{1,3}")
  pub fn language(&self) -> Option<String> {
    let label = self.label()?;
    let lang = match label.find('{') {
      Some(idx) => label[..idx].trim(),
      None => label.trim(),
    };
    if lang.is_empty() {
      None
    } else {
      Some(lang.to_string())
    }
  }

  // Parsed line ranges from the range indicator (e.g. {1,3,5-8} -> [(1,1), (3,3), (5,8)])
  pub fn line_ranges(&self) -> Option<Vec<CodeLineRange>> {
    let label = self.label()?;
    let start = label.find('{')?;
    let end = label.find('}')?;
    let inner = label.get(start + 1..end)?.trim();
    if inner.is_empty() {
      return None;
    }

    let mut ranges = Vec::new();
    for part in inner.split(',') {
      let part = part.trim();
      if let Some(dash) = part.find('-') {
        let start_line = part[..dash].trim().parse::<usize>().ok()?;
        let end_line = part[dash + 1..].trim().parse::<usize>().ok()?;
        ranges.push(CodeLineRange {
          start: start_line,
          end: end_line,
        });
      } else {
        let line = part.parse::<usize>().ok()?;
        ranges.push(CodeLineRange {
          start: line,
          end: line,
        });
      }
    }

    Some(ranges)
  }

  pub fn value(&self) -> Option<String> {
    let text = self.0.as_token()?.text()?.to_string();
    let fence_count = text.chars().take_while(|c| *c == '`').count();
    // Skip opening fence and optional language tag, then the newline
    let after_fence = text.get(fence_count..)?;
    let content_start = after_fence.find('\n')? + 1;
    let content = after_fence.get(content_start..)?;
    // Strip closing fence
    let content = content.get(..content.len().checked_sub(fence_count)?)?;
    Some(content.to_string())
  }
}

// Extract the content from a block scalar node by processing its raw text.
// The raw text is the full node text, e.g. ` |\n  line one\n  line two\n`.
// Content starts after the first newline. Indent size is the number of leading
// spaces on the first content line.
fn block_scalar_content(node: &RedNode) -> (Vec<String>, usize) {
  let raw = node.text();

  // Skip past the `|`/`>` marker and the newline that follows it.
  let content_start = match raw.find('\n') {
    Some(pos) => pos + 1,
    None => return (vec![], 0),
  };
  let content = &raw[content_start..];

  // Determine indent from the leading spaces of the first non-empty line.
  let indent_size = content
    .lines()
    .find(|l| !l.trim().is_empty())
    .map_or(0, |l| l.len() - l.trim_start_matches(' ').len());

  let lines = content.lines().map(|l| l.to_string()).collect();
  (lines, indent_size)
}

fn strip_indent(line: &str, indent_size: usize) -> &str {
  if line.len() >= indent_size && line.starts_with(' ') {
    &line[indent_size..]
  } else {
    line.trim_start_matches(' ')
  }
}

// Reconstruct a literal block string (`|`): preserve newlines, strip indentation.
fn extract_literal_block(node: &RedNode) -> String {
  let (lines, indent_size) = block_scalar_content(node);
  let mut result = String::new();

  for line in &lines {
    result.push_str(strip_indent(line, indent_size));
    result.push('\n');
  }

  // Clip chomping: keep exactly one trailing newline.
  while result.ends_with("\n\n") {
    result.pop();
  }

  result
}

// Reconstruct a folded block string (`>`): fold newlines into spaces,
// blank lines become paragraph breaks.
fn extract_folded_block(node: &RedNode) -> String {
  let (lines, indent_size) = block_scalar_content(node);

  // Drop trailing blank lines.
  let last_non_blank = lines
    .iter()
    .rposition(|l| !strip_indent(l, indent_size).is_empty());
  let lines = match last_non_blank {
    Some(pos) => &lines[..=pos],
    None => return "\n".to_string(),
  };

  let mut result = String::new();
  let mut pending_space = false;

  for line in lines {
    let stripped = strip_indent(line, indent_size);
    if stripped.is_empty() {
      result.push('\n');
      pending_space = false;
    } else {
      if pending_space {
        result.push(' ');
      }
      result.push_str(stripped);
      pending_space = true;
    }
  }

  result.push('\n');
  result
}
