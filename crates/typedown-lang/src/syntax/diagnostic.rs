use typedown_macros::{Decodable, Encodable, StableCompare, StableHash, typedown_diagnostic_kind};

use crate::syntax::syntax_kind::SyntaxKind;

#[typedown_diagnostic_kind]
pub enum TdDiagnosticKind {
  /* Lexer */
  #[message("unexpected end of input, expected '{expected}'")]
  UnexpectedEof { expected: char },
  #[message("expected '{expected}', found '{encountered}'")]
  UnexpectedChar { expected: char, encountered: char },
  #[message("unterminated string literal")]
  UnterminatedString,
  #[message("unterminated interpolation '${{'")] // escaped brace
  UnterminatedInterpolation,
  #[message("unterminated code block '```'")]
  UnterminatedCodeBlock,
  #[message("unterminated inline code '`'")]
  UnterminatedInlineCode,
  #[message("unterminated math block '$$'")]
  UnterminatedMathBlock,
  #[message("unterminated inline math '$'")]
  UnterminatedInlineMath,
  #[message("missing newline after code block fence")]
  MissingCodeBlockNewline,
  #[message("missing newline after math block delimiter")]
  MissingMathBlockNewline,
  #[message("invalid character '{encountered}' in this context")]
  InvalidChar { encountered: char },
  #[message("invalid UTF-8 byte sequence")]
  InvalidUtf8,
  #[message("mixed tabs and spaces in indentation")]
  MixedIndentation,
  #[message("inconsistent indentation: expected '{expected}', found '{encountered}'")]
  InconsistentIndentation { expected: char, encountered: char },
  #[message("indentation does not match any outer block")]
  UnmatchedDedent { indent: usize },
  #[message("expected digits after 'e' in number")]
  MissingExponentDigits,

  /* Parser */
  #[message("frontmatter '---' line should not contain other text")]
  UnexpectedTokensOnFrontmatterMarkerLine,
  #[message("unexpected token in container properties")]
  UnexpectedContainerPropItem,
  #[message("unexpected token in container properties")]
  UnexpectedContainerPropValue,
  #[message("expected a value after '='")]
  MissingContainerPropValueAfterEq,
  #[message("unclosed '{{' in container properties")] // escaped brace
  UnclosedContainerPropBlock,
  #[message("slot separator '---' line should not contain other text")]
  UnexpectedContainerSlotSeparatorToken,
  #[message("missing frontmatter marker '---'")]
  MissingFrontmatterMarker,
  #[message("missing '#' for markdown heading")]
  MissingMarkdownHeadingHash,
  #[message("missing required space after block marker")]
  MissingRequiredSpace,
  #[message("expected {}", syntax_kind_label(*expected))]
  MissingSyntaxNode { expected: SyntaxKind },
  #[message("unclosed link '[', expected '](url)'")]
  UnclosedLink,
  #[message("unclosed bold span")]
  UnclosedBold,
  #[message("unclosed italic span")]
  UnclosedItalic,
  #[message("unclosed strikethrough span")]
  UnclosedStrikethrough,
  #[message("unclosed bold-italic span")]
  UnclosedBoldItalic,
  #[message("italic closing delimiter does not match opening delimiter")]
  MismatchedItalicDelimiter,
  #[message("expected '{expected_prefix}' at start of line")]
  MissingExpectMdPrefix { expected_prefix: String },
  #[message("missing separator row after table header")]
  MissingTableSeparatorRow,
  #[message("table row has {found} columns, expected {expected}")]
  TableColumnCountMismatch { expected: usize, found: usize },
  #[message("block content must be indented further than its parent")]
  InsufficientBlockIndent {
    expected_more_than: usize,
    found: usize,
  },

  /* Vault config */
  #[message("no typedown.yaml or typedown.yml found in '{root_dir}'")]
  MissingVaultConfig { root_dir: String },
  #[message("failed to read vault config '{path}': {message}")]
  VaultConfigReadError { path: String, message: String },
  #[message("failed to parse vault config '{path}': {message}")]
  VaultConfigParseError { path: String, message: String },
  #[message("vault config '{path}' is empty")]
  VaultConfigEmpty { path: String },
  #[message("vault config '{path}' is missing required field '{field}'")]
  VaultConfigMissingField { path: String, field: String },
  #[message("vault config '{path}' has unknown field '{field}'")]
  VaultConfigUnknownField { path: String, field: String },

  /* Typechecker */
  #[message("missing '_type' field in top-level mapping")]
  MissingSchemaField,
  #[message("cannot resolve type '{name}'")]
  UnresolvedSchema { name: String },
  #[message("wrong number of type arguments: expected {expected}, got {got}")]
  WrongTypArgCount { expected: usize, got: usize },
  #[message("type argument at index {index} violates bound '{expected_bound}': got '{got}'")]
  TypArgBoundViolation {
    index: usize,
    expected_bound: String,
    got: String,
  },
  #[message("cannot resolve extended schema '{name}'")]
  UnresolvedExtends { name: String },
  #[message("circular extension chain involving '{}': {}", name, cycle.join(" -> "))]
  CircularExtension { name: String, cycle: Vec<String> },
  #[message(
    "field '{field}' redefined with type '{got}' which is not a subtype of '{expected}' from parent schema '{parent_schema}'"
  )]
  FieldRefinementViolation {
    field: String,
    parent_schema: String,
    expected: String,
    got: String,
  },
  #[message("this value is not a function")]
  NotCallable,
  #[message("wrong number of arguments: expected {expected}, got {got}")]
  WrongArgCount { expected: usize, got: usize },
  #[message("argument type mismatch: expected {expected}")]
  ArgTypMismatch { expected: String },
  #[message("field '{field}' type mismatch: expected {expected}")]
  FieldTypMismatch { field: String, expected: String },
  #[message("this value does not support indexing")]
  NotIndexable,
  #[message("index type mismatch: expected {expected}")]
  IndexTypMismatch { expected: String },
  #[message("tag type mismatch: expected {expected}")]
  TagTypMismatch { expected: String },
  #[message("operand type mismatch for '{op}': expected {expected}")]
  OperandTypMismatch { op: String, expected: String },
  #[message("missing required field '{field}'")]
  MissingRequiredField { field: String },
  #[message("element type mismatch: expected {expected}")]
  ElementTypMismatch { expected: String },
  #[message("duplicate key '{key}'")]
  DuplicateKey { key: String },
  #[message("cannot resolve file reference '{path}'")]
  UnresolvedFileRef { path: String },
  #[message(
    "invalid fref path '{path}': must start with '/' (absolute) or './' / '../' (relative)"
  )]
  InvalidFrefPath { path: String },
  #[message("cannot resolve identifier '{name}'")]
  UnresolvedIdentifier { name: String },
  #[message("cannot resolve import '{path}'")]
  UnresolvedImport { path: String },
  #[message("unknown field '{field}' on type '{on_typ}'")]
  UnknownField { field: String, on_typ: String },
  #[message("index {index} is out of bounds for length {length}")]
  IndexOutOfBounds { index: usize, length: usize },
  #[message("duplicate schema name '{name}' in '{path}', already defined in '{duplicate_of}'")]
  DuplicateSchemaName {
    name: String,
    path: String,
    duplicate_of: String,
  },
  #[message("vault config '{path}' field '{field}': {message}")]
  VaultConfigInvalidValue {
    path: String,
    field: String,
    message: String,
  },
  #[message("invalid code block line range indicator")]
  InvalidCodeRangeIndicator,
  #[message("parameter list must be followed by `->`")]
  DanglingParamList,
  #[message("closure parameters must be identifiers")]
  InvalidClosureParams,
  #[message("self cannot be used as a closure parameter name")]
  InvalidSelfClosureParams,
  #[message("unclosed parameter list")]
  UnclosedParamList,
  #[message("unknown component \"{name}\"")]
  UnknownComponent { name: String },
  #[message("missing required prop \"{prop}\" on component \"{component}\"")]
  MissingRequiredProp { prop: String, component: String },
  #[message("unknown prop \"{prop}\" on component \"{component}\"")]
  UnknownProp { prop: String, component: String },
  #[message("unknown slot \"{slot}\" on component \"{component}\"")]
  UnknownSlot { slot: String, component: String },
  #[message("component \"{component}\" is deprecated: {message}")]
  DeprecatedComponent { component: String, message: String },
  #[message("type '{type_name}' is not constructible at runtime")]
  NotConstructible { type_name: String },

  /* Lint */
  #[message("image is missing alt text")]
  MissingAltText,
  #[message("avoid generic link text \"{text}\"; use a descriptive phrase")]
  GenericLinkText { text: String },
  #[message("multiple H1 headings; a document should have only one")]
  MultipleH1,
  #[message("duplicate heading \"{text}\"")]
  DuplicateHeading { text: String },
  #[message("trailing whitespace")]
  TrailingWhitespace,
  #[message("missing space after # in heading")]
  HeadingMissingSpace,
  #[message("extra spaces after # in heading; use exactly one")]
  HeadingExtraSpaces,
  #[message("missing blank line before block")]
  BlockBlankLine,
  #[message("multiple consecutive blank lines")]
  ConsecutiveBlankLines,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Encodable, Decodable, StableHash, StableCompare)]
pub enum Severity {
  Error,
  Warning,
}

#[derive(Debug, Clone, PartialEq, Eq, Encodable, Decodable, StableHash, StableCompare)]
pub struct TdDiagnostic {
  pub kind: TdDiagnosticKind,
  pub severity: Severity,
  pub start_offset: usize,
  pub end_offset: usize,
}

impl TdDiagnostic {
  pub fn code_str(&self) -> &'static str {
    self.kind.code_str()
  }

  pub fn offsets(&self) -> (usize, usize) {
    (self.start_offset, self.end_offset)
  }

  pub fn message(&self) -> String {
    self.kind.message()
  }
}

fn syntax_kind_label(kind: SyntaxKind) -> &'static str {
  match kind {
    SyntaxKind::PrimaryExpr => "an expression",
    SyntaxKind::Ident => "an identifier",
    SyntaxKind::Newline => "a line break",
    SyntaxKind::Colon => "':'",
    SyntaxKind::LParen => "'('",
    SyntaxKind::RParen => "')'",
    SyntaxKind::LBracket => "'['",
    SyntaxKind::RBracket => "']'",
    SyntaxKind::LBrace => "'{'",
    SyntaxKind::RBrace => "'}'",
    SyntaxKind::InterpEnd => "'}'",
    SyntaxKind::YamlIndent => "indentation",
    SyntaxKind::YamlMappingEntryKey => "a field name",
    SyntaxKind::YamlMappingEntryValue => "a value",
    SyntaxKind::DictEntryKey => "a key",
    SyntaxKind::DictEntryValue => "a value",
    SyntaxKind::MdOrderedListItem => "'.' after list number",
    SyntaxKind::MdBulletListItem => "a list item",
    SyntaxKind::MdHeading => "heading text",
    SyntaxKind::MdTableCell => "a table cell",
    SyntaxKind::MdParagraph => "text content",
    SyntaxKind::MdText => "text",
    SyntaxKind::MdLink => "a link",
    SyntaxKind::MdMedia => "an image or media",
    SyntaxKind::MdContainerBlock => "a block",
    _ => "a token",
  }
}
