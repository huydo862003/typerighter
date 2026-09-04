use super::helpers::*;
use crate::syntax::diagnostic::Diagnostic;

fn parse_frontmatter(input: &str) -> String {
  let full = format!("---\n{}\n---\n", input);
  let (ast, _) = parse(&full);
  let root = ast.as_node().unwrap();
  let frontmatter = &root.children()[0];
  render_tree(frontmatter)
}

fn parse_frontmatter_with_diagnostics(input: &str) -> (String, Vec<Diagnostic>) {
  let full = format!("---\n{}\n---\n", input);
  let (ast, diagnostics) = parse(&full);
  let root = ast.as_node().unwrap();
  let frontmatter = &root.children()[0];
  (render_tree(frontmatter), diagnostics)
}

// Yaml frontmatter
#[test]
fn empty_frontmatter() {
  let tree = parse_frontmatter(r#""#);
  let expected = r#"(YamlFrontmatter
  ""
  "---"
  "\n"
  "\n"
  ""
  "---"
  "\n")"#;
  assert_eq!(tree, expected);
}

#[test]
fn recover_yaml_missing_closing_marker() {
  let full = r#"---
key: 1
"#;
  let (ast, diags) = parse(full);
  let root = ast.as_node().unwrap();
  let frontmatter = &root.children()[0];
  let tree = render_tree(frontmatter);
  assert_eq!(
    tree,
    r####"(YamlFrontmatter
  ""
  "---"
  "\n"
  (YamlMapping
    ""
    (YamlMappingEntry
      (YamlMappingEntryKey
        "key")
      ":"
      (YamlMappingEntryValue
        (NumberLit
          " "
          "1"))))
  "\n"
  (Error
    ""))"####
  );
  assert_eq!(
    diags,
    vec![Diagnostic::MissingFrontmatterMarker { offset: 11 },]
  );
}

// Mapping

#[test]
fn single_key_value() {
  let tree = parse_frontmatter(r#"key: 1"#);
  let expected = r#"(YamlFrontmatter
  ""
  "---"
  "\n"
  (YamlMapping
    ""
    (YamlMappingEntry
      (YamlMappingEntryKey
        "key")
      ":"
      (YamlMappingEntryValue
        (NumberLit
          " "
          "1")))
    "\n"
    "")
  "---"
  "\n")"#;
  assert_eq!(tree, expected);
}

#[test]
fn multiple_key_values() {
  let tree = parse_frontmatter(
    r#"a: 1
b: 2"#,
  );
  let expected = r#"(YamlFrontmatter
  ""
  "---"
  "\n"
  (YamlMapping
    ""
    (YamlMappingEntry
      (YamlMappingEntryKey
        "a")
      ":"
      (YamlMappingEntryValue
        (NumberLit
          " "
          "1")))
    "\n"
    ""
    (YamlMappingEntry
      (YamlMappingEntryKey
        "b")
      ":"
      (YamlMappingEntryValue
        (NumberLit
          " "
          "2")))
    "\n"
    "")
  "---"
  "\n")"#;
  assert_eq!(tree, expected);
}

// Strings
#[test]
fn string_value() {
  let tree = parse_frontmatter(r#"title: "hello""#);
  let expected = r#"(YamlFrontmatter
  ""
  "---"
  "\n"
  (YamlMapping
    ""
    (YamlMappingEntry
      (YamlMappingEntryKey
        "title")
      ":"
      (YamlMappingEntryValue
        (StrLit
          " "
          "\""
          "hello"
          "\"")))
    "\n"
    "")
  "---"
  "\n")"#;
  assert_eq!(tree, expected);
}

// Flow sequences
#[test]
fn flow_list_value() {
  let tree = parse_frontmatter(r#"tags: [1, 2]"#);
  let expected = r#"(YamlFrontmatter
  ""
  "---"
  "\n"
  (YamlMapping
    ""
    (YamlMappingEntry
      (YamlMappingEntryKey
        "tags")
      ":"
      (YamlMappingEntryValue
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
          "]")))
    "\n"
    "")
  "---"
  "\n")"#;
  assert_eq!(tree, expected);
}

// Block sequences
#[test]
fn block_sequence() {
  let tree = parse_frontmatter(
    r#"items:
  - 1
  - 2"#,
  );
  let expected = r#"(YamlFrontmatter
  ""
  "---"
  "\n"
  (YamlMapping
    ""
    (YamlMappingEntry
      (YamlMappingEntryKey
        "items")
      ":"
      (YamlMappingEntryValue
        (YamlSequence
          "\n"
          "  "
          (YamlSequenceItem
            "-"
            (NumberLit
              " "
              "1"))
          "\n"
          "  "
          (YamlSequenceItem
            "-"
            (NumberLit
              " "
              "2")))))
    "\n"
    "")
  "---"
  "\n")"#;
  assert_eq!(tree, expected);
}

// Nested mappings
#[test]
fn nested_mapping() {
  let tree = parse_frontmatter(
    r#"outer:
  inner: 1"#,
  );
  let expected = r#"(YamlFrontmatter
  ""
  "---"
  "\n"
  (YamlMapping
    ""
    (YamlMappingEntry
      (YamlMappingEntryKey
        "outer")
      ":"
      (YamlMappingEntryValue
        (YamlMapping
          "\n"
          "  "
          (YamlMappingEntry
            (YamlMappingEntryKey
              "inner")
            ":"
            (YamlMappingEntryValue
              (NumberLit
                " "
                "1"))))))
    "\n"
    "")
  "---"
  "\n")"#;
  assert_eq!(tree, expected);
}

// Sequence in mapping
#[test]
fn sequence_in_mapping() {
  let tree = parse_frontmatter(
    r#"key:
  - a
  - b"#,
  );
  let expected = r#"(YamlFrontmatter
  ""
  "---"
  "\n"
  (YamlMapping
    ""
    (YamlMappingEntry
      (YamlMappingEntryKey
        "key")
      ":"
      (YamlMappingEntryValue
        (YamlSequence
          "\n"
          "  "
          (YamlSequenceItem
            "-"
            (IdentLit
              " "
              "a"))
          "\n"
          "  "
          (YamlSequenceItem
            "-"
            (IdentLit
              " "
              "b")))))
    "\n"
    "")
  "---"
  "\n")"#;
  assert_eq!(tree, expected);
}

// Mapping in sequence
#[test]
fn mapping_in_sequence() {
  let tree = parse_frontmatter(
    r#"items:
  - name: alice
  - name: bob"#,
  );
  let expected = r#"(YamlFrontmatter
  ""
  "---"
  "\n"
  (YamlMapping
    ""
    (YamlMappingEntry
      (YamlMappingEntryKey
        "items")
      ":"
      (YamlMappingEntryValue
        (YamlSequence
          "\n"
          "  "
          (YamlSequenceItem
            "-"
            (YamlMapping
              " "
              (YamlMappingEntry
                (YamlMappingEntryKey
                  "name")
                ":"
                (YamlMappingEntryValue
                  (IdentLit
                    " "
                    "alice")))
              "\n"
              "  "
              (Error
                "-"
                " ")
              (YamlMappingEntry
                (YamlMappingEntryKey
                  "name")
                ":"
                (YamlMappingEntryValue
                  (IdentLit
                    " "
                    "bob"))))))))
    "\n"
    "")
  "---"
  "\n")"#;
  assert_eq!(tree, expected);
}

// Nested sequences
#[test]
fn nested_sequence() {
  let tree = parse_frontmatter(
    r#"matrix:
  -
    - 1
    - 2
  -
    - 3
    - 4"#,
  );
  let expected = r#"(YamlFrontmatter
  ""
  "---"
  "\n"
  (YamlMapping
    ""
    (YamlMappingEntry
      (YamlMappingEntryKey
        "matrix")
      ":"
      (YamlMappingEntryValue
        (YamlSequence
          "\n"
          "  "
          (YamlSequenceItem
            "-"
            (YamlSequence
              "\n"
              "    "
              (YamlSequenceItem
                "-"
                (NumberLit
                  " "
                  "1"))
              "\n"
              "    "
              (YamlSequenceItem
                "-"
                (NumberLit
                  " "
                  "2"))))
          "\n"
          "  "
          (YamlSequenceItem
            "-"
            (YamlSequence
              "\n"
              "    "
              (YamlSequenceItem
                "-"
                (NumberLit
                  " "
                  "3"))
              "\n"
              "    "
              (YamlSequenceItem
                "-"
                (NumberLit
                  " "
                  "4")))))))
    "\n"
    "")
  "---"
  "\n")"#;
  assert_eq!(tree, expected);
}

#[test]
fn inline_nested_sequence() {
  let tree = parse_frontmatter(
    r#"items:
  - - 1
  - - 2"#,
  );
  assert!(tree.contains("YamlSequence"));
}

#[test]
fn block_string_in_sequence() {
  let tree = parse_frontmatter(
    r#"items:
  - >
    hello
    world
  - >
    foo"#,
  );
  assert!(tree.contains("YamlFoldedBlockStrLit"));
}

#[test]
fn multi_entry_inline_mapping() {
  let tree = parse_frontmatter(
    r#"items:
  - name: alice
    age: 30
  - name: bob
    age: 25"#,
  );
  let expected = r#"(YamlFrontmatter
  ""
  "---"
  "\n"
  (YamlMapping
    ""
    (YamlMappingEntry
      (YamlMappingEntryKey
        "items")
      ":"
      (YamlMappingEntryValue
        (YamlSequence
          "\n"
          "  "
          (YamlSequenceItem
            "-"
            (YamlMapping
              " "
              (YamlMappingEntry
                (YamlMappingEntryKey
                  "name")
                ":"
                (YamlMappingEntryValue
                  (IdentLit
                    " "
                    "alice")))
              "\n"
              "    "
              (YamlMappingEntry
                (YamlMappingEntryKey
                  "age")
                ":"
                (YamlMappingEntryValue
                  (NumberLit
                    " "
                    "30")))))
          "\n"
          "  "
          (YamlSequenceItem
            "-"
            (YamlMapping
              " "
              (YamlMappingEntry
                (YamlMappingEntryKey
                  "name")
                ":"
                (YamlMappingEntryValue
                  (IdentLit
                    " "
                    "bob")))
              "\n"
              "    "
              (YamlMappingEntry
                (YamlMappingEntryKey
                  "age")
                ":"
                (YamlMappingEntryValue
                  (NumberLit
                    " "
                    "25"))))))))
    "\n"
    "")
  "---"
  "\n")"#;
  assert_eq!(tree, expected);
}

// Inline nested sequence with continuation items
#[test]
fn inline_nested_sequence_multi_items() {
  let full = format!(
    "---\n{}\n---\n",
    r#"items:
  - - 1
    - 2
  - - 3
    - 4"#
  );
  let (ast, _) = parse(&full);
  let tree = render_tree(&ast);
  assert!(!tree.contains("Error"));
}

// Multiple top-level keys, values mixing sequences and mappings
#[test]
fn multi_key_mixed_seq_and_map() {
  let full = format!(
    "---\n{}\n---\n",
    r#"users:
  - name: alice
    tags:
      - admin
      - editor
  - name: bob
    tags:
      - viewer
config:
  - ports:
      - 8080
      - 443
    host: localhost
  - ports:
      - 3000
    host: staging
extra:
  enabled: true"#
  );
  let (ast, _) = parse(&full);
  let tree = render_tree(&ast);
  assert!(!tree.contains("Error"));
}

// Sequence where first item is a list, second is a map, third is a list again
#[test]
fn seq_alternating_list_and_map() {
  let full = format!(
    "---\n{}\n---\n",
    r#"data:
  - - 10
    - 20
    - 30
  - key: value
    other: stuff
  - - 40
    - 50"#
  );
  let (ast, diags) = parse(&full);
  assert_eq!(diags, vec![]);
  let tree = render_tree(&ast);
  let expected = r#"(SourceFile
  (YamlFrontmatter
    ""
    "---"
    "\n"
    (YamlMapping
      ""
      (YamlMappingEntry
        (YamlMappingEntryKey
          "data")
        ":"
        (YamlMappingEntryValue
          (YamlSequence
            "\n"
            "  "
            (YamlSequenceItem
              "-"
              (YamlSequence
                " "
                (YamlSequenceItem
                  "-"
                  (NumberLit
                    " "
                    "10"))
                "\n"
                "    "
                (YamlSequenceItem
                  "-"
                  (NumberLit
                    " "
                    "20"))
                "\n"
                "    "
                (YamlSequenceItem
                  "-"
                  (NumberLit
                    " "
                    "30"))))
            "\n"
            "  "
            (YamlSequenceItem
              "-"
              (YamlMapping
                " "
                (YamlMappingEntry
                  (YamlMappingEntryKey
                    "key")
                  ":"
                  (YamlMappingEntryValue
                    (IdentLit
                      " "
                      "value")))
                "\n"
                "    "
                (YamlMappingEntry
                  (YamlMappingEntryKey
                    "other")
                  ":"
                  (YamlMappingEntryValue
                    (IdentLit
                      " "
                      "stuff")))))
            "\n"
            "  "
            (YamlSequenceItem
              "-"
              (YamlSequence
                " "
                (YamlSequenceItem
                  "-"
                  (NumberLit
                    " "
                    "40"))
                "\n"
                "    "
                (YamlSequenceItem
                  "-"
                  (NumberLit
                    " "
                    "50")))))))
      "\n"
      "")
    "---"
    "\n")
  (MdBody))"#;
  assert_eq!(tree, expected);
}

// Deeply nested sequence of sequences of sequences
#[test]
fn triple_nested_sequence() {
  let full = format!(
    "---\n{}\n---\n",
    r#"matrix:
  - - - 1
      - 2
    - - 3
      - 4
  - - - 5
      - 6"#
  );
  let (ast, _) = parse(&full);
  let tree = render_tree(&ast);
  assert!(!tree.contains("Error"));
}

// Map value is a sequence whose items are maps containing sequences
#[test]
fn map_of_seq_of_map_of_seq() {
  let full = format!(
    "---\n{}\n---\n",
    r#"teams:
  - members:
      - alice
      - bob
    projects:
      - alpha
      - beta
  - members:
      - charlie
    projects:
      - gamma
settings:
  debug: true"#
  );
  let (ast, _) = parse(&full);
  let tree = render_tree(&ast);
  let expected = r#"(SourceFile
  (YamlFrontmatter
    ""
    "---"
    "\n"
    (YamlMapping
      ""
      (YamlMappingEntry
        (YamlMappingEntryKey
          "teams")
        ":"
        (YamlMappingEntryValue
          (YamlSequence
            "\n"
            "  "
            (YamlSequenceItem
              "-"
              (YamlMapping
                " "
                (YamlMappingEntry
                  (YamlMappingEntryKey
                    "members")
                  ":"
                  (YamlMappingEntryValue
                    (YamlSequence
                      "\n"
                      "      "
                      (YamlSequenceItem
                        "-"
                        (IdentLit
                          " "
                          "alice"))
                      "\n"
                      "      "
                      (YamlSequenceItem
                        "-"
                        (IdentLit
                          " "
                          "bob")))))
                "\n"
                "    "
                (YamlMappingEntry
                  (YamlMappingEntryKey
                    "projects")
                  ":"
                  (YamlMappingEntryValue
                    (YamlSequence
                      "\n"
                      "      "
                      (YamlSequenceItem
                        "-"
                        (IdentLit
                          " "
                          "alpha"))
                      "\n"
                      "      "
                      (YamlSequenceItem
                        "-"
                        (IdentLit
                          " "
                          "beta")))))))
            "\n"
            "  "
            (YamlSequenceItem
              "-"
              (YamlMapping
                " "
                (YamlMappingEntry
                  (YamlMappingEntryKey
                    "members")
                  ":"
                  (YamlMappingEntryValue
                    (YamlSequence
                      "\n"
                      "      "
                      (YamlSequenceItem
                        "-"
                        (IdentLit
                          " "
                          "charlie")))))
                "\n"
                "    "
                (YamlMappingEntry
                  (YamlMappingEntryKey
                    "projects")
                  ":"
                  (YamlMappingEntryValue
                    (YamlSequence
                      "\n"
                      "      "
                      (YamlSequenceItem
                        "-"
                        (IdentLit
                          " "
                          "gamma"))))))))))
      "\n"
      ""
      (YamlMappingEntry
        (YamlMappingEntryKey
          "settings")
        ":"
        (YamlMappingEntryValue
          (YamlMapping
            "\n"
            "  "
            (YamlMappingEntry
              (YamlMappingEntryKey
                "debug")
              ":"
              (YamlMappingEntryValue
                (IdentLit
                  " "
                  "true"))))))
      "\n"
      "")
    "---"
    "\n")
  (MdBody))"#;
  assert_eq!(tree, expected);
}

// Inline sequence items where one is a map and next is a sequence
#[test]
fn inline_seq_first_map_then_list() {
  let full = format!(
    "---\n{}\n---\n",
    r#"mixed:
  - a: 1
    b: 2
  - - x
    - y
  - c: 3"#
  );
  let (ast, diags) = parse(&full);
  assert_eq!(diags, vec![]);
  let tree = render_tree(&ast);
  let expected = r#"(SourceFile
  (YamlFrontmatter
    ""
    "---"
    "\n"
    (YamlMapping
      ""
      (YamlMappingEntry
        (YamlMappingEntryKey
          "mixed")
        ":"
        (YamlMappingEntryValue
          (YamlSequence
            "\n"
            "  "
            (YamlSequenceItem
              "-"
              (YamlMapping
                " "
                (YamlMappingEntry
                  (YamlMappingEntryKey
                    "a")
                  ":"
                  (YamlMappingEntryValue
                    (NumberLit
                      " "
                      "1")))
                "\n"
                "    "
                (YamlMappingEntry
                  (YamlMappingEntryKey
                    "b")
                  ":"
                  (YamlMappingEntryValue
                    (NumberLit
                      " "
                      "2")))))
            "\n"
            "  "
            (YamlSequenceItem
              "-"
              (YamlSequence
                " "
                (YamlSequenceItem
                  "-"
                  (IdentLit
                    " "
                    "x"))
                "\n"
                "    "
                (YamlSequenceItem
                  "-"
                  (IdentLit
                    " "
                    "y"))))
            "\n"
            "  "
            (YamlSequenceItem
              "-"
              (YamlMapping
                " "
                (YamlMappingEntry
                  (YamlMappingEntryKey
                    "c")
                  ":"
                  (YamlMappingEntryValue
                    (NumberLit
                      " "
                      "3")))
                "\n"
                ""))))))
    "---"
    "\n")
  (MdBody))"#;
  assert_eq!(tree, expected);
}

#[test]
fn seq_mixed_single_and_multi_inner() {
  let full = format!(
    "---\n{}\n---\n",
    r#"values:
  - - 1
  - - 2
    - 3
  - - 4"#
  );
  let (ast, _) = parse(&full);
  let tree = render_tree(&ast);
  assert!(!tree.contains("Error"));
}

// Sequence items with folded block strings
#[test]
fn seq_with_folded_block_string() {
  let full = format!(
    "---\n{}\n---\n",
    r#"key:
  - >
    hello
    world
  - value2"#
  );
  let (ast, _) = parse(&full);
  let tree = render_tree(&ast);
  assert!(tree.contains("YamlFoldedBlockStrLit"));
  assert!(!tree.contains("Error"));
}

// Sequence items with literal block strings
#[test]
fn seq_with_literal_block_string() {
  let full = format!(
    "---\n{}\n---\n",
    r#"key:
  - |
    line1
    line2
  - other"#
  );
  let (ast, _) = parse(&full);
  let tree = render_tree(&ast);
  assert!(tree.contains("YamlLiteralBlockStrLit"));
  assert!(!tree.contains("Error"));
}

// Mapping with folded block string value
#[test]
fn mapping_with_folded_block_string() {
  let full = format!(
    "---\n{}\n---\n",
    r#"desc: >
  this is
  folded
title: hello"#
  );
  let (ast, _) = parse(&full);
  let tree = render_tree(&ast);
  assert!(tree.contains("YamlFoldedBlockStrLit"));
  assert!(!tree.contains("Error"));
}

// Mapping with literal block string value
#[test]
fn mapping_with_literal_block_string() {
  let full = format!(
    "---\n{}\n---\n",
    r#"desc: |
  line one
  line two
title: hello"#
  );
  let (ast, _) = parse(&full);
  let tree = render_tree(&ast);
  assert!(tree.contains("YamlLiteralBlockStrLit"));
  assert!(!tree.contains("Error"));
}

// Mixed: mapping values are sequences with block strings and nested mappings
#[test]
fn complex_mixed_block_strings_and_nesting() {
  let full = format!(
    "---\n{}\n---\n",
    r#"items:
  - name: alice
    bio: >
      alice is
      great
  - name: bob
    bio: |
      bob is
      cool
settings:
  debug: true"#
  );
  let (ast, _) = parse(&full);
  let tree = render_tree(&ast);
  assert!(tree.contains("YamlFoldedBlockStrLit"));
  assert!(tree.contains("YamlLiteralBlockStrLit"));
  assert!(!tree.contains("Error"));
}

// Deeply nested: map -> seq -> map -> folded block string
#[test]
fn deep_nested_with_block_string() {
  let full = format!(
    "---\n{}\n---\n",
    r#"outer:
  - inner:
      desc: >
        nested
        folded"#
  );
  let (ast, _) = parse(&full);
  let tree = render_tree(&ast);
  assert!(tree.contains("YamlFoldedBlockStrLit"));
  assert!(!tree.contains("Error"));
}

// Sequence with empty items followed by nested content
#[test]
fn seq_empty_dash_then_nested() {
  let full = format!(
    "---\n{}\n---\n",
    r#"matrix:
  -
    - 1
    - 2
  - 3"#
  );
  let (ast, _) = parse(&full);
  let tree = render_tree(&ast);
  assert!(!tree.contains("Error"));
}

// Literal block string with insufficient indent emits InsufficientBlockIndent
#[test]
fn literal_block_str_insufficient_indent() {
  let (_, diags) = parse_frontmatter_with_diagnostics(
    r#"outer:
  desc: |
  line one
"#,
  );
  assert!(
    diags
      .iter()
      .any(|d| matches!(d, Diagnostic::InsufficientBlockIndent { .. }))
  );
}

// Folded block string with insufficient indent emits InsufficientBlockIndent
#[test]
fn folded_block_str_insufficient_indent() {
  let (_, diags) = parse_frontmatter_with_diagnostics(
    r#"outer:
  desc: >
  line one
"#,
  );
  assert!(
    diags
      .iter()
      .any(|d| matches!(d, Diagnostic::InsufficientBlockIndent { .. }))
  );
}

// Unquoted multi-word value should produce a parse error
#[test]
fn unquoted_multiword_value_has_parse_error() {
  let (_, diags) = parse_frontmatter_with_diagnostics("title: hello world");
  assert!(
    !diags.is_empty(),
    "unquoted multi-word value should produce parse errors, got none"
  );
}

// Bare YAML document (no --- delimiters)

fn parse_document(input: &str) -> String {
  let (ast, _) = parse_yaml_document(input);
  let root = ast.as_node().unwrap();
  let frontmatter = &root.children()[0];
  render_tree(frontmatter)
}

fn parse_document_with_diagnostics(input: &str) -> (String, Vec<Diagnostic>) {
  let (ast, diagnostics) = parse_yaml_document(input);
  let root = ast.as_node().unwrap();
  let frontmatter = &root.children()[0];
  (render_tree(frontmatter), diagnostics)
}

#[test]
fn document_empty() {
  let tree = parse_document("");
  assert_eq!(tree, "(YamlFrontmatter)");
}

#[test]
fn document_simple_mapping() {
  let (tree, diags) = parse_document_with_diagnostics("key: value\n");
  assert!(diags.is_empty(), "unexpected diagnostics: {diags:?}");
  assert!(
    tree.contains("YamlMapping"),
    "should contain a mapping: {tree}"
  );
  assert!(
    tree.contains("YamlMappingEntry"),
    "should contain entries: {tree}"
  );
  assert!(tree.contains(r#""key""#), "should contain key: {tree}");
  assert!(tree.contains(r#""value""#), "should contain value: {tree}");
}

#[test]
fn document_nested_mapping() {
  let (tree, diags) = parse_document_with_diagnostics(
    r#"site:
  title: "My Site"
  description: "A description"
"#,
  );
  assert!(diags.is_empty(), "unexpected diagnostics: {diags:?}");
  assert!(
    tree.contains(r#""site""#),
    "should contain site key: {tree}"
  );
  assert!(
    tree.contains(r#""My Site""#),
    "should contain title value: {tree}"
  );
}

#[test]
fn document_matches_frontmatter_structure() {
  let input = r#"version: "1.0.0"
repo: "https://github.com/test"
"#;

  let bare_tree = parse_document(input);
  let fm_tree = parse_frontmatter(input);

  assert!(bare_tree.contains("YamlMapping"), "bare: {bare_tree}");
  assert!(fm_tree.contains("YamlMapping"), "frontmatter: {fm_tree}");
  assert!(
    bare_tree.contains(r#""version""#),
    "bare should have version: {bare_tree}"
  );
  assert!(
    fm_tree.contains(r#""version""#),
    "frontmatter should have version: {fm_tree}"
  );
}
