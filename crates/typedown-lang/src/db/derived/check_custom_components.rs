// Tracked query: validate custom component usage in a file against the component contract

use typedown_macros::query_derived;

use crate::db::TypedownDatabase;
use crate::db::derived::get_component_contract::get_component_contract;
use crate::db::derived::parse_file::parse_file;
use crate::db::types::{CustomComponentCheckResult, CustomComponentTyp, File, Project};
use crate::db::utils::{
  extract_container_name, extract_container_prop_names, extract_shorthand_name,
  is_builtin_container, kebab_to_pascal,
};
use crate::syntax::diagnostic::*;
use crate::syntax::red::RedNode;
use crate::syntax::syntax_kind::SyntaxKind;
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
  match node.kind() {
    SyntaxKind::MdContainerBlock => {
      if let Some(name) = extract_container_name(node) {
        validate(node, &name, contract, diagnostics);
      }
    }
    SyntaxKind::MdContainerShorthand => {
      if let Some(name) = extract_shorthand_name(node) {
        validate(node, &name, contract, diagnostics);
      }
    }
    _ => {}
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
  if is_builtin_container(name) {
    return;
  }

  let (start_offset, len) = node.trimmed_range();
  let end_offset = start_offset + len;

  let pascal = kebab_to_pascal(name);
  let Some(entry) = contract.iter().find(|e| e.name == name || e.name == pascal) else {
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

  let provided = extract_container_prop_names(node);

  for prop in props {
    if prop.required && !provided.iter().any(|p| p == &prop.name) {
      diagnostics.push(
        TdDiagnosticKind::MissingRequiredProp {
          prop: prop.name.clone(),
          component: name.to_string(),
        }
        .into_warning(start_offset, end_offset),
      );
    }
  }

  for prop_name in &provided {
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
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::db::types::CustomComponentProp;
  use crate::db::utils::{
    extract_container_name, extract_container_prop_names, extract_shorthand_name,
  };
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

  // extract_container_name

  #[test]
  fn container_name_simple() {
    let root = parse_body("::: note\n:::\n");
    let block = find_first(&root, SyntaxKind::MdContainerBlock).unwrap();
    assert_eq!(extract_container_name(&block), Some("note".to_string()));
  }

  #[test]
  fn container_name_kebab() {
    let root = parse_body("::: td-alert\n:::\n");
    let block = find_first(&root, SyntaxKind::MdContainerBlock).unwrap();
    assert_eq!(extract_container_name(&block), Some("td-alert".to_string()));
  }

  #[test]
  fn container_name_no_space() {
    let root = parse_body(":::td-alert\n:::\n");
    let block = find_first(&root, SyntaxKind::MdContainerBlock).unwrap();
    assert_eq!(extract_container_name(&block), Some("td-alert".to_string()));
  }

  // extract_shorthand_name

  #[test]
  fn shorthand_name_kebab() {
    let root = parse_body("[[directory-index]]\n");
    let node = find_first(&root, SyntaxKind::MdContainerShorthand).unwrap();
    assert_eq!(
      extract_shorthand_name(&node),
      Some("directory-index".to_string())
    );
  }

  // extract_container_prop_names

  #[test]
  fn prop_names_extracted() {
    let root = parse_body(":::chart{kind=\"info\" size=\"large\"}\n:::\n");
    let block = find_first(&root, SyntaxKind::MdContainerBlock).unwrap();
    let props = extract_container_prop_names(&block);
    assert_eq!(props, vec!["kind", "size"]);
  }

  #[test]
  fn prop_names_empty_without_block() {
    let root = parse_body(":::note\n:::\n");
    let block = find_first(&root, SyntaxKind::MdContainerBlock).unwrap();
    assert!(extract_container_prop_names(&block).is_empty());
  }

  // validate unknown component

  #[test]
  fn unknown_component_emits_diagnostic() {
    let root = parse_body(":::my-widget\n:::\n");
    let block = find_first(&root, SyntaxKind::MdContainerBlock).unwrap();
    let name = extract_container_name(&block).unwrap();
    let contract: Vec<CustomComponentTyp> = vec![];
    let mut diags = Vec::new();
    validate(&block, &name, &contract, &mut diags);
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
    let root = parse_body(":::video-embed\n:::\n");
    let block = find_first(&root, SyntaxKind::MdContainerBlock).unwrap();
    let name = extract_container_name(&block).unwrap();
    let contract = vec![CustomComponentTyp {
      name: "VideoEmbed".to_string(),
      source: "./VideoEmbed.vue".to_string(),
      props: Some(vec![]),
    }];
    let mut diags = Vec::new();
    validate(&block, &name, &contract, &mut diags);
    assert!(diags.is_empty(), "expected no diagnostics, got: {diags:?}");
  }

  #[test]
  fn missing_required_prop_emits_diagnostic() {
    let root = parse_body(":::chart\n:::\n");
    let block = find_first(&root, SyntaxKind::MdContainerBlock).unwrap();
    let name = extract_container_name(&block).unwrap();
    let contract = vec![CustomComponentTyp {
      name: "Chart".to_string(),
      source: "./Chart.vue".to_string(),
      props: Some(vec![CustomComponentProp {
        name: "data".to_string(),
        typ_name: "string".to_string(),
        required: true,
      }]),
    }];
    let mut diags = Vec::new();
    validate(&block, &name, &contract, &mut diags);
    assert!(
      diags
        .iter()
        .any(|d| matches!(d, TdDiagnostic { kind: TdDiagnosticKind::MissingRequiredProp { prop, .. }, .. } if prop == "data")),
      "expected MissingRequiredProp for 'data', got: {diags:?}"
    );
  }

  #[test]
  fn unknown_prop_emits_diagnostic() {
    let root = parse_body(":::chart{extra=\"val\"}\n:::\n");
    let block = find_first(&root, SyntaxKind::MdContainerBlock).unwrap();
    let name = extract_container_name(&block).unwrap();
    let contract = vec![CustomComponentTyp {
      name: "Chart".to_string(),
      source: "./Chart.vue".to_string(),
      props: Some(vec![]),
    }];
    let mut diags = Vec::new();
    validate(&block, &name, &contract, &mut diags);
    assert!(
      diags
        .iter()
        .any(|d| matches!(d, TdDiagnostic { kind: TdDiagnosticKind::UnknownProp { prop, .. }, .. } if prop == "extra")),
      "expected UnknownProp for 'extra', got: {diags:?}"
    );
  }

  #[test]
  fn builtin_container_skipped() {
    let root = parse_body("::: note\ncontent\n:::\n");
    let block = find_first(&root, SyntaxKind::MdContainerBlock).unwrap();
    let name = extract_container_name(&block).unwrap();
    let contract: Vec<CustomComponentTyp> = vec![];
    let mut diags = Vec::new();
    validate(&block, &name, &contract, &mut diags);
    assert!(diags.is_empty(), "builtins should produce no diagnostics");
  }
}
