use super::helpers::*;
use crate::syntax::diagnostic::Diagnostic;
use crate::syntax::syntax_kind::SyntaxKind;

fn parse_expr(input: &str) -> String {
  let program = format!("---\nkey: {}\n---\n", input);

  let (ast, _) = parse(&program);
  let root = ast.as_node().unwrap();

  // Extract the fronmatter as we inject expressions inside the frontmatter
  let frontmatter = root.children()[0].as_node().unwrap();

  // Find the BlockMapping `key: ...`
  let mapping = frontmatter
    .children()
    .iter()
    .find(|c| c.is_node() && c.as_node().unwrap().kind() == SyntaxKind::YamlMapping)
    .expect("Expected BlockMapping in frontmatter");

  // Find the `key: ...` entry
  let entry = mapping
    .as_node()
    .unwrap()
    .children()
    .iter()
    .find(|c| c.is_node() && c.as_node().unwrap().kind() == SyntaxKind::YamlMappingEntry)
    .expect("Expected MappingEntry in BlockMapping")
    .as_node()
    .unwrap();

  // Find the last node child which should be the value expression
  let value = entry
    .children()
    .iter()
    .rev()
    .find(|c| c.is_node())
    .expect("Expected value in mapping entry");

  render_tree(value)
}

fn parse_expr_with_diagnostics(input: &str) -> (String, Vec<Diagnostic>) {
  let full = format!("---\nkey: {}\n---\n", input);
  let (ast, diagnostics) = parse(&full);
  let root = ast.as_node().unwrap();
  let frontmatter = root.children()[0].as_node().unwrap();
  let mapping = frontmatter
    .children()
    .iter()
    .find(|c| c.is_node() && c.as_node().unwrap().kind() == SyntaxKind::YamlMapping)
    .expect("Expected BlockMapping in frontmatter");
  let entry = mapping
    .as_node()
    .unwrap()
    .children()
    .iter()
    .find(|c| c.is_node() && c.as_node().unwrap().kind() == SyntaxKind::YamlMappingEntry)
    .expect("Expected MappingEntry in BlockMapping")
    .as_node()
    .unwrap();
  let value = entry
    .children()
    .iter()
    .rev()
    .find(|c| c.is_node())
    .expect("Expected value in mapping entry");
  let tree = render_tree(value);
  (tree, diagnostics)
}

#[test]
fn parse_number_literal() {
  let tree = parse_expr("1");
  let expected = r#"(YamlMappingEntryValue
  (NumberLit
    " "
    "1"))"#;
  assert_eq!(tree, expected);
}

#[test]
fn parse_decimal_literal() {
  let tree = parse_expr("3.14");
  let expected = r#"(YamlMappingEntryValue
  (NumberLit
    " "
    "3.14"))"#;
  assert_eq!(tree, expected);
}

#[test]
fn parse_double_quoted_string() {
  let tree = parse_expr(r#""hello""#);
  let expected = r#"(YamlMappingEntryValue
  (StrLit
    " "
    "\""
    "hello"
    "\""))"#;
  assert_eq!(tree, expected);
}

#[test]
fn parse_single_quoted_string() {
  let tree = parse_expr("'hello'");
  let expected = r#"(YamlMappingEntryValue
  (StrLit
    " "
    "'"
    "hello"
    "'"))"#;
  assert_eq!(tree, expected);
}

#[test]
fn parse_identifier_literal() {
  let tree = parse_expr("true");
  let expected = r#"(YamlMappingEntryValue
  (IdentLit
    " "
    "true"))"#;
  assert_eq!(tree, expected);
}

#[test]
fn parse_list_literal() {
  let tree = parse_expr("[1, 2]");
  let expected = r#"(YamlMappingEntryValue
  (ListLit
    " "
    "["
    (ListItem
      (NumberLit
        "1"))
    ","
    (ListItem
      (NumberLit
        " "
        "2"))
    "]"))"#;
  assert_eq!(tree, expected);
}

#[test]
fn parse_binary_expression() {
  let tree = parse_expr("1 + 2");
  let expected = r#"(YamlMappingEntryValue
  (BinaryExpr
    (NumberLit
      " "
      "1")
    " "
    "+"
    (NumberLit
      " "
      "2")))"#;
  assert_eq!(tree, expected);
}

#[test]
fn parse_parenthesized_expression() {
  let tree = parse_expr("(1)");
  let expected = r#"(YamlMappingEntryValue
  (ParenExpr
    " "
    "("
    (NumberLit
      "1")
    ")"))"#;
  assert_eq!(tree, expected);
}

#[test]
fn parse_subtraction() {
  let tree = parse_expr("3 - 1");
  let expected = r#"(YamlMappingEntryValue
  (BinaryExpr
    (NumberLit
      " "
      "3")
    " "
    "-"
    (NumberLit
      " "
      "1")))"#;
  assert_eq!(tree, expected);
}

#[test]
fn parse_multiplication() {
  let tree = parse_expr("2 * 3");
  let expected = r#"(YamlMappingEntryValue
  (BinaryExpr
    (NumberLit
      " "
      "2")
    " "
    "*"
    (NumberLit
      " "
      "3")))"#;
  assert_eq!(tree, expected);
}

#[test]
fn parse_division() {
  let tree = parse_expr("6 / 2");
  let expected = r#"(YamlMappingEntryValue
  (BinaryExpr
    (NumberLit
      " "
      "6")
    " "
    "/"
    (NumberLit
      " "
      "2")))"#;
  assert_eq!(tree, expected);
}

#[test]
fn parse_unary_negation() {
  let tree = parse_expr("-1");
  let expected = r#"(YamlMappingEntryValue
  (PrefixExpr
    " "
    "-"
    (NumberLit
      "1")))"#;
  assert_eq!(tree, expected);
}

#[test]
fn parse_precedence_multiply_add() {
  let tree = parse_expr("1 + 2 * 3");
  let expected = r#"(YamlMappingEntryValue
  (BinaryExpr
    (NumberLit
      " "
      "1")
    " "
    "+"
    (BinaryExpr
      (NumberLit
        " "
        "2")
      " "
      "*"
      (NumberLit
        " "
        "3"))))"#;
  assert_eq!(tree, expected);
}

#[test]
fn parse_nested_parens() {
  let tree = parse_expr("(1 + 2) * 3");
  let expected = r#"(YamlMappingEntryValue
  (BinaryExpr
    (ParenExpr
      " "
      "("
      (BinaryExpr
        (NumberLit
          "1")
        " "
        "+"
        (NumberLit
          " "
          "2"))
      ")")
    " "
    "*"
    (NumberLit
      " "
      "3")))"#;
  assert_eq!(tree, expected);
}

#[test]
fn parse_empty_list_literal() {
  let tree = parse_expr("[]");
  let expected = r#"(YamlMappingEntryValue
  (ListLit
    " "
    "["
    "]"))"#;
  assert_eq!(tree, expected);
}

#[test]
fn parse_nested_list_literal() {
  let tree = parse_expr("[[1], [2]]");
  let expected = r#"(YamlMappingEntryValue
  (ListLit
    " "
    "["
    (ListItem
      (ListLit
        "["
        (ListItem
          (NumberLit
            "1"))
        "]"))
    ","
    (ListItem
      (ListLit
        " "
        "["
        (ListItem
          (NumberLit
            "2"))
        "]"))
    "]"))"#;
  assert_eq!(tree, expected);
}

#[test]
fn parse_dictionary_literal() {
  let tree = parse_expr("{a: 1}");
  let expected = r#"(YamlMappingEntryValue
  (DictLit
    " "
    "{"
    (DictEntry
      (DictEntryKey
        "a")
      ":"
      (DictEntryValue
        (NumberLit
          " "
          "1")))
    "}"))"#;
  assert_eq!(tree, expected);
}

#[test]
fn parse_call_expression() {
  let tree = parse_expr("f(1, 2)");
  let expected = r#"(YamlMappingEntryValue
  (CallExpr
    (IdentLit
      " "
      "f")
    "("
    (NumberLit
      "1")
    ","
    (NumberLit
      " "
      "2")
    ")"))"#;
  assert_eq!(tree, expected);
}

#[test]
fn parse_complex_expression() {
  let tree = parse_expr("f(1 + 2, [3])");
  let expected = r#"(YamlMappingEntryValue
  (CallExpr
    (IdentLit
      " "
      "f")
    "("
    (BinaryExpr
      (NumberLit
        "1")
      " "
      "+"
      (NumberLit
        " "
        "2"))
    ","
    (ListLit
      " "
      "["
      (ListItem
        (NumberLit
          "3"))
      "]")
    ")"))"#;
  assert_eq!(tree, expected);
}

#[test]
fn parse_left_associative_addition() {
  let tree = parse_expr("1 + 2 + 3");
  let expected = r#"(YamlMappingEntryValue
  (BinaryExpr
    (BinaryExpr
      (NumberLit
        " "
        "1")
      " "
      "+"
      (NumberLit
        " "
        "2"))
    " "
    "+"
    (NumberLit
      " "
      "3")))"#;
  assert_eq!(tree, expected);
}

#[test]
fn parse_left_associative_subtraction() {
  let tree = parse_expr("5 - 3 - 1");
  let expected = r#"(YamlMappingEntryValue
  (BinaryExpr
    (BinaryExpr
      (NumberLit
        " "
        "5")
      " "
      "-"
      (NumberLit
        " "
        "3"))
    " "
    "-"
    (NumberLit
      " "
      "1")))"#;
  assert_eq!(tree, expected);
}

#[test]
fn parse_multiply_before_subtract() {
  let tree = parse_expr("5 - 2 * 3");
  let expected = r#"(YamlMappingEntryValue
  (BinaryExpr
    (NumberLit
      " "
      "5")
    " "
    "-"
    (BinaryExpr
      (NumberLit
        " "
        "2")
      " "
      "*"
      (NumberLit
        " "
        "3"))))"#;
  assert_eq!(tree, expected);
}

#[test]
fn parse_divide_before_add() {
  let tree = parse_expr("1 + 6 / 2");
  let expected = r#"(YamlMappingEntryValue
  (BinaryExpr
    (NumberLit
      " "
      "1")
    " "
    "+"
    (BinaryExpr
      (NumberLit
        " "
        "6")
      " "
      "/"
      (NumberLit
        " "
        "2"))))"#;
  assert_eq!(tree, expected);
}

#[test]
fn parse_unary_minus_in_binary() {
  let tree = parse_expr("-1 + 2");
  let expected = r#"(YamlMappingEntryValue
  (BinaryExpr
    (PrefixExpr
      " "
      "-"
      (NumberLit
        "1"))
    " "
    "+"
    (NumberLit
      " "
      "2")))"#;
  assert_eq!(tree, expected);
}

#[test]
fn parse_unary_minus_right_side() {
  let tree = parse_expr("1 + -2");
  let expected = r#"(YamlMappingEntryValue
  (BinaryExpr
    (NumberLit
      " "
      "1")
    " "
    "+"
    (PrefixExpr
      " "
      "-"
      (NumberLit
        "2"))))"#;
  assert_eq!(tree, expected);
}

#[test]
fn parse_comparison() {
  let tree = parse_expr("1 == 2");
  let expected = r#"(YamlMappingEntryValue
  (BinaryExpr
    (NumberLit
      " "
      "1")
    " "
    "=="
    (NumberLit
      " "
      "2")))"#;
  assert_eq!(tree, expected);
}

#[test]
fn parse_logical_and() {
  let tree = parse_expr("true && false");
  let expected = r#"(YamlMappingEntryValue
  (BinaryExpr
    (IdentLit
      " "
      "true")
    " "
    "&&"
    (IdentLit
      " "
      "false")))"#;
  assert_eq!(tree, expected);
}

#[test]
fn parse_precedence() {
  let tree = parse_expr("1 + 2 == 3");
  let expected = r#"(YamlMappingEntryValue
  (BinaryExpr
    (BinaryExpr
      (NumberLit
        " "
        "1")
      " "
      "+"
      (NumberLit
        " "
        "2"))
    " "
    "=="
    (NumberLit
      " "
      "3")))"#;
  assert_eq!(tree, expected);
}

#[test]
fn error_missing_operand() {
  let (tree, diags) = parse_expr_with_diagnostics("1 +");
  let expected = r#"(YamlMappingEntryValue
  (BinaryExpr
    (NumberLit
      " "
      "1")
    " "
    "+"
    (PrimaryExpr)))"#;
  assert_eq!(tree, expected);
  assert!(diags.iter().any(|d| matches!(
    d,
    Diagnostic::MissingSyntaxNode {
      expected: SyntaxKind::PrimaryExpr,
      ..
    }
  )));
}

#[test]
fn error_unclosed_paren() {
  let (tree, diags) = parse_expr_with_diagnostics("(1");
  let expected = r#"(YamlMappingEntryValue
  (ParenExpr
    " "
    "("
    (NumberLit
      "1")
    "\n"
    ""
    (Error
      "---")))"#;
  assert_eq!(tree, expected);
  assert!(diags.iter().any(|d| matches!(
    d,
    Diagnostic::MissingSyntaxNode {
      expected: SyntaxKind::RParen,
      ..
    }
  )));
}

#[test]
fn error_unclosed_list() {
  let (tree, _diags) = parse_expr_with_diagnostics("[1, 2");
  let expected = r#"(YamlMappingEntryValue
  (ListLit
    " "
    "["
    (ListItem
      (NumberLit
        "1"))
    ","
    (ListItem
      (NumberLit
        " "
        "2"))))"#;
  assert_eq!(tree, expected);
}

#[test]
fn error_unclosed_dict() {
  let (tree, _diags) = parse_expr_with_diagnostics("{a: 1");
  let expected = r#"(YamlMappingEntryValue
  (DictLit
    " "
    "{"
    (DictEntry
      (DictEntryKey
        "a")
      ":"
      (DictEntryValue
        (NumberLit
          " "
          "1")))))"#;
  assert_eq!(tree, expected);
}

#[test]
fn error_unclosed_string() {
  let (tree, diags) = parse_expr_with_diagnostics(r#""hello"#);
  let expected = r#"(YamlMappingEntryValue
  (StrLit
    " "
    "\""
    "hello"))"#;
  assert_eq!(tree, expected);
  assert!(
    diags
      .iter()
      .any(|d| matches!(d, Diagnostic::UnterminatedString { .. }))
  );
}

#[test]
fn error_missing_value_in_mapping() {
  let (tree, diags) = parse_expr_with_diagnostics("{a:}");
  let expected = r#"(YamlMappingEntryValue
  (DictLit
    " "
    "{"
    (DictEntry
      (DictEntryKey
        "a")
      ":"
      (DictEntryValue))
    "}"))"#;
  assert_eq!(tree, expected);
  assert!(diags.iter().any(|d| matches!(
    d,
    Diagnostic::MissingSyntaxNode {
      expected: SyntaxKind::DictEntryValue,
      ..
    }
  )));
}

#[test]
fn error_extra_comma_in_list() {
  let (tree, diags) = parse_expr_with_diagnostics("[1,,2]");
  let expected = r#"(YamlMappingEntryValue
  (ListLit
    " "
    "["
    (ListItem
      (NumberLit
        "1"))
    ","
    (ListItem
      (PrimaryExpr))
    ","
    (ListItem
      (NumberLit
        "2"))
    "]"))"#;
  assert_eq!(tree, expected);
  assert!(diags.iter().any(|d| matches!(
    d,
    Diagnostic::MissingSyntaxNode {
      expected: SyntaxKind::PrimaryExpr,
      ..
    }
  )));
}

#[test]
fn error_empty_expression() {
  let (tree, diags) = parse_expr_with_diagnostics("");
  let expected = r#"(YamlMappingEntryValue)"#;
  assert_eq!(tree, expected);
  assert!(diags.iter().any(|d| matches!(
    d,
    Diagnostic::MissingSyntaxNode {
      expected: SyntaxKind::YamlMappingEntryValue,
      ..
    }
  )));
}

#[test]
fn parse_index_single() {
  let tree = parse_expr("x[0]");
  assert_eq!(
    tree,
    r#"(YamlMappingEntryValue
  (IndexExpr
    (IdentLit
      " "
      "x")
    "["
    (NumberLit
      "0")
    "]"))"#
  );
}

#[test]
fn parse_index_multiple() {
  let tree = parse_expr("x[0, 1]");
  assert_eq!(
    tree,
    r#"(YamlMappingEntryValue
  (IndexExpr
    (IdentLit
      " "
      "x")
    "["
    (NumberLit
      "0")
    ","
    (NumberLit
      " "
      "1")
    "]"))"#
  );
}

#[test]
fn parse_index_with_expression() {
  let tree = parse_expr("x[a + 1]");
  assert_eq!(
    tree,
    r#"(YamlMappingEntryValue
  (IndexExpr
    (IdentLit
      " "
      "x")
    "["
    (BinaryExpr
      (IdentLit
        "a")
      " "
      "+"
      (NumberLit
        " "
        "1"))
    "]"))"#
  );
}

#[test]
fn parse_index_chained() {
  let tree = parse_expr("x[0][1]");
  assert_eq!(
    tree,
    r#"(YamlMappingEntryValue
  (IndexExpr
    (IndexExpr
      (IdentLit
        " "
        "x")
      "["
      (NumberLit
        "0")
      "]")
    "["
    (NumberLit
      "1")
    "]"))"#
  );
}

#[test]
fn parse_index_on_call() {
  let tree = parse_expr("f(x)[0]");
  assert_eq!(
    tree,
    r#"(YamlMappingEntryValue
  (IndexExpr
    (CallExpr
      (IdentLit
        " "
        "f")
      "("
      (IdentLit
        "x")
      ")")
    "["
    (NumberLit
      "0")
    "]"))"#
  );
}
