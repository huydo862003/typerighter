//! Export typedown resources

pub mod content;
pub mod frontmatter;
pub mod props;
pub mod types;
pub mod utils;

pub use crate::integrations::bindings::{
  FrefTarget, ResolvedRef, evaluate_lazy_field, resolve_file_ref, resolve_fref_target,
  resolve_schema_label, serialize_handle, serialize_value, serialize_value_with_project,
};
pub use content::{export_resource_html, export_resource_markdown};
pub use frontmatter::{export_resource_nav, export_resource_summary};
pub use props::{Widget, export_prop_descriptors};
pub use types::*;

pub use crate::db::derived::name_resolver::file_symbol::file_symbol;

#[cfg(test)]
mod tests {
  use super::*;
  use crate::db::fixtures::load_vault_fixture;

  #[test]
  fn exports_list_field_schema() {
    let (db, project, file) = load_vault_fixture("evaluate/my_vault", "_types/WithListField.td");
    let props =
      export_prop_descriptors(&db, project, file).expect("WithListField schema should export");
    assert_eq!(props["tags"]["widget"], "list");
    assert_eq!(props["tags"]["items"]["widget"], "text");
    assert_eq!(props["scores"]["widget"], "list");
    assert_eq!(props["scores"]["items"]["widget"], "number");
  }

  #[test]
  fn exports_schema_props() {
    let (db, project, file) = load_vault_fixture("evaluate/my_vault", "_types/Person.td");
    let props = export_prop_descriptors(&db, project, file).expect("Person schema should export");
    assert_eq!(props["name"]["widget"], "text");
    assert_eq!(props["age"]["widget"], "number");
  }

  #[test]
  fn exports_schema_select_prop() {
    let (db, project, file) = load_vault_fixture("evaluate/my_vault", "_types/Status.td");
    let props = export_prop_descriptors(&db, project, file).expect("Status schema should export");
    assert_eq!(props["status"]["widget"], "select");
    assert_eq!(
      props["status"]["options"],
      serde_json::json!(["archived", "draft", "published"])
    );
  }

  #[test]
  fn exports_schema_relation_prop() {
    let (db, project, file) = load_vault_fixture("evaluate/my_vault", "_types/Event.td");
    let props = export_prop_descriptors(&db, project, file).expect("Event schema should export");
    assert_eq!(props["title"]["widget"], "text");
    assert_eq!(props["location"]["widget"], "relation");
    assert_eq!(props["location"]["schema"], "Address");
  }

  #[test]
  fn exports_schema_list_relation_prop() {
    let (db, project, file) = load_vault_fixture("evaluate/my_vault", "_types/BookType.td");
    let props = export_prop_descriptors(&db, project, file).expect("BookType schema should export");
    assert_eq!(
      props["parent"]["widget"], "relation",
      "parent should be relation widget"
    );
    assert_eq!(props["parent"]["schema"], "Person");
    assert_eq!(
      props["concepts"]["widget"], "list",
      "concepts should be list widget"
    );
    assert_eq!(
      props["concepts"]["items"]["widget"], "relation",
      "concepts items should be relation widget"
    );
    assert_eq!(props["concepts"]["items"]["schema"], "Person");
  }

  #[test]
  fn returns_none_for_non_schema() {
    let (db, project, file) = load_vault_fixture("evaluate/my_vault", "valid_person.td");
    let result = export_prop_descriptors(&db, project, file);
    assert!(
      result.is_none(),
      "resource file should return None from export_prop_descriptors"
    );
  }
}
