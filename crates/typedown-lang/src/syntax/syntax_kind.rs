use strum::{EnumIter, FromRepr};

#[derive(Debug, PartialEq, Eq, Clone, Copy, Hash, EnumIter, FromRepr)]
#[repr(u16)]
pub enum SyntaxKind {
  // Top-level
  SourceFile = 0, // root: frontmatter + body

  // Frontmatter (YAML mode) nodes
  YamlFrontmatter = 100,
  YamlMapping,           // block mapping (indentation-based)
  YamlMappingEntry,      // one key: value pair in block mapping
  YamlMappingEntryKey,   // key in a block mapping entry
  YamlMappingEntryValue, // value in a block mapping entry
  YamlSequence,
  YamlSequenceItem, // one - item

  // Body (Markdown mode) nodes
  MdBody = 200,
  MdHeading,
  MdParagraph,
  MdBlockquote,
  MdTable,
  MdTableHeaderRow,
  MdTableDataRow,
  MdTableSeparatorRow,
  MdTableCell,
  MdBulletList,
  MdBulletListItem,
  MdTaskListItem, // `- [ ] ...` or `- [x] ...`
  MdCheckbox,     // `[ ]` or `[x]`
  MdOrderedList,
  MdOrderedListItem,
  MdToggleList,
  MdToggleListItem,
  MdToggleListSummary,
  MdToggleListDetails,
  MdContainerBlock,     // ::: label ... :::
  MdContainerPropBlock, // {key=value}
  MdContainerPropItem,  // key=value or key
  MdContainerSlot,
  MdContainerSlotSeparator, // ===
  MdLink,           // [text](url)
  MdMedia,          // ![alt](src)
  MdBold,           // **text**
  MdItalic,         // *text* or _text_
  MdBoldItalic,     // ***text***
  MdStrikethrough,  // ~~text~~
  MdText,           // plain text run

  // Expression nodes
  PrimaryExpr = 300, // An operand in an expression
  ParenExpr,         // (expr)
  CallExpr,          // func(args)
  IndexExpr,         // expr[expr, ...]
  UnaryExpr,
  BinaryExpr,

  // Literals
  // All literals must be wrapped in a primary expr to be treated as an expression
  ListLit,                // Flow sequence in yaml frontmatter & formula mode
  ListItem,               // An item inside a ListLit
  DictLit,                // Flow mapping `{key: value, ...}` in yaml frontmatter & formula mode
  DictEntry,              // one key: value pair in a dict
  DictEntryKey,           // key in a dict entry
  DictEntryValue,         // value in a dict entry
  YamlLiteralBlockStrLit, // | block scalar (preserves newlines)
  YamlFoldedBlockStrLit,  // > block scalar (folds newlines to spaces)
  StrLit,                 // String literal + interpolation + math
  InterpFragment,         // Interpolation fragment: ${...}
  MathLit,                // Inline + block math expression
  CodeLit,                // Inline + block code expression
  NumberLit,
  IdentLit,

  // Shared tokens
  Ident = 400,
  Number,
  DqStrStart,   // opening "
  DqStrContent, // text between " and " or ${
  DqStrEnd,     // closing "
  SqStrStart,   // opening '
  SqStrContent, // text between ' and ' or ${
  SqStrEnd,     // closing '
  InterpStart,  // ${
  InterpEnd,    // } closing an interpolation
  Colon,        // :
  Comma,        // ,
  LParen,       // (
  RParen,       // )
  LBracket,     // [
  RBracket,     // ]
  LBrace,       // {
  RBrace,       // }

  InlineMath, // matched $ delimiters, content on same line
  MathBlock,  // matched $ delimiters, content between newlines
  InlineCode, // matched ` delimiters, content on same line
  CodeBlock,  // matched ` delimiters, optional language tag, content between newlines

  // YAML mode tokens
  YamlOp,      // operators: +, -, ., ->, ==, !string, etc.
  YamlComment, // # ...
  YamlIndent,

  // Markdown mode tokens
  MdSymbol,     // a single special char (#, *, ~, -, :, ., etc.)
  MdNumber,     // integer only (for ordered list markers like `1.`, `23.`)
  MdHtmlEntity, // &name; or &#digits; or &#xhex; (e.g. &amp;, &#42;, &#x2A;)

  // Trivia
  Whitespace = 600,
  Newline,
  Eof,

  // Error
  Error,
}

impl SyntaxKind {
  /// Whether this token is trivia (whitespace, newline, indent, comment)
  pub fn is_trivia(self) -> bool {
    matches!(
      self,
      SyntaxKind::Whitespace
        | SyntaxKind::Newline
        | SyntaxKind::YamlIndent
        | SyntaxKind::YamlComment
    )
  }
}
