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
            "2")))
      "\n")))"####
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
                    "three")))
              "\n"))
          ""))
      "")))"####
  );
}

// Toggle list nested inside ordered list nested inside bullet list
#[test]
fn parse_toggle_in_ordered_in_bullet() {
  let tree = parse_body(
    r#"- top
 1. middle
  >- toggle summary

     toggle details
"#,
  );
  assert!(
    tree.contains("(MdBulletList")
      && tree.contains("(MdOrderedList")
      && tree.contains("(MdToggleList"),
    "should have toggle > ordered > bullet nesting:\n{tree}"
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
            "2")))
      "\n")))"####
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
            "2")))
      "\n")))"####
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
            "checked")))
      "\n")))"####
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
            "item")))
      "\n")))"####
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
                "two")))
          "\n"))
      "")))"####
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
            "second")))
      "\n")))"####
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
            "|")))
      "\n")))"####
  );
}

// Parses a toggle list
#[test]
fn parse_toggle_list_simple() {
  let tree = parse_body(
    r#">- summary

   details here
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
    (MdToggleList
      (MdToggleListItem
        ">"
        "-"
        " "
        (MdToggleListSummary
          (MdText
            "summary"))
        "\n"
        "\n"
        " "
        " "
        " "
        "\n"
        (MdToggleListDetails
          (MdParagraph
            (MdText
              "details"
              " "
              "here"))))
      "")))"####
  );
}

// Toggle list inside a blockquote
#[test]
fn parse_toggle_list_in_blockquote() {
  let tree = parse_body(
    r#"> >- summary
>
>    details here
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
      (MdToggleList
        (MdToggleListItem
          ">"
          "-"
          " "
          (MdToggleListSummary
            (MdText
              "summary"))
          "\n"
          ">"
          "\n"
          ">"
          " "
          " "
          " "
          " "
          "\n"
          (MdToggleListDetails
            (MdParagraph
              (MdText
                "details"
                " "
                "here"))))
        ""))))"####
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
  let tree = parse_body(
    r#"```js{1,3,5-8}
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
      "```js{1,3,5-8}\ncode\n```")
    "\n"))"####
  );
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
            "normal")))
      "\n")))"####
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
            "info")))
      "\n")))"####
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
            "|")))
      "\n")))"####
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
            "b")))
      "\n")))"####
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
            "|")))
      "\n")
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
            "listed")))
      "\n")))"####
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
            "b")))
      "\n")
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
            "second")))
      "\n")
    "\n"
    (MdBulletList
      (MdBulletListItem
        "-"
        " "
        (MdParagraph
          (MdText
            "bullet")))
      "\n")))"####
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
            ")")))
      "\n")))"####
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
            "|")))
      "\n")))"####
  );
}

// Error recovery

// Recovers from unclosed link, emits UnclosedLink diagnostic
#[test]
fn recover_unclosed_link() {
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
      (MdLink
        "["
        (MdText
          "text"
          " "
          "without"
          " "
          "closing")))
    "\n"))"####
  );
  assert_eq!(
    diags,
    vec![Diagnostic::UnclosedLink {
      start_offset: 9,
      end_offset: 30,
    },]
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
