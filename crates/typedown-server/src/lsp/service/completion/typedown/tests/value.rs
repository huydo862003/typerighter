use super::fixtures::*;
use lsp_types::CompletionResponse;

#[test]
fn boolean_keywords_suggested_in_any_value_position() {
  // true/false are keywords usable in any value position, not limited to boolean-typed fields
  let (content, offset) = cursor(
    r#"---
_type: Person
name: tru|
---
"#,
  );
  let (analysis, uri) = setup(&content);
  let params = make_params(uri, &content, offset);

  let response = completion(&analysis, params);
  let Some(CompletionResponse::Array(items)) = response else {
    panic!("expected value completions");
  };
  let labels: Vec<&str> = items.iter().map(|item| item.label.as_str()).collect();
  assert!(
    labels.contains(&"true"),
    "should suggest 'true' in any value position"
  );
  assert!(
    labels.contains(&"false"),
    "should suggest 'false' in any value position"
  );
  assert!(
    !labels.contains(&"null"),
    "non-optional field should not suggest 'null'"
  );
}

#[test]
fn null_completion_for_optional_field() {
  // Cursor in the value of a nullable field: suggest null
  let (content, offset) = cursor(
    r#"---
_type: Person
nickname: nu|
---
"#,
  );
  let (analysis, uri) = setup(&content);
  let params = make_params(uri, &content, offset);

  let response = completion(&analysis, params);
  let Some(CompletionResponse::Array(items)) = response else {
    panic!("expected value completions");
  };
  let labels: Vec<&str> = items.iter().map(|item| item.label.as_str()).collect();
  assert!(
    labels.contains(&"null"),
    "optional field should suggest 'null'"
  );
}

// Global symbols (fref, string, number, vault, icon, etc.) should be suggested in expression positions:
// (|), [|], : [], f(|), f(a, |), | + 1, a - |
#[test]
fn global_symbols_suggested_in_expression_positions() {
  for raw_input in [
    r#"---
_type: Person
name: (|)
---
"#,
    r#"---
_type: Person
name: [|]
---
"#,
    r#"---
_type: Person
name: []|
---
"#,
    r#"---
_type: Person
name: f(|)
---
"#,
    r#"---
_type: Person
name: f(a, |)
---
"#,
    r#"---
_type: Person
name: | + 1
---
"#,
    r#"---
_type: Person
name: a - |
---
"#,
    r#"---
_type: Directory
featured: (|)
---
"#,
    r#"---
_type: Directory
featured: f(|)
---
"#,
  ] {
    let (content, offset) = cursor(raw_input);
    let (analysis, uri) = setup_with_content(&content);
    let params = make_params(uri, &content, offset);

    let response = completion(&analysis, params);
    let Some(CompletionResponse::Array(items)) = response else {
      panic!("expected completions for expression position:\n{raw_input}");
    };
    let labels: Vec<&str> = items.iter().map(|item| item.label.as_str()).collect();
    for expected in [
      "fref", "string", "number", "boolean", "vault", "icon", "self",
    ] {
      assert!(
        labels.contains(&expected),
        "should suggest global '{expected}' in:\n{raw_input}\ngot: {labels:?}"
      );
    }
  }
}

// Enum field suggests literal values
#[test]
fn enum_field_suggests_literal_values() {
  let (content, offset) = cursor(
    r#"---
_type: Task
status: t|
priority: "high"
---
"#,
  );
  let (analysis, uri) = setup(&content);
  let params = make_params(uri, &content, offset);

  let response = completion(&analysis, params);
  let Some(CompletionResponse::Array(items)) = response else {
    panic!("expected value completions for enum field");
  };
  let labels: Vec<&str> = items.iter().map(|item| item.label.as_str()).collect();
  assert!(
    labels.contains(&r#""todo""#),
    "should suggest 'todo': {:?}",
    labels
  );
  assert!(
    labels.contains(&r#""in_progress""#),
    "should suggest 'in_progress': {:?}",
    labels
  );
  assert!(
    labels.contains(&r#""done""#),
    "should suggest 'done': {:?}",
    labels
  );
}

// Date field suggests ISO 8601 placeholder and "today" with current date
#[test]
fn date_field_suggests_date_placeholder() {
  let (content, offset) = cursor(
    r#"---
_type: Event
date: d|
---
"#,
  );
  let (analysis, uri) = setup(&content);
  let params = make_params(uri, &content, offset);

  let response = completion(&analysis, params);
  let Some(CompletionResponse::Array(items)) = response else {
    panic!("expected value completions for date field");
  };
  let labels: Vec<&str> = items.iter().map(|item| item.label.as_str()).collect();
  assert!(
    labels.iter().any(|label| label.contains("YYYY-MM-DD")),
    "date field should suggest ISO placeholder: {:?}",
    labels
  );
  assert!(
    labels.contains(&"today"),
    "date field should suggest 'today': {:?}",
    labels
  );
  // "today" insert_text should be a quoted YYYY-MM-DD value
  let today_item = items.iter().find(|item| item.label == "today").unwrap();
  let insert = today_item.insert_text.as_deref().unwrap_or("");
  assert!(
    insert.starts_with('"') && insert.ends_with('"') && insert.len() == 12,
    "today insert_text should be a quoted YYYY-MM-DD, got: {:?}",
    insert
  );
}

// Time field suggests HH:MM:SS placeholder and "now" with current time
#[test]
fn time_field_suggests_time_placeholder_and_now() {
  let (content, offset) = cursor(
    r#"---
_type: Event
start_time: n|
---
"#,
  );
  let (analysis, uri) = setup(&content);
  let params = make_params(uri, &content, offset);

  let response = completion(&analysis, params);
  let Some(CompletionResponse::Array(items)) = response else {
    panic!("expected value completions for time field");
  };
  let labels: Vec<&str> = items.iter().map(|item| item.label.as_str()).collect();
  assert!(
    labels.contains(&"now"),
    "time field should suggest 'now': {:?}",
    labels
  );
  assert!(
    labels.iter().any(|label| label.contains("HH:MM:SS")),
    "time field should suggest HH:MM:SS placeholder: {:?}",
    labels
  );
}

// Datetime field suggests ISO datetime placeholder and "now" with current datetime
#[test]
fn datetime_field_suggests_datetime_placeholder_and_now() {
  let (content, offset) = cursor(
    r#"---
_type: Event
created_at: n|
---
"#,
  );
  let (analysis, uri) = setup(&content);
  let params = make_params(uri, &content, offset);

  let response = completion(&analysis, params);
  let Some(CompletionResponse::Array(items)) = response else {
    panic!("expected value completions for datetime field");
  };
  let labels: Vec<&str> = items.iter().map(|item| item.label.as_str()).collect();
  assert!(
    labels.contains(&"now"),
    "datetime field should suggest 'now': {:?}",
    labels
  );
  assert!(
    labels.iter().any(|label| label.contains("YYYY-MM-DD")),
    "datetime field should suggest ISO placeholder: {:?}",
    labels
  );
}

#[test]
fn value_position_suggests_builtin_variables() {
  let (content, offset) = cursor(
    r#"---
_type: Person
name: i|
---
"#,
  );
  let (analysis, uri) = setup(&content);
  let params = make_params(uri, &content, offset);

  let response = completion(&analysis, params);
  let Some(CompletionResponse::Array(items)) = response else {
    panic!("expected value completions");
  };
  let labels: Vec<&str> = items.iter().map(|item| item.label.as_str()).collect();
  assert!(labels.contains(&"icon"), "should suggest builtin 'icon'");
  assert!(labels.contains(&"fref"), "should suggest builtin 'fref'");
  assert!(labels.contains(&"vault"), "should suggest builtin 'vault'");
}

#[test]
fn value_position_suggests_typ_names() {
  let (content, offset) = cursor(
    r#"---
_type: Person
name: s|
---
"#,
  );
  let (analysis, uri) = setup(&content);
  let params = make_params(uri, &content, offset);

  let response = completion(&analysis, params);
  let Some(CompletionResponse::Array(items)) = response else {
    panic!("expected value completions");
  };
  let labels: Vec<&str> = items.iter().map(|item| item.label.as_str()).collect();
  assert!(
    labels.contains(&"string"),
    "should suggest builtin type 'string'"
  );
  assert!(
    labels.contains(&"number"),
    "should suggest builtin type 'number'"
  );
  assert!(
    labels.contains(&"boolean"),
    "should suggest builtin type 'boolean'"
  );
}

#[test]
fn value_position_suggests_project_file_names() {
  let (content, offset) = cursor(
    r#"---
_type: Person
name: a|
---
"#,
  );
  let (analysis, uri) = setup_with_content(&content);
  let params = make_params(uri, &content, offset);

  let response = completion(&analysis, params);
  let Some(CompletionResponse::Array(items)) = response else {
    panic!("expected value completions");
  };
  let labels: Vec<&str> = items.iter().map(|item| item.label.as_str()).collect();
  assert!(
    labels.contains(&"alice"),
    "should suggest project file 'alice'"
  );
  assert!(
    labels.contains(&"birthday"),
    "should suggest project file 'birthday'"
  );
}

#[test]
fn value_position_excludes_underscore_prefixed() {
  let (content, offset) = cursor(
    r#"---
_type: Person
name: x|
---
"#,
  );
  let (analysis, uri) = setup(&content);
  let params = make_params(uri, &content, offset);

  let response = completion(&analysis, params);
  let Some(CompletionResponse::Array(items)) = response else {
    panic!("expected value completions");
  };
  let labels: Vec<&str> = items.iter().map(|item| item.label.as_str()).collect();
  assert!(
    !labels.iter().any(|label| label.starts_with('_')),
    "should not suggest underscore-prefixed items: {:?}",
    labels
  );
}

#[test]
fn value_position_includes_keywords_and_variables() {
  let (content, offset) = cursor(
    r#"---
_type: Person
name: ic|
---
"#,
  );
  let (analysis, uri) = setup(&content);
  let params = make_params(uri, &content, offset);

  let response = completion(&analysis, params);
  let Some(CompletionResponse::Array(items)) = response else {
    panic!("expected value completions");
  };
  let labels: Vec<&str> = items.iter().map(|item| item.label.as_str()).collect();
  // Keywords
  assert!(labels.contains(&"true"), "should include keyword 'true'");
  assert!(labels.contains(&"false"), "should include keyword 'false'");
  // Variables
  assert!(labels.contains(&"icon"), "should include variable 'icon'");
}

#[test]
fn closure_param_suggested_in_body() {
  let (content, offset) = cursor(
    r#"---
_type: Person
name: (x) -> x|
---
"#,
  );
  let (analysis, uri) = setup(&content);
  let params = make_params(uri, &content, offset);

  let response = completion(&analysis, params);
  let Some(CompletionResponse::Array(items)) = response else {
    panic!("expected completions inside closure body");
  };
  let labels: Vec<&str> = items.iter().map(|item| item.label.as_str()).collect();
  assert!(
    labels.contains(&"x"),
    "should suggest closure param 'x': {:?}",
    labels
  );
  assert!(
    labels.contains(&"icon"),
    "should also suggest builtins: {:?}",
    labels
  );
}

#[test]
fn multi_param_closure_suggests_all_params() {
  let (content, offset) = cursor(
    r#"---
_type: Person
name: (a, b) -> a|
---
"#,
  );
  let (analysis, uri) = setup(&content);
  let params = make_params(uri, &content, offset);

  let response = completion(&analysis, params);
  let Some(CompletionResponse::Array(items)) = response else {
    panic!("expected completions inside closure body");
  };
  let labels: Vec<&str> = items.iter().map(|item| item.label.as_str()).collect();
  assert!(
    labels.contains(&"a"),
    "should suggest param 'a': {:?}",
    labels
  );
  assert!(
    labels.contains(&"b"),
    "should suggest param 'b': {:?}",
    labels
  );
}

#[test]
fn self_suggested_in_value_position() {
  let (content, offset) = cursor(
    r#"---
_type: Person
name: s|
---
"#,
  );
  let (analysis, uri) = setup(&content);
  let params = make_params(uri, &content, offset);

  let response = completion(&analysis, params);
  let Some(CompletionResponse::Array(items)) = response else {
    panic!("expected value completions");
  };
  let labels: Vec<&str> = items.iter().map(|item| item.label.as_str()).collect();
  assert!(
    labels.contains(&"self"),
    "should suggest 'self': {:?}",
    labels
  );
}
