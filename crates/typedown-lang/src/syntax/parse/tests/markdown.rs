use super::helpers::*;
use crate::syntax::diagnostic::Diagnostic;

fn parse_body(input: &str) -> String {
  let full_input = format!("---\n---\n{}", input);
  let (ast, _) = parse(&full_input);
  render_tree(&ast)
}

fn parse_body_with_diags(input: &str) -> (String, Vec<Diagnostic>) {
  let full_input = format!("---\n---\n{}", input);
  let (ast, diags) = parse(&full_input);
  (render_tree(&ast), diags)
}

// Simple block elements

// Parses a single-line paragraph
#[test]
fn parse_paragraph_simple() {
  let tree = parse_body(
    r#"hello world
"#,
  );
  assert_eq!(
    tree,
    r####"(SourceFile
  (YamlFrontmatter
    ""
    "---"
    "\n"
    ""
    "---"
    "\n")
  (MdBody
    (MdParagraph
      (MdText
        "hello"
        " "
        "world"))
    "\n"))"####
  );
}

// Parses a level-1 heading
#[test]
fn parse_heading_simple() {
  let tree = parse_body(
    r#"# Hello
"#,
  );
  assert_eq!(
    tree,
    r####"(SourceFile
  (YamlFrontmatter
    ""
    "---"
    "\n"
    ""
    "---"
    "\n")
  (MdBody
    (MdHeading
      "#"
      " "
      (MdText
        "Hello"))
    "\n"))"####
  );
}

// Parses headings of levels 1, 2, and 3
#[test]
fn parse_heading_levels() {
  let tree = parse_body(
    r#"# H1
## H2
### H3
"#,
  );
  assert_eq!(
    tree,
    r####"(SourceFile
  (YamlFrontmatter
    ""
    "---"
    "\n"
    ""
    "---"
    "\n")
  (MdBody
    (MdHeading
      "#"
      " "
      (MdText
        "H"
        "1"))
    "\n"
    (MdHeading
      "##"
      " "
      (MdText
        "H"
        "2"))
    "\n"
    (MdHeading
      "###"
      " "
      (MdText
        "H"
        "3"))
    "\n"))"####
  );
}

// Parses a bullet list with dash markers
#[test]
fn parse_bullet_list_dash() {
  let tree = parse_body(
    r#"- item 1
- item 2
"#,
  );
  assert_eq!(
    tree,
    r####"(SourceFile
  (YamlFrontmatter
    ""
    "---"
    "\n"
    ""
    "---"
    "\n")
  (MdBody
    (MdBulletList
      (MdBulletListItem
        "-"
        " "
        (MdParagraph
          (MdText
            "item"
            " "
            "1")))
      "\n"
      (MdBulletListItem
        "-"
        " "
        (MdParagraph
          (MdText
            "item"
            " "
            "2"))))
    "\n"))"####
  );
}

// Nested bullet list inside bullet list
#[test]
fn parse_nested_bullet_list() {
  let tree = parse_body(
    r#"- parent
 - child one
 - child two
"#,
  );
  assert!(
    tree.contains("(MdBulletList") && tree.matches("(MdBulletList").count() == 2,
    "should have nested MdBulletList:\n{tree}"
  );
}

// Ordered list nested inside bullet list
#[test]
fn parse_ordered_in_bullet_list() {
  let tree = parse_body(
    r#"- parent
 1. first
 2. second
"#,
  );
  assert!(
    tree.contains("(MdBulletList") && tree.contains("(MdOrderedList"),
    "should have MdOrderedList nested in MdBulletList:\n{tree}"
  );
}

// Bullet list nested inside ordered list
#[test]
fn parse_bullet_in_ordered_list() {
  let tree = parse_body(
    r#"1. parent
 - child one
 - child two
"#,
  );
  assert!(
    tree.contains("(MdOrderedList") && tree.contains("(MdBulletList"),
    "should have MdBulletList nested in MdOrderedList:\n{tree}"
  );
}

// Three levels deep: bullet > ordered > bullet
#[test]
fn parse_triple_nested_list() {
  let tree = parse_body(
    r#"- level one
 1. level two
  - level three
"#,
  );
  assert_eq!(
    tree,
    r####"(SourceFile
  (YamlFrontmatter
    ""
    "---"
    "\n"
    ""
    "---"
    "\n")
  (MdBody
    (MdBulletList
      (MdBulletListItem
        "-"
        " "
        (MdParagraph
          (MdText
            "level"
            " "
            "one"))
        "\n"
        " "
        (MdOrderedList
          (MdOrderedListItem
            "1"
            "."
            " "
            (MdParagraph
              (MdText
                "level"
                " "
                "two"))
            "\n"
            " "
            " "
            (MdBulletList
              (MdBulletListItem
                "-"
                " "
                (MdParagraph
                  (MdText
                    "level"
                    " "
                    "three"))))))))
    "\n"))"####
  );
}

// Parses a bullet list with star markers
#[test]
fn parse_bullet_list_star() {
  let tree = parse_body(
    r#"* item 1
* item 2
"#,
  );
  assert_eq!(
    tree,
    r####"(SourceFile
  (YamlFrontmatter
    ""
    "---"
    "\n"
    ""
    "---"
    "\n")
  (MdBody
    (MdBulletList
      (MdBulletListItem
        "*"
        " "
        (MdParagraph
          (MdText
            "item"
            " "
            "1")))
      "\n"
      (MdBulletListItem
        "*"
        " "
        (MdParagraph
          (MdText
            "item"
            " "
            "2"))))
    "\n"))"####
  );
}

// Parses a bullet list with plus markers
#[test]
fn parse_bullet_list_plus() {
  let tree = parse_body(
    r#"+ item 1
+ item 2
"#,
  );
  assert_eq!(
    tree,
    r####"(SourceFile
  (YamlFrontmatter
    ""
    "---"
    "\n"
    ""
    "---"
    "\n")
  (MdBody
    (MdBulletList
      (MdBulletListItem
        "+"
        " "
        (MdParagraph
          (MdText
            "item"
            " "
            "1")))
      "\n"
      (MdBulletListItem
        "+"
        " "
        (MdParagraph
          (MdText
            "item"
            " "
            "2"))))
    "\n"))"####
  );
}

// Parses a task list with unchecked and checked items
#[test]
fn parse_task_list_simple() {
  let tree = parse_body(
    r#"- [ ] unchecked
- [x] checked
"#,
  );
  assert_eq!(
    tree,
    r####"(SourceFile
  (YamlFrontmatter
    ""
    "---"
    "\n"
    ""
    "---"
    "\n")
  (MdBody
    (MdBulletList
      (MdTaskListItem
        "-"
        " "
        (MdCheckbox
          "["
          " "
          "]")
        (MdParagraph
          (MdText
            " "
            "unchecked")))
      "\n"
      (MdTaskListItem
        "-"
        " "
        (MdCheckbox
          "["
          "x"
          "]")
        (MdParagraph
          (MdText
            " "
            "checked"))))
    "\n"))"####
  );
}

// Parses a bullet list mixed with a task list item
#[test]
fn parse_task_list_mixed() {
  let tree = parse_body(
    r#"- plain item
- [ ] task item
"#,
  );
  assert_eq!(
    tree,
    r####"(SourceFile
  (YamlFrontmatter
    ""
    "---"
    "\n"
    ""
    "---"
    "\n")
  (MdBody
    (MdBulletList
      (MdBulletListItem
        "-"
        " "
        (MdParagraph
          (MdText
            "plain"
            " "
            "item")))
      "\n"
      (MdTaskListItem
        "-"
        " "
        (MdCheckbox
          "["
          " "
          "]")
        (MdParagraph
          (MdText
            " "
            "task"
            " "
            "item"))))
    "\n"))"####
  );
}

// Ordered list nested inside task list item
#[test]
fn parse_nested_task_list() {
  let tree = parse_body(
    r#"- [x] parent task
 1. substep one
 2. substep two
"#,
  );
  assert_eq!(
    tree,
    r####"(SourceFile
  (YamlFrontmatter
    ""
    "---"
    "\n"
    ""
    "---"
    "\n")
  (MdBody
    (MdBulletList
      (MdTaskListItem
        "-"
        " "
        (MdCheckbox
          "["
          "x"
          "]")
        (MdParagraph
          (MdText
            " "
            "parent"
            " "
            "task"))
        "\n"
        " "
        (MdOrderedList
          (MdOrderedListItem
            "1"
            "."
            " "
            (MdParagraph
              (MdText
                "substep"
                " "
                "one")))
          "\n"
          " "
          (MdOrderedListItem
            "2"
            "."
            " "
            (MdParagraph
              (MdText
                "substep"
                " "
                "two"))))))
    "\n"))"####
  );
}

// Parses an ordered list
#[test]
fn parse_ordered_list_simple() {
  let tree = parse_body(
    r#"1. first
2. second
"#,
  );
  assert_eq!(
    tree,
    r####"(SourceFile
  (YamlFrontmatter
    ""
    "---"
    "\n"
    ""
    "---"
    "\n")
  (MdBody
    (MdOrderedList
      (MdOrderedListItem
        "1"
        "."
        " "
        (MdParagraph
          (MdText
            "first")))
      "\n"
      (MdOrderedListItem
        "2"
        "."
        " "
        (MdParagraph
          (MdText
            "second"))))
    "\n"))"####
  );
}

// Parses a blockquote
#[test]
fn parse_blockquote_simple() {
  let tree = parse_body(
    r#"> quoted text
"#,
  );
  assert_eq!(
    tree,
    r####"(SourceFile
  (YamlFrontmatter
    ""
    "---"
    "\n"
    ""
    "---"
    "\n")
  (MdBody
    (MdBlockquote
      ">"
      " "
      (MdParagraph
        (MdText
          "quoted"
          " "
          "text")))
    "\n"))"####
  );
}

// Two consecutive `>` lines form a single blockquote
#[test]
fn parse_blockquote_multiline() {
  let tree = parse_body(
    r#"> line one
> line two
"#,
  );
  assert_eq!(
    tree,
    r####"(SourceFile
  (YamlFrontmatter
    ""
    "---"
    "\n"
    ""
    "---"
    "\n")
  (MdBody
    (MdBlockquote
      ">"
      " "
      (MdParagraph
        (MdText
          "line"
          " "
          "one")
        "\n"
        (MdText
          ">"
          " "
          "line"
          " "
          "two")))
    "\n"))"####
  );
}

// Parses a table
#[test]
fn parse_table_simple() {
  let tree = parse_body(
    r#"| a | b |
| --- | --- |
| 1 | 2 |
"#,
  );
  assert_eq!(
    tree,
    r####"(SourceFile
  (YamlFrontmatter
    ""
    "---"
    "\n"
    ""
    "---"
    "\n")
  (MdBody
    (MdTable
      (MdTableHeaderRow
        "|"
        (MdTableCell
          " "
          (MdText
            "a"
            " "
            "|"
            " "
            "b"
            " "
            "|")))
      "\n"
      (MdTableSeparatorRow
        "|"
        " "
        "---"
        " "
        "|"
        " "
        "---"
        " "
        "|")
      "\n"
      (MdTableDataRow
        "|"
        (MdTableCell
          " "
          (MdText
            "1"
            " "
            "|"
            " "
            "2"
            " "
            "|"))))
    "\n"))"####
  );
}

// Parses a container block
#[test]
fn parse_container_block_simple() {
  let tree = parse_body(
    r#"::: note
content
:::
"#,
  );
  assert_eq!(
    tree,
    r####"(SourceFile
  (YamlFrontmatter
    ""
    "---"
    "\n"
    ""
    "---"
    "\n")
  (MdBody
    (MdContainerBlock
      ":::"
      " "
      "note"
      "\n"
      (MdContainerSlot
        (MdParagraph
          (MdText
            "content")))
      "\n"
      ":::")
    "\n"))"####
  );
}

#[test]
fn parse_container_block_with_number_props() {
  let tree = parse_body(
    r#"::: grid {cols=2 rows=10}
content
:::
"#,
  );
  assert_eq!(
    tree,
    r####"(SourceFile
  (YamlFrontmatter
    ""
    "---"
    "\n"
    ""
    "---"
    "\n")
  (MdBody
    (MdContainerBlock
      ":::"
      " "
      "grid"
      (MdContainerPropBlock
        " "
        "{"
        (MdContainerPropItem
          "cols"
          "="
          "2")
        " "
        (MdContainerPropItem
          "rows"
          "="
          "10")
        "}")
      "\n"
      (MdContainerSlot
        (MdParagraph
          (MdText
            "content")))
      "\n"
      ":::")
    "\n"))"####
  );
}

// A well-formed numeric prop should not produce diagnostics
#[test]
fn parse_container_block_with_number_prop_no_diagnostics() {
  let (_, diags) = parse_body_with_diags(
    r#"::: grid {cols=2}
content
:::
"#,
  );
  assert!(
    diags.is_empty(),
    "should produce no diagnostics, got: {diags:?}"
  );
}

// FIXME: decimal not supported
#[test]
fn parse_container_block_with_decimal_number_prop() {
  let (_, diags) = parse_body_with_diags(
    r#"::: grid {ratio=2.5}
content
:::
"#,
  );
  assert!(
    diags
      .iter()
      .any(|d| matches!(d, Diagnostic::UnexpectedContainerPropItem { .. })),
    "decimal prop values are not supported, got: {diags:?}"
  );
}

// FIXME: Same story for a negative value: `-` is an MdSymbol, not part of the number...
#[test]
fn parse_container_block_with_negative_number_prop() {
  let (_, diags) = parse_body_with_diags(
    r#"::: grid {offset=-1}
content
:::
"#,
  );
  assert!(
    !diags.is_empty(),
    "negative prop values are not supported, expected a diagnostic"
  );
}

#[test]
fn parse_container_block_with_bare_boolean_prop() {
  let tree = parse_body(
    r#"::: card {collapsed bordered}
content
:::
"#,
  );
  assert_eq!(
    tree,
    r####"(SourceFile
  (YamlFrontmatter
    ""
    "---"
    "\n"
    ""
    "---"
    "\n")
  (MdBody
    (MdContainerBlock
      ":::"
      " "
      "card"
      (MdContainerPropBlock
        " "
        "{"
        (MdContainerPropItem
          "collapsed")
        " "
        (MdContainerPropItem
          "bordered")
        "}")
      "\n"
      (MdContainerSlot
        (MdParagraph
          (MdText
            "content")))
      "\n"
      ":::")
    "\n"))"####
  );
}

#[test]
fn parse_container_block_with_bare_boolean_prop_no_diagnostics() {
  let (_, diags) = parse_body_with_diags(
    r#"::: card {collapsed}
content
:::
"#,
  );
  assert!(
    diags.is_empty(),
    "should produce no diagnostics, got: {diags:?}"
  );
}

#[test]
fn parse_container_block_with_props_only() {
  let tree = parse_body(
    r#"::: card {title="Hello" variant="wide"}
content
:::
"#,
  );
  assert_eq!(
    tree,
    r####"(SourceFile
  (YamlFrontmatter
    ""
    "---"
    "\n"
    ""
    "---"
    "\n")
  (MdBody
    (MdContainerBlock
      ":::"
      " "
      "card"
      (MdContainerPropBlock
        " "
        "{"
        (MdContainerPropItem
          "title"
          "="
          "\""
          "Hello"
          "\"")
        " "
        (MdContainerPropItem
          "variant"
          "="
          "\""
          "wide"
          "\"")
        "}")
      "\n"
      (MdContainerSlot
        (MdParagraph
          (MdText
            "content")))
      "\n"
      ":::")
    "\n"))"####
  );
}

#[test]
fn parse_container_block_with_leading_slot_separator_no_diagnostics() {
  let (_, diags) = parse_body_with_diags(
    r#"::: tabs
=== one
content
:::
"#,
  );
  assert!(
    diags.is_empty(),
    "a container may start with a slot separator, got: {diags:?}"
  );
}

// Same, with a prop block between the label and the first separator.
#[test]
fn parse_container_block_with_props_and_leading_slot_separator_no_diagnostics() {
  let (_, diags) = parse_body_with_diags(
    r#"::: tabs {variant="pill"}
=== one
content
:::
"#,
  );
  assert!(
    diags.is_empty(),
    "a container with props may start with a slot separator, got: {diags:?}"
  );
}

// Two separators back to back: the slot between them is empty.
#[test]
fn parse_container_block_with_consecutive_slot_separators_no_diagnostics() {
  let (_, diags) = parse_body_with_diags(
    r#"::: tabs
=== one
=== two
content
:::
"#,
  );
  assert!(
    diags.is_empty(),
    "an empty slot between separators is allowed, got: {diags:?}"
  );
}

// A valueless prop (`key` with no `=`) is allowed
#[test]
fn parse_container_block_with_valueless_prop() {
  let tree = parse_body(
    r#"::: card {collapsed}
content
:::
"#,
  );
  assert_eq!(
    tree,
    r####"(SourceFile
  (YamlFrontmatter
    ""
    "---"
    "\n"
    ""
    "---"
    "\n")
  (MdBody
    (MdContainerBlock
      ":::"
      " "
      "card"
      (MdContainerPropBlock
        " "
        "{"
        (MdContainerPropItem
          "collapsed")
        "}")
      "\n"
      (MdContainerSlot
        (MdParagraph
          (MdText
            "content")))
      "\n"
      ":::")
    "\n"))"####
  );
}

// A well-formed prop block should not produce diagnostics
#[test]
fn parse_container_block_with_props_no_diagnostics() {
  let (_, diags) = parse_body_with_diags(
    r#"::: card {title="Hello" variant="wide"}
content
:::
"#,
  );
  assert!(
    diags.is_empty(),
    "should produce no diagnostics, got: {diags:?}"
  );
}

// An unclosed prop block reports UnclosedContainerPropBlock
#[test]
fn parse_container_block_with_unclosed_props() {
  let (_, diags) = parse_body_with_diags(
    r#"::: card {title="Hello"
content
:::
"#,
  );
  assert!(
    diags
      .iter()
      .any(|d| matches!(d, Diagnostic::UnclosedContainerPropBlock { .. })),
    "should report UnclosedContainerPropBlock, got: {diags:?}"
  );
}

// A non-identifier where a prop key is expected reports UnexpectedContainerPropItem
#[test]
fn parse_container_block_with_invalid_prop_key() {
  let (_, diags) = parse_body_with_diags(
    r#"::: card {"title"}
content
:::
"#,
  );
  assert!(
    diags
      .iter()
      .any(|d| matches!(d, Diagnostic::UnexpectedContainerPropItem { .. })),
    "should report UnexpectedContainerPropItem, got: {diags:?}"
  );
}

// A prop with `=` and no value should report MissingContainerPropValueAfterEq
#[test]
fn parse_container_block_with_missing_prop_value() {
  let (_, diags) = parse_body_with_diags(
    r#"::: card {title=}
content
:::
"#,
  );
  assert!(
    diags
      .iter()
      .any(|d| matches!(d, Diagnostic::MissingContainerPropValueAfterEq { .. })),
    "should report MissingContainerPropValueAfterEq, got: {diags:?}"
  );
}

// Containers with slots, no props
#[test]
fn parse_container_block_with_slots_only() {
  let tree = parse_body(
    r#"::: tabs
first
=== second
more
:::
"#,
  );
  assert_eq!(
    tree,
    r####"(SourceFile
  (YamlFrontmatter
    ""
    "---"
    "\n"
    ""
    "---"
    "\n")
  (MdBody
    (MdContainerBlock
      ":::"
      " "
      "tabs"
      "\n"
      (MdContainerSlot
        (MdParagraph
          (MdText
            "first")))
      "\n"
      (MdContainerSlotSeparator
        "==="
        " "
        "second"
        "\n")
      (MdContainerSlot
        (MdParagraph
          (MdText
            "more")))
      "\n"
      ":::")
    "\n"))"####
  );
}

// Tokens trailing a slot separator report UnexpectedContainerSlotSeparatorToken
#[test]
fn parse_container_block_with_slot_separator_trailing_tokens() {
  let (_, diags) = parse_body_with_diags(
    r#"::: tabs
first
=== second extra
more
:::
"#,
  );
  assert!(
    diags
      .iter()
      .any(|d| matches!(d, Diagnostic::UnexpectedContainerSlotSeparatorToken { .. })),
    "should report UnexpectedContainerSlotSeparatorToken, got: {diags:?}"
  );
}

// Conatiners with both props and slots
#[test]
fn parse_container_block_with_props_and_slots() {
  let tree = parse_body(
    r#"::: tabs {variant="pill"}
first
=== second
more
:::
"#,
  );
  assert_eq!(
    tree,
    r####"(SourceFile
  (YamlFrontmatter
    ""
    "---"
    "\n"
    ""
    "---"
    "\n")
  (MdBody
    (MdContainerBlock
      ":::"
      " "
      "tabs"
      (MdContainerPropBlock
        " "
        "{"
        (MdContainerPropItem
          "variant"
          "="
          "\""
          "pill"
          "\"")
        "}")
      "\n"
      (MdContainerSlot
        (MdParagraph
          (MdText
            "first")))
      "\n"
      (MdContainerSlotSeparator
        "==="
        " "
        "second"
        "\n")
      (MdContainerSlot
        (MdParagraph
          (MdText
            "more")))
      "\n"
      ":::")
    "\n"))"####
  );
}

// Neither props nor slots in container
#[test]
fn parse_container_block_without_props_or_slots() {
  let tree = parse_body(
    r#"::: note Plain Title
content
:::
"#,
  );
  assert_eq!(
    tree,
    r####"(SourceFile
  (YamlFrontmatter
    ""
    "---"
    "\n"
    ""
    "---"
    "\n")
  (MdBody
    (MdContainerBlock
      ":::"
      " "
      "note"
      " "
      "Plain"
      " "
      "Title"
      "\n"
      (MdContainerSlot
        (MdParagraph
          (MdText
            "content")))
      "\n"
      ":::")
    "\n"))"####
  );
}

// Container with title and multi-line content should produce no diagnostics
#[test]
fn parse_container_block_with_title_no_diagnostics() {
  let (_, diags) = parse_body_with_diags(
    r#"::: tip Current status
Design mockups are complete. Authentication is currently being implemented.
Integration tests are next in the queue once auth is merged.
:::
"#,
  );
  assert!(
    diags.is_empty(),
    "should produce no diagnostics, got: {diags:?}"
  );
}

// Multiple containers with titles should produce no diagnostics
#[test]
fn parse_multiple_containers_with_titles_no_diagnostics() {
  let (_, diags) = parse_body_with_diags(
    r#"::: details Architecture overview
Some content here.
:::

::: tip Current status
More content here.
:::
"#,
  );
  assert!(
    diags.is_empty(),
    "should produce no diagnostics, got: {diags:?}"
  );
}

// Parses a container block with a title
#[test]
fn parse_container_block_with_title() {
  let tree = parse_body(
    r#"::: details My Custom Title
content here
:::
"#,
  );
  assert_eq!(
    tree,
    r####"(SourceFile
  (YamlFrontmatter
    ""
    "---"
    "\n"
    ""
    "---"
    "\n")
  (MdBody
    (MdContainerBlock
      ":::"
      " "
      "details"
      " "
      "My"
      " "
      "Custom"
      " "
      "Title"
      "\n"
      (MdContainerSlot
        (MdParagraph
          (MdText
            "content"
            " "
            "here")))
      "\n"
      ":::")
    "\n"))"####
  );
}

#[test]
fn parse_container_block_empty() {
  let tree = parse_body(
    r#"::: note
:::
"#,
  );
  assert_eq!(
    tree,
    r####"(SourceFile
  (YamlFrontmatter
    ""
    "---"
    "\n"
    ""
    "---"
    "\n")
  (MdBody
    (MdContainerBlock
      ":::"
      " "
      "note"
      "\n"
      ":::")
    "\n"))"####
  );
}

#[test]
fn parse_container_shorthand_simple() {
  let (tree, diags) = parse_body_with_diags(
    r#"[[toc]]
"#,
  );
  assert_eq!(
    tree,
    r####"(SourceFile
  (YamlFrontmatter
    ""
    "---"
    "\n"
    ""
    "---"
    "\n")
  (MdBody
    (MdContainerShorthand
      "["
      "["
      "toc"
      "]"
      "]")
    "\n"))"####
  );
  assert!(
    diags.is_empty(),
    "should produce no diagnostics, got: {diags:?}"
  );
}

#[test]
fn parse_container_shorthand_with_props() {
  let (tree, diags) = parse_body_with_diags(
    r#"[[grid {cols=2}]]
"#,
  );
  assert_eq!(
    tree,
    r####"(SourceFile
  (YamlFrontmatter
    ""
    "---"
    "\n"
    ""
    "---"
    "\n")
  (MdBody
    (MdContainerShorthand
      "["
      "["
      "grid"
      (MdContainerPropBlock
        " "
        "{"
        (MdContainerPropItem
          "cols"
          "="
          "2")
        "}")
      "]"
      "]")
    "\n"))"####
  );
  assert!(
    diags.is_empty(),
    "should produce no diagnostics, got: {diags:?}"
  );
}

#[test]
fn parse_container_block_kebab_case() {
  let (tree, diags) = parse_body_with_diags(
    r#"::: directory-index
:::
"#,
  );
  assert_eq!(
    tree,
    r####"(SourceFile
  (YamlFrontmatter
    ""
    "---"
    "\n"
    ""
    "---"
    "\n")
  (MdBody
    (MdContainerBlock
      ":::"
      " "
      "directory"
      "-"
      "index"
      "\n"
      ":::")
    "\n"))"####
  );
  assert!(
    diags.is_empty(),
    "should produce no diagnostics, got: {diags:?}"
  );
}

#[test]
fn parse_container_shorthand_kebab_case() {
  let (tree, diags) = parse_body_with_diags(
    r#"[[directory-index]]
"#,
  );
  assert_eq!(
    tree,
    r####"(SourceFile
  (YamlFrontmatter
    ""
    "---"
    "\n"
    ""
    "---"
    "\n")
  (MdBody
    (MdContainerShorthand
      "["
      "["
      "directory"
      "-"
      "index"
      "]"
      "]")
    "\n"))"####
  );
  assert!(
    diags.is_empty(),
    "should produce no diagnostics, got: {diags:?}"
  );
}

// Parses a fenced code block
#[test]
fn parse_code_block_simple() {
  let tree = parse_body(
    r#"```
code
```
"#,
  );
  assert_eq!(
    tree,
    r####"(SourceFile
  (YamlFrontmatter
    ""
    "---"
    "\n"
    ""
    "---"
    "\n")
  (MdBody
    (CodeBlock
      "```\ncode\n```")
    "\n"))"####
  );
}

// Code block with line range indicator parses as CodeBlock
#[test]
fn parse_code_block_with_line_ranges() {
  let (tree, diags) = parse_body_with_diags(
    r#"```js{1,3,5-8}
code
```

```json{2,5}
{
  "sub": "user-id",
  "exp": 1720000000,
  "iat": 1719996400,
  "scope": "read write"
}
```
"#,
  );
  assert_eq!(
    tree,
    r####"(SourceFile
  (YamlFrontmatter
    ""
    "---"
    "\n"
    ""
    "---"
    "\n")
  (MdBody
    (CodeBlock
      "```js{1,3,5-8}\ncode\n```")
    "\n"
    "\n"
    (CodeBlock
      "```json{2,5}\n{\n  \"sub\": \"user-id\",\n  \"exp\": 1720000000,\n  \"iat\": 1719996400,\n  \"scope\": \"read write\"\n}\n```")
    "\n"))"####
  );
  assert_eq!(diags, &[] as &[Diagnostic], "expected no diagnostics");
}

// Parses an empty body
#[test]
fn parse_body_empty() {
  let tree = parse_body("");
  assert_eq!(
    tree,
    r####"(SourceFile
  (YamlFrontmatter
    ""
    "---"
    "\n"
    ""
    "---"
    "\n")
  (MdBody))"####
  );
}

// Parses a body with only blank lines
#[test]
fn parse_body_only_blank_lines() {
  let tree = parse_body("\n\n\n");
  assert_eq!(
    tree,
    r####"(SourceFile
  (YamlFrontmatter
    ""
    "---"
    "\n"
    ""
    "---"
    "\n")
  (MdBody
    "\n"
    "\n"
    "\n"))"####
  );
}

// Simple inline elements

// Parses a link
#[test]
fn parse_link_simple() {
  let tree = parse_body(
    r#"[text](url)
"#,
  );
  assert_eq!(
    tree,
    r####"(SourceFile
  (YamlFrontmatter
    ""
    "---"
    "\n"
    ""
    "---"
    "\n")
  (MdBody
    (MdParagraph
      (MdLink
        "["
        (MdText
          "text")
        "]"
        "("
        (MdText
          "url")
        ")"))
    "\n"))"####
  );
}

// Parses a media embed
#[test]
fn parse_media_simple() {
  let tree = parse_body(
    r#"![alt](image.png)
"#,
  );
  assert_eq!(
    tree,
    r####"(SourceFile
  (YamlFrontmatter
    ""
    "---"
    "\n"
    ""
    "---"
    "\n")
  (MdBody
    (MdMedia
      "!"
      "["
      (MdText
        "alt")
      "]"
      "("
      (MdText
        "image"
        "."
        "png")
      ")")
    "\n"))"####
  );
}

// Parses bold text
#[test]
fn parse_bold_simple() {
  let tree = parse_body(
    r#"**bold**
"#,
  );
  assert_eq!(
    tree,
    r####"(SourceFile
  (YamlFrontmatter
    ""
    "---"
    "\n"
    ""
    "---"
    "\n")
  (MdBody
    (MdParagraph
      (MdBold
        "**"
        (MdText
          "bold")
        "**"))
    "\n"))"####
  );
}

// Parses italic text
#[test]
fn parse_italic_simple() {
  let tree = parse_body(
    r#"*italic*
"#,
  );
  assert_eq!(
    tree,
    r####"(SourceFile
  (YamlFrontmatter
    ""
    "---"
    "\n"
    ""
    "---"
    "\n")
  (MdBody
    (MdParagraph
      (MdItalic
        "*"
        (MdText
          "italic")
        "*"))
    "\n"))"####
  );
}

// Parses bold italic text
#[test]
fn parse_bold_italic_simple() {
  let tree = parse_body(
    r#"***bold italic***
"#,
  );
  assert_eq!(
    tree,
    r####"(SourceFile
  (YamlFrontmatter
    ""
    "---"
    "\n"
    ""
    "---"
    "\n")
  (MdBody
    (MdParagraph
      (MdBoldItalic
        "***"
        (MdText
          "bold"
          " "
          "italic")
        "***"))
    "\n"))"####
  );
}

// Parses strikethrough text
#[test]
fn parse_strikethrough_simple() {
  let tree = parse_body(
    r#"~~struck~~
"#,
  );
  assert_eq!(
    tree,
    r####"(SourceFile
  (YamlFrontmatter
    ""
    "---"
    "\n"
    ""
    "---"
    "\n")
  (MdBody
    (MdParagraph
      (MdStrikethrough
        "~~"
        (MdText
          "struck")
        "~~"))
    "\n"))"####
  );
}

// Parses inline code
#[test]
fn parse_inline_code_simple() {
  let tree = parse_body(
    r#"`code`
"#,
  );
  assert_eq!(
    tree,
    r####"(SourceFile
  (YamlFrontmatter
    ""
    "---"
    "\n"
    ""
    "---"
    "\n")
  (MdBody
    (MdParagraph
      (InlineCode
        "`code`"))
    "\n"))"####
  );
}

// Inline elements inside block elements

// Parses a paragraph with italic and link
#[test]
fn parse_paragraph_with_inline() {
  let tree = parse_body(
    r#"Hello *world* and [link](url)
"#,
  );
  assert_eq!(
    tree,
    r####"(SourceFile
  (YamlFrontmatter
    ""
    "---"
    "\n"
    ""
    "---"
    "\n")
  (MdBody
    (MdParagraph
      (MdText
        "Hello"
        " ")
      (MdItalic
        "*"
        (MdText
          "world")
        "*")
      (MdText
        " "
        "and"
        " ")
      (MdLink
        "["
        (MdText
          "link")
        "]"
        "("
        (MdText
          "url")
        ")"))
    "\n"))"####
  );
}

// Parses bold in paragraph
#[test]
fn parse_bold_in_paragraph() {
  let tree = parse_body(
    r#"Hello **world**!
"#,
  );
  assert_eq!(
    tree,
    r####"(SourceFile
  (YamlFrontmatter
    ""
    "---"
    "\n"
    ""
    "---"
    "\n")
  (MdBody
    (MdParagraph
      (MdText
        "Hello"
        " ")
      (MdBold
        "**"
        (MdText
          "world")
        "**")
      (MdText
        "!"))
    "\n"))"####
  );
}

// Parses italic in heading
#[test]
fn parse_italic_in_heading() {
  let tree = parse_body(
    r#"# *emphasis* title
"#,
  );
  assert_eq!(
    tree,
    r####"(SourceFile
  (YamlFrontmatter
    ""
    "---"
    "\n"
    ""
    "---"
    "\n")
  (MdBody
    (MdHeading
      "#"
      " "
      (MdItalic
        "*"
        (MdText
          "emphasis")
        "*")
      (MdText
        " "
        "title"))
    "\n"))"####
  );
}

// Parses strikethrough in heading
#[test]
fn parse_heading_with_strikethrough() {
  let tree = parse_body(
    r#"# ~~old~~ new
"#,
  );
  assert_eq!(
    tree,
    r####"(SourceFile
  (YamlFrontmatter
    ""
    "---"
    "\n"
    ""
    "---"
    "\n")
  (MdBody
    (MdHeading
      "#"
      " "
      (MdStrikethrough
        "~~"
        (MdText
          "old")
        "~~")
      (MdText
        " "
        "new"))
    "\n"))"####
  );
}

// Parses link in blockquote
#[test]
fn parse_blockquote_with_link() {
  let tree = parse_body(
    r#"> see [here](url)
"#,
  );
  assert_eq!(
    tree,
    r####"(SourceFile
  (YamlFrontmatter
    ""
    "---"
    "\n"
    ""
    "---"
    "\n")
  (MdBody
    (MdBlockquote
      ">"
      " "
      (MdParagraph
        (MdText
          "see"
          " ")
        (MdLink
          "["
          (MdText
            "here")
          "]"
          "("
          (MdText
            "url")
          ")")))
    "\n"))"####
  );
}

// Parses strikethrough in blockquote
#[test]
fn parse_strikethrough_in_blockquote() {
  let tree = parse_body(
    r#"> ~~removed~~ text
"#,
  );
  assert_eq!(
    tree,
    r####"(SourceFile
  (YamlFrontmatter
    ""
    "---"
    "\n"
    ""
    "---"
    "\n")
  (MdBody
    (MdBlockquote
      ">"
      " "
      (MdParagraph
        (MdStrikethrough
          "~~"
          (MdText
            "removed")
          "~~")
        (MdText
          " "
          "text")))
    "\n"))"####
  );
}

// Parses bold in list item
#[test]
fn parse_list_with_bold() {
  let tree = parse_body(
    r#"- **bold item**
- normal
"#,
  );
  assert_eq!(
    tree,
    r####"(SourceFile
  (YamlFrontmatter
    ""
    "---"
    "\n"
    ""
    "---"
    "\n")
  (MdBody
    (MdBulletList
      (MdBulletListItem
        "-"
        " "
        (MdParagraph
          (MdBold
            "**"
            (MdText
              "bold"
              " "
              "item")
            "**")))
      "\n"
      (MdBulletListItem
        "-"
        " "
        (MdParagraph
          (MdText
            "normal"))))
    "\n"))"####
  );
}

// Parses link in list item
#[test]
fn parse_link_in_list_item() {
  let tree = parse_body(
    r#"- see [here](url) for info
"#,
  );
  assert_eq!(
    tree,
    r####"(SourceFile
  (YamlFrontmatter
    ""
    "---"
    "\n"
    ""
    "---"
    "\n")
  (MdBody
    (MdBulletList
      (MdBulletListItem
        "-"
        " "
        (MdParagraph
          (MdText
            "see"
            " ")
          (MdLink
            "["
            (MdText
              "here")
            "]"
            "("
            (MdText
              "url")
            ")")
          (MdText
            " "
            "for"
            " "
            "info"))))
    "\n"))"####
  );
}

// Parses media in paragraph
#[test]
fn parse_media_in_paragraph() {
  let tree = parse_body(
    r#"See ![photo](img.png) here
"#,
  );
  assert_eq!(
    tree,
    r####"(SourceFile
  (YamlFrontmatter
    ""
    "---"
    "\n"
    ""
    "---"
    "\n")
  (MdBody
    (MdParagraph
      (MdText
        "See"
        " ")
      (MdMedia
        "!"
        "["
        (MdText
          "photo")
        "]"
        "("
        (MdText
          "img"
          "."
          "png")
        ")")
      (MdText
        " "
        "here"))
    "\n"))"####
  );
}

// Parses nested bold inside italic
#[test]
fn parse_nested_bold_in_italic() {
  let tree = parse_body(
    r#"*hello **world***
"#,
  );
  assert_eq!(
    tree,
    r####"(SourceFile
  (YamlFrontmatter
    ""
    "---"
    "\n"
    ""
    "---"
    "\n")
  (MdBody
    (MdParagraph
      (MdItalic
        "*"
        (MdText
          "hello"
          " ")
        (MdBold
          "**"
          (MdText
            "world")
          (MdBoldItalic
            "***"
            "\n"))))))"####
  );
}

// Parses bold in table cells
#[test]
fn parse_table_with_bold_cells() {
  let tree = parse_body(
    r#"| **h** |
| --- |
| cell |
"#,
  );
  assert_eq!(
    tree,
    r####"(SourceFile
  (YamlFrontmatter
    ""
    "---"
    "\n"
    ""
    "---"
    "\n")
  (MdBody
    (MdTable
      (MdTableHeaderRow
        "|"
        (MdTableCell
          " "
          (MdBold
            "**"
            (MdText
              "h")
            "**")
          (MdText
            " "
            "|")))
      "\n"
      (MdTableSeparatorRow
        "|"
        " "
        "---"
        " "
        "|")
      "\n"
      (MdTableDataRow
        "|"
        (MdTableCell
          " "
          (MdText
            "cell"
            " "
            "|"))))
    "\n"))"####
  );
}

// Multiple block elements in sequence

// Parses multiple paragraphs
#[test]
fn parse_paragraph_multiple() {
  let tree = parse_body(
    r#"First paragraph.

Second paragraph.
"#,
  );
  assert_eq!(
    tree,
    r####"(SourceFile
  (YamlFrontmatter
    ""
    "---"
    "\n"
    ""
    "---"
    "\n")
  (MdBody
    (MdParagraph
      (MdText
        "First"
        " "
        "paragraph"
        "."))
    "\n"
    "\n"
    (MdParagraph
      (MdText
        "Second"
        " "
        "paragraph"
        "."))
    "\n"))"####
  );
}

// Parses heading, paragraph, heading sequence
#[test]
fn parse_heading_paragraph_heading() {
  let tree = parse_body(
    r#"# One

text

# Two
"#,
  );
  assert_eq!(
    tree,
    r####"(SourceFile
  (YamlFrontmatter
    ""
    "---"
    "\n"
    ""
    "---"
    "\n")
  (MdBody
    (MdHeading
      "#"
      " "
      (MdText
        "One"))
    "\n"
    "\n"
    (MdParagraph
      (MdText
        "text"))
    "\n"
    "\n"
    (MdHeading
      "#"
      " "
      (MdText
        "Two"))
    "\n"))"####
  );
}

// Parses heading, paragraph, list sequence
#[test]
fn parse_heading_then_paragraph_then_list() {
  let tree = parse_body(
    r#"# Title

Some text.

- a
- b
"#,
  );
  assert_eq!(
    tree,
    r####"(SourceFile
  (YamlFrontmatter
    ""
    "---"
    "\n"
    ""
    "---"
    "\n")
  (MdBody
    (MdHeading
      "#"
      " "
      (MdText
        "Title"))
    "\n"
    "\n"
    (MdParagraph
      (MdText
        "Some"
        " "
        "text"
        "."))
    "\n"
    "\n"
    (MdBulletList
      (MdBulletListItem
        "-"
        " "
        (MdParagraph
          (MdText
            "a")))
      "\n"
      (MdBulletListItem
        "-"
        " "
        (MdParagraph
          (MdText
            "b"))))
    "\n"))"####
  );
}

// Parses table followed by paragraph
#[test]
fn parse_table_then_paragraph() {
  let tree = parse_body(
    r#"| h |
| - |
| c |

text
"#,
  );
  assert_eq!(
    tree,
    r####"(SourceFile
  (YamlFrontmatter
    ""
    "---"
    "\n"
    ""
    "---"
    "\n")
  (MdBody
    (MdTable
      (MdTableHeaderRow
        "|"
        (MdTableCell
          " "
          (MdText
            "h"
            " "
            "|")))
      "\n"
      (MdTableSeparatorRow
        "|"
        " "
        "-"
        " "
        "|")
      "\n"
      (MdTableDataRow
        "|"
        (MdTableCell
          " "
          (MdText
            "c"
            " "
            "|"))))
    "\n"
    "\n"
    (MdParagraph
      (MdText
        "text"))
    "\n"))"####
  );
}

// Parses blockquote followed by bullet list
#[test]
fn parse_blockquote_then_list() {
  let tree = parse_body(
    r#"> quoted

- listed
"#,
  );
  assert_eq!(
    tree,
    r####"(SourceFile
  (YamlFrontmatter
    ""
    "---"
    "\n"
    ""
    "---"
    "\n")
  (MdBody
    (MdBlockquote
      ">"
      " "
      (MdParagraph
        (MdText
          "quoted")))
    "\n"
    "\n"
    (MdBulletList
      (MdBulletListItem
        "-"
        " "
        (MdParagraph
          (MdText
            "listed"))))
    "\n"))"####
  );
}

// Parses bullet list followed by heading
#[test]
fn parse_list_then_heading() {
  let tree = parse_body(
    r#"- a
- b

# After
"#,
  );
  assert_eq!(
    tree,
    r####"(SourceFile
  (YamlFrontmatter
    ""
    "---"
    "\n"
    ""
    "---"
    "\n")
  (MdBody
    (MdBulletList
      (MdBulletListItem
        "-"
        " "
        (MdParagraph
          (MdText
            "a")))
      "\n"
      (MdBulletListItem
        "-"
        " "
        (MdParagraph
          (MdText
            "b"))))
    "\n"
    "\n"
    (MdHeading
      "#"
      " "
      (MdText
        "After"))
    "\n"))"####
  );
}

// Parses ordered list followed by unordered list
#[test]
fn parse_ordered_then_unordered() {
  let tree = parse_body(
    r#"1. first
2. second

- bullet
"#,
  );
  assert_eq!(
    tree,
    r####"(SourceFile
  (YamlFrontmatter
    ""
    "---"
    "\n"
    ""
    "---"
    "\n")
  (MdBody
    (MdOrderedList
      (MdOrderedListItem
        "1"
        "."
        " "
        (MdParagraph
          (MdText
            "first")))
      "\n"
      (MdOrderedListItem
        "2"
        "."
        " "
        (MdParagraph
          (MdText
            "second"))))
    "\n"
    "\n"
    (MdBulletList
      (MdBulletListItem
        "-"
        " "
        (MdParagraph
          (MdText
            "bullet"))))
    "\n"))"####
  );
}

// Mixed inline formatting

// Parses interpolation in paragraph
#[test]
fn parse_interpolation_in_paragraph() {
  let tree = parse_body(
    r#"hello ${name} world
"#,
  );
  assert_eq!(
    tree,
    r####"(SourceFile
  (YamlFrontmatter
    ""
    "---"
    "\n"
    ""
    "---"
    "\n")
  (MdBody
    (MdParagraph
      (MdText
        "hello"
        " ")
      (InterpFragment
        "${"
        (IdentLit
          "name")
        "}")
      (MdText
        " "
        "world"))
    "\n"))"####
  );
}

// Parses inline math
#[test]
fn parse_inline_math_simple() {
  let tree = parse_body(
    r#"the formula $E=mc^2$ is
"#,
  );
  assert_eq!(
    tree,
    r####"(SourceFile
  (YamlFrontmatter
    ""
    "---"
    "\n"
    ""
    "---"
    "\n")
  (MdBody
    (MdParagraph
      (MdText
        "the"
        " "
        "formula"
        " ")
      (InlineMath
        "$E=mc^2$")
      (MdText
        " "
        "is"))
    "\n"))"####
  );
}

// Parses bold and italic in one paragraph
#[test]
fn parse_bold_and_italic_mixed() {
  let tree = parse_body(
    r#"**bold** and *italic* text
"#,
  );
  assert_eq!(
    tree,
    r####"(SourceFile
  (YamlFrontmatter
    ""
    "---"
    "\n"
    ""
    "---"
    "\n")
  (MdBody
    (MdParagraph
      (MdBold
        "**"
        (MdText
          "bold")
        "**")
      (MdText
        " "
        "and"
        " ")
      (MdItalic
        "*"
        (MdText
          "italic")
        "*")
      (MdText
        " "
        "text"))
    "\n"))"####
  );
}

// Parses bold and strikethrough in one paragraph
#[test]
fn parse_bold_then_strikethrough() {
  let tree = parse_body(
    r#"**bold** ~~struck~~ end
"#,
  );
  assert_eq!(
    tree,
    r####"(SourceFile
  (YamlFrontmatter
    ""
    "---"
    "\n"
    ""
    "---"
    "\n")
  (MdBody
    (MdParagraph
      (MdBold
        "**"
        (MdText
          "bold")
        "**")
      (MdText
        " ")
      (MdStrikethrough
        "~~"
        (MdText
          "struck")
        "~~")
      (MdText
        " "
        "end"))
    "\n"))"####
  );
}

// Parses ordered list with links
#[test]
fn parse_ordered_list_with_links() {
  let tree = parse_body(
    r#"1. [first](a)
2. [second](b)
"#,
  );
  assert_eq!(
    tree,
    r####"(SourceFile
  (YamlFrontmatter
    ""
    "---"
    "\n"
    ""
    "---"
    "\n")
  (MdBody
    (MdOrderedList
      (MdOrderedListItem
        "1"
        "."
        " "
        (MdParagraph
          (MdLink
            "["
            (MdText
              "first")
            "]"
            "("
            (MdText
              "a")
            ")")))
      "\n"
      (MdOrderedListItem
        "2"
        "."
        " "
        (MdParagraph
          (MdLink
            "["
            (MdText
              "second")
            "]"
            "("
            (MdText
              "b")
            ")"))))
    "\n"))"####
  );
}

// Parses multiple links in paragraph
#[test]
fn parse_multiple_links_in_paragraph() {
  let tree = parse_body(
    r#"[a](x) and [b](y) and [c](z)
"#,
  );
  assert_eq!(
    tree,
    r####"(SourceFile
  (YamlFrontmatter
    ""
    "---"
    "\n"
    ""
    "---"
    "\n")
  (MdBody
    (MdParagraph
      (MdLink
        "["
        (MdText
          "a")
        "]"
        "("
        (MdText
          "x")
        ")")
      (MdText
        " "
        "and"
        " ")
      (MdLink
        "["
        (MdText
          "b")
        "]"
        "("
        (MdText
          "y")
        ")")
      (MdText
        " "
        "and"
        " ")
      (MdLink
        "["
        (MdText
          "c")
        "]"
        "("
        (MdText
          "z")
        ")"))
    "\n"))"####
  );
}

// Parses links in table cells
#[test]
fn parse_table_with_links() {
  let tree = parse_body(
    r#"| [a](x) | [b](y) |
| --- | --- |
| 1 | 2 |
"#,
  );
  assert_eq!(
    tree,
    r####"(SourceFile
  (YamlFrontmatter
    ""
    "---"
    "\n"
    ""
    "---"
    "\n")
  (MdBody
    (MdTable
      (MdTableHeaderRow
        "|"
        (MdTableCell
          " "
          (MdLink
            "["
            (MdText
              "a")
            "]"
            "("
            (MdText
              "x")
            ")")
          (MdText
            " "
            "|"
            " ")
          (MdLink
            "["
            (MdText
              "b")
            "]"
            "("
            (MdText
              "y")
            ")")
          (MdText
            " "
            "|")))
      "\n"
      (MdTableSeparatorRow
        "|"
        " "
        "---"
        " "
        "|"
        " "
        "---"
        " "
        "|")
      "\n"
      (MdTableDataRow
        "|"
        (MdTableCell
          " "
          (MdText
            "1"
            " "
            "|"
            " "
            "2"
            " "
            "|"))))
    "\n"))"####
  );
}

#[test]
fn parse_table_indented_rows() {
  let tree = parse_body(
    r#"  | a | b |
    | --- | --- |
  | 1 | 2 |
"#,
  );
  assert_eq!(
    tree,
    r####"(SourceFile
  (YamlFrontmatter
    ""
    "---"
    "\n"
    ""
    "---"
    "\n")
  (MdBody
    (MdTable
      " "
      " "
      (MdTableHeaderRow
        "|"
        (MdTableCell
          " "
          (MdText
            "a"
            " "
            "|"
            " "
            "b"
            " "
            "|")))
      "\n"
      " "
      " "
      " "
      " "
      (MdTableSeparatorRow
        "|"
        " "
        "---"
        " "
        "|"
        " "
        "---"
        " "
        "|")
      "\n"
      " "
      " "
      (MdTableDataRow
        "|"
        (MdTableCell
          " "
          (MdText
            "1"
            " "
            "|"
            " "
            "2"
            " "
            "|"))))
    "\n"))"####
  );
}

#[test]
fn parse_paragraph_then_indented_table() {
  let tree = parse_body(
    r#"hello world
  | a | b |
  | --- | --- |
  | 1 | 2 |
"#,
  );
  assert_eq!(
    tree,
    r####"(SourceFile
  (YamlFrontmatter
    ""
    "---"
    "\n"
    ""
    "---"
    "\n")
  (MdBody
    (MdParagraph
      (MdText
        "hello"
        " "
        "world"))
    "\n"
    (MdTable
      " "
      " "
      (MdTableHeaderRow
        "|"
        (MdTableCell
          " "
          (MdText
            "a"
            " "
            "|"
            " "
            "b"
            " "
            "|")))
      "\n"
      " "
      " "
      (MdTableSeparatorRow
        "|"
        " "
        "---"
        " "
        "|"
        " "
        "---"
        " "
        "|")
      "\n"
      " "
      " "
      (MdTableDataRow
        "|"
        (MdTableCell
          " "
          (MdText
            "1"
            " "
            "|"
            " "
            "2"
            " "
            "|"))))
    "\n"))"####
  );
}

#[test]
fn parse_indented_table_then_paragraph() {
  let tree = parse_body(
    r#"  | a | b |
  | --- | --- |
  | 1 | 2 |
hello world
"#,
  );
  assert_eq!(
    tree,
    r####"(SourceFile
  (YamlFrontmatter
    ""
    "---"
    "\n"
    ""
    "---"
    "\n")
  (MdBody
    (MdTable
      " "
      " "
      (MdTableHeaderRow
        "|"
        (MdTableCell
          " "
          (MdText
            "a"
            " "
            "|"
            " "
            "b"
            " "
            "|")))
      "\n"
      " "
      " "
      (MdTableSeparatorRow
        "|"
        " "
        "---"
        " "
        "|"
        " "
        "---"
        " "
        "|")
      "\n"
      " "
      " "
      (MdTableDataRow
        "|"
        (MdTableCell
          " "
          (MdText
            "1"
            " "
            "|"
            " "
            "2"
            " "
            "|"))))
    "\n"
    (MdParagraph
      (MdText
        "hello"
        " "
        "world"))
    "\n"))"####
  );
}

#[test]
fn parse_table_mixed_indentation() {
  let tree = parse_body(
    r#"| a | b |
    | --- | --- |
  | 1 | 2 |
"#,
  );
  assert_eq!(
    tree,
    r####"(SourceFile
  (YamlFrontmatter
    ""
    "---"
    "\n"
    ""
    "---"
    "\n")
  (MdBody
    (MdTable
      (MdTableHeaderRow
        "|"
        (MdTableCell
          " "
          (MdText
            "a"
            " "
            "|"
            " "
            "b"
            " "
            "|")))
      "\n"
      " "
      " "
      " "
      " "
      (MdTableSeparatorRow
        "|"
        " "
        "---"
        " "
        "|"
        " "
        "---"
        " "
        "|")
      "\n"
      " "
      " "
      (MdTableDataRow
        "|"
        (MdTableCell
          " "
          (MdText
            "1"
            " "
            "|"
            " "
            "2"
            " "
            "|"))))
    "\n"))"####
  );
}

#[test]
fn parse_indented_table_in_list() {
  let tree = parse_body(
    r#"- item

  | a | b |
  | --- | --- |
  | 1 | 2 |
- item2
"#,
  );
  assert_eq!(
    tree,
    r####"(SourceFile
  (YamlFrontmatter
    ""
    "---"
    "\n"
    ""
    "---"
    "\n")
  (MdBody
    (MdBulletList
      (MdBulletListItem
        "-"
        " "
        (MdParagraph
          (MdText
            "item"))
        "\n"
        "\n"
        " "
        " "
        (MdTable
          (MdTableHeaderRow
            "|"
            (MdTableCell
              " "
              (MdText
                "a"
                " "
                "|"
                " "
                "b"
                " "
                "|")))
          "\n"
          " "
          " "
          (MdTableSeparatorRow
            "|"
            " "
            "---"
            " "
            "|"
            " "
            "---"
            " "
            "|")
          "\n"
          " "
          " "
          (MdTableDataRow
            "|"
            (MdTableCell
              " "
              (MdText
                "1"
                " "
                "|"
                " "
                "2"
                " "
                "|")))))
      "\n"
      (MdBulletListItem
        "-"
        " "
        (MdParagraph
          (MdText
            "item"
            "2"))))
    "\n"))"####
  );
}

#[test]
fn parse_indented_table_in_blockquote() {
  let tree = parse_body(
    r#"> | a | b |
> | --- | --- |
> | 1 | 2 |
"#,
  );
  assert_eq!(
    tree,
    r####"(SourceFile
  (YamlFrontmatter
    ""
    "---"
    "\n"
    ""
    "---"
    "\n")
  (MdBody
    (MdBlockquote
      ">"
      " "
      (MdTable
        (MdTableHeaderRow
          "|"
          (MdTableCell
            " "
            (MdText
              "a"
              " "
              "|"
              " "
              "b"
              " "
              "|")))
        "\n"
        ">"
        " "
        (MdTableSeparatorRow
          "|"
          " "
          "---"
          " "
          "|"
          " "
          "---"
          " "
          "|")
        "\n"
        ">"
        " "
        (MdTableDataRow
          "|"
          (MdTableCell
            " "
            (MdText
              "1"
              " "
              "|"
              " "
              "2"
              " "
              "|")))))
    "\n"))"####
  );
}

#[test]
fn parse_indented_table_in_list_with_continuation() {
  let tree = parse_body(
    r#"- parent

  | a | b |
  | --- | --- |
  | 1 | 2 |

  continued
"#,
  );
  assert_eq!(
    tree,
    r####"(SourceFile
  (YamlFrontmatter
    ""
    "---"
    "\n"
    ""
    "---"
    "\n")
  (MdBody
    (MdBulletList
      (MdBulletListItem
        "-"
        " "
        (MdParagraph
          (MdText
            "parent"))
        "\n"
        "\n"
        " "
        " "
        (MdTable
          (MdTableHeaderRow
            "|"
            (MdTableCell
              " "
              (MdText
                "a"
                " "
                "|"
                " "
                "b"
                " "
                "|")))
          "\n"
          " "
          " "
          (MdTableSeparatorRow
            "|"
            " "
            "---"
            " "
            "|"
            " "
            "---"
            " "
            "|")
          "\n"
          " "
          " "
          (MdTableDataRow
            "|"
            (MdTableCell
              " "
              (MdText
                "1"
                " "
                "|"
                " "
                "2"
                " "
                "|"))))
        "\n"
        "\n"
        " "
        " "
        (MdParagraph
          (MdText
            "continued"))))
    "\n"))"####
  );
}

#[test]
fn parse_container_after_list_item_no_diagnostics() {
  let (tree, diags) = parse_body_with_diags(
    r#"- item

    indented content

::: details Title

content

:::

## Heading
"#,
  );
  assert!(
    diags.is_empty(),
    "should produce no diagnostics, got: {diags:?}"
  );
  assert_eq!(
    tree,
    r####"(SourceFile
  (YamlFrontmatter
    ""
    "---"
    "\n"
    ""
    "---"
    "\n")
  (MdBody
    (MdBulletList
      (MdBulletListItem
        "-"
        " "
        (MdParagraph
          (MdText
            "item"))
        "\n"
        "\n"
        " "
        " "
        " "
        " "
        (MdParagraph
          (MdText
            "indented"
            " "
            "content"))))
    "\n"
    "\n"
    (MdContainerBlock
      ":::"
      " "
      "details"
      " "
      "Title"
      "\n"
      (MdContainerSlot
        (MdText
          "\n")
        (MdParagraph
          (MdText
            "content")))
      "\n"
      "\n"
      ":::")
    "\n"
    "\n"
    (MdHeading
      "##"
      " "
      (MdText
        "Heading"))
    "\n"))"####
  );
}

#[test]
fn parse_sequential_containers_no_diagnostics() {
  let (tree, diags) = parse_body_with_diags(
    r#"::: details Solution

content

:::

::: details Solution

more content

:::

### Heading
"#,
  );
  assert!(
    diags.is_empty(),
    "should produce no diagnostics, got: {diags:?}"
  );
  assert_eq!(
    tree,
    r####"(SourceFile
  (YamlFrontmatter
    ""
    "---"
    "\n"
    ""
    "---"
    "\n")
  (MdBody
    (MdContainerBlock
      ":::"
      " "
      "details"
      " "
      "Solution"
      "\n"
      (MdContainerSlot
        (MdText
          "\n")
        (MdParagraph
          (MdText
            "content")))
      "\n"
      "\n"
      ":::")
    "\n"
    "\n"
    (MdContainerBlock
      ":::"
      " "
      "details"
      " "
      "Solution"
      "\n"
      (MdContainerSlot
        (MdText
          "\n")
        (MdParagraph
          (MdText
            "more"
            " "
            "content")))
      "\n"
      "\n"
      ":::")
    "\n"
    "\n"
    (MdHeading
      "###"
      " "
      (MdText
        "Heading"))
    "\n"))"####
  );
}

#[test]
fn parse_container_blank_line_before_close() {
  let (tree, diags) = parse_body_with_diags(
    r#"::: note
content

:::
"#,
  );
  assert!(
    diags.is_empty(),
    "should produce no diagnostics, got: {diags:?}"
  );
  assert_eq!(
    tree,
    r####"(SourceFile
  (YamlFrontmatter
    ""
    "---"
    "\n"
    ""
    "---"
    "\n")
  (MdBody
    (MdContainerBlock
      ":::"
      " "
      "note"
      "\n"
      (MdContainerSlot
        (MdParagraph
          (MdText
            "content")))
      "\n"
      "\n"
      ":::")
    "\n"))"####
  );
}

#[test]
fn parse_container_multiple_blank_lines_before_close() {
  let (tree, diags) = parse_body_with_diags(
    r#"::: note
content



:::
"#,
  );
  assert!(
    diags.is_empty(),
    "should produce no diagnostics, got: {diags:?}"
  );
  assert_eq!(
    tree,
    r####"(SourceFile
  (YamlFrontmatter
    ""
    "---"
    "\n"
    ""
    "---"
    "\n")
  (MdBody
    (MdContainerBlock
      ":::"
      " "
      "note"
      "\n"
      (MdContainerSlot
        (MdParagraph
          (MdText
            "content")))
      "\n"
      "\n"
      "\n"
      "\n"
      ":::")
    "\n"))"####
  );
}

#[test]
fn parse_container_with_slot_separator_blank_lines() {
  let (tree, diags) = parse_body_with_diags(
    r#"::: tabs
first

=== second
second content

:::
"#,
  );
  assert!(
    diags.is_empty(),
    "should produce no diagnostics, got: {diags:?}"
  );
  assert_eq!(
    tree,
    r####"(SourceFile
  (YamlFrontmatter
    ""
    "---"
    "\n"
    ""
    "---"
    "\n")
  (MdBody
    (MdContainerBlock
      ":::"
      " "
      "tabs"
      "\n"
      (MdContainerSlot
        (MdParagraph
          (MdText
            "first")))
      "\n"
      (MdContainerSlot)
      "\n"
      (MdContainerSlotSeparator
        "==="
        " "
        "second"
        "\n")
      (MdContainerSlot
        (MdParagraph
          (MdText
            "second"
            " "
            "content")))
      "\n"
      "\n"
      ":::")
    "\n"))"####
  );
}

// Error recovery

// Recovers from unclosed link, emits UnclosedLink diagnostic
// Unclosed [ before newline is treated as plain text, no diagnostic
#[test]
fn parse_unclosed_bracket_as_text() {
  let (tree, diags) = parse_body_with_diags(
    r#"[text without closing
"#,
  );
  assert_eq!(
    tree,
    r####"(SourceFile
  (YamlFrontmatter
    ""
    "---"
    "\n"
    ""
    "---"
    "\n")
  (MdBody
    (MdParagraph
      (MdText
        "["
        (MdText
          "text"
          " "
          "without"
          " "
          "closing")))
    "\n"))"####
  );
  assert!(
    diags.is_empty(),
    "should produce no diagnostics, got: {diags:?}"
  );
}

// Unclosed ![ before newline is treated as plain text, no diagnostic
#[test]
fn parse_unclosed_media_bracket_as_text() {
  let (tree, diags) = parse_body_with_diags(
    r#"![alt without closing
"#,
  );
  assert_eq!(
    tree,
    r####"(SourceFile
  (YamlFrontmatter
    ""
    "---"
    "\n"
    ""
    "---"
    "\n")
  (MdBody
    (MdText
      "!"
      "["
      (MdText
        "alt"
        " "
        "without"
        " "
        "closing"))
    "\n"))"####
  );
  assert!(
    diags.is_empty(),
    "should produce no diagnostics, got: {diags:?}"
  );
}

// Recovers from unclosed bold, emits UnclosedBold diagnostic
#[test]
fn recover_unclosed_bold() {
  let (tree, diags) = parse_body_with_diags(
    r#"**unclosed bold
"#,
  );
  assert_eq!(
    tree,
    r####"(SourceFile
  (YamlFrontmatter
    ""
    "---"
    "\n"
    ""
    "---"
    "\n")
  (MdBody
    (MdParagraph
      (MdBold
        "**"
        (MdText
          "unclosed"
          " "
          "bold")
        "\n"))))"####
  );
  assert_eq!(
    diags,
    vec![Diagnostic::UnclosedBold {
      start_offset: 10,
      end_offset: 24
    },]
  );
}

// Recovers from mismatched italic and bold markers
#[test]
fn recover_mismatched_inline_formatting() {
  let (tree, diags) = parse_body_with_diags(
    r#"*italic **and bold*
"#,
  );
  assert_eq!(
    tree,
    r####"(SourceFile
  (YamlFrontmatter
    ""
    "---"
    "\n"
    ""
    "---"
    "\n")
  (MdBody
    (MdParagraph
      (MdItalic
        "*"
        (MdText
          "italic"
          " ")
        (MdBold
          "**"
          (MdText
            "and"
            " "
            "bold")
          (MdItalic
            "*"
            "\n"))))))"####
  );

  assert_eq!(
    diags,
    vec![
      Diagnostic::UnclosedItalic {
        start_offset: 27,
        end_offset: 28
      },
      Diagnostic::UnclosedBold {
        start_offset: 18,
        end_offset: 28
      },
      Diagnostic::UnclosedItalic {
        start_offset: 15,
        end_offset: 28
      },
    ]
  );
}

// Square brackets without `(url)` are plain text, not links
#[test]
fn parse_brackets_without_url_are_plain_text() {
  let (tree, diags) = parse_body_with_diags(
    r#"[Cardelli, 1996] is a reference.
"#,
  );
  assert_eq!(
    tree,
    r####"(SourceFile
  (YamlFrontmatter
    ""
    "---"
    "\n"
    ""
    "---"
    "\n")
  (MdBody
    (MdParagraph
      (MdText
        "["
        (MdText
          "Cardelli,"
          " "
          "1996")
        "]")
      (MdText
        " "
        "is"
        " "
        "a"
        " "
        "reference"
        "."))
    "\n"))"####
  );
  assert!(diags.is_empty());
}

// Tag-style brackets like [Rocq] are plain text
#[test]
fn parse_tag_brackets_are_plain_text() {
  let (tree, diags) = parse_body_with_diags(
    r#"[Rocq] A command in Rocq.
"#,
  );
  assert_eq!(
    tree,
    r####"(SourceFile
  (YamlFrontmatter
    ""
    "---"
    "\n"
    ""
    "---"
    "\n")
  (MdBody
    (MdParagraph
      (MdText
        "["
        (MdText
          "Rocq")
        "]")
      (MdText
        " "
        "A"
        " "
        "command"
        " "
        "in"
        " "
        "Rocq"
        "."))
    "\n"))"####
  );
  assert!(diags.is_empty());
}

// $$ math block should not trigger missing-heading-space
#[test]
fn parse_math_block_no_heading_error() {
  let (_, diags) = parse_body_with_diags(
    r#"$$
x + y
$$
"#,
  );
  let heading_diags: Vec<_> = diags
    .iter()
    .filter(|d| {
      matches!(
        d,
        Diagnostic::MissingRequiredSpacesBetweenHashAndHeading { .. }
      )
    })
    .collect();
  assert!(
    heading_diags.is_empty(),
    "$$ should not trigger missing-heading-space, got: {:?}",
    heading_diags
  );
}

// $$ after list with empty item should not trigger missing-heading-space

// `->` at line start should be treated as text, not a list bullet
#[test]
fn parse_arrow_not_list() {
  let (tree, diags) = parse_body_with_diags(
    r#"-> **Trapped error** vs **untrapped error**.
"#,
  );
  let heading_diags: Vec<_> = diags
    .iter()
    .filter(|d| {
      matches!(
        d,
        Diagnostic::MissingRequiredSpacesBetweenHashAndHeading { .. }
      )
    })
    .collect();
  assert!(
    heading_diags.is_empty(),
    "-> should not trigger missing-heading-space, got: {:?}",
    heading_diags
  );
  assert!(
    !tree.contains("MdListItem"),
    "-> should not be parsed as a list item"
  );
}

// `->` inside a list context
#[test]
fn parse_arrow_inside_list_no_error() {
  let (_, diags) = parse_body_with_diags(
    r#"- List item.
-> **Trapped error** vs **untrapped error**.
"#,
  );
  let heading_diags: Vec<_> = diags
    .iter()
    .filter(|d| {
      matches!(
        d,
        Diagnostic::MissingRequiredSpacesBetweenHashAndHeading { .. }
      )
    })
    .collect();
  assert!(
    heading_diags.is_empty(),
    "-> after list should not trigger missing-heading-space, got: {:?}",
    heading_diags
  );
}

// Nested list siblings with 2-space indent
#[test]
fn parse_nested_siblings() {
  let (tree, diags) = parse_body_with_diags(
    r#"- outer
  - first inner
  - second inner
- back
"#,
  );
  assert_eq!(
    tree.matches("(MdBulletListItem\n").count(),
    4,
    "expected 4 items (outer, first inner, second inner, back):\n{tree}"
  );
  assert!(diags.is_empty(), "got: {diags:?}");
}

// Nested ordered list siblings with 2-space indent
#[test]
fn parse_nested_ordered_siblings() {
  let (tree, diags) = parse_body_with_diags(
    r#"1. outer
  1. first inner
  2. second inner
2. back
"#,
  );
  assert_eq!(
    tree.matches("(MdOrderedListItem\n").count(),
    4,
    "expected 4 items (outer, first inner, second inner, back):\n{tree}"
  );
  assert!(diags.is_empty(), "got: {diags:?}");
}

// Deep nesting exits correctly back to outer level
#[test]
fn parse_deep_nested_exits() {
  let (tree, diags) = parse_body_with_diags(
    r#"- level 1
  - level 2
    - level 3
- back to 1
"#,
  );
  assert_eq!(
    tree,
    r####"(SourceFile
  (YamlFrontmatter
    ""
    "---"
    "\n"
    ""
    "---"
    "\n")
  (MdBody
    (MdBulletList
      (MdBulletListItem
        "-"
        " "
        (MdParagraph
          (MdText
            "level"
            " "
            "1"))
        "\n"
        " "
        " "
        (MdBulletList
          (MdBulletListItem
            "-"
            " "
            (MdParagraph
              (MdText
                "level"
                " "
                "2"))
            "\n"
            " "
            " "
            " "
            " "
            (MdBulletList
              (MdBulletListItem
                "-"
                " "
                (MdParagraph
                  (MdText
                    "level"
                    " "
                    "3")))))))
      "\n"
      (MdBulletListItem
        "-"
        " "
        (MdParagraph
          (MdText
            "back"
            " "
            "to"
            " "
            "1"))))
    "\n"))"####
  );
  assert!(diags.is_empty());
}

// Ambiguous dedent: second child at less indent than first is not a sibling
#[test]
fn parse_ambiguous_dedent_not_sibling() {
  let (tree, _diags) = parse_body_with_diags(
    r#"- outer
    - first (4 spaces)
  - second (2 spaces)
"#,
  );
  // "second" should NOT be a sibling of "first" since it dedented
  // It should be parsed as text or a separate context
  assert!(
    tree.matches("(MdBulletListItem\n").count() <= 3,
    "dedented item should not create a 4th sibling:\n{tree}"
  );
}

// Loose list: blank line between items keeps items in same list
#[test]
fn parse_bullet_list_blank_line_between_items() {
  let tree = parse_body(
    r#"- item 1

- item 2
"#,
  );
  // Both items should be in the same MdBulletList
  assert_eq!(
    tree,
    r####"(SourceFile
  (YamlFrontmatter
    ""
    "---"
    "\n"
    ""
    "---"
    "\n")
  (MdBody
    (MdBulletList
      (MdBulletListItem
        "-"
        " "
        (MdParagraph
          (MdText
            "item"
            " "
            "1")))
      "\n"
      "\n"
      (MdBulletListItem
        "-"
        " "
        (MdParagraph
          (MdText
            "item"
            " "
            "2"))))
    "\n"))"####
  );
}

// Loose list item: blank line before continuation paragraph should nest inside li
#[test]
fn parse_bullet_list_blank_line_before_continuation() {
  let tree = parse_body(
    r#"- item 1

    continuation
"#,
  );
  assert_eq!(
    tree,
    r####"(SourceFile
  (YamlFrontmatter
    ""
    "---"
    "\n"
    ""
    "---"
    "\n")
  (MdBody
    (MdBulletList
      (MdBulletListItem
        "-"
        " "
        (MdParagraph
          (MdText
            "item"
            " "
            "1"))
        "\n"
        "\n"
        " "
        " "
        " "
        " "
        (MdParagraph
          (MdText
            "continuation"))))
    "\n"))"####
  );
}

// Loose list item: blank line before nested list should nest inside li
#[test]
fn parse_bullet_list_blank_line_before_nested_list() {
  let tree = parse_body(
    r#"- parent

  - child
"#,
  );
  assert_eq!(
    tree,
    r####"(SourceFile
  (YamlFrontmatter
    ""
    "---"
    "\n"
    ""
    "---"
    "\n")
  (MdBody
    (MdBulletList
      (MdBulletListItem
        "-"
        " "
        (MdParagraph
          (MdText
            "parent"))
        "\n"
        "\n"
        " "
        " "
        (MdBulletList
          (MdBulletListItem
            "-"
            " "
            (MdParagraph
              (MdText
                "child"))))))
    "\n"))"####
  );
}

// Loose list item: blank line before blockquote should nest inside li
#[test]
fn parse_bullet_list_blank_line_before_blockquote() {
  let tree = parse_body(
    r#"- item

    > quoted text
"#,
  );
  assert_eq!(
    tree,
    r####"(SourceFile
  (YamlFrontmatter
    ""
    "---"
    "\n"
    ""
    "---"
    "\n")
  (MdBody
    (MdBulletList
      (MdBulletListItem
        "-"
        " "
        (MdParagraph
          (MdText
            "item"))
        "\n"
        "\n"
        " "
        " "
        " "
        " "
        (MdBlockquote
          ">"
          " "
          (MdParagraph
            (MdText
              "quoted"
              " "
              "text")))))
    "\n"))"####
  );
}

// Loose list item: blank line before multiple continuation paragraphs
#[test]
fn parse_bullet_list_blank_line_before_nested_paragraphs() {
  let tree = parse_body(
    r#"- item

    first paragraph

    second paragraph
"#,
  );
  assert_eq!(
    tree,
    r####"(SourceFile
  (YamlFrontmatter
    ""
    "---"
    "\n"
    ""
    "---"
    "\n")
  (MdBody
    (MdBulletList
      (MdBulletListItem
        "-"
        " "
        (MdParagraph
          (MdText
            "item"))
        "\n"
        "\n"
        " "
        " "
        " "
        " "
        (MdParagraph
          (MdText
            "first"
            " "
            "paragraph"))
        "\n"
        "\n"
        " "
        " "
        " "
        " "
        (MdParagraph
          (MdText
            "second"
            " "
            "paragraph"))))
    "\n"))"####
  );
}

// Ordered list: blank line before continuation paragraph should nest inside li
#[test]
fn parse_ordered_list_blank_line_before_continuation() {
  let tree = parse_body(
    r#"1. item 1

    continuation
"#,
  );
  assert_eq!(
    tree,
    r####"(SourceFile
  (YamlFrontmatter
    ""
    "---"
    "\n"
    ""
    "---"
    "\n")
  (MdBody
    (MdOrderedList
      (MdOrderedListItem
        "1"
        "."
        " "
        (MdParagraph
          (MdText
            "item"
            " "
            "1"))
        "\n"
        "\n"
        " "
        " "
        " "
        " "
        (MdParagraph
          (MdText
            "continuation"))))
    "\n"))"####
  );
}

// Ordered list: blank line before nested bullet list should nest inside li
#[test]
fn parse_ordered_list_blank_line_before_nested_list() {
  let tree = parse_body(
    r#"1. parent

  - child
"#,
  );
  assert_eq!(
    tree,
    r####"(SourceFile
  (YamlFrontmatter
    ""
    "---"
    "\n"
    ""
    "---"
    "\n")
  (MdBody
    (MdOrderedList
      (MdOrderedListItem
        "1"
        "."
        " "
        (MdParagraph
          (MdText
            "parent"))
        "\n"
        "\n"
        " "
        " "
        (MdBulletList
          (MdBulletListItem
            "-"
            " "
            (MdParagraph
              (MdText
                "child"))))))
    "\n"))"####
  );
}

// Ordered list: blank line between items keeps items in same list
#[test]
fn parse_ordered_list_blank_line_between_items() {
  let tree = parse_body(
    r#"1. item 1

2. item 2
"#,
  );
  assert_eq!(
    tree,
    r####"(SourceFile
  (YamlFrontmatter
    ""
    "---"
    "\n"
    ""
    "---"
    "\n")
  (MdBody
    (MdOrderedList
      (MdOrderedListItem
        "1"
        "."
        " "
        (MdParagraph
          (MdText
            "item"
            " "
            "1")))
      "\n"
      "\n"
      (MdOrderedListItem
        "2"
        "."
        " "
        (MdParagraph
          (MdText
            "item"
            " "
            "2"))))
    "\n"))"####
  );
}

// Ordered list: blank line before blockquote should nest inside li
#[test]
fn parse_ordered_list_blank_line_before_blockquote() {
  let tree = parse_body(
    r#"1. item

    > quoted text
"#,
  );
  assert_eq!(
    tree,
    r####"(SourceFile
  (YamlFrontmatter
    ""
    "---"
    "\n"
    ""
    "---"
    "\n")
  (MdBody
    (MdOrderedList
      (MdOrderedListItem
        "1"
        "."
        " "
        (MdParagraph
          (MdText
            "item"))
        "\n"
        "\n"
        " "
        " "
        " "
        " "
        (MdBlockquote
          ">"
          " "
          (MdParagraph
            (MdText
              "quoted"
              " "
              "text")))))
    "\n"))"####
  );
}

// Ordered list: blank line before multiple continuation paragraphs
#[test]
fn parse_ordered_list_blank_line_before_nested_paragraphs() {
  let tree = parse_body(
    r#"1. item

    first paragraph

    second paragraph
"#,
  );
  assert_eq!(
    tree,
    r####"(SourceFile
  (YamlFrontmatter
    ""
    "---"
    "\n"
    ""
    "---"
    "\n")
  (MdBody
    (MdOrderedList
      (MdOrderedListItem
        "1"
        "."
        " "
        (MdParagraph
          (MdText
            "item"))
        "\n"
        "\n"
        " "
        " "
        " "
        " "
        (MdParagraph
          (MdText
            "first"
            " "
            "paragraph"))
        "\n"
        "\n"
        " "
        " "
        " "
        " "
        (MdParagraph
          (MdText
            "second"
            " "
            "paragraph"))))
    "\n"))"####
  );
}

// Blank line with trailing spaces should not break list item containment
#[test]
fn parse_bullet_list_blank_line_with_spaces() {
  let tree = parse_body("- item\n   \n  - child\n");
  assert_eq!(
    tree,
    r####"(SourceFile
  (YamlFrontmatter
    ""
    "---"
    "\n"
    ""
    "---"
    "\n")
  (MdBody
    (MdBulletList
      (MdBulletListItem
        "-"
        " "
        (MdParagraph
          (MdText
            "item")
          "\n"
          (MdText
            " "
            " "
            " "))
        "\n"
        " "
        " "
        (MdBulletList
          (MdBulletListItem
            "-"
            " "
            (MdParagraph
              (MdText
                "child"))))))
    "\n"))"####
  );
}

// Bare `>` on a blank line continues the blockquote and list item
#[test]
fn parse_list_in_blockquote_blank_line_with_prefix() {
  let tree = parse_body(
    r#"> - item
>
>   continuation
"#,
  );
  assert_eq!(
    tree,
    r####"(SourceFile
  (YamlFrontmatter
    ""
    "---"
    "\n"
    ""
    "---"
    "\n")
  (MdBody
    (MdBlockquote
      ">"
      " "
      (MdBulletList
        (MdBulletListItem
          "-"
          " "
          (MdParagraph
            (MdText
              "item"))
          "\n"
          ">"
          "\n"
          ">"
          " "
          " "
          " "
          (MdParagraph
            (MdText
              "continuation")))))
    "\n"))"####
  );
}

// Multiple bare `>` blank lines in a row inside blockquote
#[test]
fn parse_blockquote_multiple_bare_blank_lines() {
  let tree = parse_body(
    r#"> first
>
>
> second
"#,
  );
  assert_eq!(
    tree,
    r####"(SourceFile
  (YamlFrontmatter
    ""
    "---"
    "\n"
    ""
    "---"
    "\n")
  (MdBody
    (MdBlockquote
      ">"
      " "
      (MdParagraph
        (MdText
          "first"))
      "\n"
      ">"
      "\n"
      ">"
      "\n"
      ">"
      " "
      (MdParagraph
        (MdText
          "second")))
    "\n"))"####
  );
}

// Blockquote ends on truly empty line (no `>`)
#[test]
fn parse_blockquote_ends_on_empty_line() {
  let tree = parse_body(
    r#"> first

second
"#,
  );
  assert_eq!(
    tree,
    r####"(SourceFile
  (YamlFrontmatter
    ""
    "---"
    "\n"
    ""
    "---"
    "\n")
  (MdBody
    (MdBlockquote
      ">"
      " "
      (MdParagraph
        (MdText
          "first")))
    "\n"
    "\n"
    (MdParagraph
      (MdText
        "second"))
    "\n"))"####
  );
}

// Two paragraphs in blockquote separated by bare `>`
#[test]
fn parse_blockquote_two_paragraphs_bare_separator() {
  let tree = parse_body(
    r#"> first
>
> second
"#,
  );
  assert_eq!(
    tree,
    r####"(SourceFile
  (YamlFrontmatter
    ""
    "---"
    "\n"
    ""
    "---"
    "\n")
  (MdBody
    (MdBlockquote
      ">"
      " "
      (MdParagraph
        (MdText
          "first"))
      "\n"
      ">"
      "\n"
      ">"
      " "
      (MdParagraph
        (MdText
          "second")))
    "\n"))"####
  );
}

// Nested blockquote with bare `>` blank line
#[test]
fn parse_nested_blockquote_bare_blank_line() {
  let tree = parse_body(
    r#"> > inner
>
> > continued
"#,
  );
  assert_eq!(
    tree,
    r####"(SourceFile
  (YamlFrontmatter
    ""
    "---"
    "\n"
    ""
    "---"
    "\n")
  (MdBody
    (MdBlockquote
      ">"
      " "
      (MdBlockquote
        ">"
        " "
        (MdParagraph
          (MdText
            "inner")))
      "\n"
      ">"
      "\n"
      ">"
      " "
      (MdBlockquote
        ">"
        " "
        (MdParagraph
          (MdText
            "continued"))))
    "\n"))"####
  );
}

// List in blockquote with multiple bare `>` blank lines
#[test]
fn parse_list_in_blockquote_multiple_bare_blank_lines() {
  let tree = parse_body(
    r#"> - item
>
>
>   continuation
"#,
  );
  assert_eq!(
    tree,
    r####"(SourceFile
  (YamlFrontmatter
    ""
    "---"
    "\n"
    ""
    "---"
    "\n")
  (MdBody
    (MdBlockquote
      ">"
      " "
      (MdBulletList
        (MdBulletListItem
          "-"
          " "
          (MdParagraph
            (MdText
              "item"))
          "\n"
          ">"
          "\n"
          ">"
          "\n"
          ">"
          " "
          " "
          " "
          (MdParagraph
            (MdText
              "continuation")))))
    "\n"))"####
  );
}

// Backslash escapes prevent markdown interpretation
#[test]
fn parse_backslash_escape_italic() {
  let (tree, diags) = parse_body_with_diags(
    r#"\*not italic\*
"#,
  );
  assert!(
    !tree.contains("MdItalic"),
    "escaped * should not create italic:\n{tree}"
  );
  assert!(
    tree.contains("*"),
    "escaped * should appear as literal:\n{tree}"
  );
  assert!(diags.is_empty(), "got: {diags:?}");
}

// Backslash escapes work for brackets
#[test]
fn parse_backslash_escape_bracket() {
  let (tree, diags) = parse_body_with_diags(
    r#"\[not a link\]
"#,
  );
  assert!(
    !tree.contains("MdLink"),
    "escaped [ should not create link:\n{tree}"
  );
  assert!(diags.is_empty(), "got: {diags:?}");
}

// Backslash escapes work for hash
#[test]
fn parse_backslash_escape_hash() {
  let (tree, diags) = parse_body_with_diags(
    r#"\# not a heading
"#,
  );
  assert!(
    !tree.contains("MdHeading"),
    "escaped # should not create heading:\n{tree}"
  );
  assert!(diags.is_empty(), "got: {diags:?}");
}

// Backslash before non-escapable char is kept as literal
#[test]
fn parse_backslash_non_escapable() {
  let (tree, diags) = parse_body_with_diags(
    r#"\a plain text
"#,
  );
  assert!(tree.contains("\\"), "backslash should be literal:\n{tree}");
  assert!(diags.is_empty(), "got: {diags:?}");
}

// Horizontal rules
#[test]
fn parse_horizontal_rule_dashes() {
  let (tree, diags) = parse_body_with_diags("---\n");
  assert!(tree.contains("MdHorizontalRule"), "expected hr:\n{tree}");
  assert!(diags.is_empty(), "got: {diags:?}");
}

#[test]
fn parse_horizontal_rule_stars() {
  let (tree, diags) = parse_body_with_diags("***\n");
  assert!(tree.contains("MdHorizontalRule"), "expected hr:\n{tree}");
  assert!(diags.is_empty(), "got: {diags:?}");
}

#[test]
fn parse_horizontal_rule_underscores() {
  let (tree, diags) = parse_body_with_diags("___\n");
  assert!(tree.contains("MdHorizontalRule"), "expected hr:\n{tree}");
  assert!(diags.is_empty(), "got: {diags:?}");
}

#[test]
fn parse_not_horizontal_rule_with_text() {
  let (tree, _) = parse_body_with_diags("--- some text\n");
  assert!(
    !tree.contains("MdHorizontalRule"),
    "should not be hr:\n{tree}"
  );
}
