use super::fixtures::*;
use lsp_types::CompletionResponse;

#[test]
fn no_completion_in_plain_markdown_body() {
  let (content, offset) = cursor(
    r#"---
_type: Person
name: Alice
---

Some bod|y text.
"#,
  );
  let (analysis, uri) = setup(&content);
  let params = make_params(uri, &content, offset);

  let response = completion(&analysis, params);
  let is_empty = response.is_none()
    || matches!(response, Some(CompletionResponse::Array(ref items)) if items.is_empty());
  assert!(
    is_empty,
    "should not suggest anything in plain markdown body"
  );
}

#[test]
fn no_completions_inside_code_block() {
  let (content, offset) = cursor(
    r#"---
_type: Person
name: Alice
---

```js
co|de here
```
"#,
  );
  let (analysis, uri) = setup(&content);
  let params = make_params(uri, &content, offset);
  let response = completion(&analysis, params);
  assert!(
    response.is_none(),
    "should not suggest completions inside a code block"
  );
}

// No completions when cursor is inside a quoted string in code block body
#[test]
fn no_completions_inside_code_block_string_content() {
  let (content, offset) = cursor(
    r#"---
_type: Person
name: Alice
---

```
'tex|t'
```
"#,
  );
  let (analysis, uri) = setup(&content);
  let params = make_params(uri, &content, offset);
  let response = completion(&analysis, params);
  assert!(
    response.is_none(),
    "should not suggest completions inside a string within a code block"
  );
}

// No completions when cursor is inside a quoted string in the code fence info string
#[test]
fn no_completions_inside_code_block_string_lang() {
  let (content, offset) = cursor(
    r#"---
_type: Person
name: Alice
---

```'j|s'
code here
```
"#,
  );
  let (analysis, uri) = setup(&content);
  let params = make_params(uri, &content, offset);
  let response = completion(&analysis, params);
  assert!(
    response.is_none(),
    "should not suggest completions inside a quoted lang in a code block"
  );
}

#[test]
fn no_completions_inside_inline_code() {
  let (content, offset) = cursor(
    r#"---
_type: Person
name: Alice
---

Some text with `inl|ine code` here.
"#,
  );
  let (analysis, uri) = setup(&content);
  let params = make_params(uri, &content, offset);
  let response = completion(&analysis, params);
  assert!(
    response.is_none(),
    "should not suggest completions inside inline code"
  );
}

#[test]
fn no_completions_inside_inline_math() {
  let (content, offset) = cursor(
    r#"---
_type: Person
name: Alice
---

Some text with $x^|2$ here.
"#,
  );
  let (analysis, uri) = setup(&content);
  let params = make_params(uri, &content, offset);
  let response = completion(&analysis, params);
  assert!(
    response.is_none(),
    "should not suggest completions inside inline math"
  );
}

#[test]
fn no_completions_inside_math_block() {
  let (content, offset) = cursor(
    r#"---
_type: Person
name: Alice
---

$$
x^|2
$$
"#,
  );
  let (analysis, uri) = setup(&content);
  let params = make_params(uri, &content, offset);
  let response = completion(&analysis, params);
  assert!(
    response.is_none(),
    "should not suggest completions inside a math block"
  );
}
