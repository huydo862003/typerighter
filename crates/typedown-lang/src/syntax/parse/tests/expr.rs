use super::helpers::*;
use crate::syntax::ast::{AstNode, ClosureExpr, SourceFile};
use crate::syntax::diagnostic::Diagnostic;
use crate::syntax::red::RedNode;
use crate::syntax::syntax_kind::SyntaxKind;
use typedown_types::either::Either;

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
fn parse_postfix_question() {
  let tree = parse_expr("string?");
  let expected = r#"(YamlMappingEntryValue
  (PostfixExpr
    (IdentLit
      " "
      "string")
    "?"))"#;
  assert_eq!(tree, expected);
}

#[test]
fn parse_postfix_question_on_member_access() {
  let tree = parse_expr("a.b?");
  let expected = r#"(YamlMappingEntryValue
  (PostfixExpr
    (BinaryExpr
      (IdentLit
        " "
        "a")
      "."
      (IdentLit
        "b"))
    "?"))"#;
  assert_eq!(tree, expected);
}

#[test]
fn parse_postfix_question_on_index() {
  let tree = parse_expr("list[string]?");
  let expected = r#"(YamlMappingEntryValue
  (PostfixExpr
    (IndexExpr
      (IdentLit
        " "
        "list")
      "["
      (IdentLit
        "string")
      "]")
    "?"))"#;
  assert_eq!(tree, expected);
}

#[test]
fn parse_nullary_closure() {
  let tree = parse_expr("-> today()");
  let expected = r#"(YamlMappingEntryValue
  (ClosureExpr
    " "
    "->"
    (CallExpr
      (IdentLit
        " "
        "today")
      "("
      ")")))"#;
  assert_eq!(tree, expected);
}

#[test]
fn parse_closure_with_params() {
  let tree = parse_expr("(self) -> self.title");
  let expected = r#"(YamlMappingEntryValue
  (ClosureExpr
    (ParamListExpr
      " "
      "("
      (IdentLit
        "self")
      ")")
    " "
    "->"
    (BinaryExpr
      (IdentLit
        " "
        "self")
      "."
      (IdentLit
        "title"))))"#;
  assert_eq!(tree, expected);
}

#[test]
fn parse_closure_with_method_call() {
  let tree = parse_expr("(self) -> self.title.slugify()");
  let expected = r#"(YamlMappingEntryValue
  (ClosureExpr
    (ParamListExpr
      " "
      "("
      (IdentLit
        "self")
      ")")
    " "
    "->"
    (CallExpr
      (BinaryExpr
        (BinaryExpr
          (IdentLit
            " "
            "self")
          "."
          (IdentLit
            "title"))
        "."
        (IdentLit
          "slugify"))
      "("
      ")")))"#;
  assert_eq!(tree, expected);
}

// Nullary closure with simple identifier body
#[test]
fn parse_nullary_closure_ident() {
  let tree = parse_expr("-> null");
  let expected = r#"(YamlMappingEntryValue
  (ClosureExpr
    " "
    "->"
    (IdentLit
      " "
      "null")))"#;
  assert_eq!(tree, expected);
}

// Closure body captures the full expression
#[test]
fn parse_nullary_closure_with_binary_body() {
  let tree = parse_expr("-> 1 + 2");
  let expected = r#"(YamlMappingEntryValue
  (ClosureExpr
    " "
    "->"
    (BinaryExpr
      (NumberLit
        " "
        "1")
      " "
      "+"
      (NumberLit
        " "
        "2"))))"#;
  assert_eq!(tree, expected);
}

// Closure with postfix ? in body
#[test]
fn parse_closure_with_postfix() {
  let tree = parse_expr("(self) -> self.name?");
  let expected = r#"(YamlMappingEntryValue
  (ClosureExpr
    (ParamListExpr
      " "
      "("
      (IdentLit
        "self")
      ")")
    " "
    "->"
    (PostfixExpr
      (BinaryExpr
        (IdentLit
          " "
          "self")
        "."
        (IdentLit
          "name"))
      "?")))"#;
  assert_eq!(tree, expected);
}

// Empty param list closure
#[test]
fn parse_empty_param_list_closure() {
  let tree = parse_expr("() -> today()");
  let expected = r#"(YamlMappingEntryValue
  (ClosureExpr
    (ParamListExpr
      " "
      "("
      ")")
    " "
    "->"
    (CallExpr
      (IdentLit
        " "
        "today")
      "("
      ")")))"#;
  assert_eq!(tree, expected);
}

// Multi-param closure
#[test]
fn parse_multi_param_closure() {
  let tree = parse_expr("(a, b) -> a + b");
  let expected = r#"(YamlMappingEntryValue
  (ClosureExpr
    (ParamListExpr
      " "
      "("
      (IdentLit
        "a")
      ","
      (IdentLit
        " "
        "b")
      ")")
    " "
    "->"
    (BinaryExpr
      (IdentLit
        " "
        "a")
      " "
      "+"
      (IdentLit
        " "
        "b"))))"#;
  assert_eq!(tree, expected);
}

// Nested closure
#[test]
fn parse_nested_closure() {
  let tree = parse_expr("-> -> 1");
  let expected = r#"(YamlMappingEntryValue
  (ClosureExpr
    " "
    "->"
    (ClosureExpr
      " "
      "->"
      (NumberLit
        " "
        "1"))))"#;
  assert_eq!(tree, expected);
}

// Closure as right operand of binary expression
#[test]
fn parse_closure_as_rhs() {
  let tree = parse_expr("a + -> b");
  let expected = r#"(YamlMappingEntryValue
  (BinaryExpr
    (IdentLit
      " "
      "a")
    " "
    "+"
    (ClosureExpr
      " "
      "->"
      (IdentLit
        " "
        "b"))))"#;
  assert_eq!(tree, expected);
}

// Dangling param list without -> emits diagnostic
#[test]
fn parse_dangling_param_list() {
  let (_, diagnostics) = parse_expr_with_diagnostics("(a, b)");
  assert!(
    diagnostics
      .iter()
      .any(|d| matches!(d, Diagnostic::DanglingParamList { .. })),
    "expected DanglingParamList: {:?}",
    diagnostics
  );
}

// Empty param list without -> emits diagnostic
#[test]
fn parse_dangling_empty_param_list() {
  let (_, diagnostics) = parse_expr_with_diagnostics("()");
  assert!(
    diagnostics
      .iter()
      .any(|d| matches!(d, Diagnostic::DanglingParamList { .. })),
    "expected DanglingParamList: {:?}",
    diagnostics
  );
}

// Dangling param list with trailing whitespace emits diagnostic
#[test]
fn parse_dangling_param_list_with_whitespace() {
  let (_, diagnostics) = parse_expr_with_diagnostics("(a, b)   ");
  assert!(
    diagnostics
      .iter()
      .any(|d| matches!(d, Diagnostic::DanglingParamList { .. })),
    "expected DanglingParamList: {:?}",
    diagnostics
  );
}

// Dangling param list as LHS of binary expression emits diagnostic
#[test]
fn parse_dangling_param_list_as_lhs() {
  let (_, diagnostics) = parse_expr_with_diagnostics("(a, b) + 1");
  assert!(
    diagnostics
      .iter()
      .any(|d| matches!(d, Diagnostic::DanglingParamList { .. })),
    "expected DanglingParamList: {:?}",
    diagnostics
  );
}

// Dangling param list as RHS of binary expression emits diagnostic
#[test]
fn parse_dangling_param_list_as_rhs() {
  let (_, diagnostics) = parse_expr_with_diagnostics("1 + (a, b)");
  assert!(
    diagnostics
      .iter()
      .any(|d| matches!(d, Diagnostic::DanglingParamList { .. })),
    "expected DanglingParamList: {:?}",
    diagnostics
  );
}

// Unclosed param list missing closing paren emits UnclosedParamList
#[test]
fn parse_unclosed_param_list() {
  let (_, diagnostics) = parse_expr_with_diagnostics("(a, b");
  assert!(
    diagnostics
      .iter()
      .any(|d| matches!(d, Diagnostic::UnclosedParamList { .. })),
    "expected UnclosedParamList for unclosed param list: {:?}",
    diagnostics
  );
}

// Unclosed empty param list `(` emits MissingSyntaxNode for PrimaryExpr
#[test]
fn parse_unclosed_empty_param_list() {
  let (_, diagnostics) = parse_expr_with_diagnostics("(");
  assert!(
    diagnostics.iter().any(|d| matches!(
      d,
      Diagnostic::MissingSyntaxNode {
        expected: SyntaxKind::PrimaryExpr,
        ..
      }
    )),
    "expected MissingSyntaxNode(PrimaryExpr): {:?}",
    diagnostics
  );
}

// Positive test: bare identifier as closure param with full tree assertion
#[test]
fn parse_closure_bare_ident_param() {
  let tree = parse_expr("x -> x + 1");
  let expected = r#"(YamlMappingEntryValue
  (ClosureExpr
    (IdentLit
      " "
      "x")
    " "
    "->"
    (BinaryExpr
      (IdentLit
        " "
        "x")
      " "
      "+"
      (NumberLit
        " "
        "1"))))"#;
  assert_eq!(tree, expected);
}

// Positive test: param list with trailing comma with full tree assertion
#[test]
fn parse_closure_trailing_comma_params() {
  let tree = parse_expr("(a, b,) -> a * b");
  let expected = r#"(YamlMappingEntryValue
  (ClosureExpr
    (ParamListExpr
      " "
      "("
      (IdentLit
        "a")
      ","
      (IdentLit
        " "
        "b")
      ","
      ")")
    " "
    "->"
    (BinaryExpr
      (IdentLit
        " "
        "a")
      " "
      "*"
      (IdentLit
        " "
        "b"))))"#;
  assert_eq!(tree, expected);
}

// AST node tests for ClosureExpr and ParamListExpr
#[test]
fn ast_closure_expr_and_param_list() {
  let (root_syntax, _) = parse(
    r#"---
key: (a, b) -> a + b
---
"#,
  );
  let root_red = RedNode::new_root(root_syntax.as_node().unwrap().clone());
  let source = SourceFile::cast(root_red).unwrap();
  let expr = source
    .frontmatter()
    .unwrap()
    .mapping()
    .unwrap()
    .values()
    .next()
    .unwrap();

  let closure = ClosureExpr::cast(expr.syntax().clone()).expect("expected ClosureExpr");

  // Test params() returning Either::Left(ParamListExpr)
  match closure.params() {
    Some(Either::Left(param_list)) => {
      let params: Vec<String> = param_list.params().filter_map(|id| id.value()).collect();
      assert_eq!(params, vec!["a", "b"]);
    }
    _ => panic!("expected ParamListExpr in closure params"),
  }

  // Test body() returning Expr
  assert!(closure.body().is_some());

  // Test single bare ident parameter closure
  let (root_syntax2, _) = parse(
    r#"---
key: x -> x + 1
---
"#,
  );
  let root_red2 = RedNode::new_root(root_syntax2.as_node().unwrap().clone());
  let source2 = SourceFile::cast(root_red2).unwrap();
  let expr2 = source2
    .frontmatter()
    .unwrap()
    .mapping()
    .unwrap()
    .values()
    .next()
    .unwrap();

  let closure2 =
    ClosureExpr::cast(expr2.syntax().clone()).expect("expected ClosureExpr for bare ident");

  match closure2.params() {
    Some(Either::Right(ident)) => {
      assert_eq!(ident.value().as_deref(), Some("x"));
    }
    _ => panic!("expected IdentLit in closure params"),
  }
}

// Non-identifier in param list emits diagnostic
#[test]
fn parse_closure_non_ident_param() {
  let (_, diagnostics) = parse_expr_with_diagnostics("(1, 2) -> 3");
  assert!(
    diagnostics
      .iter()
      .any(|d| matches!(d, Diagnostic::InvalidClosureParams { .. })),
    "expected InvalidClosureParams: {:?}",
    diagnostics
  );
}

// Expression in single-param paren emits diagnostic
#[test]
fn parse_closure_expr_param() {
  let (_, diagnostics) = parse_expr_with_diagnostics("(1 + 2) -> 3");
  assert!(
    diagnostics
      .iter()
      .any(|d| matches!(d, Diagnostic::InvalidClosureParams { .. })),
    "expected InvalidClosureParams: {:?}",
    diagnostics
  );
}

// Invalid LHS like a number literal emits diagnostic
#[test]
fn parse_closure_invalid_lhs() {
  let (_, diagnostics) = parse_expr_with_diagnostics("42 -> 3");
  assert!(
    diagnostics
      .iter()
      .any(|d| matches!(d, Diagnostic::InvalidClosureParams { .. })),
    "expected InvalidClosureParams: {:?}",
    diagnostics
  );
}

// Valid single-param closure has no closure diagnostics
#[test]
fn parse_closure_valid_single_param_no_diagnostic() {
  let (_, diagnostics) = parse_expr_with_diagnostics("(x) -> x");
  assert!(
    !diagnostics
      .iter()
      .any(|d| matches!(d, Diagnostic::InvalidClosureParams { .. })),
    "unexpected InvalidClosureParams: {:?}",
    diagnostics
  );
}

// Valid multi-param closure has no closure diagnostics
#[test]
fn parse_closure_valid_multi_param_no_diagnostic() {
  let (_, diagnostics) = parse_expr_with_diagnostics("(a, b) -> a");
  assert!(
    !diagnostics
      .iter()
      .any(|d| matches!(d, Diagnostic::InvalidClosureParams { .. })),
    "unexpected InvalidClosureParams: {:?}",
    diagnostics
  );
}

// Bare identifier as closure param is valid
#[test]
fn parse_closure_bare_ident_no_diagnostic() {
  let (_, diagnostics) = parse_expr_with_diagnostics("x -> x");
  assert!(
    !diagnostics
      .iter()
      .any(|d| matches!(d, Diagnostic::InvalidClosureParams { .. })),
    "unexpected InvalidClosureParams: {:?}",
    diagnostics
  );
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
