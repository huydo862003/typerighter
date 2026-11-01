//! Export typedown resources

pub mod content;
pub mod file_ref;
pub mod frontmatter;
pub mod html;
pub mod markdown;
pub mod media;
pub mod props;
pub mod serialize;
pub mod types;
pub mod utils;

pub use crate::integrations::bindings::{evaluate_lazy_field, serialize_handle};
pub use serialize::CircularRef;
pub use content::{export_resource_html, export_resource_markdown};
pub use file_ref::{
  FrefTarget, ResolvedRef, resolve_file_ref, resolve_fref_target, resolve_schema_label,
};
pub use frontmatter::{export_resource_nav, export_resource_summary};
pub use props::{Widget, export_prop_descriptors};
pub use serialize::serialize_to_json;
pub use types::*;

pub use crate::db::derived::name_resolver::file_symbol::file_symbol;

#[cfg(test)]
mod tests {
  use super::*;
  use crate::db::derived::evaluate::evaluate_typ::evaluate_typ;
  use crate::db::derived::name_resolver::file_symbol::file_symbol;
  use crate::db::fixtures::load_vault_fixture;
  use crate::db::types::{File, FileHandle, FileMetadata, Project, TdRuntimeObj};
  use crate::db::{QueryStorage, TypedownDatabase};
  use std::collections::HashMap;
  use std::path::PathBuf;

  #[test]
  fn exports_header_fields() {
    let (db, project, file) = load_vault_fixture("evaluate/my_vault", "valid_person.td");
    let result = export_resource_markdown(&db, project, file);
    let exported = result.expect("should export");
    assert_eq!(
      exported.schema,
      Some("Person".to_string()),
      "schema should be Person"
    );
    assert_eq!(exported.header["$kind"], "schema", "header should be $kind-tagged");
    let fields = exported.header.get("fields").and_then(|val| val.as_object())
      .expect("header should have fields object");
    assert!(
      !fields.is_empty(),
      "fields should not be empty"
    );
    assert_eq!(fields["name"], "Alice");
    assert_eq!(fields["age"], 30.0);
    assert!(
      !fields.contains_key("_content"),
      "_content should be stripped from fields"
    );
  }

  #[test]
  fn exports_content_body() {
    let (db, project, file) = load_vault_fixture("evaluate/my_vault", "md_with_content.td");
    let result = export_resource_markdown(&db, project, file);
    let exported = result.expect("should export");
    assert!(
      exported.content.contains("Hello world"),
      "content should contain body text: {}",
      exported.content
    );
  }

  #[test]
  fn exports_markdown_body() {
    let (db, project, file) = load_vault_fixture("evaluate/my_vault", "md_with_content.td");
    let exported = export_resource_markdown(&db, project, file).expect("should export");
    assert_eq!(
      exported.content,
      r"Hello world
"
    );
  }

  #[test]
  fn exports_all_markdown_elements() {
    let (db, project, file) = load_vault_fixture("evaluate/my_vault", "all_md_elements.td");
    let exported = export_resource_markdown(&db, project, file).expect("should export");
    let content = &exported.content;
    assert!(content.contains("# Heading 1"), "should contain h1");
    assert!(content.contains("## Heading 2"), "should contain h2");
    assert!(content.contains("**bold**"), "should contain bold");
    assert!(content.contains("- bullet one"), "should contain bullet");
    assert!(content.contains("[link text]"), "should contain link");
    assert!(
      content.contains("```js{1,3}"),
      "should preserve code block range indicator: {content}",
    );
  }

  #[test]
  fn exports_container_shorthand() {
    let (db, project, file) = load_vault_fixture("evaluate/my_vault", "md_container_shorthand.td");
    let exported = export_resource_markdown(&db, project, file).expect("should export");
    let content = &exported.content;
    assert!(
      content.contains(
        r"::: toc
:::
"
      ),
      "should expand [[toc]] to empty container: {content}",
    );
    assert!(
      content.contains(
        r"::: grid {cols=2}
:::
"
      ),
      "should expand [[grid {{cols=2}}]] to container with props: {content}",
    );
    assert!(
      content.contains(
        r"::: directory-index
:::
"
      ),
      "should expand [[directory-index]] with kebab-case: {content}",
    );
  }

  #[test]
  fn exports_unresolved_interpolation_as_empty() {
    let (db, project, file) = load_vault_fixture("evaluate/my_vault", "md_unresolved_interp.td");
    let exported = export_resource_markdown(&db, project, file).expect("should export");
    let content = &exported.content;
    assert!(
      !content.contains("question"),
      "unresolved interpolation should not appear in output: {content}",
    );
    assert!(
      content.contains(
        r"::: info
"
      ),
      "container should still be present: {content}",
    );
  }

  #[test]
  fn exports_nested_blocks_without_extra_indentation() {
    let (db, project, file) = load_vault_fixture("evaluate/my_vault", "md_nested_blocks.td");
    let exported = export_resource_markdown(&db, project, file).expect("should export");
    let content = &exported.content;

    for line in content.lines() {
      let indent = line.len() - line.trim_start().len();
      assert!(
        indent < 4 || line.trim_start().is_empty(),
        r"line has {indent} spaces of indentation (would become code block): '{line}'
full content:
{content}",
      );
    }

    assert!(
      content.contains(
        r"nested paragraph
"
      ),
      "nested paragraph should not have leading whitespace: {content}",
    );
    assert!(
      content.contains(
        r"> blockquote content
"
      ),
      "blockquote should use > prefix: {content}",
    );
    assert!(
      content.contains(
        r"- item one
"
      ),
      "list item should use - marker: {content}",
    );
  }

  #[test]
  fn exports_container_with_title() {
    let (db, project, file) = load_vault_fixture("evaluate/my_vault", "all_md_elements.td");
    let exported = export_resource_markdown(&db, project, file).expect("should export");
    assert!(
      exported.content.contains("::: details Click to expand"),
      "should contain container with title: {}",
      exported.content,
    );
  }

  #[test]
  fn returns_none_for_schema() {
    let (db, project, file) = load_vault_fixture("evaluate/my_vault", "_types/Person.td");
    let result = export_resource_markdown(&db, project, file);
    assert!(result.is_none(), "schema should return None");
  }

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
    eprintln!("PROPS: {}", serde_json::to_string_pretty(&props).unwrap());
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

  #[test]
  fn exports_asset_as_blob_descriptor() {
    let (db, project, file) = load_vault_fixture("evaluate/my_vault", "icon.svg");
    let exported = export_resource_markdown(&db, project, file).expect("asset should export");
    assert_eq!(exported.schema, Some("blob".to_string()));
    assert!(exported.content.is_empty(), "asset has no markdown body");
  }

  #[test]
  fn fref_uses_base_path() {
    let (db, project, file) = load_vault_fixture("evaluate/base_path_vault", "with_fref.td");
    let exported = export_resource_markdown(&db, project, file).expect("should export");
    assert!(
      exported.content.contains("[Alice](/blog/alice)"),
      "fref should use base_path /blog: {}",
      exported.content
    );
  }

  #[test]
  fn fref_resolves_image_asset_to_markdown_image() {
    let (db, project, file) = load_vault_fixture("evaluate/my_vault", "with_asset_fref.td");
    let exported = export_resource_markdown(&db, project, file).expect("should export");
    assert!(
      exported.content.contains("![icon](/icon.svg)"),
      "image asset fref should produce markdown image: {}",
      exported.content
    );
  }

  #[test]
  fn fref_default_base_path() {
    let (db, project, file) = load_vault_fixture("evaluate/my_vault", "with_fref.td");
    let exported = export_resource_markdown(&db, project, file).expect("should export");
    assert!(
      !exported.content.contains("/blog"),
      "default base_path should not have /blog prefix: {}",
      exported.content
    );
  }

  #[test]
  fn export_separates_blocks_with_blank_lines() {
    let (db, project, file) = load_vault_fixture("evaluate/my_vault", "md_with_content.td");
    let exported = export_resource_markdown(&db, project, file).expect("should export");
    let lines: Vec<&str> = exported.content.lines().collect();
    let heading_indices: Vec<usize> = lines
      .iter()
      .enumerate()
      .filter(|(_, line)| line.starts_with('#'))
      .map(|(index, _)| index)
      .collect();
    for &idx in &heading_indices {
      if idx > 0 {
        assert_eq!(
          lines[idx - 1],
          "",
          r"heading at line {idx} should be preceded by blank line:
{}",
          exported.content
        );
      }
    }
  }

  #[test]
  fn exports_schemaless_file() {
    let (db, project, file) = load_vault_fixture("evaluate/my_vault", "schemaless.td");
    let result = export_resource_markdown(&db, project, file);
    let exported = result.expect("schemaless file should export");
    assert_eq!(
      exported.schema, None,
      "schemaless file should have no schema"
    );
    assert!(
      exported.content.contains("Hello"),
      "should contain markdown body: {}",
      exported.content
    );
  }

  #[test]
  fn exports_schemaless_file_with_label_and_icon() {
    let (db, project, file) = load_vault_fixture("evaluate/my_vault", "schemaless_with_label.td");
    let exported = export_resource_markdown(&db, project, file).expect("should export");
    assert_eq!(exported.schema, None);
    assert_eq!(exported.label.as_deref(), Some("My Custom Title"));
    assert!(
      exported.icon.is_some(),
      "schemaless file with _icon should have icon"
    );
    assert_eq!(exported.icon.unwrap().name, "book-open");
  }

  #[test]
  fn resolve_schema_label_uses_label_field() {
    let (db, project, file) = load_vault_fixture("evaluate/my_vault", "_types/LabeledSchema.td");
    let label = resolve_schema_label(&db, project, file);
    assert_eq!(label, "Custom Label");
  }

  #[test]
  fn resolve_schema_label_falls_back_to_pascal_split() {
    let (db, project, file) = load_vault_fixture("evaluate/my_vault", "_types/Person.td");
    let label = resolve_schema_label(&db, project, file);
    assert_eq!(label, "Person");
  }

  #[test]
  fn incremental_add_icon_to_schema_no_cycle() {
    let mut db = TypedownDatabase {
      storage: QueryStorage::default(),
    };

    let vault = PathBuf::from("/test_vault");
    let schema_path = vault.join("vault/_types/MySchema.td");
    let config_path = vault.join("typedown.yaml");
    let content_path = vault.join("vault/page.td");

    let meta = || FileMetadata::default();

    let schema_file = File::new(
      &db,
      FileHandle::Content(
        schema_path.clone(),
        r#"---
_type: schema
properties:
  name:
    type: string
---
"#
        .to_string(),
        meta(),
      ),
    );
    let config_file = File::new(
      &db,
      FileHandle::Content(
        config_path.clone(),
        r#"version: "1.0.0"
vault:
  root_dir: "vault"
"#
        .to_string(),
        meta(),
      ),
    );
    let content_file = File::new(
      &db,
      FileHandle::Content(
        content_path.clone(),
        r#"---
_type: MySchema
name: "Alice"
---
"#
        .to_string(),
        meta(),
      ),
    );

    let files: HashMap<PathBuf, File> = [
      (schema_path.clone(), schema_file),
      (config_path.clone(), config_file),
      (content_path.clone(), content_file),
    ]
    .into_iter()
    .collect();
    let project = Project::new(&db, vault.clone(), files, None);

    let symbol = file_symbol(&db, project, schema_file).value(&db).unwrap();
    let result = evaluate_typ(&db, symbol);
    assert!(
      result.diagnostics(&db).is_empty(),
      "{:?}",
      result.diagnostics(&db)
    );
    assert!(
      result
        .typ(&db)
        .unwrap()
        .get_builtin_field(&db, "_icon")
        .is_none()
    );

    let exported = export_resource_markdown(&db, project, content_file).expect("should export");
    assert_eq!(exported.schema.as_deref(), Some("MySchema"));

    schema_file.set_handle(
      &mut db,
      FileHandle::Content(
        schema_path.clone(),
        r#"---
_type: schema
_icon: icon.book
properties:
  name:
    type: string
---
"#
        .to_string(),
        meta(),
      ),
    );

    let symbol2 = file_symbol(&db, project, schema_file).value(&db).unwrap();
    let result2 = evaluate_typ(&db, symbol2);
    assert!(
      result2.diagnostics(&db).is_empty(),
      "after adding _icon: {:?}",
      result2.diagnostics(&db)
    );
    assert!(
      result2
        .typ(&db)
        .unwrap()
        .get_builtin_field(&db, "_icon")
        .is_some(),
      "should have _icon after mutation"
    );

    let exported2 =
      export_resource_markdown(&db, project, content_file).expect("should export after mutation");
    assert_eq!(exported2.schema.as_deref(), Some("MySchema"));
  }

  // HTML export tests

  #[test]
  fn html_export_simple_paragraph() {
    let (db, project, file) = load_vault_fixture("evaluate/my_vault", "md_with_content.td");
    let exported = export_resource_html(&db, project, file).expect("should export");
    assert_eq!(
      exported.content,
      r"<p>Hello world</p>
"
    );
  }

  #[test]
  fn html_export_headings() {
    let (db, project, file) = load_vault_fixture("evaluate/my_vault", "all_md_elements.td");
    let exported = export_resource_html(&db, project, file).expect("should export");
    eprintln!("{:?}", exported.content);
    assert!(
      exported.content.contains(
        r##"<h1 id="heading-1">Heading 1 <a class="td-header-anchor" href="#heading-1""##
      )
    );
    assert!(
      exported.content.contains(
        r##"<h2 id="heading-2">Heading 2 <a class="td-header-anchor" href="#heading-2""##
      )
    );
    assert_eq!(exported.headings.len(), 2);
    assert_eq!(exported.headings[0].level, 1);
    assert_eq!(exported.headings[0].title, "Heading 1");
    assert_eq!(exported.headings[0].slug, "heading-1");
    assert_eq!(exported.headings[1].level, 2);
    assert_eq!(exported.headings[1].slug, "heading-2");
    assert_eq!(exported.title.as_deref(), Some("Heading 1"));
  }

  #[test]
  fn html_export_heading_titles_preserve_special_chars() {
    let (db, project, file) = load_vault_fixture("evaluate/my_vault", "md_special_headings.td");
    let exported = export_resource_html(&db, project, file).expect("should export");
    let titles: Vec<&str> = exported
      .headings
      .iter()
      .map(|heading| heading.title.as_str())
      .collect();
    assert!(
      titles.contains(&"Version v1.2.3"),
      "dots preserved: {titles:?}"
    );
    assert!(
      titles.contains(&"Key: Value Pairs"),
      "colon preserved: {titles:?}"
    );
    assert!(
      titles.contains(&"C++ and C#"),
      "plus and hash preserved: {titles:?}"
    );
    assert!(
      titles.contains(&"node.js vs Deno"),
      "dots preserved: {titles:?}"
    );
  }

  #[test]
  fn html_export_heading_with_inline_markup() {
    let (db, project, file) = load_vault_fixture("evaluate/my_vault", "md_special_headings.td");
    let exported = export_resource_html(&db, project, file).expect("should export");
    assert!(
      exported.content.contains(r#"href="https://example.com""#),
      r"heading should contain rendered link:
{}",
      exported.content
    );
    assert!(
      exported.content.contains("`def1`"),
      r"heading should preserve inline code in link:
{}",
      exported.content
    );
    assert!(
      exported.content.contains("<strong>bold heading</strong>"),
      r"heading should contain rendered bold:
{}",
      exported.content
    );
  }

  #[test]
  fn html_export_inline_markup() {
    let (db, project, file) = load_vault_fixture("evaluate/my_vault", "all_md_elements.td");
    let exported = export_resource_html(&db, project, file).expect("should export");
    assert!(exported.content.contains("<strong>bold</strong>"));
    assert!(exported.content.contains("<em>italic</em>"));
    assert!(
      exported
        .content
        .contains("<strong><em>bold italic</em></strong>")
    );
    assert!(exported.content.contains("<s>strikethrough</s>"));
  }

  #[test]
  fn html_export_inline_code() {
    let (db, project, file) = load_vault_fixture("evaluate/my_vault", "all_md_elements.td");
    let exported = export_resource_html(&db, project, file).expect("should export");
    assert!(exported.content.contains("<code>inline code</code>"));
  }

  #[test]
  fn html_export_code_block_placeholder() {
    let (db, project, file) = load_vault_fixture("evaluate/my_vault", "all_md_elements.td");
    let exported = export_resource_html(&db, project, file).expect("should export");
    assert!(exported.content.contains(
      r#"<pre class="td-code-placeholder" data-lang="python" data-meta="python"><code>print(&quot;hello&quot;)"#
    ));
    assert!(exported.content.contains(
      r#"<pre class="td-code-placeholder" data-lang="js" data-meta="js{1,3}"><code>const x = 1;"#
    ));
  }

  #[test]
  fn html_export_code_block_with_title() {
    let (db, project, file) = load_vault_fixture("evaluate/my_vault", "all_md_elements.td");
    let exported = export_resource_html(&db, project, file).expect("should export");
    assert!(exported.content.contains(r#"data-title="config.ts""#));
  }

  #[test]
  fn html_export_code_block_with_line_numbers() {
    let (db, project, file) = load_vault_fixture("evaluate/my_vault", "all_md_elements.td");
    let exported = export_resource_html(&db, project, file).expect("should export");
    assert!(exported.content.contains(r#"data-line-numbers="5""#));
  }

  #[test]
  fn html_export_inline_math_placeholder() {
    let (db, project, file) = load_vault_fixture("evaluate/my_vault", "all_md_elements.td");
    let exported = export_resource_html(&db, project, file).expect("should export");
    assert!(
      exported
        .content
        .contains(r#"<span class="td-math-inline">E = mc^2</span>"#)
    );
  }

  #[test]
  fn html_export_math_block_placeholder() {
    let (db, project, file) = load_vault_fixture("evaluate/my_vault", "all_md_elements.td");
    let exported = export_resource_html(&db, project, file).expect("should export");
    assert!(
      exported
        .content
        .contains(r#"<div class="td-math-block">\int_0^\infty e^{-x^2} dx"#)
    );
  }

  #[test]
  fn html_export_blockquote() {
    let (db, project, file) = load_vault_fixture("evaluate/my_vault", "all_md_elements.td");
    let exported = export_resource_html(&db, project, file).expect("should export");
    assert!(exported.content.contains(
      r"<blockquote>
<p>blockquote line</p>
</blockquote>"
    ));
  }

  #[test]
  fn html_export_table() {
    let (db, project, file) = load_vault_fixture("evaluate/my_vault", "all_md_elements.td");
    let exported = export_resource_html(&db, project, file).expect("should export");
    assert!(exported.content.contains(r#"<table tabindex="0">"#));
    assert!(exported.content.contains("<th>"));
    assert!(exported.content.contains("Col1"));
    assert!(exported.content.contains("Col2"));
    assert!(exported.content.contains("<td>"));
    let th_count = exported.content.matches("<th>").count();
    let td_count = exported.content.matches("<td>").count();
    assert_eq!(th_count, 2, "should have 2 header cells");
    assert_eq!(td_count, 2, "should have 2 data cells");
  }

  #[test]
  fn html_export_table_inline_code() {
    let (db, project, file) = load_vault_fixture("evaluate/my_vault", "md_table_inline_code.td");
    let exported = export_resource_html(&db, project, file).expect("should export");
    assert!(
      exported.content.contains("<code>_type</code>"),
      "inline code _type should be preserved in table cell: {}",
      exported.content
    );
    assert!(
      exported.content.contains("<code>_label</code>"),
      "inline code _label should be preserved: {}",
      exported.content
    );
    assert!(
      exported.content.contains("<code>_content</code>"),
      "inline code _content should be preserved: {}",
      exported.content
    );
    assert!(
      exported.content.contains("<code>icon.rocket</code>"),
      "inline code icon.rocket should be preserved: {}",
      exported.content
    );
  }

  #[test]
  fn html_export_bullet_list() {
    let (db, project, file) = load_vault_fixture("evaluate/my_vault", "all_md_elements.td");
    let exported = export_resource_html(&db, project, file).expect("should export");
    assert!(exported.content.contains(
      r"<ul>
<li>"
    ));
    assert!(exported.content.contains("bullet one"));
    assert!(exported.content.contains("bullet two"));
  }

  #[test]
  fn html_export_ordered_list() {
    let (db, project, file) = load_vault_fixture("evaluate/my_vault", "all_md_elements.td");
    let exported = export_resource_html(&db, project, file).expect("should export");
    assert!(exported.content.contains(
      r"<ol>
<li>"
    ));
    assert!(exported.content.contains("ordered one"));
  }

  #[test]
  fn html_export_callout_container() {
    let (db, project, file) = load_vault_fixture("evaluate/my_vault", "all_md_elements.td");
    let exported = export_resource_html(&db, project, file).expect("should export");
    assert!(exported.content.contains(
      r#"<div class="note td-callout"><p class="td-callout-title td-callout-title-default">NOTE</p>"#
    ));
    assert!(exported.content.contains("<p>callout content</p>"));
  }

  #[test]
  fn html_export_code_block_in_callout() {
    let (db, project, file) = load_vault_fixture("evaluate/my_vault", "all_md_elements.td");
    let exported = export_resource_html(&db, project, file).expect("should export");
    assert!(
      exported
        .content
        .contains(r#"<pre class="td-code-placeholder" data-lang="python" data-meta="python"><code>print(&quot;in callout&quot;)"#)
    );
  }

  #[test]
  fn html_export_details_container() {
    let (db, project, file) = load_vault_fixture("evaluate/my_vault", "all_md_elements.td");
    let exported = export_resource_html(&db, project, file).expect("should export");
    assert!(
      exported
        .content
        .contains(r#"<details class="details td-callout"><summary>Click to expand</summary>"#)
    );
    assert!(exported.content.contains("<p>details content</p>"));
  }

  #[test]
  fn html_export_external_link() {
    let (db, project, file) = load_vault_fixture("evaluate/my_vault", "all_md_elements.td");
    let exported = export_resource_html(&db, project, file).expect("should export");
    assert!(exported.content.contains(
      r#"<LucideIcon name="arrow-up-right" class="td-external-link-icon" /><a href="https://example.com" class="td-external-link" target="_blank" rel="noopener noreferrer">link text</a>"#
    ));
  }

  #[test]
  fn html_export_code_in_link_text() {
    let (db, project, file) = load_vault_fixture("evaluate/my_vault", "all_md_elements.td");
    let exported = export_resource_html(&db, project, file).expect("should export");
    assert!(
      exported.content.contains("<code>code</code> in link</a>"),
      "inline code in link should render as <code>code</code> inside <a>: {}",
      exported.content
    );
    assert!(
      !exported.content.contains("`code` in link"),
      "backticks should not appear as raw text in rendered link: {}",
      exported.content
    );
  }

  #[test]
  fn html_export_container_shorthand() {
    let (db, project, file) = load_vault_fixture("evaluate/my_vault", "md_container_shorthand.td");
    let exported = export_resource_html(&db, project, file).expect("should export");
    assert!(exported.content.contains(
      r"<toc />
"
    ));
    assert!(exported.content.contains(
      r"<directory-index />
"
    ));
    assert!(exported.content.contains(
      r#"<grid :cols="2" />
"#
    ));
  }

  #[test]
  fn html_export_container_empty_named_slot() {
    let (db, project, file) = load_vault_fixture("evaluate/my_vault", "md_container_empty_slot.td");
    let exported = export_resource_html(&db, project, file).expect("should export");
    let content = &exported.content;
    assert!(
      content.contains("<template #key>"),
      "should have named slot: {content}"
    );
    assert!(
      content.contains("</template>"),
      "named slot template should be closed: {content}"
    );
    assert!(
      content.contains("</flashcard>"),
      "container should be closed: {content}"
    );
  }

  #[test]
  fn html_export_blob_empty() {
    let (db, project, file) = load_vault_fixture("evaluate/my_vault", "icon.svg");
    let exported = export_resource_html(&db, project, file).expect("should export");
    assert_eq!(exported.schema, Some("blob".to_string()));
    assert!(exported.content.is_empty());
    assert!(exported.headings.is_empty());
    assert!(exported.title.is_none());
  }

  #[test]
  fn html_export_schema_and_header() {
    let (db, project, file) = load_vault_fixture("evaluate/my_vault", "valid_person.td");
    let exported = export_resource_html(&db, project, file).expect("should export");
    assert_eq!(exported.schema, Some("Person".to_string()));
    // Header now uses $kind-tagged bindings format
    let fields = &exported.header["fields"];
    assert_eq!(fields["name"], "Alice");
    assert_eq!(fields["age"], 30.0);
  }

  #[test]
  fn html_export_fref_link() {
    let (db, project, file) = load_vault_fixture("evaluate/base_path_vault", "with_fref.td");
    let exported = export_resource_html(&db, project, file).expect("should export");
    assert!(
      exported
        .content
        .contains(r#"<a href="/blog/alice" class="td-fref-link">Alice</a>"#)
    );
  }

  #[test]
  fn html_export_fref_image() {
    let (db, project, file) = load_vault_fixture("evaluate/my_vault", "with_asset_fref.td");
    let exported = export_resource_html(&db, project, file).expect("should export");
    assert!(
      exported
        .content
        .contains(r#"<img src="/icon.svg" alt="icon" loading="lazy">"#)
    );
  }

  #[test]
  fn html_export_relative_fref_image() {
    let (db, project, file) = load_vault_fixture("evaluate/my_vault", "subdir/relative_fref.td");
    let exported = export_resource_html(&db, project, file).expect("should export");
    assert!(
      exported.content.contains("<img src=") && exported.content.contains("test.svg"),
      "relative fref should render as img tag: {}",
      exported.content
    );
  }

  #[test]
  fn html_export_no_raw_markdown() {
    let (db, project, file) = load_vault_fixture("evaluate/my_vault", "all_md_elements.td");
    let exported = export_resource_html(&db, project, file).expect("should export");
    assert!(!exported.content.contains(
      r"
> "
    ));
    assert!(!exported.content.contains(
      r"
- "
    ));
    assert!(!exported.content.contains(
      r"
1. "
    ));
    assert!(!exported.content.contains("**"));
    assert!(!exported.content.contains("~~"));
  }

  #[test]
  fn html_export_heading_slugs_unique() {
    let (db, project, file) = load_vault_fixture("evaluate/my_vault", "all_md_elements.td");
    let exported = export_resource_html(&db, project, file).expect("should export");
    let slugs: Vec<&str> = exported
      .headings
      .iter()
      .map(|heading| heading.slug.as_str())
      .collect();
    let mut deduped = slugs.clone();
    deduped.sort();
    deduped.dedup();
    assert_eq!(
      slugs.len(),
      deduped.len(),
      "slugs should be unique: {:?}",
      slugs
    );
  }

  // Body-only files (no frontmatter)

  #[test]
  fn html_export_body_only_file() {
    let (db, project, file) = load_vault_fixture("evaluate/my_vault", "body_only.td");
    let exported = export_resource_html(&db, project, file).expect("body-only file should export");
    assert!(
      exported
        .content
        .contains("<p>This file has no frontmatter at all.</p>")
    );
    assert!(exported.content.contains("<h2"));
    assert!(exported.schema.is_none());
    assert!(exported.label.is_none());
    // Body-only files produce an empty product with bindings format
    assert!(exported.header.is_object(), "header should be object");
  }

  #[test]
  fn markdown_export_body_only_file() {
    let (db, project, file) = load_vault_fixture("evaluate/my_vault", "body_only.td");
    let exported =
      export_resource_markdown(&db, project, file).expect("body-only file should export");
    assert!(
      exported
        .content
        .contains("This file has no frontmatter at all.")
    );
    assert!(exported.schema.is_none());
  }

  #[test]
  fn summary_export_body_only_file() {
    let (db, project, file) = load_vault_fixture("evaluate/my_vault", "body_only.td");
    let exported =
      export_resource_summary(&db, project, file).expect("body-only file should export");
    assert!(exported.schema.is_none());
    assert!(exported.excerpt.is_some());
  }

  #[test]
  fn nav_export_body_only_file() {
    let (db, project, file) = load_vault_fixture("evaluate/my_vault", "body_only.td");
    let exported = export_resource_nav(&db, project, file).expect("body-only file should export");
    assert!(exported.schema.is_none());
  }

  #[test]
  fn html_linked_image_badge() {
    let (db, project, file) = load_vault_fixture("evaluate/my_vault", "linked_image.td");
    let exported = export_resource_html(&db, project, file).expect("should export");
    let content = &exported.content;
    assert!(
      content.contains(r#"<a href="mailto:huydo862003@gmail.com""#),
      "should contain mailto link: {content}",
    );
    assert!(
      content.contains(
        r#"<img src="https://img.shields.io/badge/Email-blue" alt="Email badge" loading="lazy">"#
      ),
      "should contain image inside link: {content}",
    );
    assert!(
      !content.contains("LucideIcon"),
      "mailto links should not have arrow icon: {content}",
    );
  }

  #[test]
  fn html_export_body_only_nested_vault() {
    let (db, project, file) = load_vault_fixture("evaluate/nested_vault", "vault/body_only.td");
    let exported = export_resource_html(&db, project, file);
    assert!(
      exported.is_some(),
      "body-only file in nested vault should export"
    );
    let exported = exported.unwrap();
    assert!(
      exported
        .content
        .contains("<p>This file has no frontmatter.</p>")
    );
  }

  #[test]
  fn html_export_extracts_seo_metadata() {
    let (db, project, file) = load_vault_fixture("evaluate/my_vault", "with_meta.td");
    let exported = export_resource_html(&db, project, file).expect("should export");
    let seo = exported.seo_metadata.expect("_meta should be extracted");
    assert_eq!(seo.title.as_deref(), Some("Custom SEO Title"));
    assert_eq!(seo.description.as_deref(), Some("Custom SEO description"));
    assert_eq!(seo.image.as_deref(), Some("/images/og.png"));
  }

  #[test]
  fn html_export_preserves_external_url() {
    let (db, project, file) = load_vault_fixture("evaluate/my_vault", "url_resolution.td");
    let exported = export_resource_html(&db, project, file).expect("should export");
    assert!(
      exported.content.contains(r#"href="https://example.com""#),
      "external URL should pass through unchanged: {}",
      exported.content
    );
  }

  #[test]
  fn html_export_preserves_anchor_url() {
    let (db, project, file) = load_vault_fixture("evaluate/my_vault", "url_resolution.td");
    let exported = export_resource_html(&db, project, file).expect("should export");
    assert!(
      exported.content.contains(r##"href="#section""##),
      "anchor URL should pass through unchanged: {}",
      exported.content
    );
  }

  #[test]
  fn html_export_resolves_relative_link() {
    let (db, project, file) = load_vault_fixture("evaluate/my_vault", "url_resolution.td");
    let exported = export_resource_html(&db, project, file).expect("should export");
    assert!(
      exported.content.contains(r#"href="/other.td""#),
      "relative link should resolve with base path: {}",
      exported.content
    );
  }

  #[test]
  fn html_export_no_seo_metadata_when_absent() {
    let (db, project, file) = load_vault_fixture("evaluate/my_vault", "valid_person.td");
    let exported = export_resource_html(&db, project, file).expect("should export");
    assert!(
      exported.seo_metadata.is_none(),
      "seo_metadata should be None when not set"
    );
  }

  #[test]
  fn html_export_media_with_title() {
    let (db, project, file) = load_vault_fixture("evaluate/my_vault", "media_with_title.td");
    let exported = export_resource_html(&db, project, file).expect("should export");
    let content = &exported.content;
    assert!(
      content.contains("<figure>"),
      "image with title should be wrapped in <figure>: {content}",
    );
    assert!(
      content.contains(r#"title="The Typedown project logo""#),
      "image should have title attribute: {content}",
    );
    assert!(
      content.contains("<figcaption>The Typedown project logo</figcaption>"),
      "image should have figcaption: {content}",
    );
  }

  #[test]
  fn html_export_link_with_title() {
    let (db, project, file) = load_vault_fixture("evaluate/my_vault", "link_with_title.td");
    let exported = export_resource_html(&db, project, file).expect("should export");
    let content = &exported.content;
    assert!(
      content.contains(r#"title="Visit Example""#),
      "link should have title attribute: {content}",
    );
    assert!(
      content.contains("href=\"https://example.com"),
      "link should have href: {content}",
    );
  }

  #[test]
  fn html_export_github_file_embed() {
    let (db, project, file) = load_vault_fixture("evaluate/my_vault", "github_embed.td");
    let exported = export_resource_html(&db, project, file).expect("should export");
    let content = &exported.content;
    assert!(
      content.contains("class=\"td-github-placeholder\""),
      "github file URL should emit placeholder: {content}",
    );
    assert!(
      content.contains("data-url=\"https://github.com/vuejs/vite/blob/main/packages/vite/src/node/server/index.ts#L150-L165"),
      "placeholder should have data-url: {content}",
    );
    assert!(
      content.contains("data-label=\"server init\""),
      "placeholder should have data-label from alt text: {content}",
    );
    assert!(
      content.contains("data-title=\"Vite dev server\""),
      "placeholder should have data-title: {content}",
    );
  }

  #[test]
  fn html_export_github_gist_embed() {
    let (db, project, file) = load_vault_fixture("evaluate/my_vault", "github_embed.td");
    let exported = export_resource_html(&db, project, file).expect("should export");
    let content = &exported.content;
    assert!(
      content.contains("data-url=\"https://gist.github.com/octocat/6cad326836d38bd3a7ae\""),
      "gist URL should emit placeholder with data-url: {content}",
    );
  }

  #[test]
  fn html_export_nested_bullet_list() {
    let (db, project, file) = load_vault_fixture("evaluate/my_vault", "nested_list.td");
    let exported = export_resource_html(&db, project, file).expect("should export");
    assert_eq!(
      exported.content,
      r##"<ul>
<li>
<p><strong><LucideIcon name="arrow-up-right" class="td-external-link-icon" /><a href="https://github.com/huydo862003/typerighter" class="td-external-link" target="_blank" rel="noopener noreferrer">typerighter</a></strong>: A typed content framework.</p>
<ul>
<li>
<p>Somewhat stabilized with lots of optimizations!</p>
</li>
<li>
<p><code>npm</code> package: <LucideIcon name="arrow-up-right" class="td-external-link-icon" /><a href="https://www.npmjs.com/package/typerighter" class="td-external-link" target="_blank" rel="noopener noreferrer">npm</a></p>
</li>
<li>
<p>documentation: <LucideIcon name="arrow-up-right" class="td-external-link-icon" /><a href="https://huydo862003.github.io/typerighter" class="td-external-link" target="_blank" rel="noopener noreferrer">docs</a></p>
</li>
</ul>
</li>
<li>
<p><strong><LucideIcon name="arrow-up-right" class="td-external-link-icon" /><a href="https://github.com/huydo862003/sqlingo" class="td-external-link" target="_blank" rel="noopener noreferrer">sqlingo</a></strong>: Porting sqlglot to TypeScript.</p>
<ul>
<li>
<p>Stabilized &amp; Synced with sqlglot version 30.0.0.</p>
</li>
<li>
<p><code>npm</code> package: <LucideIcon name="arrow-up-right" class="td-external-link-icon" /><a href="https://www.npmjs.com/package/sqlingo" class="td-external-link" target="_blank" rel="noopener noreferrer">npm</a></p>
</li>
<li>
<p>documentation: <LucideIcon name="arrow-up-right" class="td-external-link-icon" /><a href="https://huydo862003.github.io/sqlingo/" class="td-external-link" target="_blank" rel="noopener noreferrer">docs</a></p>
</li>
</ul>
</li>
</ul>
"##
    );
  }

  #[test]
  fn html_export_nested_container_indented() {
    let (db, project, file) = load_vault_fixture("evaluate/my_vault", "md_nested_container.td");
    let exported = export_resource_html(&db, project, file).expect("should export");
    let content = &exported.content;
    assert!(
      content.contains("<grid"),
      "should contain outer grid component: {content}"
    );
    assert!(
      content.contains("<grid-cell>"),
      "should contain inner grid-cell component: {content}"
    );
    assert!(
      content.contains("</grid-cell>"),
      "grid-cell should be closed: {content}"
    );
    assert!(
      content.contains("<p>Alpha</p>"),
      "first cell content should render: {content}"
    );
    assert!(
      content.contains("<p>Beta</p>"),
      "second cell content should render: {content}"
    );
  }

  #[test]
  fn html_export_nested_container_not_indented() {
    let (db, project, file) =
      load_vault_fixture("evaluate/my_vault", "md_nested_container_flat.td");
    let exported = export_resource_html(&db, project, file).expect("should export");
    let content = &exported.content;
    assert!(
      content.contains("<grid-cell>"),
      "should contain grid-cell: {content}"
    );
    assert!(
      content.contains("<p>Alpha</p>"),
      "first cell content should render: {content}"
    );
  }

  #[test]
  fn html_export_nested_container_with_props() {
    let (db, project, file) =
      load_vault_fixture("evaluate/my_vault", "md_nested_container_props.td");
    let exported = export_resource_html(&db, project, file).expect("should export");
    let content = &exported.content;
    assert!(
      content.contains(r#":columns="3""#),
      "outer grid should have columns prop: {content}"
    );
    assert!(
      content.contains(r#":col-span="2""#),
      "inner grid-cell should have col-span prop: {content}"
    );
  }

  #[test]
  fn exports_fref_list_in_header() {
    let (db, project, file) = load_vault_fixture("evaluate/my_vault", "book_with_frefs.td");
    let exported = export_resource_html(&db, project, file).expect("should export");
    let header = &exported.header;
    assert_eq!(header["$kind"], "schema", "header should be $kind-tagged schema");
    let fields = header.get("fields").and_then(|val| val.as_object())
      .expect("header should have fields");

    // Single fref in header: serialized as full schema object
    assert_eq!(
      fields["parent"]["$kind"], "schema",
      "parent fref should serialize as schema object: {}",
      fields["parent"]
    );
    assert!(
      fields["parent"]["fields"]["name"].is_string(),
      "parent should have name field: {}",
      fields["parent"]
    );

    // List of frefs in header
    let concepts = fields["concepts"]
      .as_array()
      .expect("concepts should be array");
    assert_eq!(concepts.len(), 2, "should have 2 concepts in header");
    for (index, concept) in concepts.iter().enumerate() {
      assert!(
        !concept.is_null(),
        "concept {index} should not be null: {concept}"
      );
      assert_eq!(
        concept["$kind"], "schema",
        "concept {index} should be a schema object: {concept}"
      );
    }
  }

  #[test]
  fn exports_absolute_fref_in_header() {
    let (db, project, file) =
      load_vault_fixture("evaluate/my_vault", "book_with_absolute_frefs.td");
    let exported = export_resource_html(&db, project, file).expect("should export");
    let header = &exported.header;
    assert_eq!(header["$kind"], "schema", "header should be $kind-tagged");
    let fields = header.get("fields").and_then(|val| val.as_object())
      .expect("header should have fields");

    // Absolute path fref
    assert_eq!(
      fields["parent"]["$kind"], "schema",
      "absolute fref should serialize as schema: {}",
      fields["parent"]
    );

    let concepts = fields["concepts"]
      .as_array()
      .expect("concepts should be array");
    assert_eq!(concepts.len(), 2);
    for (index, concept) in concepts.iter().enumerate() {
      assert!(
        !concept.is_null(),
        "absolute fref concept {index} should not be null"
      );
      assert_eq!(
        concept["$kind"], "schema",
        "absolute fref concept {index} should serialize as schema: {concept}"
      );
    }
  }
}
