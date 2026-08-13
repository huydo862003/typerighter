use std::collections::HashMap;
use std::path::PathBuf;
use std::time::SystemTime;

use crate::db::derived::parse_file::parse_file;
use crate::db::types::{File, FileHandle, FileMetadata, Project};
use crate::db::{QueryStorage, TypedownDatabase};

use super::helpers::*;

// Parses YAML frontmatter with nested mappings and sequences followed by rich markdown body
#[test]
fn parse_typedown_yaml_then_markdown() {
  let input = r#"---
title: test
tags:
  - rust
  - parser
config:
  debug: true
---
# Title

A paragraph with **bold** and *italic*.

- item with [link](url)
- plain item

> A blockquote
"#;
  let (ast, _) = parse(input);
  let tree = render_tree(&ast);
  assert_eq!(
    tree,
    r####"(SourceFile
  (YamlFrontmatter
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
          (IdentLit
            " "
            "test")))
      "\n"
      ""
      (YamlMappingEntry
        (YamlMappingEntryKey
          "tags")
        ":"
        (YamlMappingEntryValue
          (YamlSequence
            "\n"
            "  "
            (YamlSequenceItem
              "-"
              (IdentLit
                " "
                "rust"))
            "\n"
            "  "
            (YamlSequenceItem
              "-"
              (IdentLit
                " "
                "parser")))))
      "\n"
      ""
      (YamlMappingEntry
        (YamlMappingEntryKey
          "config")
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
        "A"
        " "
        "paragraph"
        " "
        "with"
        " ")
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
        "."))
    "\n"
    "\n"
    (MdBulletList
      (MdBulletListItem
        "-"
        " "
        (MdParagraph
          (MdText
            "item"
            " "
            "with"
            " ")
          (MdLink
            "["
            (MdText
              "link")
            "]"
            "("
            (MdText
              "url")
            ")")))
      "\n"
      (MdBulletListItem
        "-"
        " "
        (MdParagraph
          (MdText
            "plain"
            " "
            "item")))
      "\n")
    "\n"
    (MdBlockquote
      ">"
      " "
      (MdParagraph
        (MdText
          "A"
          " "
          "blockquote")))
    "\n"))"####
  );
}

// Parses frontmatter with function call (fref) followed by markdown body
#[test]
fn parse_typedown_fref_in_frontmatter_then_markdown() {
  let input = r#"---
_type: Task
title: "Design mockups"
project: fref("projects/website-redesign.td")
---

Completed **ahead of schedule**.
"#;
  let (ast, _) = parse(input);
  let tree = render_tree(&ast);
  assert!(tree.contains("YamlFrontmatter"), "should have frontmatter");
  assert!(tree.contains("MdBody"), "should have markdown body");
}

// Parse all files from the project_tracker example through the query engine
#[test]
fn parse_all_project_tracker_files() {
  let project_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
    .parent()
    .unwrap()
    .parent()
    .unwrap()
    .join("examples/project_tracker");
  if !project_dir.exists() {
    return;
  }

  let db = TypedownDatabase {
    storage: QueryStorage::default(),
  };

  let mut file_map = HashMap::new();
  fn collect_files(dir: &std::path::Path, out: &mut Vec<PathBuf>) {
    for entry in std::fs::read_dir(dir).unwrap().flatten() {
      let p = entry.path();
      if p.is_dir() {
        collect_files(&p, out);
      } else {
        let ext = p.extension().and_then(|e| e.to_str());
        let name = p.file_name().and_then(|n| n.to_str());
        if ext == Some("td") || matches!(name, Some("typedown.yaml") | Some("typedown.yml")) {
          out.push(p);
        }
      }
    }
  }
  let mut paths = Vec::new();
  collect_files(&project_dir, &mut paths);

  for path in &paths {
    let meta = std::fs::metadata(path).ok();
    let mtime = meta
      .as_ref()
      .and_then(|m| m.modified().ok())
      .unwrap_or(SystemTime::UNIX_EPOCH);
    let ctime = meta
      .as_ref()
      .and_then(|m| m.created().ok())
      .unwrap_or(mtime);
    let handle = FileHandle::Path(path.clone(), FileMetadata { mtime, ctime });
    let file = File::new(&db, handle);
    file_map.insert(path.clone(), file);
  }
  let project = Project::new(&db, project_dir, file_map.clone());

  let mut sorted_files: Vec<_> = file_map.iter().collect();
  sorted_files.sort_by_key(|(p, _)| *p);
  for (path, file) in sorted_files {
    if path.extension().and_then(|e| e.to_str()) != Some("td") {
      continue;
    }
    eprintln!("parsing: {}", path.display());
    let result = parse_file(&db, project, *file);
    let _ = result.diagnostics(&db);
  }
}

// Interpolation with function call in markdown body
#[test]
fn parse_typedown_interpolation_with_fref() {
  let input = r#"---
_type: Person
name: "Alice"
---

Inline formula reference: ${fref("tasks/implement-auth.td")}
"#;
  let (ast, _) = parse(input);
  let tree = render_tree(&ast);
  assert!(tree.contains("InterpFragment"), "should have interpolation");
  assert!(tree.contains("CallExpr"), "should have function call");
}

// Exact content from write-tests.td
#[test]
fn parse_typedown_write_tests_file() {
  let input = r#"---
_type: Task
title: "Write integration tests for auth"
status: "todo"
priority: "medium"
project: fref("projects/website-redesign.td")
assignee: fref("people/alice.td")
---

Depends on the auth implementation being merged first.
Tests should cover login success, login failure, session expiry, and password reset flow.
Use the existing test harness in `tests/integration/`.

## Test Cases

| Scenario | Expected Result |
|----------|----------------|
| Valid credentials | 200 with access token |
| Wrong password | 401 |
| Expired session | 401, prompt re-login |
| Password reset request | 200, email sent |
| Rate limit exceeded | 429 |

## Checklist

- [ ] Set up test database fixtures
- [ ] Write happy path tests
- [ ] Write failure and edge case tests
- [ ] Assert token expiry behavior
"#;
  let (ast, _) = parse(input);
  let tree = render_tree(&ast);
  assert!(tree.contains("YamlFrontmatter"), "should have frontmatter");
  assert!(tree.contains("MdBody"), "should have markdown body");
}

// Parses YAML frontmatter with folded block scalar followed by markdown body
#[test]
fn parse_typedown_folded_block_then_markdown() {
  let input = r#"---
desc: >
  folded
  text
title: hi
---
# Hello

world
"#;
  let (ast, _) = parse(input);
  let tree = render_tree(&ast);
  assert_eq!(
    tree,
    r####"(SourceFile
  (YamlFrontmatter
    ""
    "---"
    "\n"
    (YamlMapping
      ""
      (YamlMappingEntry
        (YamlMappingEntryKey
          "desc")
        ":"
        (YamlMappingEntryValue
          (StrLit
            (YamlFoldedBlockStrLit
              " "
              ">"
              "\n"
              "  "
              "folded"
              "\n"
              "  "
              "text"
              "\n"))))
      ""
      (YamlMappingEntry
        (YamlMappingEntryKey
          "title")
        ":"
        (YamlMappingEntryValue
          (IdentLit
            " "
            "hi")))
      "\n"
      "")
    "---"
    "\n")
  (MdBody
    (MdHeading
      "#"
      " "
      (MdText
        "Hello"))
    "\n"
    "\n"
    (MdParagraph
      (MdText
        "world"))
    "\n"))"####
  );
}

// File with no frontmatter parses as empty frontmatter + markdown body
#[test]
fn parse_typedown_no_frontmatter() {
  let input = "# Hello\n\nparagraph here\n";
  let (ast, diagnostics) = parse(input);
  assert!(
    diagnostics.is_empty(),
    "should have no diagnostics: {:?}",
    diagnostics
  );
  let tree = render_tree(&ast);
  assert_eq!(
    tree,
    r####"(SourceFile
  (YamlFrontmatter)
  (MdBody
    (MdHeading
      "#"
      " "
      (MdText
        "Hello"))
    "\n"
    "\n"
    (MdParagraph
      (MdText
        "paragraph"
        " "
        "here"))
    "\n"))"####
  );
}

// Empty file produces empty frontmatter and empty body
#[test]
fn parse_typedown_empty_file() {
  let (ast, diagnostics) = parse("");
  assert!(diagnostics.is_empty());
  let tree = render_tree(&ast);
  assert_eq!(
    tree,
    r####"(SourceFile
  (YamlFrontmatter)
  (MdBody))"####
  );
}

// Container shorthand in a schemaless file (no frontmatter)
#[test]
fn parse_typedown_container_shorthand_no_frontmatter() {
  let input = "[[toc]]\n";
  let (ast, diagnostics) = parse(input);
  let tree = render_tree(&ast);
  assert_eq!(
    tree,
    r####"(SourceFile
  (YamlFrontmatter)
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
    diagnostics.is_empty(),
    "should produce no diagnostics, got: {diagnostics:?}"
  );
}

// Single dash is not a frontmatter opener
#[test]
fn parse_typedown_single_dash_no_frontmatter() {
  let input = "- list item\n";
  let (ast, diagnostics) = parse(input);
  assert!(
    diagnostics.is_empty(),
    "should have no diagnostics: {:?}",
    diagnostics
  );
  let tree = render_tree(&ast);
  assert!(
    tree.contains("(YamlFrontmatter)"),
    "should have empty frontmatter"
  );
  assert!(tree.contains("MdBulletList"), "should parse as bullet list");
}

// Double dash is not a frontmatter opener
#[test]
fn parse_typedown_double_dash_no_frontmatter() {
  let input = "-- not frontmatter\n";
  let (ast, diagnostics) = parse(input);
  assert!(diagnostics.is_empty());
  let tree = render_tree(&ast);
  assert!(
    tree.contains("(YamlFrontmatter)"),
    "should have empty frontmatter"
  );
  assert!(tree.contains("MdBody"), "should have markdown body");
}

// Four dashes is not a frontmatter opener
#[test]
fn parse_typedown_four_dashes_no_frontmatter() {
  let input = "---- not frontmatter\n";
  let (ast, diagnostics) = parse(input);
  assert!(diagnostics.is_empty());
  let tree = render_tree(&ast);
  assert!(
    tree.contains("(YamlFrontmatter)"),
    "should have empty frontmatter"
  );
  assert!(tree.contains("MdBody"), "should have markdown body");
}

// Parses simple frontmatter and markdown body
#[test]
fn parse_typedown_simple_frontmatter_and_body() {
  let input = r#"---
title: hello
---
# Welcome

paragraph here
"#;
  let (ast, _) = parse(input);
  let tree = render_tree(&ast);
  assert_eq!(
    tree,
    r####"(SourceFile
  (YamlFrontmatter
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
          (IdentLit
            " "
            "hello")))
      "\n"
      "")
    "---"
    "\n")
  (MdBody
    (MdHeading
      "#"
      " "
      (MdText
        "Welcome"))
    "\n"
    "\n"
    (MdParagraph
      (MdText
        "paragraph"
        " "
        "here"))
    "\n"))"####
  );
}

// Parses frontmatter with nested YAML and markdown body
#[test]
fn parse_typedown_complex_frontmatter_and_body() {
  let input = r#"---
title: hello
tags:
  - a
  - b
---
# Heading

- list item
"#;
  let (ast, _) = parse(input);
  let tree = render_tree(&ast);
  assert_eq!(
    tree,
    r####"(SourceFile
  (YamlFrontmatter
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
          (IdentLit
            " "
            "hello")))
      "\n"
      ""
      (YamlMappingEntry
        (YamlMappingEntryKey
          "tags")
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
    "\n")
  (MdBody
    (MdHeading
      "#"
      " "
      (MdText
        "Heading"))
    "\n"
    "\n"
    (MdBulletList
      (MdBulletListItem
        "-"
        " "
        (MdParagraph
          (MdText
            "list"
            " "
            "item")))
      "\n")))"####
  );
}

// Braces in prose are parsed as text tokens
#[test]
fn parse_braces_in_prose() {
  let input = "hello {world}\n";
  let (ast, _) = parse(input);
  let tree = render_tree(&ast);
  assert_eq!(
    tree,
    r####"(SourceFile
  (YamlFrontmatter)
  (MdBody
    (MdParagraph
      (MdText
        "hello"
        " "
        "{"
        "world"
        "}"))
    "\n"))"####
  );
}

// Braces inside a container block are parsed as text
#[test]
fn parse_braces_in_container() {
  let input = r#"::: tip
braces {} here
:::
"#;
  let (ast, _) = parse(input);
  let tree = render_tree(&ast);
  assert_eq!(
    tree,
    r####"(SourceFile
  (YamlFrontmatter)
  (MdBody
    (MdContainerBlock
      ":::"
      " "
      "tip"
      "\n"
      (MdContainerSlot
        (MdParagraph
          (MdText
            "braces"
            " "
            "{"
            "}"
            " "
            "here")))
      "\n"
      ":::")
    "\n"))"####
  );
}

// Braces inside inline code are part of the code token
#[test]
fn parse_inline_code_with_braces() {
  let input = r#"text `{}` more
"#;
  let (ast, _) = parse(input);
  let tree = render_tree(&ast);
  assert_eq!(
    tree,
    r####"(SourceFile
  (YamlFrontmatter)
  (MdBody
    (MdParagraph
      (MdText
        "text"
        " ")
      (InlineCode
        "`{}`")
      (MdText
        " "
        "more"))
    "\n"))"####
  );
}

// Backslash-escaped braces are parsed as text
#[test]
fn parse_backslash_brace() {
  let input = r#"hello \{world\}
"#;
  let (ast, _) = parse(input);
  let tree = render_tree(&ast);
  assert_eq!(
    tree,
    r####"(SourceFile
  (YamlFrontmatter)
  (MdBody
    (MdParagraph
      (MdText
        "hello"
        " "
        "\\{"
        "world"
        "\\}"))
    "\n"))"####
  );
}

// Container block nested inside a bullet list item
#[test]
fn parse_container_nested_in_bullet_list() {
  let input = r#"- item
  ::: info
  content here
  :::
"#;
  let (ast, _) = parse(input);
  let tree = render_tree(&ast);
  assert_eq!(
    tree,
    r#"(SourceFile
  (YamlFrontmatter)
  (MdBody
    (MdBulletList
      (MdBulletListItem
        "-"
        " "
        (MdParagraph
          (MdText
            "item"))
        "\n"
        " "
        " "
        (MdContainerBlock
          ":::"
          " "
          "info"
          "\n"
          (MdContainerSlot
            (MdParagraph
              (MdText
                " "
                " "
                "content"
                " "
                "here")))
          "\n"
          " "
          " "
          ":::"))
      "\n")))"#
  );
}

// Container block nested inside an ordered list item
// 2-space indent: 1 for list prefix, 1 consumed by consume_md_indent
#[test]
fn parse_container_nested_in_ordered_list() {
  let input = r#"1. item
  ::: info
  content here
  :::
"#;
  let (ast, _) = parse(input);
  let tree = render_tree(&ast);
  assert_eq!(
    tree,
    r#"(SourceFile
  (YamlFrontmatter)
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
        " "
        " "
        (MdContainerBlock
          ":::"
          " "
          "info"
          "\n"
          (MdContainerSlot
            (MdParagraph
              (MdText
                " "
                " "
                "content"
                " "
                "here")))
          "\n"
          " "
          " "
          ":::"))
      "\n")))"#
  );
}

// Container block nested inside a blockquote
// Known limitation: the closing ::: is re-parsed as a new container inside the blockquote
// because the blockquote prefix [>, " "] plus the container's extra " " require ">  :::"
// but the natural blockquote syntax uses "> :::" (one space)
#[test]
fn parse_container_nested_in_blockquote() {
  let input = "> ::: info\n>  content here\n> :::\n";
  let (ast, _) = parse(input);
  let tree = render_tree(&ast);
  assert_eq!(
    tree,
    r#"(SourceFile
  (YamlFrontmatter)
  (MdBody
    (MdBlockquote
      ">"
      " "
      (MdContainerBlock
        ":::"
        " "
        "info"
        "\n"
        (MdContainerSlot
          (MdBlockquote
            ">"
            " "
            (MdParagraph
              (MdText
                " "
                "content"
                " "
                "here"))))
        "\n"
        (MdContainerSlot
          (MdBlockquote
            ">"
            " "
            (MdContainerBlock
              ":::"
              "\n"
              (Error
                ""))))
        (Error
          "")))))"#
  );
}

// Container with slots nested inside a bullet list item
#[test]
fn parse_container_with_slots_nested_in_bullet_list() {
  let input = r#"- item
  ::: card
  front content
  === back
  back content
  :::
"#;
  let (ast, _) = parse(input);
  let tree = render_tree(&ast);
  assert_eq!(
    tree,
    r#"(SourceFile
  (YamlFrontmatter)
  (MdBody
    (MdBulletList
      (MdBulletListItem
        "-"
        " "
        (MdParagraph
          (MdText
            "item"))
        "\n"
        " "
        " "
        (MdContainerBlock
          ":::"
          " "
          "card"
          "\n"
          (MdContainerSlot
            (MdParagraph
              (MdText
                " "
                " "
                "front"
                " "
                "content")))
          "\n"
          " "
          " "
          (MdContainerSlotSeparator
            "==="
            " "
            "back"
            "\n")
          (MdContainerSlot
            (MdParagraph
              (MdText
                " "
                " "
                "back"
                " "
                "content")))
          "\n"
          " "
          " "
          ":::"))
      "\n")))"#
  );
}
