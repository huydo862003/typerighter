//! Arbitrary implementations for syntax types, used in property-based tests.

use proptest::prelude::*;
use strum::IntoEnumIterator;

use crate::syntax::diagnostic::Diagnostic;
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

impl Arbitrary for Diagnostic {
  type Parameters = ();
  type Strategy = BoxedStrategy<Self>;

  fn arbitrary_with(_: ()) -> Self::Strategy {
    prop_oneof![
      // {expected: char, start_offset, end_offset}
      (any::<char>(), arb_offsets()).prop_map(|(expected, (start_offset, end_offset))| {
        Diagnostic::UnexpectedEof {
          expected,
          start_offset,
          end_offset,
        }
      }),
      // {expected: char, encountered: char, start_offset, end_offset}
      (any::<char>(), any::<char>(), arb_offsets()).prop_map(
        |(expected, encountered, (start_offset, end_offset))| {
          Diagnostic::UnexpectedChar {
            expected,
            encountered,
            start_offset,
            end_offset,
          }
        }
      ),
      // {start_offset, end_offset} variants
      arb_offsets().prop_map(
        |(start_offset, end_offset)| Diagnostic::UnterminatedString {
          start_offset,
          end_offset
        }
      ),
      arb_offsets().prop_map(
        |(start_offset, end_offset)| Diagnostic::UnterminatedInterpolation {
          start_offset,
          end_offset
        }
      ),
      arb_offsets().prop_map(
        |(start_offset, end_offset)| Diagnostic::UnterminatedCodeBlock {
          start_offset,
          end_offset
        }
      ),
      arb_offsets().prop_map(
        |(start_offset, end_offset)| Diagnostic::UnterminatedInlineCode {
          start_offset,
          end_offset
        }
      ),
      arb_offsets().prop_map(
        |(start_offset, end_offset)| Diagnostic::UnterminatedMathBlock {
          start_offset,
          end_offset
        }
      ),
      arb_offsets().prop_map(
        |(start_offset, end_offset)| Diagnostic::UnterminatedInlineMath {
          start_offset,
          end_offset
        }
      ),
      arb_offsets().prop_map(
        |(start_offset, end_offset)| Diagnostic::MissingCodeBlockNewline {
          start_offset,
          end_offset
        }
      ),
      arb_offsets().prop_map(
        |(start_offset, end_offset)| Diagnostic::MissingMathBlockNewline {
          start_offset,
          end_offset
        }
      ),
      // {encountered: char, start_offset, end_offset}
      (any::<char>(), arb_offsets()).prop_map(|(encountered, (start_offset, end_offset))| {
        Diagnostic::InvalidChar {
          encountered,
          start_offset,
          end_offset,
        }
      }),
      arb_offsets().prop_map(|(start_offset, end_offset)| Diagnostic::InvalidUtf8 {
        start_offset,
        end_offset
      }),
      arb_offsets().prop_map(|(start_offset, end_offset)| Diagnostic::MixedIndentation {
        start_offset,
        end_offset
      }),
      // {expected: char, encountered: char, start_offset, end_offset}
      (any::<char>(), any::<char>(), arb_offsets()).prop_map(
        |(expected, encountered, (start_offset, end_offset))| {
          Diagnostic::InconsistentIndentation {
            expected,
            encountered,
            start_offset,
            end_offset,
          }
        }
      ),
      // {indent: usize, start_offset, end_offset}
      (any::<usize>(), arb_offsets()).prop_map(|(indent, (start_offset, end_offset))| {
        Diagnostic::UnmatchedDedent {
          indent,
          start_offset,
          end_offset,
        }
      }),
      arb_offsets().prop_map(
        |(start_offset, end_offset)| Diagnostic::MissingExponentDigits {
          start_offset,
          end_offset
        }
      ),
      arb_offsets().prop_map(|(start_offset, end_offset)| {
        Diagnostic::UnexpectedTokensOnFrontmatterMarkerLine {
          start_offset,
          end_offset,
        }
      }),
      any::<usize>().prop_map(|offset| Diagnostic::MissingFrontmatterMarker { offset }),
      arb_offsets().prop_map(
        |(start_offset, end_offset)| Diagnostic::MissingMarkdownHeadingHash {
          start_offset,
          end_offset
        }
      ),
      arb_offsets().prop_map(|(start_offset, end_offset)| {
        Diagnostic::MissingRequiredSpacesBetweenHashAndHeading {
          start_offset,
          end_offset,
        }
      }),
      // {expected: SyntaxKind, start_offset, end_offset}
      (any::<SyntaxKind>(), arb_offsets()).prop_map(|(expected, (start_offset, end_offset))| {
        Diagnostic::MissingSyntaxNode {
          expected,
          start_offset,
          end_offset,
        }
      }),
      arb_offsets().prop_map(|(start_offset, end_offset)| Diagnostic::UnclosedLink {
        start_offset,
        end_offset
      }),
      arb_offsets().prop_map(|(start_offset, end_offset)| Diagnostic::UnclosedBold {
        start_offset,
        end_offset
      }),
      arb_offsets().prop_map(|(start_offset, end_offset)| Diagnostic::UnclosedItalic {
        start_offset,
        end_offset
      }),
      arb_offsets().prop_map(
        |(start_offset, end_offset)| Diagnostic::UnclosedStrikethrough {
          start_offset,
          end_offset
        }
      ),
      arb_offsets().prop_map(
        |(start_offset, end_offset)| Diagnostic::UnclosedBoldItalic {
          start_offset,
          end_offset
        }
      ),
      arb_offsets().prop_map(
        |(start_offset, end_offset)| Diagnostic::MismatchedItalicDelimiter {
          start_offset,
          end_offset
        }
      ),
      // {expected_prefix: String, start_offset, end_offset}
      (".*", arb_offsets()).prop_map(|(expected_prefix, (start_offset, end_offset))| {
        Diagnostic::MissingExpectMdPrefix {
          expected_prefix,
          start_offset,
          end_offset,
        }
      }),
      arb_offsets().prop_map(
        |(start_offset, end_offset)| Diagnostic::MissingTableSeparatorRow {
          start_offset,
          end_offset
        }
      ),
      // {expected: usize, found: usize, start_offset, end_offset}
      (any::<usize>(), any::<usize>(), arb_offsets()).prop_map(
        |(expected, found, (start_offset, end_offset))| {
          Diagnostic::TableColumnCountMismatch {
            expected,
            found,
            start_offset,
            end_offset,
          }
        }
      ),
      (any::<usize>(), any::<usize>(), arb_offsets()).prop_map(
        |(expected_more_than, found, (start_offset, end_offset))| {
          Diagnostic::InsufficientBlockIndent {
            expected_more_than,
            found,
            start_offset,
            end_offset,
          }
        }
      ),
      // Vault config diagnostics
      ".*".prop_map(|root_dir| Diagnostic::MissingVaultConfig { root_dir }),
      (".*", ".*").prop_map(|(path, message)| Diagnostic::VaultConfigReadError { path, message }),
      (".*", ".*", arb_offsets()).prop_map(|(path, message, (start_offset, end_offset))| {
        Diagnostic::VaultConfigParseError {
          path,
          message,
          start_offset,
          end_offset,
        }
      }),
      ".*".prop_map(|path| Diagnostic::VaultConfigEmpty { path }),
      (".*", ".*", arb_offsets()).prop_map(|(path, field, (start_offset, end_offset))| {
        Diagnostic::VaultConfigMissingField {
          path,
          field,
          start_offset,
          end_offset,
        }
      }),
      (".*", ".*", arb_offsets()).prop_map(|(path, field, (start_offset, end_offset))| {
        Diagnostic::VaultConfigUnknownField {
          path,
          field,
          start_offset,
          end_offset,
        }
      }),
      arb_offsets().prop_map(
        |(start_offset, end_offset)| Diagnostic::MissingSchemaField {
          start_offset,
          end_offset
        }
      ),
      // {name: String, start_offset, end_offset}
      (".*", arb_offsets()).prop_map(|(name, (start_offset, end_offset))| {
        Diagnostic::UnresolvedSchema {
          name,
          start_offset,
          end_offset,
        }
      }),
      (any::<usize>(), any::<usize>())
        .prop_map(|(expected, got)| Diagnostic::WrongTypeArgCount { expected, got }),
      arb_offsets().prop_map(|(start_offset, end_offset)| Diagnostic::NotCallable {
        start_offset,
        end_offset
      }),
      (any::<usize>(), any::<usize>(), arb_offsets()).prop_map(
        |(expected, got, (start_offset, end_offset))| {
          Diagnostic::WrongArgCount {
            expected,
            got,
            start_offset,
            end_offset,
          }
        }
      ),
      (".*", arb_offsets()).prop_map(|(expected, (start_offset, end_offset))| {
        Diagnostic::ArgTypeMismatch {
          expected,
          start_offset,
          end_offset,
        }
      }),
      (".*", ".*", arb_offsets()).prop_map(|(field, expected, (start_offset, end_offset))| {
        Diagnostic::FieldTypeMismatch {
          field,
          expected,
          start_offset,
          end_offset,
        }
      }),
      arb_offsets().prop_map(|(start_offset, end_offset)| Diagnostic::NotIndexable {
        start_offset,
        end_offset
      }),
      (".*", arb_offsets()).prop_map(|(expected, (start_offset, end_offset))| {
        Diagnostic::IndexTypeMismatch {
          expected,
          start_offset,
          end_offset,
        }
      }),
      (".*", arb_offsets()).prop_map(|(expected, (start_offset, end_offset))| {
        Diagnostic::TagTypeMismatch {
          expected,
          start_offset,
          end_offset,
        }
      }),
      (".*", ".*", arb_offsets()).prop_map(|(op, expected, (start_offset, end_offset))| {
        Diagnostic::OperandTypeMismatch {
          op,
          expected,
          start_offset,
          end_offset,
        }
      }),
      (".*", arb_offsets()).prop_map(|(field, (start_offset, end_offset))| {
        Diagnostic::MissingRequiredField {
          field,
          start_offset,
          end_offset,
        }
      }),
      (".*", arb_offsets()).prop_map(|(expected, (start_offset, end_offset))| {
        Diagnostic::ElementTypeMismatch {
          expected,
          start_offset,
          end_offset,
        }
      }),
      (".*", arb_offsets()).prop_map(|(key, (start_offset, end_offset))| {
        Diagnostic::DuplicateKey {
          key,
          start_offset,
          end_offset,
        }
      }),
      (".*", arb_offsets()).prop_map(|(path, (start_offset, end_offset))| {
        Diagnostic::UnresolvedFileRef {
          path,
          start_offset,
          end_offset,
        }
      }),
      (".*", ".*", arb_offsets()).prop_map(|(field, on_type, (start_offset, end_offset))| {
        Diagnostic::UnknownField {
          field,
          on_type,
          start_offset,
          end_offset,
        }
      }),
      (any::<usize>(), any::<usize>(), arb_offsets()).prop_map(
        |(index, length, (start_offset, end_offset))| {
          Diagnostic::IndexOutOfBounds {
            index,
            length,
            start_offset,
            end_offset,
          }
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
