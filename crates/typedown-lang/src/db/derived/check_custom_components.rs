// Tracked query: validate custom component usage in a file against the component contract

use typedown_macros::query_derived;

use crate::db::TypedownDatabase;
use crate::db::derived::get_component_contract::get_component_contract;
use crate::db::derived::parse_file::parse_file;
use crate::db::types::{CustomComponentCheckResult, CustomComponentTyp, File, Project};
use crate::db::utils::{is_callout, kebab_to_pascal};
use crate::syntax::ast::{AstNode, MdContainerBlock, MdContainerShorthand};
use crate::syntax::diagnostic::*;
use crate::syntax::red::RedNode;
use typedown_incremental::QueryDatabase;

#[query_derived]
pub fn check_custom_components<'db>(
  db: &'db TypedownDatabase,
  project: Project,
  file: File,
) -> CustomComponentCheckResult<'db> {
  let Some(contract_file) = project.custom_component_contract(db) else {
    // No .typedown/components.json: skip component validation
    return CustomComponentCheckResult::new(db, vec![]);
  };

  let contract = get_component_contract(db, contract_file)
    .entries(db)
    .clone();
  let root = parse_file(db, project, file).ast(db).node.clone();
  let mut diagnostics = Vec::new();
  collect(&root, &contract, &mut diagnostics);
  CustomComponentCheckResult::new(db, diagnostics)
}

fn collect(node: &RedNode, contract: &[CustomComponentTyp], diagnostics: &mut Vec<TdDiagnostic>) {
  if let Some(block) = MdContainerBlock::cast(node.clone()) {
    if let Some(name) = block.label() {
      validate(node, &name, contract, diagnostics);
    }
  } else if let Some(shorthand) = MdContainerShorthand::cast(node.clone())
    && let Some(name) = shorthand.label()
  {
    validate(node, &name, contract, diagnostics);
  }

  for child in node.children() {
    collect(&child, contract, diagnostics);
  }
}

fn validate(
  node: &RedNode,
  name: &str,
  contract: &[CustomComponentTyp],
  diagnostics: &mut Vec<TdDiagnostic>,
) {
  let (start_offset, len) = node.trimmed_range();
  let end_offset = start_offset + len;

  let pascal_name = kebab_to_pascal(name);
  let Some(entry) = contract
    .iter()
    .find(|component_entry| component_entry.name == name || component_entry.name == pascal_name)
  else {
    // Callouts (e.g. "note", "warning") are valid without a contract entry.
    // If a callout name is registered in the contract, it falls through and is validated normally.
    if is_callout(name) {
      return;
    }
    diagnostics.push(
      TdDiagnosticKind::UnknownComponent {
        name: name.to_string(),
      }
      .into_warning(start_offset, end_offset),
    );
    return;
  };

  let Some(props) = &entry.props else {
    // No props declared in contract: skip prop validation
    return;
  };

  let provided_props = MdContainerBlock::cast(node.clone())
    .map(|b| b.prop_names())
    .or_else(|| MdContainerShorthand::cast(node.clone()).map(|s| s.prop_names()))
    .unwrap_or_default();

  for prop in props {
    if prop.required && !provided_props.iter().any(|p| p == &prop.name) {
      diagnostics.push(
        TdDiagnosticKind::MissingRequiredProp {
          prop: prop.name.clone(),
          component: name.to_string(),
        }
        .into_warning(start_offset, end_offset),
      );
    }
  }

  for prop_name in &provided_props {
    if !props.iter().any(|p| &p.name == prop_name) {
      diagnostics.push(
        TdDiagnosticKind::UnknownProp {
          prop: prop_name.clone(),
          component: name.to_string(),
        }
        .into_warning(start_offset, end_offset),
      );
    }
  }

  // Deprecation warning
  if let Some(message) = &entry.deprecated {
    diagnostics.push(
      TdDiagnosticKind::DeprecatedComponent {
        component: name.to_string(),
        message: message.clone(),
      }
      .into_warning(start_offset, end_offset),
    );
  }

  // Slot validation (only container blocks have slots, not shorthands)
  if !entry.slots.is_empty() {
    let used_slots = MdContainerBlock::cast(node.clone())
      .map(|b| b.slot_names())
      .unwrap_or_default();
    for slot in &used_slots {
      if !entry.slots.iter().any(|s| s == slot) {
        diagnostics.push(
          TdDiagnosticKind::UnknownSlot {
            slot: slot.clone(),
            component: name.to_string(),
          }
          .into_warning(start_offset, end_offset),
        );
      }
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::db::types::CustomComponentProp;
  use crate::syntax::ast::{AstNode, MdContainerBlock, MdContainerShorthand};
  use crate::syntax::red::RedNode;
  use crate::syntax::syntax_kind::SyntaxKind;

  fn parse_body(input: &str) -> RedNode {
    use crate::syntax::green::cache::Cache;
    use crate::syntax::parse::ctx::{ParseCtx, ParseResult};
    use std::cell::RefCell;
    use std::rc::Rc;
    use typedown_types::string_stream::StringStream;

    let full = format!("---\n---\n{}", input);
    let stream = StringStream::new(&full);
    let cache = Rc::new(RefCell::new(Cache::new()));
    let ctx = ParseCtx::new(stream, cache);
    let ParseResult { ast, .. } = ctx.parse();
    RedNode::from_green(0, ast)
  }

  fn find_first(node: &RedNode, kind: SyntaxKind) -> Option<RedNode> {
    if node.kind() == kind {
      return Some(node.clone());
    }
    for child in node.children() {
      if let Some(found) = find_first(&child, kind) {
        return Some(found);
      }
    }
    None
  }

  fn find_block(root: &RedNode) -> MdContainerBlock {
    let node = find_first(root, SyntaxKind::MdContainerBlock).unwrap();
    MdContainerBlock::cast(node).unwrap()
  }

  fn find_shorthand(root: &RedNode) -> MdContainerShorthand {
    let node = find_first(root, SyntaxKind::MdContainerShorthand).unwrap();
    MdContainerShorthand::cast(node).unwrap()
  }

  // label

  #[test]
  fn container_name_simple() {
    let root = parse_body(
      r#"
::: note
:::
"#,
    );
    assert_eq!(find_block(&root).label(), Some("note".to_string()));
  }

  #[test]
  fn container_name_kebab() {
    let root = parse_body(
      r#"
::: td-alert
:::
"#,
    );
    assert_eq!(find_block(&root).label(), Some("td-alert".to_string()));
  }

  #[test]
  fn container_name_no_space() {
    let root = parse_body(
      r#"
:::td-alert
:::
"#,
    );
    assert_eq!(find_block(&root).label(), Some("td-alert".to_string()));
  }

  #[test]
  fn shorthand_name_kebab() {
    let root = parse_body(
      r#"
[[directory-index]]
"#,
    );
    assert_eq!(
      find_shorthand(&root).label(),
      Some("directory-index".to_string())
    );
  }

  // prop_names

  #[test]
  fn prop_names_extracted() {
    let root = parse_body(
      r#"
:::chart{kind="info" size="large"}
:::
"#,
    );
    assert_eq!(find_block(&root).prop_names(), vec!["kind", "size"]);
  }

  #[test]
  fn prop_value_bare_ident() {
    let root = parse_body(
      r#"
:::flashcard{revealed=true disabled=false}
:::
"#,
    );
    assert_eq!(find_block(&root).prop_names(), vec!["revealed", "disabled"]);
  }

  #[test]
  fn prop_value_array() {
    let root = parse_body(
      r#"
:::chart{data=[1, 2, 3]}
:::
"#,
    );
    assert_eq!(find_block(&root).prop_names(), vec!["data"]);
  }

  #[test]
  fn prop_value_dict() {
    let root = parse_body(
      r#"
:::chart{config={color: "red", size: 10}}
:::
"#,
    );
    assert_eq!(find_block(&root).prop_names(), vec!["config"]);
  }

  #[test]
  fn prop_value_nested_array() {
    let root = parse_body(
      r#"
:::chart{matrix=[[1, 2], [3, 4]]}
:::
"#,
    );
    assert_eq!(find_block(&root).prop_names(), vec!["matrix"]);
  }

  #[test]
  fn prop_value_negative_number() {
    let root = parse_body(
      r#"
:::widget{offset=-1}
:::
"#,
    );
    assert_eq!(find_block(&root).prop_names(), vec!["offset"]);
  }

  #[test]
  fn prop_value_mixed() {
    let root = parse_body(
      r#"
:::comp{a=true b="hello" c=[1, 2] d=42}
:::
"#,
    );
    assert_eq!(find_block(&root).prop_names(), vec!["a", "b", "c", "d"]);
  }

  #[test]
  fn prop_names_empty_without_block() {
    let root = parse_body(
      r#"
:::note
:::
"#,
    );
    assert!(find_block(&root).prop_names().is_empty());
  }

  // validate unknown component

  #[test]
  fn unknown_component_emits_diagnostic() {
    let root = parse_body(
      r#"
:::my-widget
:::
"#,
    );
    let block = find_block(&root);
    let name = block.label().unwrap();
    let contract: Vec<CustomComponentTyp> = vec![];
    let mut diags = Vec::new();
    validate(block.syntax(), &name, &contract, &mut diags);
    assert!(
      diags.iter().any(|d| matches!(
        d,
        TdDiagnostic {
          kind: TdDiagnosticKind::UnknownComponent { .. },
          ..
        }
      )),
      "expected UnknownComponent, got: {diags:?}"
    );
  }

  #[test]
  fn known_component_by_kebab_matches_pascal_contract() {
    let root = parse_body(
      r#"
:::video-embed
:::
"#,
    );
    let block = find_block(&root);
    let name = block.label().unwrap();
    let contract = vec![CustomComponentTyp {
      name: "VideoEmbed".to_string(),
      source: "./VideoEmbed.vue".to_string(),
      props: Some(vec![]),
      slots: vec![],
      deprecated: None,
    }];
    let mut diags = Vec::new();
    validate(block.syntax(), &name, &contract, &mut diags);
    assert!(diags.is_empty(), "expected no diagnostics, got: {diags:?}");
  }

  #[test]
  fn missing_required_prop_emits_diagnostic() {
    let root = parse_body(
      r#"
:::chart
:::
"#,
    );
    let block = find_block(&root);
    let name = block.label().unwrap();
    let contract = vec![CustomComponentTyp {
      name: "Chart".to_string(),
      source: "./Chart.vue".to_string(),
      props: Some(vec![CustomComponentProp {
        name: "data".to_string(),
        typ_name: "string".to_string(),
        required: true,
        description: None,
      }]),
      slots: vec![],
      deprecated: None,
    }];
    let mut diags = Vec::new();
    validate(block.syntax(), &name, &contract, &mut diags);
    assert!(
      diags
        .iter()
        .any(|d| matches!(d, TdDiagnostic { kind: TdDiagnosticKind::MissingRequiredProp { prop, .. }, .. } if prop == "data")),
      "expected MissingRequiredProp for 'data', got: {diags:?}"
    );
  }

  #[test]
  fn unknown_prop_emits_diagnostic() {
    let root = parse_body(
      r#"
:::chart{extra="val"}
:::
"#,
    );
    let block = find_block(&root);
    let name = block.label().unwrap();
    let contract = vec![CustomComponentTyp {
      name: "Chart".to_string(),
      source: "./Chart.vue".to_string(),
      props: Some(vec![]),
      slots: vec![],
      deprecated: None,
    }];
    let mut diags = Vec::new();
    validate(block.syntax(), &name, &contract, &mut diags);
    assert!(
      diags
        .iter()
        .any(|d| matches!(d, TdDiagnostic { kind: TdDiagnosticKind::UnknownProp { prop, .. }, .. } if prop == "extra")),
      "expected UnknownProp for 'extra', got: {diags:?}"
    );
  }

  #[test]
  fn callout_skipped() {
    let root = parse_body(
      r#"
::: note
content
:::
"#,
    );
    let block = find_block(&root);
    let name = block.label().unwrap();
    let contract: Vec<CustomComponentTyp> = vec![];
    let mut diags = Vec::new();
    validate(block.syntax(), &name, &contract, &mut diags);
    assert!(diags.is_empty(), "callouts should produce no diagnostics");
  }

  #[test]
  fn callout_with_contract_entry_is_validated() {
    // A callout name registered in the contract should be validated like any other component.
    let root = parse_body(
      r#"
::: note{extra="val"}
content
:::
"#,
    );
    let block = find_block(&root);
    let name = block.label().unwrap();
    let contract = vec![CustomComponentTyp {
      name: "note".to_string(),
      source: "./Note.vue".to_string(),
      props: Some(vec![]),
      slots: vec![],
      deprecated: None,
    }];
    let mut diags = Vec::new();
    validate(block.syntax(), &name, &contract, &mut diags);
    assert!(
      diags
        .iter()
        .any(|d| matches!(d, TdDiagnostic { kind: TdDiagnosticKind::UnknownProp { prop, .. }, .. } if prop == "extra")),
      "callout with contract entry should be validated; expected UnknownProp, got: {diags:?}"
    );
  }

  #[test]
  fn deprecated_component_emits_warning() {
    let root = parse_body(
      r#"
::: old-widget
:::
"#,
    );
    let block = find_block(&root);
    let name = block.label().unwrap();
    let contract = vec![CustomComponentTyp {
      name: "old-widget".to_string(),
      source: "./OldWidget.vue".to_string(),
      props: Some(vec![]),
      slots: vec![],
      deprecated: Some("use new-widget instead".to_string()),
    }];
    let mut diags = Vec::new();
    validate(block.syntax(), &name, &contract, &mut diags);
    assert!(
      diags.iter().any(|d| matches!(
        d,
        TdDiagnostic {
          kind: TdDiagnosticKind::DeprecatedComponent { .. },
          ..
        }
      )),
      "expected DeprecatedComponent, got: {diags:?}"
    );
  }

  #[test]
  fn unknown_slot_emits_warning() {
    let root = parse_body(
      r#"
::: flashcard
question
=== answer
the answer
:::
"#,
    );
    let block = find_block(&root);
    let name = block.label().unwrap();
    let contract = vec![CustomComponentTyp {
      name: "flashcard".to_string(),
      source: "./TdFlashcard.vue".to_string(),
      props: Some(vec![]),
      slots: vec!["key".to_string()],
      deprecated: None,
    }];
    let mut diags = Vec::new();
    validate(block.syntax(), &name, &contract, &mut diags);
    assert!(
      diags.iter().any(|d| matches!(
        d,
        TdDiagnostic { kind: TdDiagnosticKind::UnknownSlot { slot, .. }, .. } if slot == "answer"
      )),
      "expected UnknownSlot for 'answer', got: {diags:?}"
    );
  }

  #[test]
  fn valid_slot_no_diagnostic() {
    let root = parse_body(
      r#"
::: flashcard
question
=== key
the answer
:::
"#,
    );
    let block = find_block(&root);
    let name = block.label().unwrap();
    let contract = vec![CustomComponentTyp {
      name: "flashcard".to_string(),
      source: "./TdFlashcard.vue".to_string(),
      props: Some(vec![]),
      slots: vec!["key".to_string()],
      deprecated: None,
    }];
    let mut diags = Vec::new();
    validate(block.syntax(), &name, &contract, &mut diags);
    assert!(
      diags.is_empty(),
      "valid slot should produce no diagnostics, got: {diags:?}"
    );
  }
}
