//! Export typedown resources

pub mod file_ref;
pub mod html;
pub mod json;
pub mod markdown;
pub mod properties;
pub mod types;
pub mod utils;

pub use file_ref::{
  FrefTarget, ResolvedRef, resolve_fref_target, resolve_file_ref, resolve_schema_label,
};
pub use json::evaluate_lazy_field;
pub use properties::{Widget, export_property_descriptors};
pub use types::*;

pub use crate::db::derived::name_resolver::file_symbol::file_symbol;

use crate::db::TypedownDatabase;
use crate::db::derived::evaluate::evaluate_resource::evaluate_resource;
use crate::db::derived::get_vault_config::get_vault_config;
use crate::db::derived::parse_file::parse_file;
use crate::db::types::derived::object_system::TdStaticType;
use crate::db::types::{File, FileHandle, Project, TdBlobType, TdObjectEnum, TdRuntimeObject};
use crate::syntax::ast::{AstNode, MdBody, SourceFile};
use crate::syntax::red::RedNode;
use crate::syntax::syntax_kind::SyntaxKind;

// Shared fields extracted before body rendering
struct ResourceFields {
  schema: Option<String>,
  label: Option<String>,
  icon: Option<ExportedIcon>,
  seo_metadata: Option<ExportedSeoMeta>,
  header: serde_json::Value,
  metadata: ExportedFileMetadata,
  excerpt: Option<String>,
  body: MdBody,
}

// Fields for binary asset files (no body, schema is always blob type)
struct BlobFields {
  header: serde_json::Value,
  metadata: ExportedFileMetadata,
}

enum ResourceKind {
  Blob(BlobFields),
  Content(ResourceFields),
}

/// Export a content listing entry (no body, includes header)
pub fn export_resource_summary(
  db: &TypedownDatabase,
  project: Project,
  file: File,
) -> Option<ExportedResourceSummary> {
  match extract_resource(db, project, file)? {
    ResourceKind::Blob(_) => None,
    ResourceKind::Content(fields) => Some(ExportedResourceSummary {
      schema: fields.schema,
      label: fields.label,
      icon: fields.icon,
      header: fields.header,
      excerpt: fields.excerpt,
      metadata: fields.metadata,
    }),
  }
}

/// Export a lightweight sidebar nav entry (no header or body)
pub fn export_resource_nav(
  db: &TypedownDatabase,
  project: Project,
  file: File,
) -> Option<ExportedResourceNav> {
  match extract_resource(db, project, file)? {
    ResourceKind::Blob(_) => None,
    ResourceKind::Content(fields) => Some(ExportedResourceNav {
      schema: fields.schema,
      label: fields.label,
      icon: fields.icon,
      metadata: fields.metadata,
      excerpt: fields.excerpt,
    }),
  }
}

/// Export a resource file as structured header and CommonMark body
pub fn export_resource_markdown(
  db: &TypedownDatabase,
  project: Project,
  file: File,
) -> Option<ExportedResourceMarkdown> {
  match extract_resource(db, project, file)? {
    ResourceKind::Blob(blob) => Some(ExportedResourceMarkdown {
      schema: Some(TdBlobType::get(db).display_name(db)),
      label: None,
      icon: None,
      header: blob.header,
      content: String::new(),
      metadata: blob.metadata,
    }),
    ResourceKind::Content(fields) => {
      let content = markdown::export_markdown_body(db, project, file, &fields.body);
      Some(ExportedResourceMarkdown {
        schema: fields.schema,
        label: fields.label,
        icon: fields.icon,
        header: fields.header,
        content,
        metadata: fields.metadata,
      })
    }
  }
}

/// Export a resource file as structured header and HTML body
pub fn export_resource_html(
  db: &TypedownDatabase,
  project: Project,
  file: File,
) -> Option<ExportedResourceHtml> {
  match extract_resource(db, project, file)? {
    ResourceKind::Blob(blob) => Some(ExportedResourceHtml {
      schema: Some(TdBlobType::get(db).display_name(db)),
      label: None,
      icon: None,
      header: blob.header,
      content: String::new(),
      headings: Vec::new(),
      title: None,
      metadata: blob.metadata,
      seo_metadata: None,
    }),
    ResourceKind::Content(fields) => {
      let content = html::export_html_body(db, project, file, &fields.body);
      Some(ExportedResourceHtml {
        schema: fields.schema,
        label: fields.label,
        icon: fields.icon,
        header: fields.header,
        content: content.html,
        headings: content.headings,
        title: content.title,
        metadata: fields.metadata,
        seo_metadata: fields.seo_metadata,
      })
    }
  }
}

// Internal helpers

// Extract the resource kind and fields for a file, returning None for schema files
fn extract_resource(db: &TypedownDatabase, project: Project, file: File) -> Option<ResourceKind> {
  // Skip schema type definitions
  let symbol = file_symbol(db, project, file).value(db)?;
  if symbol.kind(db).is_schema() {
    return None;
  }

  let metadata = export_file_metadata(&file.handle(db));

  // Evaluate the runtime object: body-only files have no frontmatter object
  let obj = evaluate_resource(db, symbol).value(db);

  // Blob files (binary assets) have no body or navigation fields
  if let Some(ref obj) = obj
    && obj.as_td_blob_obj().is_some()
  {
    return Some(ResourceKind::Blob(BlobFields {
      header: json::serialize_to_json(db, project, obj).unwrap_or_default(),
      metadata,
    }));
  }

  let (schema, header, label, icon, seo_metadata) = if let Some(ref obj) = obj {
    let schema = if let Some(schema_obj) = obj.as_td_schema_obj() {
      Some(schema_obj.schema(db).display_name(db))
    } else if obj.as_td_product_obj().is_some() || obj.as_td_dict_obj().is_some() {
      None
    } else {
      return None;
    };

    let mut header = json::serialize_to_json(db, project, obj).unwrap_or_default();
    if let serde_json::Value::Object(ref mut map) = header {
      map.retain(|k, v| !k.starts_with('_') && !v.is_null());
    }

    let label = obj
      .get_builtin_field(db, "_label")
      .and_then(|o| o.as_td_str_obj().map(|s| s.value(db)));
    let icon = obj.get_builtin_field(db, "_icon").and_then(|obj| {
      obj.as_td_icon_obj().map(|icon| ExportedIcon {
        name: icon.lucide_name(db),
      })
    });

    let seo_metadata = export_seo_metadata(db, project, obj);

    (schema, header, label, icon, seo_metadata)
  } else {
    // Body-only file: no frontmatter, all fields default to empty
    (
      None,
      serde_json::Value::Object(Default::default()),
      None,
      None,
      None,
    )
  };

  // Parse the markdown body
  let parse_result = parse_file(db, project, file);
  let root = parse_result.ast(db).node.clone();
  let source_file = SourceFile::cast(root)?;
  let body = source_file.body()?;

  // Prefer description or summary fields from the object, fall back to first body paragraph
  let excerpt = obj
    .as_ref()
    .and_then(|o| {
      o.get_owned_field(db, "description")
        .and_then(|f| f.as_td_str_obj().map(|s| s.value(db)))
        .or_else(|| {
          o.get_owned_field(db, "summary")
            .and_then(|f| f.as_td_str_obj().map(|s| s.value(db)))
        })
        .filter(|s| !s.is_empty())
    })
    .or_else(|| export_body_excerpt(body.syntax()));

  Some(ResourceKind::Content(ResourceFields {
    schema,
    label,
    icon,
    seo_metadata,
    header,
    metadata,
    excerpt,
    body,
  }))
}

fn export_file_metadata(handle: &FileHandle) -> ExportedFileMetadata {
  let meta = handle.metadata();
  ExportedFileMetadata {
    mtime: meta.mtime_epoch_secs(),
    ctime: meta.ctime_epoch_secs(),
  }
}

fn export_seo_metadata(
  db: &TypedownDatabase,
  project: Project,
  obj: &TdObjectEnum,
) -> Option<ExportedSeoMeta> {
  let meta_obj = obj.get_builtin_field(db, "_meta")?;

  if meta_obj.is_td_null_obj() {
    return None;
  }

  let title = meta_obj
    .get_owned_field(db, "title")
    .and_then(|o| o.as_td_str_obj().map(|s| s.value(db)));
  let description = meta_obj
    .get_owned_field(db, "description")
    .and_then(|o| o.as_td_str_obj().map(|s| s.value(db)));
  let image = meta_obj.get_owned_field(db, "image").and_then(|o| {
    o.as_td_str_obj().map(|s| {
      let raw = s.value(db);
      if utils::is_external_url(&raw) || raw.starts_with('/') {
        raw
      } else {
        let config = get_vault_config(db, project);
        utils::prepend_base_path(&config.base_path(db), &raw)
      }
    })
  });

  if title.is_none() && description.is_none() && image.is_none() {
    return None;
  }

  Some(ExportedSeoMeta {
    title,
    description,
    image,
  })
}

fn export_body_excerpt(node: &RedNode) -> Option<String> {
  // Depth-first search for the first MdParagraph in the AST
  fn find_first_paragraph(node: &RedNode) -> Option<RedNode> {
    for child in node.children() {
      if child.kind() == SyntaxKind::MdParagraph {
        return Some(child);
      }
      if let Some(found) = find_first_paragraph(&child) {
        return Some(found);
      }
    }
    None
  }

  find_first_paragraph(node)
    .map(|p| utils::extract_plain_text(&p))
    .filter(|s| !s.is_empty())
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::db::derived::evaluate::evaluate_type::evaluate_type;
  use crate::db::derived::name_resolver::file_symbol::file_symbol;
  use crate::db::fixtures::load_vault_fixture;
  use crate::db::types::{File, FileHandle, FileMetadata, Project};
  use crate::db::{QueryStorage, TypedownDatabase};
  use std::collections::HashMap;
  use std::path::PathBuf;

  #[test]
  fn exports_header_fields() {
    let (db, project, file) = load_vault_fixture("evaluate/my_vault", "valid_person.td");
    let result = export_resource_markdown(&db, project, file);
    let exported = result.expect("should export");
    assert!(
      exported.header.as_object().is_some_and(|m| !m.is_empty()),
      "header should have fields"
    );
    assert_eq!(
      exported.schema,
      Some("Person".to_string()),
      "schema should be Person"
    );
    assert!(
      exported.header.get("_content").is_none(),
      "should not contain _content"
    );
    assert_eq!(
      exported.header["name"],
      serde_json::Value::String("Alice".to_string())
    );
    assert_eq!(exported.header["age"], serde_json::json!(30.0));
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
    assert_eq!(exported.content, "Hello world\n");
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
      content.contains("::: toc\n:::\n"),
      "should expand [[toc]] to empty container: {content}",
    );
    assert!(
      content.contains("::: grid {cols=2}\n:::\n"),
      "should expand [[grid {{cols=2}}]] to container with props: {content}",
    );
    assert!(
      content.contains("::: directory-index\n:::\n"),
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
      content.contains("::: info\n"),
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
        "line has {indent} spaces of indentation (would become code block): '{line}'\nfull content:\n{content}",
      );
    }

    assert!(
      content.contains("nested paragraph\n"),
      "nested paragraph should not have leading whitespace: {content}",
    );
    assert!(
      content.contains("> blockquote content\n"),
      "blockquote should use > prefix: {content}",
    );
    assert!(
      content.contains("- item one\n"),
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
      export_property_descriptors(&db, project, file).expect("WithListField schema should export");
    assert_eq!(props["tags"]["widget"], "list");
    assert_eq!(props["tags"]["items"]["widget"], "text");
    assert_eq!(props["scores"]["widget"], "list");
    assert_eq!(props["scores"]["items"]["widget"], "number");
  }

  #[test]
  fn exports_schema_properties() {
    let (db, project, file) = load_vault_fixture("evaluate/my_vault", "_types/Person.td");
    let props =
      export_property_descriptors(&db, project, file).expect("Person schema should export");
    assert_eq!(props["name"]["widget"], "text");
    assert_eq!(props["age"]["widget"], "number");
  }

  #[test]
  fn exports_schema_select_property() {
    let (db, project, file) = load_vault_fixture("evaluate/my_vault", "_types/Status.td");
    let props =
      export_property_descriptors(&db, project, file).expect("Status schema should export");
    assert_eq!(props["status"]["widget"], "select");
    assert_eq!(
      props["status"]["options"],
      serde_json::json!(["archived", "draft", "published"])
    );
  }

  #[test]
  fn exports_schema_relation_property() {
    let (db, project, file) = load_vault_fixture("evaluate/my_vault", "_types/Event.td");
    let props =
      export_property_descriptors(&db, project, file).expect("Event schema should export");
    assert_eq!(props["title"]["widget"], "text");
    assert_eq!(props["location"]["widget"], "relation");
    assert_eq!(props["location"]["schema"], "Address");
  }

  #[test]
  fn returns_none_for_non_schema() {
    let (db, project, file) = load_vault_fixture("evaluate/my_vault", "valid_person.td");
    let result = export_property_descriptors(&db, project, file);
    assert!(
      result.is_none(),
      "resource file should return None from export_property_descriptors"
    );
  }

  #[test]
  fn exports_asset_as_blob_descriptor() {
    let (db, project, file) = load_vault_fixture("evaluate/my_vault", "icon.svg");
    let exported = export_resource_markdown(&db, project, file).expect("asset should export");
    assert_eq!(exported.schema, Some("blob".to_string()));
    assert_eq!(
      exported.header["format"],
      serde_json::Value::String("svg".to_string())
    );
    assert!(
      exported.header.get("handle").is_some(),
      "should include handle"
    );
    assert_eq!(exported.header["handle"]["type"], "path");
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
      .filter(|(_, l)| l.starts_with('#'))
      .map(|(i, _)| i)
      .collect();
    for &idx in &heading_indices {
      if idx > 0 {
        assert_eq!(
          lines[idx - 1],
          "",
          "heading at line {idx} should be preceded by blank line:\n{}",
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

  // Regression: adding _icon to a schema during dev server hot-reload previously caused a cycle
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
    let project = Project::new(&db, vault.clone(), files);

    // First evaluation, no icon
    let symbol = file_symbol(&db, project, schema_file).value(&db).unwrap();
    let result = evaluate_type(&db, symbol);
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

    // Mutate: add _icon to the schema
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

    // Re-evaluate, should not panic with a query cycle
    let symbol2 = file_symbol(&db, project, schema_file).value(&db).unwrap();
    let result2 = evaluate_type(&db, symbol2);
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
    assert_eq!(exported.content, "<p>Hello world</p>\n");
  }

  #[test]
  fn html_export_headings() {
    let (db, project, file) = load_vault_fixture("evaluate/my_vault", "all_md_elements.td");
    let exported = export_resource_html(&db, project, file).expect("should export");
    eprintln!("{:?}", exported.content);
    assert!(exported.content.contains(
      "<h1 id=\"heading-1\">Heading 1 <a class=\"td-header-anchor\" href=\"#heading-1\""
    ));
    assert!(exported.content.contains(
      "<h2 id=\"heading-2\">Heading 2 <a class=\"td-header-anchor\" href=\"#heading-2\""
    ));
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
    let titles: Vec<&str> = exported.headings.iter().map(|h| h.title.as_str()).collect();
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
      exported.content.contains("href=\"https://example.com\""),
      "heading should contain rendered link:\n{}",
      exported.content
    );
    assert!(
      exported.content.contains("`def1`"),
      "heading should preserve inline code in link:\n{}",
      exported.content
    );
    assert!(
      exported.content.contains("<strong>bold heading</strong>"),
      "heading should contain rendered bold:\n{}",
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
      "<pre class=\"td-code-placeholder\" data-lang=\"python\" data-meta=\"python\"><code>print(&quot;hello&quot;)"
    ));
    assert!(exported.content.contains(
      "<pre class=\"td-code-placeholder\" data-lang=\"js\" data-meta=\"js{1,3}\"><code>const x = 1;"
    ));
  }

  #[test]
  fn html_export_inline_math_placeholder() {
    let (db, project, file) = load_vault_fixture("evaluate/my_vault", "all_md_elements.td");
    let exported = export_resource_html(&db, project, file).expect("should export");
    assert!(
      exported
        .content
        .contains("<span class=\"td-math-inline\">E = mc^2</span>")
    );
  }

  #[test]
  fn html_export_math_block_placeholder() {
    let (db, project, file) = load_vault_fixture("evaluate/my_vault", "all_md_elements.td");
    let exported = export_resource_html(&db, project, file).expect("should export");
    assert!(
      exported
        .content
        .contains("<div class=\"td-math-block\">\\int_0^\\infty e^{-x^2} dx")
    );
  }

  #[test]
  fn html_export_blockquote() {
    let (db, project, file) = load_vault_fixture("evaluate/my_vault", "all_md_elements.td");
    let exported = export_resource_html(&db, project, file).expect("should export");
    assert!(
      exported
        .content
        .contains("<blockquote>\n<p>blockquote line</p>\n</blockquote>")
    );
  }

  #[test]
  fn html_export_table() {
    let (db, project, file) = load_vault_fixture("evaluate/my_vault", "all_md_elements.td");
    let exported = export_resource_html(&db, project, file).expect("should export");
    assert!(exported.content.contains("<table tabindex=\"0\">"));
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
    assert!(exported.content.contains("<ul>\n<li>"));
    assert!(exported.content.contains("bullet one"));
    assert!(exported.content.contains("bullet two"));
  }

  #[test]
  fn html_export_ordered_list() {
    let (db, project, file) = load_vault_fixture("evaluate/my_vault", "all_md_elements.td");
    let exported = export_resource_html(&db, project, file).expect("should export");
    assert!(exported.content.contains("<ol>\n<li>"));
    assert!(exported.content.contains("ordered one"));
  }

  #[test]
  fn html_export_callout_container() {
    let (db, project, file) = load_vault_fixture("evaluate/my_vault", "all_md_elements.td");
    let exported = export_resource_html(&db, project, file).expect("should export");
    assert!(exported.content.contains(
      "<div class=\"note td-callout\"><p class=\"td-callout-title td-callout-title-default\">NOTE</p>"
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
        .contains("<pre class=\"td-code-placeholder\" data-lang=\"python\" data-meta=\"python\"><code>print(&quot;in callout&quot;)")
    );
  }

  #[test]
  fn html_export_details_container() {
    let (db, project, file) = load_vault_fixture("evaluate/my_vault", "all_md_elements.td");
    let exported = export_resource_html(&db, project, file).expect("should export");
    assert!(
      exported
        .content
        .contains("<details class=\"details td-callout\"><summary>Click to expand</summary>")
    );
    assert!(exported.content.contains("<p>details content</p>"));
  }

  #[test]
  fn html_export_external_link() {
    let (db, project, file) = load_vault_fixture("evaluate/my_vault", "all_md_elements.td");
    let exported = export_resource_html(&db, project, file).expect("should export");
    assert!(exported.content.contains(
      "<LucideIcon name=\"arrow-up-right\" class=\"td-external-link-icon\" /><a href=\"https://example.com\" class=\"td-external-link\" target=\"_blank\" rel=\"noopener noreferrer\">link text</a>"
    ));
  }

  #[test]
  fn html_export_container_shorthand() {
    let (db, project, file) = load_vault_fixture("evaluate/my_vault", "md_container_shorthand.td");
    let exported = export_resource_html(&db, project, file).expect("should export");
    assert!(exported.content.contains("<toc />\n"));
    assert!(exported.content.contains("<directory-index />\n"));
    assert!(exported.content.contains("<grid cols=\"2\" />\n"));
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
    assert_eq!(exported.header["name"], "Alice");
    assert_eq!(exported.header["age"], 30.0);
  }

  #[test]
  fn html_export_fref_link() {
    let (db, project, file) = load_vault_fixture("evaluate/base_path_vault", "with_fref.td");
    let exported = export_resource_html(&db, project, file).expect("should export");
    assert!(
      exported
        .content
        .contains("<a href=\"/blog/alice\" class=\"td-fref-link\">Alice</a>")
    );
  }

  #[test]
  fn html_export_fref_image() {
    let (db, project, file) = load_vault_fixture("evaluate/my_vault", "with_asset_fref.td");
    let exported = export_resource_html(&db, project, file).expect("should export");
    assert!(
      exported
        .content
        .contains("<img src=\"/icon.svg\" alt=\"icon\" loading=\"lazy\">")
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
    assert!(!exported.content.contains("\n> "));
    assert!(!exported.content.contains("\n- "));
    assert!(!exported.content.contains("\n1. "));
    assert!(!exported.content.contains("**"));
    assert!(!exported.content.contains("~~"));
  }

  #[test]
  fn html_export_heading_slugs_unique() {
    let (db, project, file) = load_vault_fixture("evaluate/my_vault", "all_md_elements.td");
    let exported = export_resource_html(&db, project, file).expect("should export");
    let slugs: Vec<&str> = exported.headings.iter().map(|h| h.slug.as_str()).collect();
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
    assert_eq!(exported.header, serde_json::json!({}));
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
      content.contains("<a href=\"mailto:huydo862003@gmail.com\""),
      "should contain mailto link: {content}",
    );
    assert!(
      content.contains(
        "<img src=\"https://img.shields.io/badge/Email-blue\" alt=\"Email badge\" loading=\"lazy\">"
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
      exported.content.contains("href=\"https://example.com\""),
      "external URL should pass through unchanged: {}",
      exported.content
    );
  }

  #[test]
  fn html_export_preserves_anchor_url() {
    let (db, project, file) = load_vault_fixture("evaluate/my_vault", "url_resolution.td");
    let exported = export_resource_html(&db, project, file).expect("should export");
    assert!(
      exported.content.contains("href=\"#section\""),
      "anchor URL should pass through unchanged: {}",
      exported.content
    );
  }

  #[test]
  fn html_export_resolves_relative_link() {
    let (db, project, file) = load_vault_fixture("evaluate/my_vault", "url_resolution.td");
    let exported = export_resource_html(&db, project, file).expect("should export");
    assert!(
      exported.content.contains("href=\"/other.td\""),
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
}
