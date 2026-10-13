// Tracked query: validate custom component usage in a file against the component contract

use typedown_macros::query_derived;

use crate::db::TypedownDatabase;
use crate::db::derived::get_component_contract::get_component_contract;
use crate::db::derived::parse_file::parse_file;
use crate::db::types::{CustomComponentCheckResult, CustomComponentTyp, File, Project};
use crate::db::utils::{
  extract_container_name, extract_container_prop_names, extract_shorthand_name,
  is_builtin_container,
};
use crate::syntax::diagnostic::Diagnostic;
use crate::syntax::red::RedNode;
use crate::syntax::syntax_kind::SyntaxKind;
use typedown_incremental::QueryDatabase;

#[query_derived]
pub fn check_custom_components<'db>(
  db: &'db TypedownDatabase,
  project: Project,
  file: File,
) -> CustomComponentCheckResult<'db> {
  let contract = project
    .custom_component_contract(db)
    .map(|f| get_component_contract(db, f).entries(db).clone())
    .unwrap_or_default();

  let root = parse_file(db, project, file).ast(db).node.clone();
  let mut diagnostics = Vec::new();
  collect(&root, &contract, &mut diagnostics);
  CustomComponentCheckResult::new(db, diagnostics)
}

fn collect(
  node: &RedNode,
  contract: &[CustomComponentTyp],
  diagnostics: &mut Vec<Diagnostic>,
) {
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
  diagnostics: &mut Vec<Diagnostic>,
) {
  if is_builtin_container(name) {
    return;
  }

  let (start_offset, len) = node.trimmed_range();
  let end_offset = start_offset + len;

  let Some(entry) = find_component(contract, name) else {
    diagnostics.push(Diagnostic::UnknownComponent {
      name: name.to_string(),
      start_offset,
      end_offset,
    });
    return;
  };

  let provided = extract_container_prop_names(node);

  for prop in &entry.props {
    if prop.required && !provided.iter().any(|p| p == &prop.name) {
      diagnostics.push(Diagnostic::MissingRequiredProp {
        prop: prop.name.clone(),
        component: name.to_string(),
        start_offset,
        end_offset,
      });
    }
  }

  for prop_name in &provided {
    if !entry.props.iter().any(|p| &p.name == prop_name) {
      diagnostics.push(Diagnostic::UnknownProp {
        prop: prop_name.clone(),
        component: name.to_string(),
        start_offset,
        end_offset,
      });
    }
  }
}

// Find by exact name or by converting kebab-case to PascalCase
fn find_component<'a>(
  contract: &'a [CustomComponentTyp],
  name: &str,
) -> Option<&'a CustomComponentTyp> {
  let pascal = to_pascal_case(name);
  contract.iter().find(|e| e.name == name || e.name == pascal)
}

fn to_pascal_case(name: &str) -> String {
  name
    .split('-')
    .map(|segment| {
      let mut chars = segment.chars();
      match chars.next() {
        Some(first) => {
          let upper: String = first.to_uppercase().collect();
          upper + chars.as_str()
        }
        None => String::new(),
      }
    })
    .collect()
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn to_pascal_case_converts_hyphenated() {
    assert_eq!(to_pascal_case("video-embed"), "VideoEmbed");
    assert_eq!(to_pascal_case("chart"), "Chart");
    assert_eq!(to_pascal_case("my-custom-chart"), "MyCustomChart");
  }

  #[test]
  fn to_pascal_case_preserves_pascal() {
    assert_eq!(to_pascal_case("Chart"), "Chart");
    assert_eq!(to_pascal_case("VideoEmbed"), "VideoEmbed");
  }

  #[test]
  fn find_component_by_exact_name() {
    let contract: Vec<CustomComponentTyp> = vec![CustomComponentTyp {
      name: "Chart".to_string(),
      source: "./Chart.vue".to_string(),
      props: vec![],
    }];
    assert!(find_component(&contract, "Chart").is_some());
    // "chart" -> PascalCase "Chart" -> matches
    assert!(find_component(&contract, "chart").is_some());
    assert!(find_component(&contract, "Unknown").is_none());
  }

  #[test]
  fn find_component_by_kebab_to_pascal() {
    let contract: Vec<CustomComponentTyp> = vec![CustomComponentTyp {
      name: "VideoEmbed".to_string(),
      source: "./VideoEmbed.vue".to_string(),
      props: vec![],
    }];
    assert!(find_component(&contract, "video-embed").is_some());
    assert!(find_component(&contract, "VideoEmbed").is_some());
  }
}
