use super::fixtures::*;
use lsp_types::CompletionResponse;

#[test]
fn prop_completion_includes_description_as_documentation() {
  let contract_json = r#"{
      "TdAlert": {
        "source": "./TdAlert.vue",
        "props": {
          "kind": {
            "schema": { "type": ["string", "null"] },
            "description": "The alert variant to display"
          }
        }
      }
    }"#;
  let (content, offset) = cursor(
    r#"---
---
:::td-alert{|}
:::
"#,
  );
  let (analysis, uri) = setup_with_contract(&content, contract_json);
  let params = make_params(uri, &content, offset);
  let Some(CompletionResponse::Array(items)) = completion(&analysis, params) else {
    panic!("expected completion items");
  };
  let kind_item = items
    .iter()
    .find(|item| item.label == "kind")
    .expect("should suggest 'kind' prop");
  let doc = kind_item
    .documentation
    .as_ref()
    .expect("'kind' should have documentation");
  let doc_text = match doc {
    lsp_types::Documentation::MarkupContent(markup) => markup.value.as_str(),
    _ => panic!("expected markup content"),
  };
  assert!(
    doc_text.contains("The alert variant to display"),
    "documentation should contain the prop description: {doc_text}"
  );
}

#[test]
fn prop_completion_without_description_has_no_documentation() {
  let contract_json = r#"{
      "TdAlert": {
        "source": "./TdAlert.vue",
        "props": {
          "kind": {
            "schema": { "type": ["string", "null"] }
          }
        }
      }
    }"#;
  let (content, offset) = cursor(
    r#"---
---
:::td-alert{|}
:::
"#,
  );
  let (analysis, uri) = setup_with_contract(&content, contract_json);
  let params = make_params(uri, &content, offset);
  let Some(CompletionResponse::Array(items)) = completion(&analysis, params) else {
    panic!("expected completion items");
  };
  let kind_item = items
    .iter()
    .find(|item| item.label == "kind")
    .expect("should suggest 'kind' prop");
  assert!(
    kind_item.documentation.is_none(),
    "prop without description should have no documentation, got: {:?}",
    kind_item.documentation
  );
}

#[test]
fn container_name_suggests_builtin_and_custom_components() {
  let contract_json = r#"{
      "VideoEmbed": {
        "source": "./VideoEmbed.vue",
        "props": {}
      },
      "Chart": {
        "source": "./Chart.vue",
        "props": {}
      }
    }"#;
  let (content, offset) = cursor(
    r#"---
---
:::|
"#,
  );
  let (analysis, uri) = setup_with_contract(&content, contract_json);
  let params = make_params(uri, &content, offset);
  let Some(CompletionResponse::Array(items)) = completion(&analysis, params) else {
    panic!("expected completion items after :::");
  };
  let labels: Vec<&str> = items.iter().map(|item| item.label.as_str()).collect();
  assert!(
    labels.contains(&"tip"),
    "should suggest builtin callout 'tip': {labels:?}"
  );
  assert!(
    labels.contains(&"video-embed"),
    "should suggest custom component 'video-embed': {labels:?}"
  );
}

#[test]
fn shorthand_name_suggests_custom_components() {
  let contract_json = r#"{
      "VideoEmbed": {
        "source": "./VideoEmbed.vue",
        "props": {}
      }
    }"#;
  let (content, offset) = cursor(
    r#"---
---
[[|]]
"#,
  );
  let (analysis, uri) = setup_with_contract(&content, contract_json);
  let params = make_params(uri, &content, offset);
  let Some(CompletionResponse::Array(items)) = completion(&analysis, params) else {
    panic!("expected completion items inside [[]]");
  };
  let labels: Vec<&str> = items.iter().map(|item| item.label.as_str()).collect();
  assert!(
    labels.contains(&"video-embed"),
    "should suggest custom component 'video-embed': {labels:?}"
  );
}
