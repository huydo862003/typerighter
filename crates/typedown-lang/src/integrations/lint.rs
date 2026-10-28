//! Markdown linter for Typedown files
//!
//! Rules (some inspired from Google's style guide):
//! - Missing alt text on images
//! - Generic link text ("click here", "here", "link")
//! - Formatting violations (specific messages per violation)
//! - Multiple H1 headings
//! - Duplicate headings

use std::collections::BTreeMap;

use crate::syntax::ast::{AstNode, MdBody, MdHeading, MdLink, MdMedia};
use crate::syntax::diagnostic::*;
use crate::syntax::red::RedNode;
use crate::syntax::syntax_kind::SyntaxKind;

const GENERIC_LINK_TEXTS: &[&str] = &[
  "click here",
  "here",
  "link",
  "this",
  "read more",
  "more",
  "this link",
];

/// Lint the markdown body of a Typedown file
pub fn lint_markdown(body: &MdBody) -> Vec<TdDiagnostic> {
  let mut diagnostics = Vec::new();

  lint_headings(body, &mut diagnostics);
  lint_inline_elements(body.syntax(), &mut diagnostics);
  lint_trailing_whitespace(body, &mut diagnostics);
  lint_heading_spacing(body, &mut diagnostics);
  lint_blank_line_after_frontmatter(body, &mut diagnostics);
  lint_block_blank_lines(body, &mut diagnostics);
  lint_consecutive_blank_lines(body, &mut diagnostics);

  diagnostics
}

/// Check for multiple H1 headings and duplicate heading text
fn lint_headings(body: &MdBody, diagnostics: &mut Vec<TdDiagnostic>) {
  let mut h1_count = 0;
  // heading text -> (level, first offset)
  let mut seen_headings: BTreeMap<String, usize> = BTreeMap::new();

  for block in body.block_elements() {
    if block.syntax().kind() != SyntaxKind::MdHeading {
      continue;
    }
    let Some(heading) = MdHeading::cast(block.syntax().clone()) else {
      continue;
    };

    let level = heading.level();
    let text = heading_text(&heading);
    let (offset, len) = heading.syntax().trimmed_range();

    // Multiple H1
    if level == 1 {
      h1_count += 1;
      if h1_count > 1 {
        diagnostics.push(TdDiagnosticKind::MultipleH1 {}.into_warning(offset, offset + len));
      }
    }

    // Duplicate heading
    if let Some(&first_offset) = seen_headings.get(&text) {
      if first_offset != offset {
        diagnostics.push(
          TdDiagnosticKind::DuplicateHeading { text: text.clone() }
            .into_warning(offset, offset + len),
        );
      }
    } else {
      seen_headings.insert(text, offset);
    }
  }
}

/// Extract the text content of a heading (after the `#` symbols)
fn heading_text(heading: &MdHeading) -> String {
  let text = heading.syntax().text();
  let trimmed = text.trim();
  let hash_count = trimmed.chars().take_while(|char| *char == '#').count();
  trimmed[hash_count..].trim().to_string()
}

/// Walk the tree recursively to find links and images
fn lint_inline_elements(node: &RedNode, diagnostics: &mut Vec<TdDiagnostic>) {
  match node.kind() {
    SyntaxKind::MdMedia => {
      if let Some(media) = MdMedia::cast(node.clone()) {
        let alt = media.alt().map(|text| text.value()).unwrap_or_default();
        if alt.trim().is_empty() {
          let (offset, len) = node.trimmed_range();
          diagnostics.push(TdDiagnosticKind::MissingAltText {}.into_warning(offset, offset + len));
        }
      }
    }
    SyntaxKind::MdLink => {
      if let Some(link) = MdLink::cast(node.clone()) {
        let text = link.alt().map(|text| text.value()).unwrap_or_default();
        let lower = text.trim().to_lowercase();
        if GENERIC_LINK_TEXTS.contains(&lower.as_str()) {
          let (offset, len) = node.trimmed_range();
          diagnostics.push(
            TdDiagnosticKind::GenericLinkText {
              text: text.trim().to_string(),
            }
            .into_warning(offset, offset + len),
          );
        }
      }
    }
    _ => {}
  }

  for child in node.children() {
    lint_inline_elements(&child, diagnostics);
  }
}

/// Check for trailing whitespace on any line in the body
fn lint_trailing_whitespace(body: &MdBody, diagnostics: &mut Vec<TdDiagnostic>) {
  let source = body.syntax().text();
  let body_offset = body.syntax().start_offset();

  let mut offset = body_offset;
  for line in source.lines() {
    if line != line.trim_end() {
      diagnostics
        .push(TdDiagnosticKind::TrailingWhitespace {}.into_warning(offset, offset + line.len()));
    }
    offset += line.len() + 1; // +1 for newline
  }
}

/// Check that headings have exactly one space after `#` symbols
fn lint_heading_spacing(body: &MdBody, diagnostics: &mut Vec<TdDiagnostic>) {
  for block in body.block_elements() {
    if block.syntax().kind() != SyntaxKind::MdHeading {
      continue;
    }
    let text = block.syntax().text();
    let trimmed = text.trim();
    let hash_count = trimmed.chars().take_while(|char| *char == '#').count();
    if hash_count == 0 {
      continue;
    }
    let after_hashes = &trimmed[hash_count..];
    // Should be exactly one space, then content (or empty heading)
    if !after_hashes.is_empty() && !after_hashes.starts_with(' ') {
      let (offset, len) = block.syntax().trimmed_range();
      diagnostics.push(TdDiagnosticKind::HeadingMissingSpace {}.into_warning(offset, offset + len));
    } else if after_hashes.starts_with("  ") {
      let (offset, len) = block.syntax().trimmed_range();
      diagnostics.push(TdDiagnosticKind::HeadingExtraSpaces {}.into_warning(offset, offset + len));
    }
  }
}

/// Check that the body starts with a blank line when there is frontmatter
fn lint_blank_line_after_frontmatter(body: &MdBody, diagnostics: &mut Vec<TdDiagnostic>) {
  // body.syntax().offset() == 0 means the body starts at the beginning of the file (no frontmatter)
  if body.syntax().start_offset() == 0 {
    return;
  }
  let text = body.syntax().text();
  // Nothing to warn about if the body has no content
  if text.trim().is_empty() {
    return;
  }
  if !text.starts_with('\n') {
    let (offset, len) = body.syntax().trimmed_range();
    diagnostics
      .push(TdDiagnosticKind::BlankLineAfterFrontmatter {}.into_warning(offset, offset + len));
  }
}

// Check that blocks have blank lines between them
fn lint_block_blank_lines(body: &MdBody, diagnostics: &mut Vec<TdDiagnostic>) {
  let source = body.syntax().text();
  let body_offset = body.syntax().start_offset();
  let blocks: Vec<_> = body.block_elements().collect();

  for (index, block) in blocks.iter().enumerate() {
    let node = block.syntax();
    let (trimmed_offset, trimmed_len) = node.trimmed_range();
    let rel_start = node.start_offset().saturating_sub(body_offset);

    // Check blank line before (skip the first block)
    if index > 0 && rel_start > 0 {
      let before = &source[..rel_start];
      if !before.trim().is_empty() && !before.ends_with("\n\n") {
        diagnostics.push(
          TdDiagnosticKind::BlockBlankLine {}
            .into_warning(trimmed_offset, trimmed_offset + trimmed_len),
        );
      }
    }
  }
}

/// Check for multiple consecutive blank lines
fn lint_consecutive_blank_lines(body: &MdBody, diagnostics: &mut Vec<TdDiagnostic>) {
  let source = body.syntax().text();
  let body_offset = body.syntax().start_offset();

  let mut consecutive_newlines = 0;
  for (index, char) in source.char_indices() {
    if char == '\n' {
      consecutive_newlines += 1;
      if consecutive_newlines == 3 {
        diagnostics.push(
          TdDiagnosticKind::ConsecutiveBlankLines {}
            .into_warning(body_offset + index, body_offset + index + 1),
        );
      }
    } else {
      consecutive_newlines = 0;
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::syntax::ast::SourceFile;
  use crate::syntax::parse::tests::helpers::parse;
  use crate::syntax::red::RedNode;

  fn lint(source: &str) -> Vec<TdDiagnostic> {
    let (green, _) = parse(source);
    let red = RedNode::from_green(0, green);
    let file = SourceFile::cast(red).expect("should parse as SourceFile");
    let body = file.body().expect("should have a body");
    lint_markdown(&body)
  }

  fn codes(diagnostics: &[TdDiagnostic]) -> Vec<&str> {
    diagnostics
      .iter()
      .map(|diagnostic| diagnostic.code_str())
      .collect()
  }

  #[test]
  fn missing_alt_text() {
    let diags = lint(
      r#"---
---
![](image.png)
"#,
    );
    assert!(codes(&diags).contains(&"missing-alt-text"));
  }

  #[test]
  fn image_with_alt_text() {
    let diags = lint(
      r#"---
---
![A cat](cat.png)
"#,
    );
    assert!(!codes(&diags).contains(&"missing-alt-text"));
  }

  #[test]
  fn generic_link_text() {
    let diags = lint(
      r#"---
---
[click here](https://example.com)
"#,
    );
    assert!(codes(&diags).contains(&"generic-link-text"));
  }

  #[test]
  fn descriptive_link_text() {
    let diags = lint(
      r#"---
---
[the full documentation](https://example.com)
"#,
    );
    assert!(!codes(&diags).contains(&"generic-link-text"));
  }

  #[test]
  fn multiple_h1() {
    let diags = lint(
      r#"---
---
# First

# Second
"#,
    );
    assert!(codes(&diags).contains(&"multiple-h1"));
  }

  #[test]
  fn single_h1() {
    let diags = lint(
      r#"---
---
# Title

## Section
"#,
    );
    assert!(!codes(&diags).contains(&"multiple-h1"));
  }

  #[test]
  fn duplicate_headings() {
    let diags = lint(
      r#"---
---
## Summary

Text.

## Summary
"#,
    );
    assert!(codes(&diags).contains(&"duplicate-heading"));
  }

  #[test]
  fn unique_headings() {
    let diags = lint(
      r#"---
---
## Overview

## Details
"#,
    );
    assert!(!codes(&diags).contains(&"duplicate-heading"));
  }

  #[test]
  fn trailing_whitespace() {
    let diags = lint(
      r"---
---
Hello   
",
    );
    assert!(codes(&diags).contains(&"trailing-whitespace"));
  }

  #[test]
  fn no_trailing_whitespace() {
    let diags = lint(
      r#"---
---
Hello
"#,
    );
    assert!(!codes(&diags).contains(&"trailing-whitespace"));
  }

  #[test]
  fn heading_missing_space() {
    let diags = lint(
      r#"---
---
##Heading
"#,
    );
    assert!(codes(&diags).contains(&"heading-missing-space"));
  }

  #[test]
  fn heading_missing_blank_line() {
    let diags = lint(
      r#"---
---
Some text.
## Heading
"#,
    );
    assert!(codes(&diags).contains(&"block-blank-line"));
  }

  #[test]
  fn list_then_paragraph_missing_blank_line() {
    let diags = lint(
      r#"---
---
- Item one
- Item two
Some text.
"#,
    );
    assert!(codes(&diags).contains(&"block-blank-line"));
  }

  #[test]
  fn paragraph_then_list_missing_blank_line() {
    let diags = lint(
      r#"---
---
Some text.
- Item one
"#,
    );
    assert!(codes(&diags).contains(&"block-blank-line"));
  }

  #[test]
  fn blocks_with_blank_lines() {
    let diags = lint(
      r#"---
---
Some text.

- Item one

More text.
"#,
    );
    assert!(!codes(&diags).contains(&"block-blank-line"));
  }

  #[test]
  fn consecutive_blank_lines() {
    let diags = lint(
      r#"---
---
First.



Second.
"#,
    );
    assert!(codes(&diags).contains(&"consecutive-blank-lines"));
  }

  #[test]
  fn heading_after_frontmatter_no_blank_line_warning() {
    let diags = lint(
      r#"---
---

# Heading

Some text.
"#,
    );
    assert!(!codes(&diags).contains(&"block-blank-line"));
  }

  #[test]
  fn blank_line_after_frontmatter_missing() {
    let diags = lint(
      r#"---
---
## Heading
"#,
    );
    assert!(codes(&diags).contains(&"blank-line-after-frontmatter"));
  }

  #[test]
  fn blank_line_after_frontmatter_present() {
    let diags = lint(
      r#"---
---

## Heading
"#,
    );
    assert!(!codes(&diags).contains(&"blank-line-after-frontmatter"));
  }

  #[test]
  fn clean_file() {
    let diags = lint(
      r#"---
---

## Heading

Some text.
"#,
    );
    assert!(diags.is_empty(), "unexpected warnings: {:?}", codes(&diags));
  }
}
