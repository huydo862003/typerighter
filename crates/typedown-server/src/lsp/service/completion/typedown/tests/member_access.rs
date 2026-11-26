use super::fixtures::*;
use lsp_types::CompletionResponse;

#[test]
fn dot_access_on_icon_suggests_icon_names() {
  let (content, offset) = cursor(
    r#"---
_icon: icon.|
---
"#,
  );
  let (analysis, uri) = setup(&content);
  let params = make_params(uri, &content, offset);

  let response = completion(&analysis, params);
  let Some(CompletionResponse::Array(items)) = response else {
    panic!("expected dot-access completions for icon");
  };
  let labels: Vec<&str> = items.iter().map(|item| item.label.as_str()).collect();
  assert!(
    labels.contains(&"book"),
    "should suggest 'book' icon: {:?}",
    labels
  );
  assert!(
    labels.contains(&"star"),
    "should suggest 'star' icon: {:?}",
    labels
  );
  // Should NOT include the "icon." prefix (just the member name)
  assert!(
    !labels.iter().any(|label| label.contains('.')),
    "labels should be bare names, not dotted: {:?}",
    labels
  );
}

#[test]
fn dot_access_on_icon_with_partial_input() {
  let (content, offset) = cursor(
    r#"---
_icon: icon.bo|
---
"#,
  );
  let (analysis, uri) = setup(&content);
  let params = make_params(uri, &content, offset);

  let response = completion(&analysis, params);
  let Some(CompletionResponse::Array(items)) = response else {
    panic!("expected dot-access completions");
  };
  let labels: Vec<&str> = items.iter().map(|item| item.label.as_str()).collect();
  assert!(
    labels.contains(&"book"),
    "should suggest 'book' with partial 'bo': {:?}",
    labels
  );
}

#[test]
fn no_dot_access_before_dot_typd() {
  // Typing just "ic" in _icon should NOT trigger icon member completions
  let (content, offset) = cursor(
    r#"---
_icon: ic|
---
"#,
  );
  let (analysis, uri) = setup(&content);
  let params = make_params(uri, &content, offset);

  let response = completion(&analysis, params);
  if let Some(CompletionResponse::Array(items)) = response {
    let labels: Vec<&str> = items.iter().map(|item| item.label.as_str()).collect();
    // Should not have icon member names like "book" without the dot
    assert!(
      !labels.contains(&"book"),
      "should not suggest icon members without dot: {:?}",
      labels
    );
  }
}

#[test]
fn dot_access_on_icon_in_regular_field() {
  // Dot-access should work in any value position, not just _icon
  let (content, offset) = cursor(
    r#"---
_type: Person
name: icon.|
---
"#,
  );
  let (analysis, uri) = setup(&content);
  let params = make_params(uri, &content, offset);

  let response = completion(&analysis, params);
  let Some(CompletionResponse::Array(items)) = response else {
    panic!("expected dot-access completions");
  };
  let labels: Vec<&str> = items.iter().map(|item| item.label.as_str()).collect();
  assert!(
    labels.contains(&"book"),
    "dot-access should work in any field: {:?}",
    labels
  );
}

// After ${fref("...").| in markdown body, only schema field names should be suggested,
// not file paths or scope variables from the interp fallthrough
#[test]
fn dot_access_on_fref_in_markdown_body_suggests_fields_not_file_paths() {
  let (content, offset) = cursor(
    r#"---
_type: Person
name: Alice
---

${fref("./alice.td").|
"#,
  );
  let (analysis, uri) = setup_with_content(&content);
  let params = make_params(uri, &content, offset);

  let response = completion(&analysis, params);
  let Some(CompletionResponse::Array(items)) = response else {
    panic!("expected dot-access completions in markdown body");
  };

  let labels: Vec<&str> = items.iter().map(|item| item.label.as_str()).collect();

  // Should suggest schema field names
  assert!(
    labels.contains(&"name"),
    "should suggest 'name' field from Person schema: {:?}",
    labels
  );
  assert!(
    labels.contains(&"age"),
    "should suggest 'age' field from Person schema: {:?}",
    labels
  );

  // Should NOT suggest file paths or fref-wrapped completions
  let has_fref_completion = items.iter().any(|item| {
    item
      .insert_text
      .as_deref()
      .is_some_and(|text| text.starts_with("fref("))
  });
  assert!(
    !has_fref_completion,
    "should not suggest fref() file completions after dot: {:?}",
    labels
  );

  // Should NOT suggest scope variables like 'self' or 'fref' (those are for interp, not dot access)
  assert!(
    !labels.contains(&"self"),
    "should not suggest 'self' in dot-access position: {:?}",
    labels
  );
}

#[test]
fn computed_closure_dot_access_suggests_schema_fields() {
  let (content, offset) = cursor(
    r#"---
_type: schema
properties:
  name:
    type: string
  age:
    type: number
  label:
    type: string
    computed: (r) -> r.|
---
"#,
  );
  let (analysis, uri) = setup_schema(&content);
  let params = make_params(uri, &content, offset);

  let response = completion(&analysis, params);
  let Some(CompletionResponse::Array(items)) = response else {
    panic!("expected completions for r. inside computed closure");
  };
  let labels: Vec<&str> = items.iter().map(|item| item.label.as_str()).collect();
  assert!(
    labels.contains(&"name"),
    "r. should suggest 'name' field: {:?}",
    labels
  );
  assert!(
    labels.contains(&"age"),
    "r. should suggest 'age' field: {:?}",
    labels
  );
}
