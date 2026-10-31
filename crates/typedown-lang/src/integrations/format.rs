//! Pretty-printer for the markdown body of a Typedown file
//!
//! Rules (some inspired from Google's style guide):
//! - Exactly one space after `#` in headings
//! - Exactly one blank line before/after headings
//! - No trailing whitespace
//! - Collapse multiple consecutive blank lines to one
//! - Indent nested list content by marker width (e.g. 2 for `- `, 3 for `1. `)
//! - Code blocks pass through verbatim
//! - Paragraph content passes through verbatim (no reflowing)
//! - File ends with exactly one newline

use crate::syntax::ast::{
  AstNode, MdBody, MdBulletList, MdBulletListItem, MdContainerBlock, MdOrderedList,
  MdOrderedListItem, MdTaskListItem,
};
use crate::syntax::red::RedNode;
use crate::syntax::syntax_kind::SyntaxKind;

/// Format the AST markdown body of a Typedown file
///
/// Returns an empty string if the body has no blocks
/// The caller is responsible for adding any separator between frontmatter and body
pub fn format_markdown(body: &MdBody) -> String {
  let blocks: Vec<_> = body.block_elements().collect();
  if blocks.is_empty() {
    return String::new();
  }

  let mut out = String::new();
  format_block_sequence(
    &mut out,
    &blocks
      .iter()
      .map(|block| block.syntax().clone())
      .collect::<Vec<_>>(),
    "",
  );

  let result = collapse_blank_lines(&out);
  ensure_trailing_newline(result)
}

/// Format a sequence of block elements with blank-line separation
fn format_block_sequence(out: &mut String, blocks: &[RedNode], indent: &str) {
  for (index, block) in blocks.iter().enumerate() {
    if index > 0 {
      ensure_blank_line(out);
    }
    format_block(out, block, indent);
  }
}

fn format_block(out: &mut String, node: &RedNode, indent: &str) {
  match node.kind() {
    SyntaxKind::MdHeading => format_heading(out, node, indent),
    SyntaxKind::MdBulletList => format_bullet_list(out, node, indent),
    SyntaxKind::MdOrderedList => format_ordered_list(out, node, indent),
    SyntaxKind::MdBlockquote => format_blockquote(out, node, indent),
    SyntaxKind::MdContainerBlock => format_container_block(out, node, indent),
    // Code and math blocks: preserve internal indentation verbatim, only add outer indent
    SyntaxKind::CodeBlock | SyntaxKind::MathBlock => emit_verbatim_block(out, node, indent),
    // Blank line separators: just emit a newline
    SyntaxKind::MdBlankLine => out.push('\n'),
    _ => {
      // Tables, paragraphs, horizontal rules: emit source text
      emit_source_lines(out, node, indent);
    }
  }
}

/// Emit a code/math block verbatim: strip only the common leading indent from the source,
/// then re-indent with `depth` levels so internal relative indentation is preserved
fn emit_verbatim_block(out: &mut String, node: &RedNode, indent: &str) {
  let text = node.text();
  // Only the opening fence needs the depth-based indent
  // Content and closing fence carry the correct source indentation already
  let mut lines = text.lines();
  if let Some(first) = lines.next() {
    if first.trim().is_empty() {
      out.push('\n');
    } else {
      out.push_str(indent);
      out.push_str(first.trim_end());
      out.push('\n');
    }
  }
  for line in lines {
    if line.trim().is_empty() {
      out.push('\n');
    } else {
      out.push_str(line.trim_end());
      out.push('\n');
    }
  }
}

/// Format a heading: normalize to exactly one space after `#` symbols
fn format_heading(out: &mut String, node: &RedNode, indent: &str) {
  let text = node.text();
  let trimmed = text.trim();

  // Count leading `#` symbols
  let hash_count = trimmed.chars().take_while(|char| *char == '#').count();
  if hash_count == 0 {
    // Not a valid heading, emit as-is
    push_trimmed_line(out, &format!("{indent}{trimmed}"));
    return;
  }

  // Extract heading content after the hashes
  let after_hashes = &trimmed[hash_count..];
  let content = after_hashes.trim_start();

  if content.is_empty() {
    push_trimmed_line(out, &format!("{indent}{}", "#".repeat(hash_count)));
  } else {
    push_trimmed_line(
      out,
      &format!("{indent}{} {content}", "#".repeat(hash_count)),
    );
  }
}
/// Format a blockquote: strip one level of `> ` prefix from source lines and re-emit with proper indent
fn format_blockquote(out: &mut String, node: &RedNode, indent: &str) {
  for line in node.text().lines() {
    // Strip one level of blockquote marker
    let inner = line
      .strip_prefix("> ")
      .or_else(|| line.strip_prefix('>'))
      .unwrap_or(line);
    if inner.trim().is_empty() {
      push_trimmed_line(out, &format!("{indent}>"));
    } else {
      push_trimmed_line(out, &format!("{indent}> {inner}"));
    }
  }
}

/// Format a container block: emit opening fence, recursively format slot content, emit closing fence
fn format_container_block(out: &mut String, node: &RedNode, indent: &str) {
  let Some(_container) = MdContainerBlock::cast(node.clone()) else {
    emit_source_lines(out, node, indent);
    return;
  };

  // Reconstruct the opening line: ::: label {props} title
  // We emit the source text of everything up to the first newline
  let text = node.text();
  let first_line = text.lines().next().unwrap_or(":::");
  push_trimmed_line(out, &format!("{indent}{}", first_line.trim()));

  // Format each slot's content
  for child in node.children() {
    match child.kind() {
      SyntaxKind::MdContainerSlot => {
        let blocks: Vec<_> = child
          .children()
          .filter(|child| child.as_node().is_some() && is_block_element(child.kind()))
          .collect();
        format_block_sequence(out, &blocks, indent);
      }
      SyntaxKind::MdContainerSlotSeparator => {
        let sep_text = child.text();
        let sep_line = sep_text.lines().next().unwrap_or("===");
        push_trimmed_line(out, &format!("{indent}{}", sep_line.trim()));
      }
      _ => {}
    }
  }

  // Closing fence
  push_trimmed_line(out, &format!("{indent}:::"));
}

fn is_block_element(kind: SyntaxKind) -> bool {
  matches!(
    kind,
    SyntaxKind::MdParagraph
      | SyntaxKind::MdHeading
      | SyntaxKind::MdBulletList
      | SyntaxKind::MdOrderedList
      | SyntaxKind::MdBlockquote
      | SyntaxKind::MdTable
      | SyntaxKind::MdContainerBlock
      | SyntaxKind::MdContainerShorthand
      | SyntaxKind::CodeBlock
      | SyntaxKind::MathBlock
      | SyntaxKind::MdHorizontalRule
      | SyntaxKind::MdMedia
      | SyntaxKind::MdBlankLine
  )
}

/// Format a bullet list, applying 2-space indentation per nesting level
fn format_bullet_list(out: &mut String, node: &RedNode, indent: &str) {
  if let Some(list) = MdBulletList::cast(node.clone()) {
    let list_text = node.text();
    let list_offset = node.start_offset();
    let items: Vec<_> = list.items().collect();
    for (index, item) in items.iter().enumerate() {
      if index > 0 {
        // Preserve blank lines between item groups in the source
        let prev = &items[index - 1];
        let prev_end = prev.syntax().end_offset() - list_offset;
        let curr_start = item.syntax().start_offset() - list_offset;
        if list_text[prev_end..curr_start].contains("\n\n") {
          ensure_blank_line(out);
        }
      }
      format_bullet_item(out, item, indent);
    }
  }
  // Also handle task list items at this level
  for child in node.children() {
    if child.kind() == SyntaxKind::MdTaskListItem
      && let Some(task) = MdTaskListItem::cast(child)
    {
      format_task_item(out, &task, indent);
    }
  }
}

/// Format a single bullet list item
fn format_bullet_item(out: &mut String, item: &MdBulletListItem, indent: &str) {
  // Emit the first block element inline after the bullet marker
  let mut blocks = item.block_elements();
  if let Some(first) = blocks.next() {
    let text = first.syntax().text();
    let content = text.trim();
    push_trimmed_line(out, &format!("{indent}- {content}"));
  }

  // Child content indented by marker width: `- ` = 2 spaces
  let child_indent = format!("{indent}  ");
  for block in blocks {
    format_block(out, block.syntax(), &child_indent);
  }
}

// Format a task list item: `- [ ] text` or `- [x] text`
fn format_task_item(out: &mut String, item: &MdTaskListItem, indent: &str) {
  // Emit checkbox + first block inline
  let checkbox = item
    .checkbox()
    .map(|checkbox| checkbox.syntax().text())
    .unwrap_or_default();
  let mut blocks = item.block_elements();
  if let Some(first) = blocks.next() {
    let text = first.syntax().text();
    let content = text.trim();
    push_trimmed_line(out, &format!("{indent}- {checkbox} {content}"));
  }

  // Child content indented by marker width: `- ` = 2 spaces
  let child_indent = format!("{indent}  ");
  for block in blocks {
    format_block(out, block.syntax(), &child_indent);
  }
}

/// Format an ordered list
fn format_ordered_list(out: &mut String, node: &RedNode, indent: &str) {
  if let Some(list) = MdOrderedList::cast(node.clone()) {
    let list_text = node.text();
    let list_offset = node.start_offset();
    let items: Vec<_> = list.items().collect();
    for (index, item) in items.iter().enumerate() {
      if index > 0 {
        // Preserve blank lines between item groups in the source
        let prev = &items[index - 1];
        let prev_end = prev.syntax().end_offset() - list_offset;
        let curr_start = item.syntax().start_offset() - list_offset;
        if list_text[prev_end..curr_start].contains("\n\n") {
          ensure_blank_line(out);
        }
      }
      format_ordered_item(out, item, indent, index + 1);
    }
  }
}

// Format a single ordered list item
fn format_ordered_item(out: &mut String, item: &MdOrderedListItem, indent: &str, number: usize) {
  // Emit the first block element inline after the number marker
  let marker = format!("{number}. ");
  let mut blocks = item.block_elements();
  if let Some(first) = blocks.next() {
    let text = first.syntax().text();
    let content = text.trim();
    push_trimmed_line(out, &format!("{indent}{marker}{content}"));
  }

  // Child content indented by marker width (e.g. `1. ` = 3, `10. ` = 4)
  let child_indent = format!("{indent}{}", " ".repeat(marker.len()));
  for block in blocks {
    format_block(out, block.syntax(), &child_indent);
  }
}

/// Emit source text lines with trailing whitespace stripped, at the given indent depth
fn emit_source_lines(out: &mut String, node: &RedNode, indent: &str) {
  let text = node.text();
  for line in text.lines() {
    if indent.is_empty() {
      push_trimmed_line(out, line);
    } else {
      let trimmed = line.trim();
      if trimmed.is_empty() {
        out.push('\n');
      } else {
        push_trimmed_line(out, &format!("{indent}{trimmed}"));
      }
    }
  }
}

/// Push a line with trailing whitespace removed
fn push_trimmed_line(out: &mut String, line: &str) {
  out.push_str(line.trim_end());
  out.push('\n');
}

/// Ensure the output ends with exactly one blank line (two newlines)
fn ensure_blank_line(out: &mut String) {
  if !out.ends_with("\n\n") {
    if out.ends_with('\n') {
      out.push('\n');
    } else {
      out.push_str("\n\n");
    }
  }
}

/// Collapse runs of 3+ consecutive newlines into exactly 2 (one blank line)
fn collapse_blank_lines(text: &str) -> String {
  let mut result = String::with_capacity(text.len());
  let mut consecutive_newlines = 0;

  for char in text.chars() {
    if char == '\n' {
      consecutive_newlines += 1;
      if consecutive_newlines <= 2 {
        result.push(char);
      }
    } else {
      consecutive_newlines = 0;
      result.push(char);
    }
  }

  result
}

/// Ensure the text ends with exactly one newline
fn ensure_trailing_newline(mut text: String) -> String {
  while text.ends_with("\n\n") {
    text.pop();
  }
  if !text.ends_with('\n') {
    text.push('\n');
  }
  text
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::syntax::ast::SourceFile;
  use crate::syntax::parse::tests::helpers::parse;

  fn fmt(source: &str) -> String {
    let (green, _) = parse(source);
    let red = RedNode::from_green(0, green);
    let file = SourceFile::cast(red).expect("should parse as SourceFile");
    let body = file.body().expect("should have a body");
    format_markdown(&body)
  }

  // Heading gets exactly one space after hashes
  #[test]
  fn heading_spacing() {
    let result = fmt(
      r#"---
---
##Heading
"#,
    );
    assert_eq!(
      result,
      r#"## Heading
"#
    );
  }

  // Extra spaces after hashes are normalized
  #[test]
  fn heading_extra_spaces() {
    let result = fmt(
      r#"---
---
##   Heading
"#,
    );
    assert_eq!(
      result,
      r#"## Heading
"#
    );
  }

  // Blank line before heading
  #[test]
  fn blank_line_before_heading() {
    let result = fmt(
      r#"---
---
Some text.
## Heading
"#,
    );
    assert_eq!(
      result,
      r#"Some text.

## Heading
"#
    );
  }

  // Blank line after heading
  #[test]
  fn blank_line_after_heading() {
    let result = fmt(
      r#"---
---
## Heading
Some text.
"#,
    );
    assert_eq!(
      result,
      r#"## Heading

Some text.
"#
    );
  }

  // Trailing whitespace removed
  #[test]
  fn trailing_whitespace() {
    let result = fmt(
      r"---
---
Hello world   
",
    );
    assert_eq!(
      result,
      r#"Hello world
"#
    );
  }

  // Multiple blank lines collapsed to one
  #[test]
  fn collapse_blank_lines_test() {
    let result = fmt(
      r#"---
---
First.



Second.
"#,
    );
    assert_eq!(
      result,
      r#"First.

Second.
"#
    );
  }

  // File ends with exactly one newline
  #[test]
  fn trailing_newline() {
    let result = fmt(
      r#"---
---
Hello


"#,
    );
    assert_eq!(result, "Hello\n", "should end with one newline: {result:?}");
  }

  // Formatter is idempotent
  #[test]
  fn idempotent() {
    let input = r#"---
---
##Heading



Some text.

## Another

Paragraph.
"#;
    let first = fmt(input);
    let second_input = format!(
      r"---
---
{first}"
    );
    let second = fmt(&second_input);
    assert_eq!(first, second, "formatter should be idempotent");
  }

  // Simple bullet list
  #[test]
  fn bullet_list() {
    let result = fmt(
      r#"---
---
- First item
- Second item
- Third item
"#,
    );
    assert_eq!(
      result,
      r#"- First item
- Second item
- Third item
"#
    );
  }

  // Mixed bullet prefixes are separate lists, each normalized to -
  #[test]
  fn mixed_bullet_prefixes() {
    let result = fmt(
      r#"---
---
* Star item
+ Plus item
- Dash item
"#,
    );
    assert_eq!(
      result,
      r#"- Star item

- Plus item

- Dash item
"#
    );
  }

  // List followed by heading
  #[test]
  fn list_then_heading() {
    let result = fmt(
      r#"---
---
- Item one
- Item two
## Next Section
"#,
    );
    assert_eq!(
      result,
      r#"- Item one
- Item two

## Next Section
"#
    );
  }

  // Heading followed by list
  #[test]
  fn heading_then_list() {
    let result = fmt(
      r#"---
---
## Section

- Item one
- Item two
"#,
    );
    assert_eq!(
      result,
      r#"## Section

- Item one
- Item two
"#
    );
  }

  // Full document with mixed elements
  #[test]
  fn full_document() {
    let result = fmt(
      r#"---
---

Alice is a **backend developer**.

## Skills

| Area | Proficiency |
|------|-------------|
| Rust | Expert |

## Responsibilities

- Lead backend development
- Review pull requests
- Mentor junior developers
"#,
    );
    assert_eq!(
      result,
      r#"Alice is a **backend developer**.

## Skills

| Area | Proficiency |
|------|-------------|
| Rust | Expert |

## Responsibilities

- Lead backend development
- Review pull requests
- Mentor junior developers
"#
    );
  }

  // Blank line between list and following paragraph
  #[test]
  fn list_then_paragraph() {
    let result = fmt(
      r#"---
---
- Item one
- Item two
Some text.
"#,
    );
    assert_eq!(
      result,
      r#"- Item one
- Item two

Some text.
"#
    );
  }

  // Blank line between paragraph and following list
  #[test]
  fn paragraph_then_list() {
    let result = fmt(
      r#"---
---
Some text.
- Item one
- Item two
"#,
    );
    assert_eq!(
      result,
      r#"Some text.

- Item one
- Item two
"#
    );
  }

  // Blank line between two paragraphs
  #[test]
  fn paragraph_then_paragraph() {
    let result = fmt(
      r#"---
---
First paragraph.

Second paragraph.
"#,
    );
    assert_eq!(
      result,
      r#"First paragraph.

Second paragraph.
"#
    );
  }

  // Idempotent with lists
  #[test]
  fn idempotent_with_lists() {
    let input = r#"---
---
## Section

- Item one
- Item two

Some text.
"#;
    let first = fmt(input);
    let second_input = format!(
      r"---
---
{first}"
    );
    let second = fmt(&second_input);
    assert_eq!(first, second, "formatter should be idempotent with lists");
  }

  // Nested bullet list content is not lost
  #[test]
  fn nested_bullet_list() {
    let result = fmt(
      r#"---
---
- parent
 - child one
 - child two
"#,
    );
    assert_eq!(
      result,
      r#"- parent

 - child one
 - child two
"#
    );
  }

  // Ordered list nested inside bullet list content is not lost
  #[test]
  fn nested_ordered_in_bullet() {
    let result = fmt(
      r#"---
---
- parent
  1. first
  2. second
"#,
    );
    assert_eq!(
      result,
      r#"- parent
  1. first
  2. second
"#
    );
  }

  // Three levels deep content is not lost
  #[test]
  fn nested_list_three_levels() {
    let result = fmt(
      r#"---
---
- level one
  1. level two
     - level three
"#,
    );
    assert_eq!(
      result,
      r#"- level one
  1. level two
     - level three
"#
    );
  }

  #[test]
  fn math_block_in_list_preserved() {
    let result = fmt(
      r#"---
---
- Example

    $$
    \frac{p : A \quad f : A \to B(t)}{B(t)}
    $$

    More text.
"#,
    );
    assert!(
      result.contains("$$"),
      "math block should be preserved in list: {result}"
    );
    assert!(
      result.contains(r"\frac"),
      "math content should be preserved: {result}"
    );
  }

  #[test]
  fn math_block_preserved() {
    let result = fmt(
      r#"---
---
Some text.

$$T_{total} = T_{TTFB} + T_{FCP}$$

More text.
"#,
    );
    assert_eq!(
      result,
      r#"Some text.

$$T_{total} = T_{TTFB} + T_{FCP}$$

More text.
"#
    );
  }

  // Media in bullet list item is preserved
  #[test]
  fn media_in_list_item() {
    let result = fmt(
      r#"---
---
- ![architecture](_assets/architecture.webp)
"#,
    );
    assert_eq!(
      result,
      r#"- ![architecture](_assets/architecture.webp)
"#
    );
  }

  // Horizontal rule is preserved
  #[test]
  fn horizontal_rule_preserved() {
    let result = fmt(
      r#"---
---
Some text.

---

More text.
"#,
    );
    assert_eq!(
      result,
      r#"Some text.

---

More text.
"#
    );
  }

  // Horizontal rule between paragraphs
  #[test]
  fn horizontal_rule_between_paragraphs() {
    let result = fmt(
      r#"---
---
First.

---

Second.
"#,
    );
    assert_eq!(
      result,
      r#"First.

---

Second.
"#
    );
  }

  // Code block at top level preserves internal indentation
  #[test]
  fn code_block_top_level_preserves_indentation() {
    let result = fmt(
      r#"---
---
```js
if (x) {
  doSomething();
}
```
"#,
    );
    assert_eq!(
      result,
      r#"```js
if (x) {
  doSomething();
}
```
"#
    );
  }

  // Code block inside ordered list preserves internal indentation
  #[test]
  fn code_block_in_ordered_list_preserves_indentation() {
    let result = fmt(
      r#"---
---
1. Example

   ```js
   if (x) {
     doSomething();
   }
   ```
"#,
    );
    assert_eq!(
      result,
      r#"1. Example
   ```js
   if (x) {
     doSomething();
   }
   ```
"#
    );
  }

  // Ordered list with two-digit numbers uses wider marker indent (4 spaces for `10. `)
  #[test]
  fn ordered_list_two_digit_marker_width() {
    let result = fmt(
      r#"---
---
10. First item
    Continuation paragraph.
"#,
    );
    assert_eq!(
      result,
      r#"1. First item
    Continuation paragraph.
"#
    );
  }

  // Container extra-indented inside bullet list normalizes to prefix width
  #[test]
  fn container_extra_indented_in_bullet_list() {
    let result = fmt(
      r#"---
---
- item
    ::: note
    content here
    :::
"#,
    );
    assert_eq!(
      result,
      r#"- item
  ::: note
  content here
  :::
"#
    );
  }

  // Container extra-indented inside ordered list normalizes to prefix width
  #[test]
  fn container_extra_indented_in_ordered_list() {
    let result = fmt(
      r#"---
---
1. item
      ::: note
      content here
      :::
"#,
    );
    assert_eq!(
      result,
      r#"1. item
   ::: note
   content here
   :::
"#
    );
  }

  // Code block inside bullet list preserves internal indentation
  #[test]
  fn code_block_in_bullet_list_preserves_indentation() {
    let result = fmt(
      r#"---
---
- Example

  ```js
  if (x) {
    doSomething();
  }
  ```
"#,
    );
    assert_eq!(
      result,
      r#"- Example
  ```js
  if (x) {
    doSomething();
  }
  ```
"#
    );
  }

  // Code block inside nested bullet list preserves internal indentation
  #[test]
  fn code_block_in_nested_bullet_list_preserves_indentation() {
    let result = fmt(
      r#"---
---
- Outer

  - Inner

    ```js
    if (x) {
      doSomething();
    }
    ```
"#,
    );
    assert_eq!(
      result,
      r#"- Outer

  - Inner

    ```js
    if (x) {
      doSomething();
    }
    ```
"#
    );
  }

  // Closing fence is at same indent as opening fence (not double-indented)
  #[test]
  fn code_block_in_nested_list_fence_not_double_indented() {
    let result = fmt(
      r#"---
---
- Limitations:
  - Cannot use from browser:
    ```
    Browser --> backend --> API
    ```
"#,
    );
    assert_eq!(
      result,
      r#"- Limitations:

  - Cannot use from browser:
    ```
    Browser --> backend --> API
    ```
"#
    );
  }

  // Blank line between unrelated bullet list groups is preserved
  #[test]
  fn bullet_list_groups_separated_by_blank_line() {
    let result = fmt(
      r#"---
---
- Item A1
- Item A2

- Item B1
- Item B2
"#,
    );
    assert_eq!(
      result,
      r#"- Item A1
- Item A2

- Item B1
- Item B2
"#
    );
  }

  // Standalone media block is preserved
  #[test]
  fn standalone_media() {
    let result = fmt(
      r#"---
---
![photo](img.png)
"#,
    );
    assert_eq!(
      result,
      r#"![photo](img.png)
"#
    );
  }

  // Container block is preserved verbatim
  #[test]
  fn container_block_preserved() {
    let result = fmt(
      r#"---
---
::: info
Some content here.
:::
"#,
    );
    assert_eq!(
      result,
      r#"::: info
Some content here.
:::
"#
    );
  }

  // Container block with props is preserved
  #[test]
  fn container_block_with_props_preserved() {
    let result = fmt(
      r#"---
---
::: card {title="Hello" collapsed}
Card content.
:::
"#,
    );
    assert_eq!(
      result,
      r#"::: card {title="Hello" collapsed}
Card content.
:::
"#
    );
  }

  // Container block with slots is preserved
  #[test]
  fn container_block_with_slots_preserved() {
    let result = fmt(
      r#"---
---
::: tabs
=== first
First tab content.
=== second
Second tab content.
:::
"#,
    );
    assert_eq!(
      result,
      r#"::: tabs
=== first
First tab content.
=== second
Second tab content.
:::
"#
    );
  }

  // Blockquote is preserved verbatim
  #[test]
  fn blockquote_preserved() {
    let result = fmt(
      r#"---
---
> This is a blockquote.
> It has multiple lines.
"#,
    );
    assert_eq!(
      result,
      r#"> This is a blockquote.
> It has multiple lines.
"#
    );
  }

  // Table is preserved verbatim
  #[test]
  fn table_preserved() {
    let result = fmt(
      r#"---
---
| A | B |
|---|---|
| 1 | 2 |
"#,
    );
    assert_eq!(
      result,
      r#"| A | B |
|---|---|
| 1 | 2 |
"#
    );
  }

  // Container inside list is preserved
  #[test]
  fn container_in_list_preserved() {
    let result = fmt(
      r#"---
---
- item
  ::: info
  content here
  :::
"#,
    );
    assert!(
      result.contains("- item"),
      "list item should be preserved: {result}"
    );
    assert!(
      result.contains("::: info"),
      "container should be preserved: {result}"
    );
    assert!(
      result.contains("content here"),
      "container content should be preserved: {result}"
    );
    assert!(
      result.contains(":::"),
      "closing fence should be preserved: {result}"
    );
  }

  // Container block heading gets normalized inside
  #[test]
  fn container_heading_normalized() {
    let result = fmt(
      r#"---
---
::: info
##Heading inside container
:::
"#,
    );
    assert_eq!(
      result,
      r#"::: info
## Heading inside container
:::
"#
    );
  }

  // Container block blank-line rules apply inside slots
  #[test]
  fn container_blank_line_between_blocks() {
    let result = fmt(
      r#"---
---
::: card
Some text.
## Section
More text.
:::
"#,
    );
    assert_eq!(
      result,
      r#"::: card
Some text.

## Section

More text.
:::
"#
    );
  }

  // Container with list inside gets list formatting
  #[test]
  fn container_list_formatted() {
    let result = fmt(
      r#"---
---
::: note
- Item one
- Item two
:::
"#,
    );
    assert_eq!(
      result,
      r#"::: note
- Item one
- Item two
:::
"#
    );
  }

  // Empty body produces empty string
  #[test]
  fn empty_body() {
    let result = fmt(
      r#"---
---
"#,
    );
    assert_eq!(result, "", "empty body should produce empty string");
  }

  // Idempotent with containers and blockquotes
  #[test]
  fn idempotent_with_containers() {
    let input = r#"---
---
Some text.

::: info
Important note.
:::

> A blockquote.

| A | B |
|---|---|
| 1 | 2 |
"#;
    let first = fmt(input);
    let second_input = format!("---\n---\n{first}");
    let second = fmt(&second_input);
    assert_eq!(
      first, second,
      "formatter should be idempotent with containers and blockquotes"
    );
  }

  /* Regression: container block nested inside list items caused parser panic/infinite loop */
  #[test]
  fn container_in_ordered_list() {
    let source = "---
---

1. Item one.
2. Item two.
   ::: details Title
   - Point A.
   - Point B.

   Summary.
   :::

## Next
";
    let (green, _) = parse(source);
    let red = RedNode::from_green(0, green);
    let file = SourceFile::cast(red).expect("should parse as SourceFile");
    let body = file.body().expect("should have a body");
    let result = format_markdown(&body);
    assert!(!result.is_empty(), "formatter should produce output");
  }

  #[test]
  fn container_in_bullet_list() {
    let source = "---
---

- Item one.
- Item two.
  ::: note
  Some content.
  :::

## Next
";
    let (green, _) = parse(source);
    let red = RedNode::from_green(0, green);
    let file = SourceFile::cast(red).expect("should parse as SourceFile");
    let body = file.body().expect("should have a body");
    let result = format_markdown(&body);
    assert!(!result.is_empty(), "formatter should produce output");
  }

  #[test]
  fn container_in_nested_bullet_list() {
    let source = "\
---
---

- Outer
  - Inner
    ::: warning
    Deep content.
    :::

## Next
";
    let (green, _) = parse(source);
    let red = RedNode::from_green(0, green);
    let file = SourceFile::cast(red).expect("should parse as SourceFile");
    let body = file.body().expect("should have a body");
    let result = format_markdown(&body);
    assert!(!result.is_empty(), "formatter should produce output");
  }

  #[test]
  fn container_with_slots_in_ordered_list() {
    let source = "\
---
---

1. Item.
   ::: tabs
   First tab.
   === second
   Second tab.
   :::

## Next
";
    let (green, _) = parse(source);
    let red = RedNode::from_green(0, green);
    let file = SourceFile::cast(red).expect("should parse as SourceFile");
    let body = file.body().expect("should have a body");
    let result = format_markdown(&body);
    assert!(!result.is_empty(), "formatter should produce output");
  }
}
