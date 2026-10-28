use std::cell::RefCell;
use std::rc::Rc;

use typedown_types::file_stream::FileStream;
use typedown_types::string_stream::StringStream;

use crate::syntax::diagnostic::*;
use crate::syntax::green::cache::Cache;
use crate::syntax::lex::ctx::{LexCtx, LexMode};
use crate::syntax::syntax_kind::SyntaxKind;

fn lex_yaml(input: &str) -> Vec<(SyntaxKind, String)> {
  let stream = StringStream::new(input);
  let cache = Rc::new(RefCell::new(Cache::new()));
  let mut lexer = LexCtx::new(stream, cache);
  let mut tokens = vec![];
  loop {
    let result = lexer.lex();
    let kind = result.token.kind();
    let text: String = result.token.chars().collect();
    tokens.push((kind, text));
    if kind == SyntaxKind::Eof {
      break;
    }
  }
  assert!(tokens.len() >= 2, "should have at least YamlIndent + Eof");
  assert_eq!(
    tokens.last().unwrap().0,
    SyntaxKind::Eof,
    "last token should be Eof"
  );
  tokens
}

fn lex_markdown(input: &str) -> Vec<(SyntaxKind, String)> {
  let (tokens, diagnostics) = lex_markdown_with_diagnostics(input);
  assert!(
    diagnostics.is_empty(),
    "valid markdown should produce zero lex diagnostics, got: {diagnostics:?}"
  );
  tokens
}

fn lex_markdown_with_diagnostics(input: &str) -> (Vec<(SyntaxKind, String)>, Vec<TdDiagnostic>) {
  let stream = StringStream::new(input);
  let cache = Rc::new(RefCell::new(Cache::new()));
  let mut lexer = LexCtx::new(stream, cache);
  lexer.set_mode(LexMode::MarkdownBody);
  let mut tokens = vec![];
  let mut diags = vec![];
  loop {
    let result = lexer.lex();
    let kind = result.token.kind();
    let text: String = result.token.chars().collect();
    if let Some(diag) = result.diagnostic {
      diags.push(diag);
    }
    tokens.push((kind, text));
    if kind == SyntaxKind::Eof {
      break;
    }
  }
  (tokens, diags)
}

/// Assert that the token sequence matches exactly (kind + text), including Eof.
fn assert_tokens(actual: &[(SyntaxKind, String)], expected: &[(SyntaxKind, &str)]) {
  assert_eq!(
    actual.len(),
    expected.len(),
    "token count mismatch: expected {} tokens, got {}.\nExpected: {:?}\nActual:   {:?}",
    expected.len(),
    actual.len(),
    expected,
    actual,
  );
  for (index, (actual_token, expected_token)) in actual.iter().zip(expected.iter()).enumerate() {
    assert_eq!(
      actual_token.0, expected_token.0,
      "token {index} kind mismatch: expected {:?}, got {:?}",
      expected_token.0, actual_token.0,
    );
    assert_eq!(
      actual_token.1, expected_token.1,
      "token {index} text mismatch: expected {:?}, got {:?}",
      expected_token.1, actual_token.1,
    );
  }
}

/* YAML mode tests */

#[test]
fn yaml_triple_dash() {
  let tokens = lex_yaml("---");
  assert_tokens(
    &tokens,
    &[
      (SyntaxKind::YamlIndent, ""),
      (SyntaxKind::YamlOp, "---"),
      (SyntaxKind::Eof, ""),
    ],
  );
}

#[test]
fn yaml_colon() {
  let tokens = lex_yaml(":");
  assert_tokens(
    &tokens,
    &[
      (SyntaxKind::YamlIndent, ""),
      (SyntaxKind::Colon, ":"),
      (SyntaxKind::Eof, ""),
    ],
  );
}

#[test]
fn yaml_ident() {
  let tokens = lex_yaml("hello_world");
  assert_tokens(
    &tokens,
    &[
      (SyntaxKind::YamlIndent, ""),
      (SyntaxKind::Ident, "hello_world"),
      (SyntaxKind::Eof, ""),
    ],
  );
}

#[test]
fn yaml_number_integer() {
  let tokens = lex_yaml("42");
  assert_tokens(
    &tokens,
    &[
      (SyntaxKind::YamlIndent, ""),
      (SyntaxKind::Number, "42"),
      (SyntaxKind::Eof, ""),
    ],
  );
}

#[test]
fn yaml_number_decimal() {
  let tokens = lex_yaml("3.14");
  assert_tokens(
    &tokens,
    &[
      (SyntaxKind::YamlIndent, ""),
      (SyntaxKind::Number, "3.14"),
      (SyntaxKind::Eof, ""),
    ],
  );
}

#[test]
fn yaml_number_scientific() {
  let tokens = lex_yaml("2.5e10");
  assert_tokens(
    &tokens,
    &[
      (SyntaxKind::YamlIndent, ""),
      (SyntaxKind::Number, "2.5e10"),
      (SyntaxKind::Eof, ""),
    ],
  );
}

#[test]
fn yaml_number_trailing_dot() {
  let tokens = lex_yaml("1.");
  assert_tokens(
    &tokens,
    &[
      (SyntaxKind::YamlIndent, ""),
      (SyntaxKind::Number, "1."),
      (SyntaxKind::Eof, ""),
    ],
  );
}

#[test]
fn yaml_number_missing_exponent_digits() {
  let tokens = lex_yaml("2.5E+");
  assert_eq!(tokens[0], (SyntaxKind::YamlIndent, "".to_string()));
  assert_eq!(tokens[1].0, SyntaxKind::Error);
  assert_eq!(
    tokens.last().unwrap().0,
    SyntaxKind::Eof,
    "should end with Eof"
  );
}

#[test]
fn yaml_dq_string() {
  let tokens = lex_yaml("\"hello\"");
  assert_tokens(
    &tokens,
    &[
      (SyntaxKind::YamlIndent, ""),
      (SyntaxKind::DqStrStart, "\""),
      (SyntaxKind::DqStrContent, "hello"),
      (SyntaxKind::DqStrEnd, "\""),
      (SyntaxKind::Eof, ""),
    ],
  );
}

#[test]
fn yaml_sq_string() {
  let tokens = lex_yaml("'hello'");
  assert_tokens(
    &tokens,
    &[
      (SyntaxKind::YamlIndent, ""),
      (SyntaxKind::SqStrStart, "'"),
      (SyntaxKind::SqStrContent, "hello"),
      (SyntaxKind::SqStrEnd, "'"),
      (SyntaxKind::Eof, ""),
    ],
  );
}

#[test]
fn yaml_empty_string() {
  let tokens = lex_yaml("\"\"");
  assert_tokens(
    &tokens,
    &[
      (SyntaxKind::YamlIndent, ""),
      (SyntaxKind::DqStrStart, "\""),
      (SyntaxKind::DqStrEnd, "\""),
      (SyntaxKind::Eof, ""),
    ],
  );
}

#[test]
fn yaml_string_with_escape() {
  let tokens = lex_yaml("\"he\\\"llo\"");
  assert_tokens(
    &tokens,
    &[
      (SyntaxKind::YamlIndent, ""),
      (SyntaxKind::DqStrStart, "\""),
      (SyntaxKind::DqStrContent, "he\\\"llo"),
      (SyntaxKind::DqStrEnd, "\""),
      (SyntaxKind::Eof, ""),
    ],
  );
}

#[test]
fn yaml_unterminated_string() {
  let tokens = lex_yaml(
    r#""hello
"#,
  );
  assert_eq!(tokens[0], (SyntaxKind::YamlIndent, "".to_string()));
  assert_eq!(tokens[1], (SyntaxKind::DqStrStart, "\"".to_string()));
  assert_eq!(tokens[2].0, SyntaxKind::Error);
  assert_eq!(
    tokens.last().unwrap().0,
    SyntaxKind::Eof,
    "should end with Eof"
  );
}

#[test]
fn yaml_comment() {
  let tokens = lex_yaml("# this is a comment");
  assert_tokens(
    &tokens,
    &[
      (SyntaxKind::YamlIndent, ""),
      (SyntaxKind::YamlComment, "# this is a comment"),
      (SyntaxKind::Eof, ""),
    ],
  );
}

#[test]
fn yaml_indent() {
  // Leading space on a non-empty line emits YamlIndent
  let tokens = lex_yaml(" a");
  assert_eq!(tokens[0].0, SyntaxKind::YamlIndent);
  assert_eq!(tokens[1], (SyntaxKind::Ident, "a".to_string()));
}

#[test]
fn yaml_empty_line_no_indent() {
  // Leading space on an empty line (before EOF) emits Whitespace, not YamlIndent
  let tokens = lex_yaml(" ");
  assert_eq!(tokens[0], (SyntaxKind::Whitespace, " ".to_string()));
}

#[test]
fn yaml_whitespace_after_token() {
  // Whitespace after a token emits Whitespace
  let tokens = lex_yaml("a b");
  assert_eq!(tokens[0], (SyntaxKind::YamlIndent, "".to_string()));
  assert_eq!(tokens[1], (SyntaxKind::Ident, "a".to_string()));
  assert_eq!(tokens[2], (SyntaxKind::Whitespace, " ".to_string()));
  assert_eq!(tokens[3], (SyntaxKind::Ident, "b".to_string()));
}

#[test]
fn yaml_newline() {
  let tokens = lex_yaml("\n");
  assert_eq!(tokens[0], (SyntaxKind::Newline, "\n".to_string()));
}

#[test]
fn yaml_crlf() {
  let tokens = lex_yaml("\r\n");
  assert_eq!(tokens[0], (SyntaxKind::Newline, "\r\n".to_string()));
}

#[test]
fn yaml_bang_op() {
  // ! immediately followed by alpha is a tag, emitted as a single YamlOp
  let tokens = lex_yaml("!string");
  assert_eq!(tokens[0], (SyntaxKind::YamlIndent, "".to_string()));
  assert_eq!(tokens[1], (SyntaxKind::YamlOp, "!string".to_string()));
}

#[test]
fn yaml_bang_equals_op() {
  let tokens = lex_yaml("!=");
  assert_eq!(tokens[0], (SyntaxKind::YamlIndent, "".to_string()));
  assert_eq!(tokens[1], (SyntaxKind::YamlOp, "!=".to_string()));
}

#[test]
fn yaml_arrow_op() {
  let tokens = lex_yaml("->");
  assert_eq!(tokens[0], (SyntaxKind::YamlIndent, "".to_string()));
  assert_eq!(tokens[1], (SyntaxKind::YamlOp, "->".to_string()));
}

#[test]
fn yaml_question_mark_op() {
  let tokens = lex_yaml("string?");
  assert_eq!(tokens[0], (SyntaxKind::YamlIndent, "".to_string()));
  assert_eq!(tokens[1], (SyntaxKind::Ident, "string".to_string()));
  assert_eq!(tokens[2], (SyntaxKind::YamlOp, "?".to_string()));
}

#[test]
fn yaml_brackets() {
  let tokens = lex_yaml("[]{}(),");
  assert_eq!(tokens[1], (SyntaxKind::LBracket, "[".to_string()));
  assert_eq!(tokens[2], (SyntaxKind::RBracket, "]".to_string()));
  assert_eq!(tokens[3], (SyntaxKind::LBrace, "{".to_string()));
  assert_eq!(tokens[4], (SyntaxKind::RBrace, "}".to_string()));
  assert_eq!(tokens[5], (SyntaxKind::LParen, "(".to_string()));
  assert_eq!(tokens[6], (SyntaxKind::RParen, ")".to_string()));
  assert_eq!(tokens[7], (SyntaxKind::Comma, ",".to_string()));
}

#[test]
fn yaml_interp_in_string() {
  let tokens = lex_yaml("\"hello ${name}\"");
  assert_eq!(tokens[0], (SyntaxKind::YamlIndent, "".to_string()));
  assert_eq!(tokens[1], (SyntaxKind::DqStrStart, "\"".to_string()));
  assert_eq!(tokens[2], (SyntaxKind::DqStrContent, "hello ".to_string()));
  assert_eq!(tokens[3], (SyntaxKind::InterpStart, "${".to_string()));
  assert_eq!(tokens[4], (SyntaxKind::Ident, "name".to_string()));
  assert_eq!(tokens[5], (SyntaxKind::InterpEnd, "}".to_string()));
  assert_eq!(tokens[6], (SyntaxKind::DqStrEnd, "\"".to_string()));
}

#[test]
fn yaml_inline_math_in_dq_string() {
  let tokens = lex_yaml("\"$E = mc^2$\"");
  assert_eq!(tokens[0], (SyntaxKind::YamlIndent, "".to_string()));
  assert_eq!(tokens[1], (SyntaxKind::DqStrStart, "\"".to_string()));
  assert_eq!(
    tokens[2],
    (SyntaxKind::InlineMath, "$E = mc^2$".to_string())
  );
  assert_eq!(tokens[3], (SyntaxKind::DqStrEnd, "\"".to_string()));
}

#[test]
fn yaml_inline_math_in_sq_string() {
  let tokens = lex_yaml("'$E = mc^2$'");
  assert_eq!(tokens[0], (SyntaxKind::YamlIndent, "".to_string()));
  assert_eq!(tokens[1], (SyntaxKind::SqStrStart, "'".to_string()));
  assert_eq!(
    tokens[2],
    (SyntaxKind::InlineMath, "$E = mc^2$".to_string())
  );
  assert_eq!(tokens[3], (SyntaxKind::SqStrEnd, "'".to_string()));
}

#[test]
fn yaml_inline_math_with_text_prefix() {
  let tokens = lex_yaml("\"value: $x^2$\"");
  assert_eq!(tokens[0], (SyntaxKind::YamlIndent, "".to_string()));
  assert_eq!(tokens[1], (SyntaxKind::DqStrStart, "\"".to_string()));
  assert_eq!(tokens[2], (SyntaxKind::DqStrContent, "value: ".to_string()));
  assert_eq!(tokens[3], (SyntaxKind::InlineMath, "$x^2$".to_string()));
  assert_eq!(tokens[4], (SyntaxKind::DqStrEnd, "\"".to_string()));
}

#[test]
fn yaml_mapping_line() {
  let tokens = lex_yaml("key: \"value\"");
  assert_eq!(tokens[0], (SyntaxKind::YamlIndent, "".to_string()));
  assert_eq!(tokens[1], (SyntaxKind::Ident, "key".to_string()));
  assert_eq!(tokens[2], (SyntaxKind::Colon, ":".to_string()));
  assert_eq!(tokens[3], (SyntaxKind::Whitespace, " ".to_string()));
  assert_eq!(tokens[4], (SyntaxKind::DqStrStart, "\"".to_string()));
  assert_eq!(tokens[5], (SyntaxKind::DqStrContent, "value".to_string()));
  assert_eq!(tokens[6], (SyntaxKind::DqStrEnd, "\"".to_string()));
}

/* Markdown mode tests */

#[test]
fn md_text() {
  let tokens = lex_markdown("hello");
  assert_eq!(tokens[0], (SyntaxKind::Ident, "hello".to_string()));
}

#[test]
fn md_symbol_hash() {
  let tokens = lex_markdown("##");
  assert_eq!(tokens[0], (SyntaxKind::MdSymbol, "##".to_string()));
}

#[test]
fn md_symbol_bold() {
  let tokens = lex_markdown("**");
  assert_eq!(tokens[0], (SyntaxKind::MdSymbol, "**".to_string()));
}

#[test]
fn md_brackets() {
  let tokens = lex_markdown("[link](url)");
  assert_eq!(tokens[0], (SyntaxKind::LBracket, "[".to_string()));
  assert_eq!(tokens[1], (SyntaxKind::Ident, "link".to_string()));
  assert_eq!(tokens[2], (SyntaxKind::RBracket, "]".to_string()));
  assert_eq!(tokens[3], (SyntaxKind::LParen, "(".to_string()));
  assert_eq!(tokens[4], (SyntaxKind::Ident, "url".to_string()));
  assert_eq!(tokens[5], (SyntaxKind::RParen, ")".to_string()));
}

#[test]
fn md_inline_math() {
  let tokens = lex_markdown("$E = mc^2$");
  assert_eq!(
    tokens[0],
    (SyntaxKind::InlineMath, "$E = mc^2$".to_string())
  );
}

#[test]
fn md_formula() {
  let tokens = lex_markdown("${name}");
  assert_eq!(tokens[0], (SyntaxKind::InterpStart, "${".to_string()));
  assert_eq!(tokens[1], (SyntaxKind::Ident, "name".to_string()));
  assert_eq!(tokens[2], (SyntaxKind::InterpEnd, "}".to_string()));
}

#[test]
fn md_inline_code() {
  let tokens = lex_markdown("`code`");
  assert_eq!(tokens[0], (SyntaxKind::InlineCode, "`code`".to_string()));
}

#[test]
fn md_code_block() {
  let tokens = lex_markdown(
    "```
code
```",
  );
  assert_eq!(
    tokens[0],
    (
      SyntaxKind::CodeBlock,
      "```
code
```"
        .to_string()
    )
  );
}

#[test]
fn md_newline() {
  let tokens = lex_markdown("\n");
  assert_eq!(tokens[0], (SyntaxKind::Newline, "\n".to_string()));
}

#[test]
fn md_whitespace() {
  let tokens = lex_markdown(" ");
  assert_eq!(tokens[0], (SyntaxKind::Whitespace, " ".to_string()));
}

#[test]
fn md_number() {
  let tokens = lex_markdown("42");
  assert_eq!(tokens.len(), 2); // Number + Eof
  assert_eq!(tokens[0], (SyntaxKind::MdNumber, "42".to_string()));
}

#[test]
fn md_math_block() {
  let tokens = lex_markdown(
    "$$
x + y
$$",
  );
  assert_eq!(tokens.len(), 2); // MathBlock + Eof
  assert_eq!(
    tokens[0],
    (
      SyntaxKind::MathBlock,
      r#"$$
x + y
$$"#
        .to_string()
    )
  );
}

#[test]
fn md_math_block_triple_dollar() {
  let tokens = lex_markdown(
    "$$$
x + y
$$$",
  );
  assert_eq!(tokens.len(), 2);
  assert_eq!(
    tokens[0],
    (
      SyntaxKind::MathBlock,
      r#"$$$
x + y
$$$"#
        .to_string()
    )
  );
}

#[test]
fn md_inline_math_double_dollar() {
  // $$ without newline is inline math
  let tokens = lex_markdown("$$x + y$$");
  assert_eq!(tokens.len(), 2);
  assert_eq!(tokens[0], (SyntaxKind::InlineMath, "$$x + y$$".to_string()));
}

#[test]
fn md_dollar_inside_double_dollar() {
  // $ inside $$ is literal
  let tokens = lex_markdown("$$x = $100$$");
  assert_eq!(tokens.len(), 2);
  assert_eq!(
    tokens[0],
    (SyntaxKind::InlineMath, "$$x = $100$$".to_string())
  );
}

#[test]
fn md_unterminated_math() {
  let (tokens, diagnostics) = lex_markdown_with_diagnostics("$hello");
  assert_eq!(tokens[0].0, SyntaxKind::Error);
  assert_eq!(
    tokens.last().unwrap().0,
    SyntaxKind::Eof,
    "should end with Eof"
  );
  assert!(
    !diagnostics.is_empty(),
    "unterminated math should produce diagnostics"
  );
}

/* HTML entity tests */

#[test]
fn md_html_entity_named() {
  let tokens = lex_markdown("&amp;");
  assert_eq!(tokens[0], (SyntaxKind::MdHtmlEntity, "&amp;".to_string()));
}

#[test]
fn md_html_entity_numeric_decimal() {
  let tokens = lex_markdown("&#42;");
  assert_eq!(tokens[0], (SyntaxKind::MdHtmlEntity, "&#42;".to_string()));
}

#[test]
fn md_html_entity_numeric_hex() {
  let tokens = lex_markdown("&#x2A;");
  assert_eq!(tokens[0], (SyntaxKind::MdHtmlEntity, "&#x2A;".to_string()));
}

#[test]
fn md_html_entity_numeric_hex_lowercase() {
  let tokens = lex_markdown("&#x2a;");
  assert_eq!(tokens[0], (SyntaxKind::MdHtmlEntity, "&#x2a;".to_string()));
}

#[test]
fn md_html_entity_invalid_no_semicolon() {
  // Missing semicolon: `&` falls back to MdSymbol, rest is plain tokens
  let tokens = lex_markdown("&amp");
  assert_eq!(tokens[0], (SyntaxKind::MdSymbol, "&".to_string()));
  assert_eq!(tokens[1], (SyntaxKind::Ident, "amp".to_string()));
}

#[test]
fn md_html_entity_bare_ampersand() {
  let tokens = lex_markdown("&");
  assert_eq!(tokens[0], (SyntaxKind::MdSymbol, "&".to_string()));
}

#[test]
fn md_html_entity_with_space() {
  // Space in entity name: `&` is MdSymbol, `abc` is Ident, space is Whitespace, rest is separate
  let tokens = lex_markdown("&abc aa;");
  assert_eq!(tokens[0], (SyntaxKind::MdSymbol, "&".to_string()));
  assert_eq!(tokens[1], (SyntaxKind::Ident, "abc".to_string()));
}

/* Markdown string literal tests */

#[test]
fn md_dq_string() {
  let tokens = lex_markdown("\"hello world\"");
  assert_eq!(tokens[0], (SyntaxKind::DqStrStart, "\"".to_string()));
  assert_eq!(tokens[1], (SyntaxKind::Ident, "hello".to_string()));
  assert_eq!(tokens[2], (SyntaxKind::Whitespace, " ".to_string()));
  assert_eq!(tokens[3], (SyntaxKind::Ident, "world".to_string()));
  assert_eq!(tokens[4], (SyntaxKind::DqStrEnd, "\"".to_string()));
}

#[test]
fn md_sq_string() {
  let tokens = lex_markdown("'hello world'");
  assert_eq!(tokens[0], (SyntaxKind::SqStrStart, "'".to_string()));
  assert_eq!(tokens[1], (SyntaxKind::Ident, "hello".to_string()));
  assert_eq!(tokens[2], (SyntaxKind::Whitespace, " ".to_string()));
  assert_eq!(tokens[3], (SyntaxKind::Ident, "world".to_string()));
  assert_eq!(tokens[4], (SyntaxKind::SqStrEnd, "'".to_string()));
}

#[test]
fn md_dq_string_with_symbols() {
  // Symbols inside a markdown string lex as normal markdown tokens
  let tokens = lex_markdown("\"**bold**\"");
  assert_eq!(tokens[0], (SyntaxKind::DqStrStart, "\"".to_string()));
  assert_eq!(tokens[1], (SyntaxKind::MdSymbol, "**".to_string()));
  assert_eq!(tokens[2], (SyntaxKind::Ident, "bold".to_string()));
  assert_eq!(tokens[3], (SyntaxKind::MdSymbol, "**".to_string()));
  assert_eq!(tokens[4], (SyntaxKind::DqStrEnd, "\"".to_string()));
}

#[test]
fn md_dq_string_with_interpolation() {
  let tokens = lex_markdown("\"hello ${name}\"");
  assert_eq!(tokens[0], (SyntaxKind::DqStrStart, "\"".to_string()));
  assert_eq!(tokens[1], (SyntaxKind::Ident, "hello".to_string()));
  assert_eq!(tokens[2], (SyntaxKind::Whitespace, " ".to_string()));
  assert_eq!(tokens[3], (SyntaxKind::InterpStart, "${".to_string()));
  assert_eq!(tokens[4], (SyntaxKind::Ident, "name".to_string()));
  assert_eq!(tokens[5], (SyntaxKind::InterpEnd, "}".to_string()));
  assert_eq!(tokens[6], (SyntaxKind::DqStrEnd, "\"".to_string()));
}

#[test]
fn md_empty_dq_string() {
  let tokens = lex_markdown("\"\"");
  assert_eq!(tokens[0], (SyntaxKind::DqStrStart, "\"".to_string()));
  assert_eq!(tokens[1], (SyntaxKind::DqStrEnd, "\"".to_string()));
}

#[test]
fn md_code_block_with_line_ranges() {
  let tokens = lex_markdown(
    r#"```js{1,3,5-8}
code
```"#,
  );
  assert_eq!(
    tokens[0],
    (
      SyntaxKind::CodeBlock,
      r#"```js{1,3,5-8}
code
```"#
        .to_string()
    )
  );
}

#[test]
fn md_code_block_with_ranges_no_diagnostics() {
  let (_, diagnostics) = lex_markdown_with_diagnostics(
    r#"```js{1,3}
code
```"#,
  );
  assert!(
    diagnostics.is_empty(),
    "valid range should have no diagnostics: {diagnostics:?}"
  );
}

#[test]
fn md_code_block_title_prop_accepted() {
  let (tokens, diagnostics) = lex_markdown_with_diagnostics(
    "```js{title=\"foo.ts\"}
code
```",
  );
  assert_eq!(tokens[0].0, SyntaxKind::CodeBlock);
  assert!(
    diagnostics.is_empty(),
    "title prop should not produce diagnostics: {diagnostics:?}",
  );
}

#[test]
fn md_code_block_line_numbers_flag_accepted() {
  let (tokens, diagnostics) = lex_markdown_with_diagnostics(
    "```js{line-numbers}
code
```",
  );
  assert_eq!(tokens[0].0, SyntaxKind::CodeBlock);
  assert!(
    diagnostics.is_empty(),
    "line-numbers flag should not produce diagnostics: {diagnostics:?}",
  );
}

#[test]
fn md_code_block_mixed_ranges_and_props() {
  let (tokens, diagnostics) = lex_markdown_with_diagnostics(
    "```js{1,3 title=\"foo.ts\" line-numbers}
code
```",
  );
  assert_eq!(tokens[0].0, SyntaxKind::CodeBlock);
  assert!(
    diagnostics.is_empty(),
    "mixed ranges and props should not produce diagnostics: {diagnostics:?}",
  );
}

#[test]
fn md_code_block_title_unquoted_accepted() {
  let (tokens, diagnostics) = lex_markdown_with_diagnostics(
    "```js{title=foo.ts}
code
```",
  );
  assert_eq!(tokens[0].0, SyntaxKind::CodeBlock);
  assert!(
    diagnostics.is_empty(),
    "unquoted title should not produce diagnostics: {diagnostics:?}",
  );
}

#[test]
fn md_code_block_unknown_prop_rejected() {
  let (tokens, diagnostics) = lex_markdown_with_diagnostics(
    "```js{unknown=value}
code
```",
  );
  assert_eq!(tokens[0].0, SyntaxKind::CodeBlock);
  assert!(
    diagnostics.iter().any(|diagnostic| matches!(
      diagnostic,
      TdDiagnostic {
        kind: TdDiagnosticKind::InvalidCodeRangeIndicator,
        ..
      }
    )),
    "unknown prop should produce diagnostic: {diagnostics:?}",
  );
}

#[test]
fn md_code_block_unclosed_range() {
  let (tokens, diagnostics) = lex_markdown_with_diagnostics(
    "```js{1,3
code
```",
  );
  assert_eq!(tokens[0].0, SyntaxKind::CodeBlock);
  assert!(
    diagnostics.iter().any(|diagnostic| matches!(
      diagnostic,
      TdDiagnostic {
        kind: TdDiagnosticKind::InvalidCodeRangeIndicator,
        ..
      }
    )),
    "should have InvalidCodeRangeIndicator: {diagnostics:?}",
  );
}

#[test]
fn file_stream_lookahead_eof() {
  let input = "# Heading
Some content";
  let stream = FileStream::new(input.as_bytes());
  let cache = Rc::new(RefCell::new(Cache::new()));
  let mut lexer = LexCtx::new(stream, cache);
  assert!(!lexer.has_frontmatter_start());
  lexer.set_mode(LexMode::MarkdownBody);
  let mut tokens = vec![];
  loop {
    let result = lexer.lex();
    let kind = result.token.kind();
    tokens.push(kind);
    if kind == SyntaxKind::Eof {
      break;
    }
  }
  assert_eq!(tokens.last(), Some(&SyntaxKind::Eof));
}

// Intraword underscore: a_b is text, not italic delimiter

#[test]
fn intraword_underscore_single() {
  let tokens = lex_markdown("a_b");
  assert_eq!(tokens[0], (SyntaxKind::Ident, "a_b".to_string()));
}

#[test]
fn intraword_underscore_multiple() {
  let tokens = lex_markdown("a_b_c");
  assert_eq!(tokens[0], (SyntaxKind::Ident, "a_b_c".to_string()));
}

#[test]
fn underscore_italic_not_intraword() {
  let tokens = lex_markdown("_italic_");
  assert_eq!(tokens[0], (SyntaxKind::MdSymbol, "_".to_string()));
  assert_eq!(tokens[1], (SyntaxKind::Ident, "italic".to_string()));
  assert_eq!(tokens[2], (SyntaxKind::MdSymbol, "_".to_string()));
}

#[test]
fn underscore_at_word_end_is_symbol() {
  // word_ should leave _ as a symbol (potential closing italic)
  let tokens = lex_markdown("word_ ");
  assert_eq!(tokens[0], (SyntaxKind::Ident, "word".to_string()));
  assert_eq!(tokens[1], (SyntaxKind::MdSymbol, "_".to_string()));
}

#[test]
fn underscore_start_of_word_is_symbol() {
  let tokens = lex_markdown(" _word");
  assert_eq!(tokens[0], (SyntaxKind::Whitespace, " ".to_string()));
  assert_eq!(tokens[1], (SyntaxKind::MdSymbol, "_".to_string()));
  assert_eq!(tokens[2], (SyntaxKind::Ident, "word".to_string()));
}
