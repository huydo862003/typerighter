//! Pretty-printer for the markdown body of a Typedown file
//!
//! Rules (some inspired from Google's style guide):
//! - Exactly one space after `#` in headings
//! - Exactly one blank line before/after headings
//! - No trailing whitespace
//! - Collapse multiple consecutive blank lines to one
//! - 2-space indent for nested list content
//! - Code blocks pass through verbatim
//! - Paragraph content passes through verbatim (no reflowing)
//! - File ends with exactly one newline

use crate::syntax::ast::{
  AstNode, MdBody, MdBulletList, MdBulletListItem, MdOrderedList, MdOrderedListItem, MdTaskListItem,
};
use crate::syntax::red::RedNode;
use crate::syntax::syntax_kind::SyntaxKind;

/// Format the AST markdown body of a Typedown file
pub fn format_markdown(body: &MdBody) -> String {
  let mut out = String::from("\n");
  let blocks: Vec<_> = body.block_elements().collect();

  for (idx, block) in blocks.iter().enumerate() {
    // One blank line between any two blocks
    if idx > 0 {
      ensure_blank_line(&mut out);
    }

    format_block(&mut out, block.syntax(), 0);
  }

  // Collapse multiple blank lines and ensure trailing newline
  let result = collapse_blank_lines(&out);
  ensure_trailing_newline(result)
}

fn format_block(out: &mut String, node: &RedNode, depth: usize) {
  match node.kind() {
    SyntaxKind::MdHeading => format_heading(out, node),
    SyntaxKind::MdBulletList => format_bullet_list(out, node, depth),
    SyntaxKind::MdOrderedList => format_ordered_list(out, node, depth),
    _ => {
      // Tables, blockquotes, containers, paragraphs: emit source text
      emit_source_lines(out, node, depth);
    }
  }
}

/// Format a heading: normalize to exactly one space after `#` symbols
fn format_heading(out: &mut String, node: &RedNode) {
  let text = node.text();
  let trimmed = text.trim();

  // Count leading `#` symbols
  let hash_count = trimmed.chars().take_while(|ch| *ch == '#').count();
  if hash_count == 0 {
    // Not a valid heading, emit as-is
    push_trimmed_line(out, trimmed);
    return;
  }

  // Extract heading content after the hashes
  let after_hashes = &trimmed[hash_count..];
  let content = after_hashes.trim_start();

  if content.is_empty() {
    push_trimmed_line(out, &"#".repeat(hash_count));
  } else {
    push_trimmed_line(out, &format!("{} {}", "#".repeat(hash_count), content));
  }
}

/// Format a bullet list, applying 2-space indentation per nesting level
fn format_bullet_list(out: &mut String, node: &RedNode, depth: usize) {
  if let Some(list) = MdBulletList::cast(node.clone()) {
    for item in list.items() {
      format_bullet_item(out, &item, depth);
    }
  }
  // Also handle task list items at this level
  for child in node.children() {
    if child.kind() == SyntaxKind::MdTaskListItem
      && let Some(task) = MdTaskListItem::cast(child)
    {
      format_task_item(out, &task, depth);
    }
  }
}

/// Format a single bullet list item
fn format_bullet_item(out: &mut String, item: &MdBulletListItem, depth: usize) {
  let indent = "  ".repeat(depth);

  // Emit the first block element inline after the bullet marker
  let mut blocks = item.block_elements();
  if let Some(first) = blocks.next() {
    let text = first.syntax().text();
    let content = text.trim();
    push_trimmed_line(out, &format!("{indent}- {content}"));
  }

  // Recurse into remaining child blocks (nested lists, paragraphs)
  for block in blocks {
    format_block(out, block.syntax(), depth + 1);
  }
}

// Format a task list item: `- [ ] text` or `- [x] text`
fn format_task_item(out: &mut String, item: &MdTaskListItem, depth: usize) {
  let indent = "  ".repeat(depth);

  // Emit checkbox + first block inline
  let checkbox = item
    .checkbox()
    .map(|c| c.syntax().text())
    .unwrap_or_default();
  let mut blocks = item.block_elements();
  if let Some(first) = blocks.next() {
    let text = first.syntax().text();
    let content = text.trim();
    push_trimmed_line(out, &format!("{indent}- {checkbox} {content}"));
  }

  // Recurse into remaining child blocks
  for block in blocks {
    format_block(out, block.syntax(), depth + 1);
  }
}

/// Format an ordered list
fn format_ordered_list(out: &mut String, node: &RedNode, depth: usize) {
  if let Some(list) = MdOrderedList::cast(node.clone()) {
    for (idx, item) in list.items().enumerate() {
      format_ordered_item(out, &item, depth, idx + 1);
    }
  }
}

// Format a single ordered list item
fn format_ordered_item(out: &mut String, item: &MdOrderedListItem, depth: usize, number: usize) {
  let indent = "  ".repeat(depth);

  // Emit the first block element inline after the number marker
  let mut blocks = item.block_elements();
  if let Some(first) = blocks.next() {
    let text = first.syntax().text();
    let content = text.trim();
    push_trimmed_line(out, &format!("{indent}{number}. {content}"));
  }

  // Recurse into remaining child blocks (nested lists, paragraphs)
  for block in blocks {
    format_block(out, block.syntax(), depth + 1);
  }
}

/// Emit source text lines with trailing whitespace stripped, at the given indent depth
fn emit_source_lines(out: &mut String, node: &RedNode, depth: usize) {
  let indent = "  ".repeat(depth);
  let text = node.text();
  for line in text.lines() {
    if depth == 0 {
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

  for ch in text.chars() {
    if ch == '\n' {
      consecutive_newlines += 1;
      if consecutive_newlines <= 2 {
        result.push(ch);
      }
    } else {
      consecutive_newlines = 0;
      result.push(ch);
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
    let result = fmt("---\n---\n##Heading\n");
    assert_eq!(result, "\n## Heading\n");
  }

  // Extra spaces after hashes are normalized
  #[test]
  fn heading_extra_spaces() {
    let result = fmt("---\n---\n##   Heading\n");
    assert_eq!(result, "\n## Heading\n");
  }

  // Blank line before heading
  #[test]
  fn blank_line_before_heading() {
    let result = fmt("---\n---\nSome text.\n## Heading\n");
    assert!(
      result.contains("\n\n## Heading"),
      "should have blank line before heading:\n{result}"
    );
  }

  // Blank line after heading
  #[test]
  fn blank_line_after_heading() {
    let result = fmt("---\n---\n## Heading\nSome text.\n");
    assert!(
      result.contains("## Heading\n\n"),
      "should have blank line after heading:\n{result}"
    );
  }

  // Trailing whitespace removed
  #[test]
  fn trailing_whitespace() {
    let result = fmt("---\n---\nHello world   \n");
    assert_eq!(result, "\nHello world\n");
  }

  // Multiple blank lines collapsed to one
  #[test]
  fn collapse_blank_lines_test() {
    let result = fmt("---\n---\nFirst.\n\n\n\nSecond.\n");
    assert_eq!(result, "\nFirst.\n\nSecond.\n");
  }

  // File ends with exactly one newline
  #[test]
  fn trailing_newline() {
    let result = fmt("---\n---\nHello\n\n\n");
    assert!(
      result.ends_with("\nHello\n"),
      "should end with one newline:\n{result:?}"
    );
  }

  // Formatter is idempotent
  #[test]
  fn idempotent() {
    let input = "---\n---\n##Heading\n\n\n\nSome text.   \n\n## Another\n\nParagraph.\n";
    let first = fmt(input);
    let second_input = format!("---\n---\n{first}");
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
      r#"
- First item
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
      r#"
- Star item

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
    assert!(
      result.contains("- Item two\n\n## Next Section"),
      "should have blank line between list and heading:\n{result}"
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
      r#"
## Section

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
    assert!(
      result.contains("- Lead backend development\n"),
      "list items preserved:\n{result}"
    );
    assert!(
      result.contains("## Skills\n\n"),
      "blank line after heading:\n{result}"
    );
    assert!(
      !result.contains("- Lead backend development\n- Lead backend development"),
      "no duplication:\n{result}"
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
    assert!(
      result.contains("- Item two\n\nSome text."),
      "should have blank line between list and paragraph:\n{result}"
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
    assert!(
      result.contains("Some text.\n\n- Item one"),
      "should have blank line between paragraph and list:\n{result}"
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
    assert_eq!(result, "\nFirst paragraph.\n\nSecond paragraph.\n");
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
    let second_input = format!("---\n---\n{first}");
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
      r#"
- parent

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
      r#"
- parent
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
      r#"
- level one
  1. level two
    - level three
"#
    );
  }
}
