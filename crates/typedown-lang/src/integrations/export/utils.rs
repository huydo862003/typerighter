//! Shared utilities for HTML and markdown export

use crate::syntax::red::RedNode;
use crate::syntax::syntax_kind::SyntaxKind;

pub fn slugify(text: &str) -> String {
  let mut slug = String::new();
  let mut prev_hyphen = false;

  for ch in text.chars() {
    if ch.is_alphanumeric() {
      slug.push(ch.to_ascii_lowercase());
      prev_hyphen = false;
    } else if !slug.is_empty() && !prev_hyphen {
      slug.push('-');
      prev_hyphen = true;
    }
  }

  slug.trim_end_matches('-').to_string()
}

pub fn html_escape(text: &str) -> String {
  let mut out = String::with_capacity(text.len());
  for ch in text.chars() {
    match ch {
      '<' => out.push_str("&lt;"),
      '>' => out.push_str("&gt;"),
      '&' => out.push_str("&amp;"),
      '"' => out.push_str("&quot;"),
      '\'' => out.push_str("&#39;"),
      _ => out.push(ch),
    }
  }
  out
}

pub fn strip_quotes(s: &str) -> &str {
  if (s.starts_with('"') && s.ends_with('"')) || (s.starts_with('\'') && s.ends_with('\'')) {
    &s[1..s.len() - 1]
  } else {
    s
  }
}

pub fn is_external_url(url: &str) -> bool {
  url.starts_with("http://")
    || url.starts_with("https://")
    || url.starts_with("//")
    || url.starts_with("mailto:")
}

pub fn extract_plain_text(node: &RedNode) -> String {
  let mut text = String::new();
  collect_plain_text(node, &mut text);
  text.trim().to_string()
}

fn collect_plain_text(node: &RedNode, buf: &mut String) {
  if let Some(token) = node.as_token() {
    let text = token.text().unwrap_or("");
    if !is_delimiter(text) && node.kind() != SyntaxKind::MdSymbol {
      buf.push_str(text);
    }
  } else {
    for child in node.children() {
      collect_plain_text(&child, buf);
    }
  }
}

pub fn is_delimiter(text: &str) -> bool {
  matches!(
    text,
    "**"
      | "*"
      | "_"
      | "~~"
      | "***"
      | "["
      | "]"
      | "("
      | ")"
      | "!"
      | "#"
      | "##"
      | "###"
      | "####"
      | "#####"
      | "######"
  )
}

// Flatten structural wrappers to get a flat list of inline-level children
// Tokens and inline elements are collected directly, structural nodes are recursed into
pub fn collect_inline_children(node: &RedNode) -> Vec<RedNode> {
  let mut result = Vec::new();
  for child in node.children() {
    let kind = child.kind();
    if child.as_token().is_some() || kind.is_md_inline() || kind == SyntaxKind::InterpFragment {
      result.push(child);
    } else {
      result.extend(collect_inline_children(&child));
    }
  }
  result
}

#[cfg(test)]
mod tests {
  use super::*;

  // Lowercase, non-alphanum becomes hyphens, collapsed
  #[test]
  fn slugify_basic() {
    assert_eq!(slugify("Hello World"), "hello-world");
    assert_eq!(slugify("  Foo  Bar  "), "foo-bar");
    assert_eq!(slugify("UPPER CASE"), "upper-case");
  }

  // Special characters stripped, consecutive hyphens collapsed
  #[test]
  fn slugify_special_chars() {
    assert_eq!(slugify("Hello & World!"), "hello-world");
    assert_eq!(slugify("foo/bar"), "foo-bar");
  }

  // HTML special characters escaped
  #[test]
  fn html_escape_basic() {
    assert_eq!(html_escape("<div>"), "&lt;div&gt;");
    assert_eq!(html_escape("a & b"), "a &amp; b");
    assert_eq!(html_escape("\"quoted\""), "&quot;quoted&quot;");
    assert_eq!(html_escape("it's"), "it&#39;s");
  }

  // Protocol-based external URL detection
  #[test]
  fn external_url_detection() {
    assert!(is_external_url("https://example.com"));
    assert!(is_external_url("http://example.com"));
    assert!(is_external_url("//cdn.example.com"));
    assert!(is_external_url("mailto:a@b.com"));
    assert!(!is_external_url("./page"));
    assert!(!is_external_url("/page"));
    assert!(!is_external_url("#anchor"));
  }
}
