use super::fixtures::*;
use crate::core::analysis::Analysis;
use crate::core::utils::uri::path_to_uri;
use lsp_types::{CompletionItemKind, CompletionResponse};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, Condvar, Mutex};
use typedown_lang::db::types::{File, FileHandle, FileMetadata, Project};
use typedown_lang::db::{QueryStorage, TypedownDatabase};

#[test]
fn fref_completion_filters_by_declared_field_typ() {
  // The 'featured' field on Directory expects type Person
  // Only content/alice.td (_type: Person) should be suggested, not content/birthday.td (_type: Event)
  let (content, offset) = cursor(
    r#"---
_type: Directory
featured: fref("|")
---
"#,
  );
  let (analysis, uri) = setup_with_content(&content);
  let params = make_params(uri, &content, offset);

  let response = completion(&analysis, params);
  let Some(CompletionResponse::Array(items)) = response else {
    panic!("expected fref completions");
  };
  // Detail contains the path, label is the display name
  let details: Vec<String> = items
    .iter()
    .filter_map(|item| item.detail.clone())
    .collect();
  assert!(
    details.iter().any(|detail| detail.contains("alice")),
    "should suggest alice.td (Person type), got: {:?}",
    details
  );
  assert!(
    !details.iter().any(|detail| detail.contains("birthday")),
    "should not suggest birthday.td (Event type), got: {:?}",
    details
  );
}

// fref completions should use vault-root-relative paths (/alice.td), not project-relative paths (vault/alice.td)
#[test]
fn fref_completion_uses_vault_relative_paths() {
  let project_root = PathBuf::from(if cfg!(windows) {
    "C:\\project"
  } else {
    "/project"
  });
  let vault_root = project_root.join("vault");
  let typ_root = vault_root.join("_types");

  let (content, offset) = cursor(
    r#"---
_type: Directory
featured: fref("|")
---
"#,
  );

  let test_path = vault_root.join("file.td");
  let uri = path_to_uri(&test_path, "file");

  let nested_config = r#"version: "1"
vault:
  root_dir: "vault"
"#;

  let db = TypedownDatabase {
    storage: QueryStorage::default(),
  };

  let files = HashMap::from([
    (
      project_root.join("typedown.yaml"),
      File::new(
        &db,
        FileHandle::Content(
          project_root.join("typedown.yaml"),
          nested_config.to_string(),
          FileMetadata::default(),
        ),
      ),
    ),
    (
      typ_root.join("Person.td"),
      File::new(
        &db,
        FileHandle::Content(
          typ_root.join("Person.td"),
          SCHEMA_PERSON.to_string(),
          FileMetadata::default(),
        ),
      ),
    ),
    (
      typ_root.join("Directory.td"),
      File::new(
        &db,
        FileHandle::Content(
          typ_root.join("Directory.td"),
          SCHEMA_DIRECTORY.to_string(),
          FileMetadata::default(),
        ),
      ),
    ),
    (
      vault_root.join("alice.td"),
      File::new(
        &db,
        FileHandle::Content(
          vault_root.join("alice.td"),
          CONTENT_ALICE.to_string(),
          FileMetadata::default(),
        ),
      ),
    ),
    (
      test_path.clone(),
      File::new(
        &db,
        FileHandle::Content(test_path, content.clone(), FileMetadata::default()),
      ),
    ),
  ]);

  let project = Project::new(&db, project_root, files, None);
  let analysis = Analysis::new(
    db,
    project,
    Arc::new(HashMap::new()),
    Arc::new(HashMap::new()),
    Arc::new((Mutex::new(1), Condvar::new())),
  );

  let params = make_params(uri, &content, offset);
  let response = completion(&analysis, params);
  let Some(CompletionResponse::Array(items)) = response else {
    panic!("expected fref completions");
  };
  // Detail contains the path, should be vault-relative
  let details: Vec<String> = items
    .iter()
    .filter_map(|item| item.detail.clone())
    .collect();

  // Should suggest "/alice.td", not "vault/alice.td" or bare "alice.td"
  assert!(
    details.iter().any(|detail| detail == "/alice.td"),
    "should suggest vault-root-relative path '/alice.td', got: {:?}",
    details
  );
  assert!(
    !details.iter().any(|detail| detail.contains("vault/")),
    "should not include vault dir prefix in path, got: {:?}",
    details
  );
}

// fref completions include _label as detail and basename in filterText
#[test]
fn fref_completion_includes_label_and_filter_text() {
  let (content, offset) = cursor(
    r#"---
_type: Directory
featured: fref("|")
---
"#,
  );
  let (analysis, uri) = setup_with_content(&content);
  let params = make_params(uri, &content, offset);

  let response = completion(&analysis, params);
  let Some(CompletionResponse::Array(items)) = response else {
    panic!("expected fref completions");
  };

  let alice_item = items
    .iter()
    .find(|item| item.label == "Alice Chen")
    .expect("should have alice completion with display name as label");

  // detail should contain the vault-root-relative path with / prefix
  assert_eq!(
    alice_item.detail.as_deref(),
    Some("/alice.td"),
    "detail should be the vault-root-relative path with / prefix"
  );

  // filterText should include path, basename, and label for fuzzy matching
  let filter = alice_item.filter_text.as_deref().unwrap_or("");
  assert!(
    filter.contains("alice") && filter.contains("Alice Chen"),
    "filterText should contain basename and label: {filter}"
  );
}

// fref(|) without quotes wraps the path in quotes: insert "path" not bare path
#[test]
fn fref_without_quotes_wraps_path_in_quotes() {
  let (content, offset) = cursor(
    r#"---
_type: Directory
featured: fref(|)
---
"#,
  );
  let (analysis, uri) = setup_with_content(&content);
  let params = make_params(uri, &content, offset);

  let response = completion(&analysis, params);
  let Some(CompletionResponse::Array(items)) = response else {
    panic!("expected fref completions inside fref() without quotes");
  };

  let alice_item = items
    .iter()
    .find(|item| {
      item
        .detail
        .as_deref()
        .is_some_and(|detail| detail.contains("alice"))
    })
    .expect("should suggest alice.td");

  let insert = alice_item
    .insert_text
    .as_deref()
    .expect("should have insert_text");
  assert!(
    insert.starts_with('"') && insert.ends_with('"'),
    "insert_text should be quoted path, got: {insert}"
  );
  assert!(
    insert.contains("alice"),
    "insert_text should contain the path, got: {insert}"
  );
}

// Empty value on a schema-typed field suggests fref snippet
#[test]
fn schema_typd_empty_value_suggests_fref() {
  let (content, offset) = cursor(
    r#"---
_type: Directory
featured: |
---
"#,
  );
  let (analysis, uri) = setup_with_content(&content);
  let params = make_params(uri, &content, offset);

  let response = completion(&analysis, params);
  let Some(CompletionResponse::Array(items)) = response else {
    panic!("expected completions for empty schema-typed field");
  };

  // Should have specific file suggestions with fref("path") insert text
  let has_alice = items.iter().any(|item| {
    item
      .insert_text
      .as_deref()
      .is_some_and(|text| text.contains("alice"))
  });
  assert!(
    has_alice,
    "should suggest alice.td: {:?}",
    items.iter().map(|item| &item.label).collect::<Vec<_>>()
  );
}

// Typing in a schema-typed field value auto-suggests fref completions
#[test]
fn schema_typd_field_suggests_fref() {
  let (content, offset) = cursor(
    r#"---
_type: Directory
featured: a|
---
"#,
  );
  let (analysis, uri) = setup_with_content(&content);
  let params = make_params(uri, &content, offset);

  let response = completion(&analysis, params);
  let Some(CompletionResponse::Array(items)) = response else {
    panic!("expected completions for schema-typed field");
  };

  // Should suggest alice with fref() wrapper
  let alice_item = items
    .iter()
    .find(|item| {
      item
        .insert_text
        .as_deref()
        .is_some_and(|text| text.contains("alice"))
    })
    .expect("should suggest alice for Person-typed field");

  assert!(
    alice_item
      .insert_text
      .as_deref()
      .is_some_and(|text| text.starts_with(r#"fref(""#) && text.ends_with(r#"")"#)),
    "insert_text should wrap path in fref(): {:?}",
    alice_item.insert_text
  );
}

// Nullable schema field (Person?) also auto-suggests fref on empty value
#[test]
fn nullable_schema_field_suggests_fref() {
  let (content, offset) = cursor(
    r#"---
_type: TaskRef
title: "Test"
assignee: |
---
"#,
  );
  let (analysis, uri) = setup_with_content(&content);
  let params = make_params(uri, &content, offset);

  let response = completion(&analysis, params);
  let Some(CompletionResponse::Array(items)) = response else {
    panic!("expected completions for nullable schema field");
  };

  let has_fref = items.iter().any(|item| {
    item
      .insert_text
      .as_deref()
      .is_some_and(|text| text.contains("fref("))
  });
  assert!(
    has_fref,
    "nullable Person? field should suggest fref completions: {:?}",
    items.iter().map(|item| &item.label).collect::<Vec<_>>()
  );
}

// Non-schema field (string) should NOT auto-suggest fref
#[test]
fn string_field_does_not_suggest_fref() {
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

  // Wrapped fref completions like fref("alice.td") should not appear for string fields
  // The bare fref function is fine since it's always in scope
  let has_wrapped_fref = items.iter().any(|item| {
    item.kind == Some(CompletionItemKind::REFERENCE)
      && item
        .insert_text
        .as_deref()
        .is_some_and(|text| text.contains("fref("))
  });
  assert!(
    !has_wrapped_fref,
    "string field should not suggest fref file completions: {:?}",
    items.iter().map(|item| &item.label).collect::<Vec<_>>()
  );
}

// fref scope variable should appear in any unquoted expression position (including string-typed fields, e.g. fref("...").name)
#[test]
fn fref_snippet_suggested_for_string_field() {
  let (content, offset) = cursor(
    r#"---
_type: Person
name: f|
---
"#,
  );
  let (analysis, uri) = setup(&content);
  let params = make_params(uri, &content, offset);

  let response = completion(&analysis, params);
  let Some(CompletionResponse::Array(items)) = response else {
    panic!("expected value completions");
  };
  let fref_item = items.iter().find(|item| item.label == "fref");
  assert!(
    fref_item.is_some(),
    "fref should be suggested in any unquoted value position, got: {:?}",
    items.iter().map(|item| &item.label).collect::<Vec<_>>()
  );
}

// fref scope variable should still appear for schema-typed fields (unquoted value)
#[test]
fn fref_snippet_suggested_for_schema_field() {
  let (content, offset) = cursor(
    r#"---
_type: Task
assignee: f|
---
"#,
  );
  let (analysis, uri) = setup(&content);
  let params = make_params(uri, &content, offset);

  let response = completion(&analysis, params);
  let Some(CompletionResponse::Array(items)) = response else {
    panic!("expected value completions");
  };
  let has_fref = items.iter().any(|item| item.label == "fref");
  assert!(
    has_fref,
    "fref should be suggested for schema-typed (Person?) field, got: {:?}",
    items.iter().map(|item| &item.label).collect::<Vec<_>>()
  );
}

// fref should not appear when typing a string literal on a schema-typed field
#[test]
fn fref_snippet_not_suggested_inside_string_on_schema_field() {
  let (content, offset) = cursor(
    r#"---
_type: TaskRef
assignee: "f|"
---
"#,
  );
  let (analysis, uri) = setup_with_content(&content);
  let params = make_params(uri, &content, offset);

  let response = completion(&analysis, params);
  let Some(CompletionResponse::Array(items)) = response else {
    panic!("expected value completions");
  };
  let has_fref = items.iter().any(|item| {
    item.label == "fref"
      || item.kind == Some(CompletionItemKind::REFERENCE)
      || item
        .insert_text
        .as_deref()
        .is_some_and(|text| text.contains("fref("))
  });
  assert!(
    !has_fref,
    "fref should not be suggested inside a string literal on a schema-typed field, got: {:?}",
    items
      .iter()
      .map(|item| (&item.label, &item.insert_text))
      .collect::<Vec<_>>()
  );
}

// fref should not appear inside a function call string argument f("|") on a schema-typed or non-schema-typed field
#[test]
fn fref_not_suggested_inside_call_string_arg() {
  for (schema_name, field_line) in [
    ("Directory", r#"featured: f("|")"#),
    ("Person", r#"name: f("|")"#),
  ] {
    let input = format!(
      r#"---
_type: {schema_name}
{field_line}
---
"#
    );
    let (content, offset) = cursor(&input);
    let (analysis, uri) = setup_with_content(&content);
    let params = make_params(uri, &content, offset);

    if let Some(CompletionResponse::Array(items)) = completion(&analysis, params) {
      let has_fref = items.iter().any(|item| {
        item.label == "fref"
          || item.kind == Some(CompletionItemKind::REFERENCE)
          || item
            .insert_text
            .as_deref()
            .is_some_and(|text| text.contains("fref("))
      });
      assert!(
        !has_fref,
        "fref should not be suggested inside f(\"|\") ({field_line}), got: {:?}",
        items
          .iter()
          .map(|item| (&item.label, &item.insert_text))
          .collect::<Vec<_>>()
      );
    }
  }
}

// f(fref("|")) and f(fref(|)) should suggest fref completions even when the top-level field is not schema-typed
#[test]
fn fref_inside_call_arg_suggests_file_paths() {
  for raw_input in [
    r#"---
_type: TaskRef
title: ((person) -> "hello")(fref("|"))
---
"#,
    r#"---
_type: TaskRef
title: ((person) -> "hello")(fref(|))
---
"#,
  ] {
    let (content, offset) = cursor(raw_input);
    let (analysis, uri) = setup_with_content(&content);
    let params = make_params(uri, &content, offset);

    let response = completion(&analysis, params);
    let Some(CompletionResponse::Array(items)) = response else {
      panic!("expected fref completions inside {raw_input}");
    };
    let details: Vec<String> = items
      .iter()
      .filter_map(|item| item.detail.clone())
      .collect();
    assert!(
      details.iter().any(|detail| detail.contains("alice")),
      "should suggest alice.td inside {raw_input}, got: {:?}",
      details
    );
  }
}

// Resource files should never appear as bare completions (no insert_text wrapping)
// They should only be suggested via fref() when the field type is a schema
#[test]
fn resource_files_not_suggested_as_bare_completions_in_string_field() {
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
    panic!("expected completions");
  };

  // No REFERENCE items (resource file completions) should appear
  let has_resource = items
    .iter()
    .any(|item| item.kind == Some(CompletionItemKind::REFERENCE));
  assert!(
    !has_resource,
    "resource files should not appear as bare completions: {:?}",
    items.iter().map(|item| &item.label).collect::<Vec<_>>()
  );
}

// Resource files should not appear in value completions outside of fref context
#[test]
fn resource_files_not_suggested_outside_fref_context() {
  let (content, offset) = cursor(
    r#"---
_type: Person
name: |
---
"#,
  );
  let (analysis, uri) = setup_with_content(&content);
  let params = make_params(uri, &content, offset);

  let response = completion(&analysis, params);
  if let Some(CompletionResponse::Array(items)) = response {
    let resource_labels: Vec<&str> = items
      .iter()
      .filter(|item| item.kind == Some(CompletionItemKind::REFERENCE))
      .map(|item| item.label.as_str())
      .collect();
    assert!(
      resource_labels.is_empty(),
      "resource files should not appear in plain string field: {:?}",
      resource_labels
    );
  }
}

#[test]
fn resource_files_not_suggested_in_number_field() {
  let (content, offset) = cursor(
    r#"---
_type: Person
age: |
---
"#,
  );
  let (analysis, uri) = setup_with_content(&content);
  let params = make_params(uri, &content, offset);

  let response = completion(&analysis, params);
  if let Some(CompletionResponse::Array(items)) = response {
    let has_resource = items
      .iter()
      .any(|item| item.kind == Some(CompletionItemKind::REFERENCE));
    assert!(
      !has_resource,
      "resource files should not appear in number field: {:?}",
      items.iter().map(|item| &item.label).collect::<Vec<_>>()
    );
  }
}

#[test]
fn resource_files_not_suggested_in_object_typ_field() {
  let (content, offset) = cursor(
    r#"---
_type: PersonWithAddress
address: |
---
"#,
  );
  let (analysis, uri) = setup(&content);
  let params = make_params(uri, &content, offset);

  let response = completion(&analysis, params);
  if let Some(CompletionResponse::Array(items)) = response {
    let has_resource = items
      .iter()
      .any(|item| item.kind == Some(CompletionItemKind::REFERENCE));
    assert!(
      !has_resource,
      "resource files should not appear in object-typed field: {:?}",
      items.iter().map(|item| &item.label).collect::<Vec<_>>()
    );
  }
}

#[test]
fn resource_files_not_suggested_inside_quoted_string() {
  let (content, offset) = cursor(
    r#"---
_type: Person
name: "ali|"
---
"#,
  );
  let (analysis, uri) = setup_with_content(&content);
  let params = make_params(uri, &content, offset);

  let response = completion(&analysis, params);
  if let Some(CompletionResponse::Array(items)) = response {
    let has_resource = items
      .iter()
      .any(|item| item.kind == Some(CompletionItemKind::REFERENCE));
    assert!(
      !has_resource,
      "resource files should not appear inside a quoted string value: {:?}",
      items.iter().map(|item| &item.label).collect::<Vec<_>>()
    );
  }
}

#[test]
fn resource_files_not_suggested_in_list_of_string_field() {
  let (content, offset) = cursor(
    r#"---
_type: Collection
tags:
  - t|
---
"#,
  );
  let (analysis, uri) = setup_with_content(&content);
  let params = make_params(uri, &content, offset);
  if let Some(CompletionResponse::Array(items)) = completion(&analysis, params) {
    assert_no_reference_items(&items, "list[string] item");
  }
}

#[test]
fn resource_files_not_suggested_in_list_of_number_field() {
  let (content, offset) = cursor(
    r#"---
_type: Collection
scores:
  - |
---
"#,
  );
  let (analysis, uri) = setup_with_content(&content);
  let params = make_params(uri, &content, offset);
  if let Some(CompletionResponse::Array(items)) = completion(&analysis, params) {
    assert_no_reference_items(&items, "list[number] item");
  }
}

#[test]
fn resource_files_not_suggested_in_dict_value_field() {
  let (content, offset) = cursor(
    r#"---
_type: Collection
labels:
  key: v|
---
"#,
  );
  let (analysis, uri) = setup_with_content(&content);
  let params = make_params(uri, &content, offset);
  if let Some(CompletionResponse::Array(items)) = completion(&analysis, params) {
    assert_no_reference_items(&items, "dict[string,string] value");
  }
}

// list[Person] items in both flow `[|]` and block `- |` sequences should suggest fref-wrapped completions (and not bare resource names)
#[test]
fn list_of_schema_field_suggests_fref_not_bare_resource() {
  for raw_input in [
    r#"---
_type: Collection
authors: [|]
---
"#,
    r#"---
_type: Collection
authors: [fref("/alice.td"), |]
---
"#,
    r#"---
_type: Collection
authors:
  - |
---
"#,
    r#"---
_type: Collection
authors:
  - |
  - fref("/alice.td")
---
"#,
  ] {
    let (content, offset) = cursor(raw_input);
    let (analysis, uri) = setup_with_content(&content);
    let params = make_params(uri, &content, offset);
    let Some(CompletionResponse::Array(items)) = completion(&analysis, params) else {
      panic!("expected completions for list[Person] item in:\n{raw_input}");
    };

    // Should suggest fref("/alice.td") (Person) and not birthday.td (Event)
    let has_alice_fref = items.iter().any(|item| {
      item.kind == Some(CompletionItemKind::REFERENCE)
        && item
          .insert_text
          .as_deref()
          .is_some_and(|text| text == r#"fref("/alice.td")"#)
    });
    assert!(
      has_alice_fref,
      "list[Person] item should suggest fref(\"/alice.td\") in:\n{raw_input}\ngot: {:?}",
      items
        .iter()
        .map(|item| (&item.label, &item.insert_text))
        .collect::<Vec<_>>()
    );
    let has_birthday = items.iter().any(|item| {
      item.kind == Some(CompletionItemKind::REFERENCE)
        && item
          .insert_text
          .as_deref()
          .is_some_and(|text| text.contains("birthday"))
    });
    assert!(
      !has_birthday,
      "list[Person] item should not suggest Event file birthday.td in:\n{raw_input}"
    );

    // No bare REFERENCE items (insert_text without fref wrapper)
    let has_bare_resource = items.iter().any(|item| {
      item.kind == Some(CompletionItemKind::REFERENCE)
        && item
          .insert_text
          .as_deref()
          .is_none_or(|text| !text.starts_with("fref("))
    });
    assert!(
      !has_bare_resource,
      "list[Person] items should not appear as bare resource names in:\n{raw_input}\ngot: {:?}",
      items
        .iter()
        .map(|item| (&item.label, &item.insert_text))
        .collect::<Vec<_>>()
    );
  }
}

// dict[string, Person] values in both flow `{field: |}` and nested block `:\n  field: |` should suggest fref-wrapped completions
#[test]
fn dict_of_schema_field_suggests_fref_not_bare_resource() {
  for raw_input in [
    r#"---
_type: Collection
members_by_role: {lead: |}
---
"#,
    r#"---
_type: Collection
members_by_role: {lead: a|}
---
"#,
    r#"---
_type: Collection
members_by_role: {lead: fref("/alice.td"), backup: |}
---
"#,
    r#"---
_type: Collection
members_by_role:
  lead: |
---
"#,
    r#"---
_type: Collection
members_by_role:
  lead: a|
---
"#,
    r#"---
_type: Collection
members_by_role:
  lead: |
  backup: fref("/alice.td")
---
"#,
  ] {
    let (content, offset) = cursor(raw_input);
    let (analysis, uri) = setup_with_content(&content);
    let params = make_params(uri, &content, offset);
    let Some(CompletionResponse::Array(items)) = completion(&analysis, params) else {
      panic!("expected completions for dict[string, Person] value in:\n{raw_input}");
    };

    let has_alice_fref = items.iter().any(|item| {
      item.kind == Some(CompletionItemKind::REFERENCE)
        && item
          .insert_text
          .as_deref()
          .is_some_and(|text| text == r#"fref("/alice.td")"#)
    });
    assert!(
      has_alice_fref,
      "dict[string, Person] value should suggest fref(\"/alice.td\") in:\n{raw_input}\ngot: {:?}",
      items
        .iter()
        .map(|item| (&item.label, &item.insert_text))
        .collect::<Vec<_>>()
    );
    let has_birthday = items.iter().any(|item| {
      item.kind == Some(CompletionItemKind::REFERENCE)
        && item
          .insert_text
          .as_deref()
          .is_some_and(|text| text.contains("birthday"))
    });
    assert!(
      !has_birthday,
      "dict[string, Person] value should not suggest Event file birthday.td in:\n{raw_input}"
    );
  }
}

// When cursor is inside fref() with unquoted partial path (e.g. fref(some|)),
// completion should use text_edit to replace the existing unquoted content with a quoted path
#[test]
fn fref_unquoted_arg_completion_uses_text_edit_to_replace_and_quote() {
  let (content, offset) = cursor(
    r#"---
_type: Directory
featured: fref(some|)
---
"#,
  );
  let (analysis, uri) = setup_with_content(&content);
  let params = make_params(uri, &content, offset);

  let response = completion(&analysis, params);
  let Some(CompletionResponse::Array(items)) = response else {
    panic!("expected fref completions");
  };

  let alice_item = items
    .iter()
    .find(|item| {
      item
        .detail
        .as_deref()
        .is_some_and(|detail| detail.contains("alice"))
    })
    .expect("should suggest alice.td");

  // Should have a text_edit that replaces existing unquoted content with quoted path
  let text_edit = alice_item
    .text_edit
    .as_ref()
    .expect("should have text_edit to replace unquoted arg");

  let new_text = match text_edit {
    lsp_types::CompletionTextEdit::Edit(edit) => &edit.new_text,
    lsp_types::CompletionTextEdit::InsertAndReplace(entry) => &entry.new_text,
  };
  assert!(
    new_text.starts_with('"') && new_text.ends_with('"'),
    "text_edit new_text should be a quoted path, got: {new_text}"
  );
  assert!(
    new_text.contains("alice"),
    "text_edit new_text should contain 'alice', got: {new_text}"
  );
}
