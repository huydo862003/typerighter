use super::fixtures::*;
use lsp_types::{CompletionItem, CompletionItemKind, CompletionResponse};

#[test]
fn field_completion_based_on_declared_typ() {
  // Cursor after typing a partial key, _typ already set
  let (content, offset) = cursor(
    r#"---
_type: Person
na|:
---
"#,
  );
  let (analysis, uri) = setup(&content);
  let params = make_params(uri, &content, offset);

  let response = completion(&analysis, params);
  let Some(CompletionResponse::Array(items)) = response else {
    panic!("expected field completions");
  };
  let labels: Vec<&str> = items.iter().map(|item| item.label.as_str()).collect();
  assert!(labels.contains(&"name"), "should suggest 'name' field");
  assert!(labels.contains(&"age"), "should suggest 'age' field");
  // Should NOT suggest _type itself as a field completion
  assert!(
    !labels.contains(&"_type"),
    "'_type' should not be suggested as a field completion"
  );
}

#[test]
fn field_completion_when_typ_declared_after_other_fields() {
  // _typ appears after the cursor position in the mapping
  let (content, offset) = cursor(
    r#"---
name: Alice
ag|:
_type: Person
---
"#,
  );
  let (analysis, uri) = setup(&content);
  let params = make_params(uri, &content, offset);

  let response = completion(&analysis, params);
  let Some(CompletionResponse::Array(items)) = response else {
    panic!("expected field completions");
  };
  let labels: Vec<&str> = items.iter().map(|item| item.label.as_str()).collect();
  assert!(
    !labels.contains(&"name"),
    "should not suggest 'name' (already exists)"
  );
  assert!(labels.contains(&"age"), "should suggest 'age' field");
  assert!(
    labels.contains(&"verified"),
    "should suggest 'verified' field"
  );
}

#[test]
fn no_field_completion_without_typ() {
  // No _typ in mapping: no field completions expected
  let (content, offset) = cursor(
    r#"---
na|:
---
"#,
  );
  let (analysis, uri) = setup(&content);
  let params = make_params(uri, &content, offset);

  let response = completion(&analysis, params);
  let is_empty = response.is_none()
    || matches!(response, Some(CompletionResponse::Array(ref items)) if items.is_empty());
  assert!(is_empty, "should not suggest fields when _typ is absent");
}

#[test]
fn field_completion_on_blank_line_between_fields() {
  let (content, offset) = cursor(
    r#"---
_type: Person
name: Alice
|
age: 30
---
"#,
  );
  let (analysis, uri) = setup(&content);
  let params = make_params(uri, &content, offset);

  let response = completion(&analysis, params);
  let Some(CompletionResponse::Array(items)) = response else {
    panic!("expected field completions on blank line");
  };
  let labels: Vec<&str> = items.iter().map(|item| item.label.as_str()).collect();
  assert!(
    labels.contains(&"verified"),
    "should suggest 'verified' on blank line between fields"
  );
}

#[test]
fn field_completion_on_last_line_of_mapping() {
  let (content, offset) = cursor(
    r#"---
_type: Person
name: Alice
|
---
"#,
  );
  let (analysis, uri) = setup(&content);
  let params = make_params(uri, &content, offset);

  let response = completion(&analysis, params);
  let Some(CompletionResponse::Array(items)) = response else {
    panic!("expected field completions on last line of mapping");
  };
  let labels: Vec<&str> = items.iter().map(|item| item.label.as_str()).collect();
  assert!(
    labels.contains(&"age"),
    "should suggest 'age' on last blank line"
  );
}

#[test]
fn field_completion_excludes_existing_keys() {
  let (content, offset) = cursor(
    r#"---
_type: Person
name: Alice
age: 30
|
---
"#,
  );
  let (analysis, uri) = setup(&content);
  let params = make_params(uri, &content, offset);

  let response = completion(&analysis, params);
  let Some(CompletionResponse::Array(items)) = response else {
    panic!("expected field completions");
  };
  let labels: Vec<&str> = items.iter().map(|item| item.label.as_str()).collect();
  assert!(
    !labels.contains(&"name"),
    "should not suggest 'name' (already exists)"
  );
  assert!(
    !labels.contains(&"age"),
    "should not suggest 'age' (already exists)"
  );
  assert!(
    labels.contains(&"verified"),
    "should suggest 'verified' (not yet used)"
  );
  assert!(
    labels.contains(&"nickname"),
    "should suggest 'nickname' (not yet used)"
  );
}

#[test]
fn field_completion_on_key_excludes_other_existing_keys() {
  let (content, offset) = cursor(
    r#"---
_type: Person
name: Alice
ver|:
---
"#,
  );
  let (analysis, uri) = setup(&content);
  let params = make_params(uri, &content, offset);

  let response = completion(&analysis, params);
  let Some(CompletionResponse::Array(items)) = response else {
    panic!("expected field completions");
  };
  let labels: Vec<&str> = items.iter().map(|item| item.label.as_str()).collect();
  assert!(
    !labels.contains(&"name"),
    "should not suggest 'name' (already exists)"
  );
  assert!(labels.contains(&"age"), "should suggest 'age'");
  assert!(labels.contains(&"verified"), "should suggest 'verified'");
}

// Cursor on a key inside a nested mapping whose type is inferred from the parent schema field
#[test]
fn field_completion_in_nested_mapping_without_typ() {
  let (content, offset) = cursor(
    r#"---
_type: PersonWithAddress
name: Alice
address:
  str|:
---
"#,
  );
  let (analysis, uri) = setup(&content);
  let params = make_params(uri, &content, offset);

  let response = completion(&analysis, params);
  let Some(CompletionResponse::Array(items)) = response else {
    panic!("expected field completions for nested address mapping");
  };
  let labels: Vec<&str> = items.iter().map(|item| item.label.as_str()).collect();
  assert!(
    labels.contains(&"street"),
    "should suggest 'street' from nested address type, got: {:?}",
    labels
  );
  assert!(
    labels.contains(&"city"),
    "should suggest 'city' from nested address type, got: {:?}",
    labels
  );
}

#[test]
fn field_completions_show_required_and_optional() {
  let (content, offset) = cursor(
    r#"---
_type: Person
|
---
"#,
  );
  let (analysis, uri) = setup(&content);
  let params = make_params(uri, &content, offset);

  let response = completion(&analysis, params);
  let Some(CompletionResponse::Array(items)) = response else {
    panic!("expected field completions");
  };

  let name_item = items
    .iter()
    .find(|item| item.label == "name")
    .expect("should suggest name");
  assert_eq!(name_item.detail.as_deref(), Some("required"));

  let nickname_item = items
    .iter()
    .find(|item| item.label == "nickname")
    .expect("should suggest nickname");
  assert_eq!(nickname_item.detail.as_deref(), Some("optional"));
}

#[test]
fn required_fields_sorted_before_optional() {
  let (content, offset) = cursor(
    r#"---
_type: Person
|
---
"#,
  );
  let (analysis, uri) = setup(&content);
  let params = make_params(uri, &content, offset);

  let response = completion(&analysis, params);
  let Some(CompletionResponse::Array(items)) = response else {
    panic!("expected field completions");
  };

  let field_items: Vec<&CompletionItem> = items
    .iter()
    .filter(|item| item.kind == Some(CompletionItemKind::FIELD))
    .collect();

  // All required fields should have sort_text starting with "0"
  // All optional fields should have sort_text starting with "1"
  for item in &field_items {
    let sort = item.sort_text.as_deref().unwrap_or("");
    let detail = item.detail.as_deref().unwrap_or("");
    if detail == "required" {
      assert!(
        sort.starts_with('0'),
        "required field {} should sort first",
        item.label
      );
    } else {
      assert!(
        sort.starts_with('1'),
        "optional field {} should sort after required",
        item.label
      );
    }
  }
}

#[test]
fn meta_snippet_suggested_in_field_position() {
  let (content, offset) = cursor(
    r#"---
_type: Person
name: "Alice"
|
---
"#,
  );
  let (analysis, uri) = setup(&content);
  let params = make_params(uri, &content, offset);

  let response = completion(&analysis, params);
  let Some(CompletionResponse::Array(items)) = response else {
    panic!("expected field completions");
  };

  let meta_item = items
    .iter()
    .find(|item| item.label == "_meta")
    .expect("should suggest _meta");
  assert_eq!(meta_item.kind, Some(CompletionItemKind::SNIPPET));
  assert_eq!(meta_item.detail.as_deref(), Some("SEO overrides"));
  assert!(
    meta_item.insert_text.as_ref().unwrap().contains("title"),
    "_meta snippet should contain title placeholder"
  );
}

#[test]
fn meta_snippet_not_suggested_when_already_present() {
  let (content, offset) = cursor(
    r#"---
_type: Person
_meta:
  title: "Custom"
name: "Alice"
|
---
"#,
  );
  let (analysis, uri) = setup(&content);
  let params = make_params(uri, &content, offset);

  let response = completion(&analysis, params);
  let Some(CompletionResponse::Array(items)) = response else {
    panic!("expected field completions");
  };

  assert!(
    !items.iter().any(|item| item.label == "_meta"),
    "_meta should not be suggested when already present"
  );
}

// Cursor on a blank line inside a nested mapping with no prior entries
// (the inner mapping has not been started yet, so the parser sees value position)
#[test]
fn field_completion_on_blank_line_in_nested_mapping_no_prior_entries() {
  let (content, offset) = cursor(
    r#"---
_type: PersonWithAddress
name: Alice
address:
  |
---
"#,
  );
  let (analysis, uri) = setup(&content);
  let params = make_params(uri, &content, offset);

  let response = completion(&analysis, params);
  let Some(CompletionResponse::Array(items)) = response else {
    panic!("expected field completions for blank line under address");
  };
  let labels: Vec<&str> = items.iter().map(|item| item.label.as_str()).collect();
  assert!(
    labels.contains(&"street"),
    "should suggest 'street' from address type, got: {:?}",
    labels
  );
  assert!(
    labels.contains(&"city"),
    "should suggest 'city' from address type, got: {:?}",
    labels
  );
}

// Cursor on a partial key without colon inside a nested mapping
#[test]
fn field_completion_on_partial_key_without_colon_in_nested_mapping() {
  let (content, offset) = cursor(
    r#"---
_type: PersonWithAddress
name: Alice
address:
  str|
---
"#,
  );
  let (analysis, uri) = setup(&content);
  let params = make_params(uri, &content, offset);

  let response = completion(&analysis, params);
  let Some(CompletionResponse::Array(items)) = response else {
    panic!("expected field completions for partial key under address");
  };
  let labels: Vec<&str> = items.iter().map(|item| item.label.as_str()).collect();
  assert!(
    labels.contains(&"street"),
    "should suggest 'street' from address type, got: {:?}",
    labels
  );
}
