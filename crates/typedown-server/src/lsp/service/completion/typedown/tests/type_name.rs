use super::fixtures::*;
use lsp_types::{CompletionItemKind, CompletionResponse};

#[test]
fn schema_name_completion_in_typ_value() {
  let (content, offset) = cursor(
    r#"---
_type: |
---
"#,
  );
  let (analysis, uri) = setup(&content);
  let params = make_params(uri, &content, offset);

  let response = completion(&analysis, params);
  let Some(CompletionResponse::Array(items)) = response else {
    panic!("expected completion items");
  };
  let labels: Vec<&str> = items.iter().map(|item| item.label.as_str()).collect();
  assert!(labels.contains(&"Person"), "should suggest Person schema");
  assert!(labels.contains(&"Event"), "should suggest Event schema");
  // Should NOT suggest non-schema items (field names, file paths, etc.)
  assert!(
    !labels.contains(&"name"),
    "should not suggest field names in _type position"
  );
  assert!(
    !labels.contains(&"age"),
    "should not suggest field names in _type position"
  );
  // All items should be class-kind (schemas)
  assert!(
    items
      .iter()
      .all(|item| item.kind == Some(CompletionItemKind::CLASS)),
    "all _type completions should be CLASS kind"
  );
}

#[test]
fn schema_completion_generates_snippet_with_fields() {
  let (content, offset) = cursor(
    r#"---
_type: |
---
"#,
  );
  let (analysis, uri) = setup(&content);
  let params = make_params(uri, &content, offset);

  let response = completion(&analysis, params);
  let Some(CompletionResponse::Array(items)) = response else {
    panic!("expected completion items");
  };

  let person = items
    .iter()
    .find(|item| item.label == "Person")
    .expect("should have Person");
  let snippet = person
    .insert_text
    .as_ref()
    .expect("should have insert_text");

  assert!(
    snippet.starts_with("Person\n"),
    "snippet should start with schema name: {snippet}"
  );
  assert!(
    snippet.contains("name:"),
    "snippet should contain name field: {snippet}"
  );
  assert!(
    snippet.contains("age:"),
    "snippet should contain age field: {snippet}"
  );
  assert!(
    snippet.contains("string"),
    "string field should have string placeholder: {snippet}"
  );
  assert!(
    snippet.contains(":0}"),
    "number field should have 0 placeholder: {snippet}"
  );
  assert!(
    person.insert_text_format == Some(lsp_types::InsertTextFormat::SNIPPET),
    "should use snippet format"
  );
}

#[test]
fn schema_snippet_includes_enum_and_relation() {
  let (content, offset) = cursor(
    r#"---
_type: |
---
"#,
  );
  let (analysis, uri) = setup(&content);
  let params = make_params(uri, &content, offset);

  let response = completion(&analysis, params);
  let Some(CompletionResponse::Array(items)) = response else {
    panic!("expected completion items");
  };

  let task = items
    .iter()
    .find(|item| item.label == "Task")
    .expect("should have Task");
  let snippet = task.insert_text.as_ref().expect("should have insert_text");

  assert!(
    snippet.contains("done") || snippet.contains("todo"),
    "enum field should have first option as placeholder: {snippet}"
  );
  assert!(
    snippet.contains(r#"fref("Person")"#),
    "relation field should have fref placeholder: {snippet}"
  );
}

#[test]
fn schema_name_completion_while_partially_typd() {
  // Cursor in the middle of a partially typed schema name
  let (content, offset) = cursor(
    r#"---
_type: Per|
---
"#,
  );
  let (analysis, uri) = setup(&content);
  let params = make_params(uri, &content, offset);

  let response = completion(&analysis, params);
  let Some(CompletionResponse::Array(items)) = response else {
    panic!("expected completion items");
  };
  let labels: Vec<&str> = items.iter().map(|item| item.label.as_str()).collect();
  assert!(labels.contains(&"Person"), "should still suggest Person");
}
