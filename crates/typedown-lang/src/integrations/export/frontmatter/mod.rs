//! Extract frontmatter (header, schema, label, icon, seo, excerpt) from resource files

use crate::db::TypedownDatabase;
use crate::db::derived::evaluate::evaluate_resource::evaluate_resource;
use crate::db::derived::get_vault_config::get_vault_config;
use crate::db::derived::parse_file::parse_file;
use crate::db::types::derived::obj_system::TdStaticTyp;
use crate::db::types::{File, FileHandle, Project, TdObjEnum, TdRuntimeObj};
use crate::db::utils::is_external_url;
use crate::db::utils::prepend_base_path;
use crate::integrations::bindings::serialize::serialize_value;
use crate::integrations::export::types::*;
use crate::integrations::export::utils::extract_plain_text;
use crate::syntax::ast::{AstNode, MdBody, SourceFile};
use crate::syntax::red::RedNode;
use crate::syntax::syntax_kind::SyntaxKind;

pub use crate::db::derived::name_resolver::file_symbol::file_symbol;

// Shared fields extracted before body rendering
pub(crate) struct ResourceFields {
  pub schema: Option<String>,
  pub label: Option<String>,
  pub icon: Option<ExportedIcon>,
  pub seo_metadata: Option<ExportedSeoMeta>,
  pub header: serde_json::Value,
  pub metadata: ExportedFileMetadata,
  pub excerpt: Option<String>,
  pub body: MdBody,
}

// Fields for binary asset files (no body, schema is always blob type)
pub(crate) struct BlobFields {
  pub header: serde_json::Value,
  pub metadata: ExportedFileMetadata,
}

pub(crate) enum ResourceKind {
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

/// Extract the resource kind and fields for a file, returning None for schema files.
pub(crate) fn extract_resource(
  db: &TypedownDatabase,
  project: Project,
  file: File,
) -> Option<ResourceKind> {
  let symbol = file_symbol(db, project, file).value(db)?;
  if symbol.kind(db).is_schema() {
    return None;
  }

  let metadata = export_file_metadata(&file.handle(db));

  let obj = evaluate_resource(db, symbol).value(db);

  if let Some(ref obj) = obj
    && obj.as_td_blob_obj().is_some()
  {
    return Some(ResourceKind::Blob(BlobFields {
      header: serialize_value(db, obj),
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

    let mut header = serialize_value(db, obj);
    // Strip _-prefixed keys and nulls from fields
    if let serde_json::Value::Object(ref mut map) = header {
      if let Some(serde_json::Value::Object(fields)) = map.get_mut("fields") {
        fields.retain(|key, value| !key.starts_with('_') && !value.is_null());
      } else {
        map.retain(|key, value| !key.starts_with('_') && !value.is_null());
      }
    }

    let label = obj
      .get_builtin_field(db, "_label")
      .and_then(|obj| obj.as_td_string_obj().map(|string| string.value(db)));
    let icon = obj.get_builtin_field(db, "_icon").and_then(|obj| {
      obj.as_td_icon_obj().map(|icon| ExportedIcon {
        name: icon.lucide_name(db),
      })
    });

    let seo_metadata = export_seo_metadata(db, project, obj);

    (schema, header, label, icon, seo_metadata)
  } else {
    (
      None,
      serde_json::Value::Object(Default::default()),
      None,
      None,
      None,
    )
  };

  let parse_result = parse_file(db, project, file);
  let root = parse_result.ast(db).node.clone();
  let source_file = SourceFile::cast(root)?;
  let body = source_file.body()?;

  let excerpt = obj
    .as_ref()
    .and_then(|obj| {
      obj
        .get_owned_field(db, "description")
        .and_then(|field| field.as_td_string_obj().map(|string| string.value(db)))
        .or_else(|| {
          obj
            .get_owned_field(db, "summary")
            .and_then(|field| field.as_td_string_obj().map(|string| string.value(db)))
        })
        .filter(|string| !string.is_empty())
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

pub(crate) fn export_file_metadata(handle: &FileHandle) -> ExportedFileMetadata {
  let meta = handle.metadata();
  ExportedFileMetadata {
    mtime: meta.mtime_epoch_secs(),
    ctime: meta.ctime_epoch_secs(),
  }
}

fn export_seo_metadata(
  db: &TypedownDatabase,
  project: Project,
  obj: &TdObjEnum,
) -> Option<ExportedSeoMeta> {
  let meta_obj = obj.get_builtin_field(db, "_meta")?;

  if meta_obj.is_td_null_obj() {
    return None;
  }

  let title = meta_obj
    .get_owned_field(db, "title")
    .and_then(|obj| obj.as_td_string_obj().map(|string| string.value(db)));
  let description = meta_obj
    .get_owned_field(db, "description")
    .and_then(|obj| obj.as_td_string_obj().map(|string| string.value(db)));
  let image = meta_obj.get_owned_field(db, "image").and_then(|obj| {
    obj.as_td_string_obj().map(|string| {
      let raw = string.value(db);
      if is_external_url(&raw) || raw.starts_with('/') {
        raw
      } else {
        let config = get_vault_config(db, project);
        prepend_base_path(&config.base_path(db), &raw)
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
    .map(|paragraph| extract_plain_text(&paragraph))
    .filter(|string| !string.is_empty())
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::db::derived::evaluate::evaluate_typ::evaluate_typ;
  use crate::db::derived::name_resolver::file_symbol::file_symbol;
  use crate::db::fixtures::load_vault_fixture;
  use crate::db::types::{File, FileHandle, FileMetadata, Project, TdRuntimeObj};
  use crate::db::{QueryStorage, TypedownDatabase};
  use crate::integrations::bindings::file_ref::resolve_schema_label;
  use crate::integrations::export::content::{export_resource_html, export_resource_markdown};
  use std::collections::HashMap;
  use std::path::PathBuf;

  #[test]
  fn exports_header_fields() {
    let (db, project, file) = load_vault_fixture("evaluate/my_vault", "valid_person.td");
    let exported = export_resource_markdown(&db, project, file).expect("should export");
    assert_eq!(
      exported.schema,
      Some("Person".to_string()),
      "schema should be Person"
    );
    assert_eq!(
      exported.header["$kind"], "schema",
      "header should be $kind-tagged"
    );
    let fields = exported
      .header
      .get("fields")
      .and_then(|val| val.as_object())
      .expect("header should have fields object");
    assert!(!fields.is_empty(), "fields should not be empty");
    assert_eq!(fields["name"], "Alice");
    assert_eq!(fields["age"], 30.0);
    assert!(
      !fields.contains_key("_content"),
      "_content should be stripped from fields"
    );
  }

  #[test]
  fn returns_none_for_schema() {
    let (db, project, file) = load_vault_fixture("evaluate/my_vault", "_types/Person.td");
    let result = export_resource_markdown(&db, project, file);
    assert!(result.is_none(), "schema should return None");
  }

  #[test]
  fn exports_asset_as_blob_descriptor() {
    let (db, project, file) = load_vault_fixture("evaluate/my_vault", "icon.svg");
    let exported = export_resource_markdown(&db, project, file).expect("asset should export");
    assert_eq!(exported.schema, Some("blob".to_string()));
    assert!(exported.content.is_empty(), "asset has no markdown body");
  }

  #[test]
  fn exports_schemaless_file() {
    let (db, project, file) = load_vault_fixture("evaluate/my_vault", "schemaless.td");
    let exported =
      export_resource_markdown(&db, project, file).expect("schemaless file should export");
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
  fn exports_fref_list_in_header() {
    let (db, project, file) = load_vault_fixture("evaluate/my_vault", "book_with_frefs.td");
    let exported = export_resource_html(&db, project, file).expect("should export");
    let header = &exported.header;
    assert_eq!(
      header["$kind"], "schema",
      "header should be $kind-tagged schema"
    );
    let fields = header
      .get("fields")
      .and_then(|val| val.as_object())
      .expect("header should have fields");

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
    let fields = header
      .get("fields")
      .and_then(|val| val.as_object())
      .expect("header should have fields");

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

  #[test]
  fn extracts_seo_metadata() {
    let (db, project, file) = load_vault_fixture("evaluate/my_vault", "with_meta.td");
    let exported = export_resource_html(&db, project, file).expect("should export");
    let seo = exported.seo_metadata.expect("_meta should be extracted");
    assert_eq!(seo.title.as_deref(), Some("Custom SEO Title"));
    assert_eq!(seo.description.as_deref(), Some("Custom SEO description"));
    assert_eq!(seo.image.as_deref(), Some("/images/og.png"));
  }

  #[test]
  fn no_seo_metadata_when_absent() {
    let (db, project, file) = load_vault_fixture("evaluate/my_vault", "valid_person.td");
    let exported = export_resource_html(&db, project, file).expect("should export");
    assert!(
      exported.seo_metadata.is_none(),
      "seo_metadata should be None when not set"
    );
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
}
