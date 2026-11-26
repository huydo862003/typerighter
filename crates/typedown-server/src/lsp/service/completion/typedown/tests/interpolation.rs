use super::fixtures::*;
use lsp_types::CompletionResponse;

// Inside ${} interpolation, suggest fref("path") completions and scope variables
#[test]
fn fref_completion_in_interpolation() {
  let (content, offset) = cursor(
    r#"---
_type: Person
name: Alice
---

Reference: ${|}
"#,
  );
  let (analysis, uri) = setup_with_content(&content);
  let params = make_params(uri, &content, offset);

  let response = completion(&analysis, params);
  let Some(CompletionResponse::Array(items)) = response else {
    panic!("expected fref completions inside interpolation");
  };

  assert!(
    !items.is_empty(),
    "should suggest fref completions inside ${{}}"
  );

  // Should have fref("path") as insert text
  let has_fref_insert = items.iter().any(|item| {
    item
      .insert_text
      .as_deref()
      .is_some_and(|text| text.starts_with(r#"fref(""#) && text.ends_with(r#"")"#))
  });
  assert!(
    has_fref_insert,
    "should wrap path in fref(): {:?}",
    items
      .iter()
      .map(|item| (&item.label, &item.insert_text))
      .collect::<Vec<_>>()
  );

  // Label should be the display name, not the path
  let alice_item = items
    .iter()
    .find(|item| item.label == "Alice Chen")
    .or_else(|| items.iter().find(|item| item.label.contains("alice")));
  assert!(
    alice_item.is_some(),
    "should have alice completion with human-readable label: {:?}",
    items.iter().map(|item| &item.label).collect::<Vec<_>>()
  );

  // Detail should be the path
  let alice = alice_item.unwrap();
  assert!(
    alice
      .detail
      .as_deref()
      .is_some_and(|detail| detail.contains("alice")),
    "detail should contain the file path: {:?}",
    alice.detail
  );

  // Should also have scope variables like `self`, `fref`, `icon`
  let labels: Vec<&str> = items.iter().map(|item| item.label.as_str()).collect();
  assert!(
    labels.contains(&"self"),
    "should suggest 'self' variable inside ${{}}:\n{labels:?}"
  );
  assert!(
    labels.contains(&"fref"),
    "should suggest 'fref' builtin inside ${{}}:\n{labels:?}"
  );
}
