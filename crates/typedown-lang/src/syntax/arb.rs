//! Arbitrary implementations for syntax types, used in property-based tests

use proptest::prelude::*;
use strum::IntoEnumIterator;

use crate::syntax::diagnostic::*;
use crate::syntax::green::GreenNode;
use crate::syntax::green::cache::Cache;
use crate::syntax::green::token::SyntaxToken;
use crate::syntax::red::RedNode;
use crate::syntax::syntax_kind::SyntaxKind;

impl Arbitrary for SyntaxKind {
  type Parameters = ();
  type Strategy = BoxedStrategy<Self>;

  fn arbitrary_with(_: ()) -> Self::Strategy {
    let all: Vec<SyntaxKind> = SyntaxKind::iter().collect();
    prop::sample::select(all).boxed()
  }
}

impl Arbitrary for GreenNode {
  type Parameters = ();
  type Strategy = BoxedStrategy<Self>;

  fn arbitrary_with(_: ()) -> Self::Strategy {
    arb_green_token()
      .prop_recursive(4, 64, 8, arb_green_node)
      .boxed()
  }
}

impl Arbitrary for RedNode {
  type Parameters = ();
  type Strategy = BoxedStrategy<Self>;

  fn arbitrary_with(_: ()) -> Self::Strategy {
    // RedNode root must be a node, not a token
    arb_green_node(arb_green_token().prop_recursive(3, 64, 8, arb_green_node))
      .prop_map(|green| RedNode::from_green(0, green))
      .boxed()
  }
}

fn arb_offsets() -> impl Strategy<Value = (usize, usize)> {
  (0..10000usize, 0..10000usize).prop_map(|(a, b)| if a <= b { (a, b) } else { (b, a) })
}

impl Arbitrary for TdDiagnostic {
  type Parameters = ();
  type Strategy = BoxedStrategy<Self>;

  fn arbitrary_with(_: ()) -> Self::Strategy {
    prop_oneof![
      (any::<char>(), arb_offsets()).prop_map(|(expected, (start_offset, end_offset))| {
        TdDiagnosticKind::UnexpectedEof { expected }.into_error(start_offset, end_offset)
      }),
      (any::<char>(), any::<char>(), arb_offsets()).prop_map(
        |(expected, encountered, (start_offset, end_offset))| {
          TdDiagnosticKind::UnexpectedChar {
            expected,
            encountered,
          }
          .into_error(start_offset, end_offset)
        }
      ),
      arb_offsets().prop_map(|(s, e)| TdDiagnosticKind::UnterminatedString {}.into_error(s, e)),
      arb_offsets()
        .prop_map(|(s, e)| TdDiagnosticKind::UnterminatedInterpolation {}.into_error(s, e)),
      arb_offsets().prop_map(|(s, e)| TdDiagnosticKind::UnterminatedCodeBlock {}.into_error(s, e)),
      arb_offsets().prop_map(|(s, e)| TdDiagnosticKind::UnterminatedInlineCode {}.into_error(s, e)),
      arb_offsets().prop_map(|(s, e)| TdDiagnosticKind::UnterminatedMathBlock {}.into_error(s, e)),
      arb_offsets().prop_map(|(s, e)| TdDiagnosticKind::UnterminatedInlineMath {}.into_error(s, e)),
      arb_offsets().prop_map(|(s, e)| TdDiagnosticKind::MissingCodeBlockNewline {}.into_error(s, e)),
      arb_offsets().prop_map(|(s, e)| TdDiagnosticKind::MissingMathBlockNewline {}.into_error(s, e)),
      (any::<char>(), arb_offsets()).prop_map(|(encountered, (start_offset, end_offset))| {
        TdDiagnosticKind::InvalidChar { encountered }.into_error(start_offset, end_offset)
      }),
      arb_offsets().prop_map(|(s, e)| TdDiagnosticKind::InvalidUtf8 {}.into_error(s, e)),
      arb_offsets().prop_map(|(s, e)| TdDiagnosticKind::MixedIndentation {}.into_error(s, e)),
      (any::<char>(), any::<char>(), arb_offsets()).prop_map(
        |(expected, encountered, (start_offset, end_offset))| {
          TdDiagnosticKind::InconsistentIndentation {
            expected,
            encountered,
          }
          .into_error(start_offset, end_offset)
        }
      ),
      (any::<usize>(), arb_offsets()).prop_map(|(indent, (start_offset, end_offset))| {
        TdDiagnosticKind::UnmatchedDedent { indent }.into_error(start_offset, end_offset)
      }),
      arb_offsets().prop_map(|(s, e)| TdDiagnosticKind::MissingExponentDigits {}.into_error(s, e)),
      arb_offsets().prop_map(|(s, e)| {
        TdDiagnosticKind::UnexpectedTokensOnFrontmatterMarkerLine {}.into_error(s, e)
      }),
      any::<usize>()
        .prop_map(|offset| TdDiagnosticKind::MissingFrontmatterMarker {}.into_error(offset, offset)),
      arb_offsets()
        .prop_map(|(s, e)| TdDiagnosticKind::MissingMarkdownHeadingHash {}.into_error(s, e)),
      arb_offsets().prop_map(|(s, e)| { TdDiagnosticKind::MissingRequiredSpace {}.into_error(s, e) }),
      (any::<SyntaxKind>(), arb_offsets()).prop_map(|(expected, (start_offset, end_offset))| {
        TdDiagnosticKind::MissingSyntaxNode { expected }.into_error(start_offset, end_offset)
      }),
      arb_offsets().prop_map(|(s, e)| TdDiagnosticKind::UnclosedLink {}.into_error(s, e)),
      arb_offsets().prop_map(|(s, e)| TdDiagnosticKind::UnclosedBold {}.into_error(s, e)),
      arb_offsets().prop_map(|(s, e)| TdDiagnosticKind::UnclosedItalic {}.into_error(s, e)),
      arb_offsets().prop_map(|(s, e)| TdDiagnosticKind::UnclosedStrikethrough {}.into_error(s, e)),
      arb_offsets().prop_map(|(s, e)| TdDiagnosticKind::UnclosedBoldItalic {}.into_error(s, e)),
      arb_offsets()
        .prop_map(|(s, e)| TdDiagnosticKind::MismatchedItalicDelimiter {}.into_error(s, e)),
      (".*", arb_offsets()).prop_map(|(expected_prefix, (start_offset, end_offset))| {
        TdDiagnosticKind::MissingExpectedMdPrefix { expected_prefix }
          .into_error(start_offset, end_offset)
      }),
      arb_offsets().prop_map(|(s, e)| TdDiagnosticKind::MissingTableSeparatorRow {}.into_error(s, e)),
      (any::<usize>(), any::<usize>(), arb_offsets()).prop_map(
        |(expected, found, (start_offset, end_offset))| {
          TdDiagnosticKind::TableColumnCountMismatch { expected, found }
            .into_error(start_offset, end_offset)
        }
      ),
      (any::<usize>(), any::<usize>(), arb_offsets()).prop_map(
        |(expected_more_than, found, (start_offset, end_offset))| {
          TdDiagnosticKind::InsufficientBlockIndent {
            expected_more_than,
            found,
          }
          .into_error(start_offset, end_offset)
        }
      ),
      ".*".prop_map(|root_dir| TdDiagnosticKind::MissingVaultConfig { root_dir }.into_error(0, 0)),
      (".*", ".*").prop_map(|(path, message)| TdDiagnosticKind::VaultConfigReadError {
        path,
        message
      }
      .into_error(0, 0)),
      (".*", ".*", arb_offsets()).prop_map(|(path, message, (start_offset, end_offset))| {
        TdDiagnosticKind::VaultConfigParseError { path, message }.into_error(start_offset, end_offset)
      }),
      ".*".prop_map(|path| TdDiagnosticKind::VaultConfigEmpty { path }.into_error(0, 0)),
      (".*", ".*", arb_offsets()).prop_map(|(path, field, (start_offset, end_offset))| {
        TdDiagnosticKind::VaultConfigMissingField { path, field }.into_error(start_offset, end_offset)
      }),
      (".*", ".*", arb_offsets()).prop_map(|(path, field, (start_offset, end_offset))| {
        TdDiagnosticKind::VaultConfigUnknownField { path, field }.into_error(start_offset, end_offset)
      }),
      arb_offsets().prop_map(|(s, e)| TdDiagnosticKind::MissingSchemaField {}.into_error(s, e)),
      (".*", arb_offsets()).prop_map(|(name, (start_offset, end_offset))| {
        TdDiagnosticKind::UnresolvedSchema { name }.into_error(start_offset, end_offset)
      }),
      (any::<usize>(), any::<usize>(), arb_offsets()).prop_map(
        |(expected, got, (start_offset, end_offset))| {
          TdDiagnosticKind::WrongTypArgCount { expected, got }.into_error(start_offset, end_offset)
        }
      ),
      arb_offsets().prop_map(|(s, e)| TdDiagnosticKind::NotCallable {}.into_error(s, e)),
      (any::<usize>(), any::<usize>(), arb_offsets()).prop_map(
        |(expected, got, (start_offset, end_offset))| {
          TdDiagnosticKind::WrongArgCount { expected, got }.into_error(start_offset, end_offset)
        }
      ),
      (".*", arb_offsets()).prop_map(|(expected, (start_offset, end_offset))| {
        TdDiagnosticKind::ArgTypMismatch { expected }.into_error(start_offset, end_offset)
      }),
      (".*", ".*", arb_offsets()).prop_map(|(field, expected, (start_offset, end_offset))| {
        TdDiagnosticKind::FieldTypMismatch { field, expected }.into_error(start_offset, end_offset)
      }),
      arb_offsets().prop_map(|(s, e)| TdDiagnosticKind::NotIndexable {}.into_error(s, e)),
      (".*", arb_offsets()).prop_map(|(expected, (start_offset, end_offset))| {
        TdDiagnosticKind::IndexTypMismatch { expected }.into_error(start_offset, end_offset)
      }),
      (".*", arb_offsets()).prop_map(|(expected, (start_offset, end_offset))| {
        TdDiagnosticKind::TagTypMismatch { expected }.into_error(start_offset, end_offset)
      }),
      (".*", ".*", arb_offsets()).prop_map(|(op, expected, (start_offset, end_offset))| {
        TdDiagnosticKind::OperandTypMismatch { op, expected }.into_error(start_offset, end_offset)
      }),
      (".*", arb_offsets()).prop_map(|(field, (start_offset, end_offset))| {
        TdDiagnosticKind::MissingRequiredField { field }.into_error(start_offset, end_offset)
      }),
      (".*", arb_offsets()).prop_map(|(expected, (start_offset, end_offset))| {
        TdDiagnosticKind::ElementTypMismatch { expected }.into_error(start_offset, end_offset)
      }),
      (".*", arb_offsets()).prop_map(|(key, (start_offset, end_offset))| {
        TdDiagnosticKind::DuplicateKey { key }.into_error(start_offset, end_offset)
      }),
      (".*", arb_offsets()).prop_map(|(path, (start_offset, end_offset))| {
        TdDiagnosticKind::UnresolvedFileRef { path }.into_error(start_offset, end_offset)
      }),
      (".*", ".*", arb_offsets()).prop_map(|(field, on_typ, (start_offset, end_offset))| {
        TdDiagnosticKind::UnknownField { field, on_typ }.into_error(start_offset, end_offset)
      }),
      (any::<usize>(), any::<usize>(), arb_offsets()).prop_map(
        |(index, length, (start_offset, end_offset))| {
          TdDiagnosticKind::IndexOutOfBounds { index, length }.into_error(start_offset, end_offset)
        }
      ),
    ]
    .boxed()
  }
}

fn arb_green_token() -> impl Strategy<Value = GreenNode> {
  (
    any::<SyntaxKind>(),
    proptest::collection::vec(any::<char>(), 0..32),
  )
    .prop_map(|(kind, bytes)| {
      let token =
        SyntaxToken::from_raw_parts(kind, bytes.into_iter().collect::<String>().into_bytes());
      GreenNode::from_token(token)
    })
}

fn arb_green_node(
  child: impl Strategy<Value = GreenNode> + 'static,
) -> impl Strategy<Value = GreenNode> {
  (any::<SyntaxKind>(), proptest::collection::vec(child, 0..8)).prop_map(|(kind, children)| {
    let mut cache = Cache::new();
    let node = cache.node(kind, &children);
    GreenNode::from_node(node)
  })
}
