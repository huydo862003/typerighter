use super::fixtures::*;
use lsp_types::{CompletionItemKind, CompletionResponse};

// _imports value: cursor inside a string value inside _imports mapping suggests file paths
#[test]
fn imports_value_completion_suggests_file_paths() {
  let (content, offset) = cursor(
    r#"---
_imports:
  colors: "|"
---
"#,
  );
  let (analysis, uri) = setup_with_content(&content);
  let params = make_params(uri, &content, offset);
  let response = completion(&analysis, params);
  let Some(CompletionResponse::Array(items)) = response else {
    panic!("expected completion items for _imports value");
  };
  assert!(
    !items.is_empty(),
    "should suggest file paths inside _imports value"
  );
  // All suggestions should have vault-root-relative paths starting with /
  for item in &items {
    let path = item.detail.as_deref().unwrap_or("");
    assert!(
      path.starts_with('/'),
      "import path suggestion should start with /: {path}"
    );
  }
}

// _imports value: should suggest alice.td and birthday.td content files
#[test]
fn imports_value_completion_includes_content_files() {
  let (content, offset) = cursor(
    r#"---
_imports:
  data: "|"
---
"#,
  );
  let (analysis, uri) = setup_with_content(&content);
  let params = make_params(uri, &content, offset);
  let response = completion(&analysis, params);
  let Some(CompletionResponse::Array(items)) = response else {
    panic!("expected completion items");
  };
  let details: Vec<&str> = items
    .iter()
    .filter_map(|item| item.detail.as_deref())
    .collect();
  assert!(
    details.iter().any(|path| path.contains("alice")),
    "should suggest alice.td: {details:?}"
  );
  assert!(
    details.iter().any(|path| path.contains("birthday")),
    "should suggest birthday.td: {details:?}"
  );
}

// _imports value: not triggered outside _imports mapping
#[test]
fn imports_value_completion_not_triggered_outside_imports() {
  let (content, offset) = cursor(
    r#"---
_type: Person
name: "|"
---
"#,
  );
  let (analysis, uri) = setup_with_content(&content);
  let params = make_params(uri, &content, offset);
  let response = completion(&analysis, params);
  // Should not return imports-style completions (REFERENCE items for content files)
  // The name field is a string field so no fref/imports completions expected
  if let Some(CompletionResponse::Array(items)) = response {
    let has_alice_ref = items.iter().any(|item| {
      item.kind == Some(CompletionItemKind::REFERENCE)
        && item
          .detail
          .as_deref()
          .map(|diagnostic| diagnostic.contains("alice"))
          .unwrap_or(false)
    });
    assert!(
      !has_alice_ref,
      "string field should not suggest _imports-style file completions: {:?}",
      items
        .iter()
        .map(|item| (&item.label, &item.detail))
        .collect::<Vec<_>>()
    );
  }
}
