use strum::FromRepr;

use crate::syntax::syntax_kind::SyntaxKind;

#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, FromRepr)]
pub enum DiagnosticCode {
  UnexpectedEof = 0,
  UnexpectedChar = 1,
  UnterminatedString = 2,
  UnterminatedInterpolation = 3,
  UnterminatedCodeBlock = 4,
  UnterminatedInlineCode = 5,
  UnterminatedMathBlock = 6,
  UnterminatedInlineMath = 7,
  MissingCodeBlockNewline = 8,
  MissingMathBlockNewline = 9,
  InvalidChar = 10,
  InvalidUtf8 = 11,
  MixedIndentation = 12,
  InconsistentIndentation = 13,
  UnmatchedDedent = 14,
  MissingExponentDigits = 15,
  UnexpectedTokensOnFrontmatterMarkerLine = 16,
  MissingFrontmatterMarker = 17,
  MissingMarkdownHeadingHash = 18,
  MissingRequiredSpacesBetweenHashAndHeading = 19,
  MissingSyntaxNode = 20,
  UnclosedLink = 21,
  UnclosedBold = 22,
  UnclosedItalic = 23,
  UnclosedStrikethrough = 24,
  UnclosedBoldItalic = 25,
  MismatchedItalicDelimiter = 26,
  MissingExpectMdPrefix = 27,
  MissingTableSeparatorRow = 28,
  TableColumnCountMismatch = 29,
  InsufficientBlockIndent = 30,
  MissingVaultConfig = 31,
  VaultConfigReadError = 32,
  VaultConfigParseError = 33,
  VaultConfigEmpty = 34,
  VaultConfigMissingField = 35,
  VaultConfigUnknownField = 36,
  MissingSchemaField = 37,
  UnresolvedSchema = 38,
  WrongTypeArgCount = 39,
  NotCallable = 40,
  WrongArgCount = 41,
  ArgTypeMismatch = 42,
  FieldTypeMismatch = 43,
  NotIndexable = 44,
  IndexTypeMismatch = 45,
  TagTypeMismatch = 46,
  OperandTypeMismatch = 47,
  MissingRequiredField = 48,
  ElementTypeMismatch = 49,
  DuplicateKey = 50,
  UnresolvedFileRef = 51,
  UnknownField = 52,
  IndexOutOfBounds = 53,
  NestedSchemaFile = 54,
  VaultConfigInvalidValue = 55,
  InvalidCodeRangeIndicator = 56, // {abc} or unclosed { in code block
  UnexpectedContainerPropItem = 57,
  UnexpectedContainerPropValue = 58,
  MissingContainerPropValueAfterEq = 59,
  UnclosedContainerPropBlock = 60,
  UnexpectedContainerSlotSeparatorToken = 61,
}

impl DiagnosticCode {
  pub fn as_str(&self) -> &'static str {
    match self {
      DiagnosticCode::UnexpectedEof => "unexpected-eof",
      DiagnosticCode::UnexpectedChar => "unexpected-char",
      DiagnosticCode::UnterminatedString => "unterminated-string",
      DiagnosticCode::UnterminatedInterpolation => "unterminated-interpolation",
      DiagnosticCode::UnterminatedCodeBlock => "unterminated-code-block",
      DiagnosticCode::UnterminatedInlineCode => "unterminated-inline-code",
      DiagnosticCode::UnterminatedMathBlock => "unterminated-math-block",
      DiagnosticCode::UnterminatedInlineMath => "unterminated-inline-math",
      DiagnosticCode::MissingCodeBlockNewline => "missing-code-block-newline",
      DiagnosticCode::MissingMathBlockNewline => "missing-math-block-newline",
      DiagnosticCode::InvalidChar => "invalid-char",
      DiagnosticCode::InvalidUtf8 => "invalid-utf8",
      DiagnosticCode::MixedIndentation => "mixed-indentation",
      DiagnosticCode::InconsistentIndentation => "inconsistent-indentation",
      DiagnosticCode::UnmatchedDedent => "unmatched-dedent",
      DiagnosticCode::MissingExponentDigits => "missing-exponent-digits",
      DiagnosticCode::UnexpectedTokensOnFrontmatterMarkerLine => {
        "unexpected-tokens-on-frontmatter-marker"
      }
      DiagnosticCode::UnexpectedContainerPropItem => "unexpected-container-prop-item",
      DiagnosticCode::UnexpectedContainerPropValue => "unexpected-container-prop-value",
      DiagnosticCode::UnexpectedContainerSlotSeparatorToken => {
        "unexpected-container-slot-separator-token"
      }
      DiagnosticCode::MissingContainerPropValueAfterEq => "missing-container-prop-value-after-eq",
      DiagnosticCode::UnclosedContainerPropBlock => "unclosed-container-prop-block",
      DiagnosticCode::MissingFrontmatterMarker => "missing-frontmatter-marker",
      DiagnosticCode::MissingMarkdownHeadingHash => "missing-heading-hash",
      DiagnosticCode::MissingRequiredSpacesBetweenHashAndHeading => "missing-heading-space",
      DiagnosticCode::MissingSyntaxNode => "missing-syntax-node",
      DiagnosticCode::UnclosedLink => "unclosed-link",
      DiagnosticCode::UnclosedBold => "unclosed-bold",
      DiagnosticCode::UnclosedItalic => "unclosed-italic",
      DiagnosticCode::UnclosedStrikethrough => "unclosed-strikethrough",
      DiagnosticCode::UnclosedBoldItalic => "unclosed-bold-italic",
      DiagnosticCode::MismatchedItalicDelimiter => "mismatched-italic-delimiter",
      DiagnosticCode::MissingExpectMdPrefix => "missing-md-prefix",
      DiagnosticCode::MissingTableSeparatorRow => "missing-table-separator",
      DiagnosticCode::TableColumnCountMismatch => "table-column-mismatch",
      DiagnosticCode::InsufficientBlockIndent => "insufficient-block-indent",
      DiagnosticCode::MissingVaultConfig => "missing-vault-config",
      DiagnosticCode::VaultConfigReadError => "vault-config-read-error",
      DiagnosticCode::VaultConfigParseError => "vault-config-parse-error",
      DiagnosticCode::VaultConfigEmpty => "vault-config-empty",
      DiagnosticCode::VaultConfigMissingField => "vault-config-missing-field",
      DiagnosticCode::VaultConfigUnknownField => "vault-config-unknown-field",
      DiagnosticCode::MissingSchemaField => "missing-schema-field",
      DiagnosticCode::UnresolvedSchema => "unresolved-schema",
      DiagnosticCode::WrongTypeArgCount => "wrong-type-arg-count",
      DiagnosticCode::NotCallable => "not-callable",
      DiagnosticCode::WrongArgCount => "wrong-arg-count",
      DiagnosticCode::ArgTypeMismatch => "arg-type-mismatch",
      DiagnosticCode::FieldTypeMismatch => "field-type-mismatch",
      DiagnosticCode::NotIndexable => "not-indexable",
      DiagnosticCode::IndexTypeMismatch => "index-type-mismatch",
      DiagnosticCode::TagTypeMismatch => "tag-type-mismatch",
      DiagnosticCode::OperandTypeMismatch => "operand-type-mismatch",
      DiagnosticCode::MissingRequiredField => "missing-required-field",
      DiagnosticCode::ElementTypeMismatch => "element-type-mismatch",
      DiagnosticCode::DuplicateKey => "duplicate-key",
      DiagnosticCode::UnresolvedFileRef => "unresolved-file-ref",
      DiagnosticCode::UnknownField => "unknown-field",
      DiagnosticCode::IndexOutOfBounds => "index-out-of-bounds",
      DiagnosticCode::NestedSchemaFile => "nested-schema-file",
      DiagnosticCode::VaultConfigInvalidValue => "vault-config-invalid-value",
      DiagnosticCode::InvalidCodeRangeIndicator => "invalid-code-range-indicator",
    }
  }
}

/// Compilation diagnostics.
/// When multiple variants match, use the first (most specific) one.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Diagnostic {
  /* Lexer diagnostics */
  /// Expected a specific character but reached end of input.
  UnexpectedEof {
    expected: char,
    start_offset: usize,
    end_offset: usize,
  },

  /// Expected a specific character but found a different one.
  UnexpectedChar {
    expected: char,
    encountered: char,
    start_offset: usize,
    end_offset: usize,
  },

  /// A "..." or '...' string literal was opened but never closed.
  UnterminatedString {
    start_offset: usize,
    end_offset: usize,
  },

  /// A ${...} interpolation was opened but never closed.
  UnterminatedInterpolation {
    start_offset: usize,
    end_offset: usize,
  },

  /// A fenced code block (```) was opened but never closed.
  UnterminatedCodeBlock {
    start_offset: usize,
    end_offset: usize,
  },

  /// An inline code span (`) was opened but never closed.
  UnterminatedInlineCode {
    start_offset: usize,
    end_offset: usize,
  },

  /// A block math ($$) was opened but never closed.
  UnterminatedMathBlock {
    start_offset: usize,
    end_offset: usize,
  },

  /// An inline math ($) was opened but never closed.
  UnterminatedInlineMath {
    start_offset: usize,
    end_offset: usize,
  },

  /// A code block fence is missing a newline after the opening fence or before the closing fence.
  MissingCodeBlockNewline {
    start_offset: usize,
    end_offset: usize,
  },

  /// A math block delimiter is missing a newline after the opening $$ or before the closing $$.
  MissingMathBlockNewline {
    start_offset: usize,
    end_offset: usize,
  },

  /// Encountered a character that is not valid in the current lexing context.
  InvalidChar {
    encountered: char,
    start_offset: usize,
    end_offset: usize,
  },

  /// Encountered an invalid UTF-8 byte sequence.
  InvalidUtf8 {
    start_offset: usize,
    end_offset: usize,
  },

  /// Mixed tabs and spaces on the same indentation line.
  MixedIndentation {
    start_offset: usize,
    end_offset: usize,
  },

  /// Indentation uses a different character than what was established earlier.
  InconsistentIndentation {
    expected: char,
    encountered: char,
    start_offset: usize,
    end_offset: usize,
  },

  /// Dedent to an indentation level that was never established.
  UnmatchedDedent {
    indent: usize,
    start_offset: usize,
    end_offset: usize,
  },

  /// Missing digits after exponent in scientific notation (e.g. 2.5E+, 1e).
  MissingExponentDigits {
    start_offset: usize,
    end_offset: usize,
  },

  /* Parser diagnostics */
  /// Unexpected tokens found before/after the frontmatter marker ---
  UnexpectedTokensOnFrontmatterMarkerLine {
    start_offset: usize,
    end_offset: usize,
  },

  /// Unexpected tokens inside a container prop block
  UnexpectedContainerPropItem {
    start_offset: usize,
    end_offset: usize,
  },
  /// Unexpected value of a prop
  UnexpectedContainerPropValue {
    start_offset: usize,
    end_offset: usize,
  },
  /// Missing value of a prop afetr =
  MissingContainerPropValueAfterEq { offset: usize },
  /// Unclosed container prop block
  UnclosedContainerPropBlock {
    start_offset: usize,
    end_offset: usize,
  },
  UnexpectedContainerSlotSeparatorToken {
    start_offset: usize,
    end_offset: usize,
  },

  /// Missing frontmatter marker ---
  MissingFrontmatterMarker { offset: usize },

  /// Expected a specific syntax node or token but it was missing.
  MissingMarkdownHeadingHash {
    start_offset: usize,
    end_offset: usize,
  },

  /// Expected a specific syntax node or token but it was missing.
  MissingRequiredSpacesBetweenHashAndHeading {
    start_offset: usize,
    end_offset: usize,
  },

  /// Expected a specific syntax node or token but it was missing.
  MissingSyntaxNode {
    expected: SyntaxKind,
    start_offset: usize,
    end_offset: usize,
  },

  /// A link was opened with `[` but never closed with `](url)`.
  UnclosedLink {
    start_offset: usize,
    end_offset: usize,
  },

  /// A bold span was opened but never closed before a blank line, block boundary, or EOF.
  UnclosedBold {
    start_offset: usize,
    end_offset: usize,
  },

  /// An italic span was opened but never closed before a blank line, block boundary, or EOF.
  UnclosedItalic {
    start_offset: usize,
    end_offset: usize,
  },

  /// A strikethrough span was opened but never closed before a blank line, block boundary, or EOF.
  UnclosedStrikethrough {
    start_offset: usize,
    end_offset: usize,
  },

  /// A bolditalic span was opened but never closed before a blank line, block boundary, or EOF.
  UnclosedBoldItalic {
    start_offset: usize,
    end_offset: usize,
  },

  /// The closing delimiter of an italic span does not match the opening delimiter (`*` vs `_`).
  MismatchedItalicDelimiter {
    start_offset: usize,
    end_offset: usize,
  },

  /// The expected line prefix (e.g. `> ` for blockquotes) was not found after a newline.
  MissingExpectMdPrefix {
    expected_prefix: String,
    start_offset: usize,
    end_offset: usize,
  },

  /// A table is missing the required separator row after the header.
  MissingTableSeparatorRow {
    start_offset: usize,
    end_offset: usize,
  },

  /// A table row has a different number of columns than the header row.
  TableColumnCountMismatch {
    expected: usize,
    found: usize,
    start_offset: usize,
    end_offset: usize,
  },

  /// The indent of a block string content is not greater than the enclosing block indent.
  InsufficientBlockIndent {
    expected_more_than: usize,
    found: usize,
    start_offset: usize,
    end_offset: usize,
  },

  /* Vault config diagnostics */
  /// No typedown.yaml or typedown.yml found in the project root.
  MissingVaultConfig { root_dir: String },

  /// Failed to read the vault config file.
  VaultConfigReadError { path: String, message: String },

  /// Failed to parse the vault config file as YAML.
  VaultConfigParseError {
    path: String,
    message: String,
    start_offset: usize,
    end_offset: usize,
  },

  /// The vault config file is empty.
  VaultConfigEmpty { path: String },

  /// A required field is missing from the vault config.
  VaultConfigMissingField {
    path: String,
    field: String,
    start_offset: usize,
    end_offset: usize,
  },

  /// An unknown field is present in the vault config.
  VaultConfigUnknownField {
    path: String,
    field: String,
    start_offset: usize,
    end_offset: usize,
  },

  /* Typechecker diagnostics */
  /// Missing _type field in top-level mapping.
  MissingSchemaField {
    start_offset: usize,
    end_offset: usize,
  },

  /// Could not resolve _type reference.
  UnresolvedSchema {
    name: String,
    start_offset: usize,
    end_offset: usize,
  },

  /// Wrong number of type arguments passed to a type constructor.
  WrongTypeArgCount { expected: usize, got: usize },

  /// Callee expression is not callable.
  NotCallable {
    start_offset: usize,
    end_offset: usize,
  },

  /// Wrong number of arguments in a function call.
  WrongArgCount {
    expected: usize,
    got: usize,
    start_offset: usize,
    end_offset: usize,
  },

  /// Argument type does not match the expected parameter type.
  ArgTypeMismatch {
    expected: String,
    start_offset: usize,
    end_offset: usize,
  },

  /// A field value does not match the expected type declared by the schema.
  FieldTypeMismatch {
    field: String,
    expected: String,
    start_offset: usize,
    end_offset: usize,
  },

  /// Expression is not indexable.
  NotIndexable {
    start_offset: usize,
    end_offset: usize,
  },

  /// Index type does not match the container's key type.
  IndexTypeMismatch {
    expected: String,
    start_offset: usize,
    end_offset: usize,
  },

  /// Tag inner expression does not match the schema.
  TagTypeMismatch {
    expected: String,
    start_offset: usize,
    end_offset: usize,
  },

  /// Operand type does not match the expected type for the operator.
  OperandTypeMismatch {
    op: String,
    expected: String,
    start_offset: usize,
    end_offset: usize,
  },

  /// A required field declared by the schema is absent in the mapping.
  MissingRequiredField {
    field: String,
    start_offset: usize,
    end_offset: usize,
  },

  /// A sequence element does not match the declared element type.
  ElementTypeMismatch {
    expected: String,
    start_offset: usize,
    end_offset: usize,
  },

  /// A mapping has duplicate keys.
  DuplicateKey {
    key: String,
    start_offset: usize,
    end_offset: usize,
  },

  /// A file reference could not be resolved
  UnresolvedFileRef {
    path: String,
    start_offset: usize,
    end_offset: usize,
  },

  /// A field does not exist on the given type.
  UnknownField {
    field: String,
    on_type: String,
    start_offset: usize,
    end_offset: usize,
  },

  /// An index is out of bounds for the container.
  IndexOutOfBounds {
    index: usize,
    length: usize,
    start_offset: usize,
    end_offset: usize,
  },
  /// Schema file is nested in a subdirectory of schema_dir
  NestedSchemaFile { path: String },
  /// A vault config field has an invalid value
  VaultConfigInvalidValue {
    path: String,
    field: String,
    message: String,
    start_offset: usize,
    end_offset: usize,
  },
  InvalidCodeRangeIndicator {
    start_offset: usize,
    end_offset: usize,
  },
}

impl Diagnostic {
  /// Return the `(start, end)` char offsets for file-level diagnostics.
  /// Returns `None` for project-level diagnostics that have no file position.
  pub fn offsets(&self) -> Option<(usize, usize)> {
    match self {
      Diagnostic::UnexpectedEof {
        start_offset,
        end_offset,
        ..
      }
      | Diagnostic::UnexpectedChar {
        start_offset,
        end_offset,
        ..
      }
      | Diagnostic::UnterminatedString {
        start_offset,
        end_offset,
      }
      | Diagnostic::UnterminatedInterpolation {
        start_offset,
        end_offset,
      }
      | Diagnostic::UnterminatedCodeBlock {
        start_offset,
        end_offset,
      }
      | Diagnostic::UnterminatedInlineCode {
        start_offset,
        end_offset,
      }
      | Diagnostic::UnterminatedMathBlock {
        start_offset,
        end_offset,
      }
      | Diagnostic::UnterminatedInlineMath {
        start_offset,
        end_offset,
      }
      | Diagnostic::MissingCodeBlockNewline {
        start_offset,
        end_offset,
      }
      | Diagnostic::MissingMathBlockNewline {
        start_offset,
        end_offset,
      }
      | Diagnostic::InvalidChar {
        start_offset,
        end_offset,
        ..
      }
      | Diagnostic::InvalidUtf8 {
        start_offset,
        end_offset,
      }
      | Diagnostic::MixedIndentation {
        start_offset,
        end_offset,
      }
      | Diagnostic::InconsistentIndentation {
        start_offset,
        end_offset,
        ..
      }
      | Diagnostic::UnmatchedDedent {
        start_offset,
        end_offset,
        ..
      }
      | Diagnostic::MissingExponentDigits {
        start_offset,
        end_offset,
      }
      | Diagnostic::UnexpectedTokensOnFrontmatterMarkerLine {
        start_offset,
        end_offset,
      }
      | Diagnostic::UnexpectedContainerPropItem {
        start_offset,
        end_offset,
      }
      | Diagnostic::UnexpectedContainerPropValue {
        start_offset,
        end_offset,
      }
      | Diagnostic::UnexpectedContainerSlotSeparatorToken {
        start_offset,
        end_offset,
      }
      | Diagnostic::UnclosedContainerPropBlock {
        start_offset,
        end_offset,
      }
      | Diagnostic::MissingMarkdownHeadingHash {
        start_offset,
        end_offset,
      }
      | Diagnostic::MissingRequiredSpacesBetweenHashAndHeading {
        start_offset,
        end_offset,
      }
      | Diagnostic::MissingSyntaxNode {
        start_offset,
        end_offset,
        ..
      }
      | Diagnostic::UnclosedLink {
        start_offset,
        end_offset,
      }
      | Diagnostic::UnclosedBold {
        start_offset,
        end_offset,
      }
      | Diagnostic::UnclosedItalic {
        start_offset,
        end_offset,
      }
      | Diagnostic::UnclosedStrikethrough {
        start_offset,
        end_offset,
      }
      | Diagnostic::UnclosedBoldItalic {
        start_offset,
        end_offset,
      }
      | Diagnostic::MismatchedItalicDelimiter {
        start_offset,
        end_offset,
      }
      | Diagnostic::MissingExpectMdPrefix {
        start_offset,
        end_offset,
        ..
      }
      | Diagnostic::MissingTableSeparatorRow {
        start_offset,
        end_offset,
      }
      | Diagnostic::TableColumnCountMismatch {
        start_offset,
        end_offset,
        ..
      }
      | Diagnostic::InsufficientBlockIndent {
        start_offset,
        end_offset,
        ..
      }
      | Diagnostic::MissingSchemaField {
        start_offset,
        end_offset,
      }
      | Diagnostic::UnresolvedSchema {
        start_offset,
        end_offset,
        ..
      }
      | Diagnostic::NotCallable {
        start_offset,
        end_offset,
      }
      | Diagnostic::WrongArgCount {
        start_offset,
        end_offset,
        ..
      }
      | Diagnostic::ArgTypeMismatch {
        start_offset,
        end_offset,
        ..
      }
      | Diagnostic::FieldTypeMismatch {
        start_offset,
        end_offset,
        ..
      }
      | Diagnostic::NotIndexable {
        start_offset,
        end_offset,
      }
      | Diagnostic::IndexTypeMismatch {
        start_offset,
        end_offset,
        ..
      }
      | Diagnostic::TagTypeMismatch {
        start_offset,
        end_offset,
        ..
      }
      | Diagnostic::OperandTypeMismatch {
        start_offset,
        end_offset,
        ..
      }
      | Diagnostic::MissingRequiredField {
        start_offset,
        end_offset,
        ..
      }
      | Diagnostic::ElementTypeMismatch {
        start_offset,
        end_offset,
        ..
      }
      | Diagnostic::DuplicateKey {
        start_offset,
        end_offset,
        ..
      }
      | Diagnostic::UnresolvedFileRef {
        start_offset,
        end_offset,
        ..
      }
      | Diagnostic::UnknownField {
        start_offset,
        end_offset,
        ..
      }
      | Diagnostic::IndexOutOfBounds {
        start_offset,
        end_offset,
        ..
      } => Some((*start_offset, *end_offset)),
      Diagnostic::MissingFrontmatterMarker { offset }
      | Diagnostic::MissingContainerPropValueAfterEq { offset } => Some((*offset, *offset)),
      Diagnostic::VaultConfigParseError {
        start_offset,
        end_offset,
        ..
      } => Some((*start_offset, *end_offset)),
      Diagnostic::VaultConfigMissingField {
        start_offset,
        end_offset,
        ..
      }
      | Diagnostic::VaultConfigUnknownField {
        start_offset,
        end_offset,
        ..
      }
      | Diagnostic::VaultConfigInvalidValue {
        start_offset,
        end_offset,
        ..
      }
      | Diagnostic::InvalidCodeRangeIndicator {
        start_offset,
        end_offset,
      } => Some((*start_offset, *end_offset)),
      Diagnostic::MissingVaultConfig { .. }
      | Diagnostic::VaultConfigReadError { .. }
      | Diagnostic::VaultConfigEmpty { .. }
      | Diagnostic::WrongTypeArgCount { .. }
      | Diagnostic::NestedSchemaFile { .. } => None,
    }
  }

  /// A human-readable message for this diagnostic.
  pub fn message(&self) -> String {
    match self {
      Diagnostic::UnexpectedEof { expected, .. } => {
        format!("unexpected end of input, expected '{expected}'")
      }
      Diagnostic::UnexpectedChar {
        expected,
        encountered,
        ..
      } => {
        format!("expected '{expected}', found '{encountered}'")
      }
      Diagnostic::UnterminatedString { .. } => "unterminated string literal".into(),
      Diagnostic::UnterminatedInterpolation { .. } => "unterminated interpolation '${'".into(),
      Diagnostic::UnterminatedCodeBlock { .. } => "unterminated code block '```'".into(),
      Diagnostic::UnterminatedInlineCode { .. } => "unterminated inline code '`'".into(),
      Diagnostic::UnterminatedMathBlock { .. } => "unterminated math block '$$'".into(),
      Diagnostic::UnterminatedInlineMath { .. } => "unterminated inline math '$'".into(),
      Diagnostic::MissingCodeBlockNewline { .. } => "missing newline after code block fence".into(),
      Diagnostic::MissingMathBlockNewline { .. } => {
        "missing newline after math block delimiter".into()
      }
      Diagnostic::InvalidChar { encountered, .. } => {
        format!("invalid character '{encountered}' in this context")
      }
      Diagnostic::InvalidUtf8 { .. } => "invalid UTF-8 byte sequence".into(),
      Diagnostic::MixedIndentation { .. } => "mixed tabs and spaces in indentation".into(),
      Diagnostic::InconsistentIndentation {
        expected,
        encountered,
        ..
      } => {
        format!("inconsistent indentation: expected '{expected}', found '{encountered}'")
      }
      Diagnostic::UnmatchedDedent { indent, .. } => {
        format!("dedent to unestablished indentation level {indent}")
      }
      Diagnostic::MissingExponentDigits { .. } => {
        "missing digits after exponent in numeric literal".into()
      }
      Diagnostic::UnexpectedTokensOnFrontmatterMarkerLine { .. } => {
        "unexpected tokens on frontmatter marker line '---'".into()
      }
      Diagnostic::UnexpectedContainerPropItem { .. } => {
        "unexpected tokens in container prop item".into()
      }
      Diagnostic::UnexpectedContainerPropValue { .. } => {
        "unexpected tokens in container prop value".into()
      }
      Diagnostic::UnexpectedContainerSlotSeparatorToken { .. } => {
        "unexpected tokens in container slot separator line".into()
      }
      Diagnostic::MissingContainerPropValueAfterEq { .. } => {
        "missing container prop value after '='".into()
      }
      Diagnostic::UnclosedContainerPropBlock { .. } => "unclosed container prop block".into(),
      Diagnostic::MissingFrontmatterMarker { .. } => "missing frontmatter marker '---'".into(),
      Diagnostic::MissingMarkdownHeadingHash { .. } => "missing '#' for markdown heading".into(),
      Diagnostic::MissingRequiredSpacesBetweenHashAndHeading { .. } => {
        "missing space between '#' and heading text".into()
      }
      Diagnostic::MissingSyntaxNode { expected, .. } => {
        format!("missing {expected:?}")
      }
      Diagnostic::UnclosedLink { .. } => "unclosed link '[', expected '](url)'".into(),
      Diagnostic::UnclosedBold { .. } => "unclosed bold span".into(),
      Diagnostic::UnclosedItalic { .. } => "unclosed italic span".into(),
      Diagnostic::UnclosedStrikethrough { .. } => "unclosed strikethrough span".into(),
      Diagnostic::UnclosedBoldItalic { .. } => "unclosed bold-italic span".into(),
      Diagnostic::MismatchedItalicDelimiter { .. } => {
        "italic closing delimiter does not match opening delimiter".into()
      }
      Diagnostic::MissingExpectMdPrefix {
        expected_prefix, ..
      } => {
        format!("missing expected prefix '{expected_prefix}'")
      }
      Diagnostic::MissingTableSeparatorRow { .. } => {
        "missing separator row after table header".into()
      }
      Diagnostic::TableColumnCountMismatch {
        expected, found, ..
      } => {
        format!("table row has {found} columns, expected {expected}")
      }
      Diagnostic::InsufficientBlockIndent {
        expected_more_than,
        found,
        ..
      } => {
        format!("block indent {found} must be greater than enclosing indent {expected_more_than}")
      }
      Diagnostic::MissingVaultConfig { root_dir } => {
        format!("no typedown.yaml or typedown.yml found in '{root_dir}'")
      }
      Diagnostic::VaultConfigReadError { path, message } => {
        format!("failed to read vault config '{path}': {message}")
      }
      Diagnostic::VaultConfigParseError { path, message, .. } => {
        format!("failed to parse vault config '{path}': {message}")
      }
      Diagnostic::VaultConfigEmpty { path } => {
        format!("vault config '{path}' is empty")
      }
      Diagnostic::VaultConfigMissingField { path, field, .. } => {
        format!("vault config '{path}' is missing required field '{field}'")
      }
      Diagnostic::VaultConfigUnknownField { path, field, .. } => {
        format!("vault config '{path}' has unknown field '{field}'")
      }
      Diagnostic::MissingSchemaField { .. } => "missing '_type' field in top-level mapping".into(),
      Diagnostic::UnresolvedSchema { name, .. } => {
        format!("cannot resolve type '{name}'")
      }
      Diagnostic::WrongTypeArgCount { expected, got } => {
        format!("wrong number of type arguments: expected {expected}, got {got}")
      }
      Diagnostic::NotCallable { .. } => "expression is not callable".into(),
      Diagnostic::WrongArgCount { expected, got, .. } => {
        format!("wrong number of arguments: expected {expected}, got {got}")
      }
      Diagnostic::ArgTypeMismatch { expected, .. } => {
        format!("argument type mismatch: expected {expected}")
      }
      Diagnostic::FieldTypeMismatch {
        field, expected, ..
      } => {
        format!("field '{field}' type mismatch: expected {expected}")
      }
      Diagnostic::NotIndexable { .. } => "expression is not indexable".into(),
      Diagnostic::IndexTypeMismatch { expected, .. } => {
        format!("index type mismatch: expected {expected}")
      }
      Diagnostic::TagTypeMismatch { expected, .. } => {
        format!("tag type mismatch: expected {expected}")
      }
      Diagnostic::OperandTypeMismatch { op, expected, .. } => {
        format!("operand type mismatch for '{op}': expected {expected}")
      }
      Diagnostic::MissingRequiredField { field, .. } => {
        format!("missing required field '{field}'")
      }
      Diagnostic::ElementTypeMismatch { expected, .. } => {
        format!("element type mismatch: expected {expected}")
      }
      Diagnostic::DuplicateKey { key, .. } => {
        format!("duplicate key '{key}'")
      }
      Diagnostic::UnresolvedFileRef { path, .. } => {
        format!("cannot resolve file reference '{path}'")
      }
      Diagnostic::UnknownField { field, on_type, .. } => {
        format!("unknown field '{field}' on type '{on_type}'")
      }
      Diagnostic::NestedSchemaFile { path } => {
        format!("unsupported nested schema file: {}", path)
      }
      Diagnostic::VaultConfigInvalidValue {
        path,
        field,
        message,
        ..
      } => {
        format!("vault config '{path}' field '{field}': {message}")
      }
      Diagnostic::IndexOutOfBounds { index, length, .. } => {
        format!("index {index} is out of bounds for length {length}")
      }
      Diagnostic::InvalidCodeRangeIndicator { .. } => {
        "invalid code block line range indicator".into()
      }
    }
  }

  /// A short kebab-case code identifying the diagnostic kind.
  pub fn code(&self) -> DiagnosticCode {
    match self {
      Diagnostic::UnexpectedEof { .. } => DiagnosticCode::UnexpectedEof,
      Diagnostic::UnexpectedChar { .. } => DiagnosticCode::UnexpectedChar,
      Diagnostic::UnterminatedString { .. } => DiagnosticCode::UnterminatedString,
      Diagnostic::UnterminatedInterpolation { .. } => DiagnosticCode::UnterminatedInterpolation,
      Diagnostic::UnterminatedCodeBlock { .. } => DiagnosticCode::UnterminatedCodeBlock,
      Diagnostic::UnterminatedInlineCode { .. } => DiagnosticCode::UnterminatedInlineCode,
      Diagnostic::UnterminatedMathBlock { .. } => DiagnosticCode::UnterminatedMathBlock,
      Diagnostic::UnterminatedInlineMath { .. } => DiagnosticCode::UnterminatedInlineMath,
      Diagnostic::MissingCodeBlockNewline { .. } => DiagnosticCode::MissingCodeBlockNewline,
      Diagnostic::MissingMathBlockNewline { .. } => DiagnosticCode::MissingMathBlockNewline,
      Diagnostic::InvalidChar { .. } => DiagnosticCode::InvalidChar,
      Diagnostic::InvalidUtf8 { .. } => DiagnosticCode::InvalidUtf8,
      Diagnostic::MixedIndentation { .. } => DiagnosticCode::MixedIndentation,
      Diagnostic::InconsistentIndentation { .. } => DiagnosticCode::InconsistentIndentation,
      Diagnostic::UnmatchedDedent { .. } => DiagnosticCode::UnmatchedDedent,
      Diagnostic::MissingExponentDigits { .. } => DiagnosticCode::MissingExponentDigits,
      Diagnostic::UnexpectedTokensOnFrontmatterMarkerLine { .. } => {
        DiagnosticCode::UnexpectedTokensOnFrontmatterMarkerLine
      }
      Diagnostic::UnexpectedContainerPropItem { .. } => DiagnosticCode::UnexpectedContainerPropItem,
      Diagnostic::UnexpectedContainerPropValue { .. } => {
        DiagnosticCode::UnexpectedContainerPropValue
      }
      Diagnostic::MissingContainerPropValueAfterEq { .. } => {
        DiagnosticCode::MissingContainerPropValueAfterEq
      }
      Diagnostic::UnclosedContainerPropBlock { .. } => DiagnosticCode::UnclosedContainerPropBlock,
      Diagnostic::UnexpectedContainerSlotSeparatorToken { .. } => {
        DiagnosticCode::UnexpectedContainerSlotSeparatorToken
      }
      Diagnostic::MissingFrontmatterMarker { .. } => DiagnosticCode::MissingFrontmatterMarker,
      Diagnostic::MissingMarkdownHeadingHash { .. } => DiagnosticCode::MissingMarkdownHeadingHash,
      Diagnostic::MissingRequiredSpacesBetweenHashAndHeading { .. } => {
        DiagnosticCode::MissingRequiredSpacesBetweenHashAndHeading
      }
      Diagnostic::MissingSyntaxNode { .. } => DiagnosticCode::MissingSyntaxNode,
      Diagnostic::UnclosedLink { .. } => DiagnosticCode::UnclosedLink,
      Diagnostic::UnclosedBold { .. } => DiagnosticCode::UnclosedBold,
      Diagnostic::UnclosedItalic { .. } => DiagnosticCode::UnclosedItalic,
      Diagnostic::UnclosedStrikethrough { .. } => DiagnosticCode::UnclosedStrikethrough,
      Diagnostic::UnclosedBoldItalic { .. } => DiagnosticCode::UnclosedBoldItalic,
      Diagnostic::MismatchedItalicDelimiter { .. } => DiagnosticCode::MismatchedItalicDelimiter,
      Diagnostic::MissingExpectMdPrefix { .. } => DiagnosticCode::MissingExpectMdPrefix,
      Diagnostic::MissingTableSeparatorRow { .. } => DiagnosticCode::MissingTableSeparatorRow,
      Diagnostic::TableColumnCountMismatch { .. } => DiagnosticCode::TableColumnCountMismatch,
      Diagnostic::InsufficientBlockIndent { .. } => DiagnosticCode::InsufficientBlockIndent,
      Diagnostic::MissingVaultConfig { .. } => DiagnosticCode::MissingVaultConfig,
      Diagnostic::VaultConfigReadError { .. } => DiagnosticCode::VaultConfigReadError,
      Diagnostic::VaultConfigParseError { .. } => DiagnosticCode::VaultConfigParseError,
      Diagnostic::VaultConfigEmpty { .. } => DiagnosticCode::VaultConfigEmpty,
      Diagnostic::VaultConfigMissingField { .. } => DiagnosticCode::VaultConfigMissingField,
      Diagnostic::VaultConfigUnknownField { .. } => DiagnosticCode::VaultConfigUnknownField,
      Diagnostic::MissingSchemaField { .. } => DiagnosticCode::MissingSchemaField,
      Diagnostic::UnresolvedSchema { .. } => DiagnosticCode::UnresolvedSchema,
      Diagnostic::WrongTypeArgCount { .. } => DiagnosticCode::WrongTypeArgCount,
      Diagnostic::NotCallable { .. } => DiagnosticCode::NotCallable,
      Diagnostic::WrongArgCount { .. } => DiagnosticCode::WrongArgCount,
      Diagnostic::ArgTypeMismatch { .. } => DiagnosticCode::ArgTypeMismatch,
      Diagnostic::FieldTypeMismatch { .. } => DiagnosticCode::FieldTypeMismatch,
      Diagnostic::NotIndexable { .. } => DiagnosticCode::NotIndexable,
      Diagnostic::IndexTypeMismatch { .. } => DiagnosticCode::IndexTypeMismatch,
      Diagnostic::TagTypeMismatch { .. } => DiagnosticCode::TagTypeMismatch,
      Diagnostic::OperandTypeMismatch { .. } => DiagnosticCode::OperandTypeMismatch,
      Diagnostic::MissingRequiredField { .. } => DiagnosticCode::MissingRequiredField,
      Diagnostic::ElementTypeMismatch { .. } => DiagnosticCode::ElementTypeMismatch,
      Diagnostic::DuplicateKey { .. } => DiagnosticCode::DuplicateKey,
      Diagnostic::UnresolvedFileRef { .. } => DiagnosticCode::UnresolvedFileRef,
      Diagnostic::UnknownField { .. } => DiagnosticCode::UnknownField,
      Diagnostic::IndexOutOfBounds { .. } => DiagnosticCode::IndexOutOfBounds,
      Diagnostic::NestedSchemaFile { .. } => DiagnosticCode::NestedSchemaFile,
      Diagnostic::VaultConfigInvalidValue { .. } => DiagnosticCode::VaultConfigInvalidValue,
      Diagnostic::InvalidCodeRangeIndicator { .. } => DiagnosticCode::InvalidCodeRangeIndicator,
    }
  }
}
