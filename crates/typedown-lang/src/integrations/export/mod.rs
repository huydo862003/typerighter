//! Export typedown resources

pub mod json;

use typedown_types::either::Either;

use crate::db::TypedownDatabase;
use crate::db::derived::evaluate::evaluate_node::evaluate_node;
use crate::db::derived::evaluate::evaluate_resource::evaluate_resource;
use crate::db::derived::evaluate::evaluate_type::evaluate_type;
use crate::db::derived::get_builtin_types::get_schemaless_type;
use crate::db::derived::get_vault_config::get_vault_config;
use crate::db::derived::hir::lower_node;
use crate::db::derived::name_resolver::file_symbol::file_symbol;
use crate::db::derived::name_resolver::referee::referee;
use crate::db::derived::parse_file::parse_file;
use crate::db::types::{
  File, FileHandle, HirValue, LiteralValue, MemberType, Project, Symbol, SymbolKind, TdBlobType,
  TdObjectEnum, TdObjectLike, TdTypeEnum, TdTypeLike, TypeMemberDescriptors,
};
use crate::db::utils::strip_content_extension;

use crate::syntax::ast::{AstNode, InterpFragment, MdBody, MdToggleList, SourceFile};
use crate::syntax::red::RedNode;
use crate::syntax::syntax_kind::SyntaxKind;

/// Structured export result
#[derive(serde::Serialize)]
pub struct ExportedResource {
  /// Schema type name
  #[serde(skip_serializing_if = "Option::is_none")]
  pub schema: Option<String>,
  /// Frontmatter fields as a JSON object
  pub header: serde_json::Value,
  /// Commonmark-compatible markdown body
  pub content: String,
  /// File metadata
  pub metadata: ExportedMetadata,
}

/// File metadata exported alongside a resource
#[derive(serde::Serialize, Clone)]
pub struct ExportedMetadata {
  /// Last modification time as seconds since UNIX epoch
  pub mtime: u64,
  /// Creation time as seconds since UNIX epoch
  pub ctime: u64,
}

/// Export a resource file as structured header and commonmark content
pub fn export_resource(
  db: &TypedownDatabase,
  project: Project,
  file: File,
) -> Option<ExportedResource> {
  let symbol = file_symbol(db, project, file).value(db)?;
  let obj = evaluate_resource(db, symbol).value(db)?;
  let metadata = export_metadata(file.handle(db));

  // Assets export as a blob descriptor with no body
  if obj.as_td_blob_obj().is_some() {
    return Some(ExportedResource {
      schema: Some(TdBlobType::get(db).display_name(db)),
      header: json::to_json(db, project, &obj).unwrap_or_default(),
      content: String::new(),
      metadata,
    });
  }

  let product = obj.as_td_product_obj()?;
  let schema_type = product.schema(db);
  let schemaless: TdTypeEnum = get_schemaless_type(db).into();
  let schema = if schema_type == schemaless {
    None
  } else {
    Some(schema_type.display_name(db))
  };
  let mut header = json::to_json(db, project, &obj).unwrap_or_default();
  // _content is available in ExportedResource.content, not the header
  if let serde_json::Value::Object(ref mut map) = header {
    map.remove("_content");
  }

  // Walk the AST and translate to somewhat commonmark-conformant markdown
  let parse_result = parse_file(db, project, file);
  let root = parse_result.ast(db);
  let source_file = SourceFile::cast(root)?;
  let body = source_file.body()?;
  let content = export_markdown_body(db, project, file, &body);

  Some(ExportedResource {
    schema,
    header,
    content,
    metadata,
  })
}

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub enum Widget {
  Text,
  Number,
  Checkbox,
  Date,
  Select,
  MultiSelect,
  Relation,
  List,
}

/// Export schema property descriptors as structured JSON for the client
pub fn export_property_descriptors(
  db: &TypedownDatabase,
  project: Project,
  file: File,
) -> Option<serde_json::Value> {
  let symbol = file_symbol(db, project, file).value(db)?;
  let typ = evaluate_type(db, symbol).typ(db)?;

  let product = typ.as_td_product_type()?;
  let fields = product.fields(db);

  let mut properties = serde_json::Map::new();

  for (name, member) in &fields {
    let mut prop = member_to_descriptor(db, &member.typ(db));

    if member
      .descriptors(db)
      .contains(TypeMemberDescriptors::OPTIONAL)
    {
      prop["optional"] = serde_json::Value::Bool(true);
    }

    properties.insert(name.clone(), prop);
  }

  return Some(serde_json::Value::Object(properties));

  // Map a MemberType to a property descriptor with a widget hint
  fn member_to_descriptor(db: &TypedownDatabase, member: &MemberType) -> serde_json::Value {
    match member {
      MemberType::Simple(typ) => simple_type_to_descriptor(db, typ),

      // Sum of string literals is a select (single value from options)
      MemberType::Sum(members) => {
        let literals: Vec<String> = members
          .iter()
          .filter_map(|m| match m.typ(db) {
            MemberType::Literal(LiteralValue::Str(s)) => Some(s),
            _ => None,
          })
          .collect();

        if literals.len() == members.len() {
          serde_json::json!({ "widget": Widget::Select, "options": literals })
        } else {
          serde_json::json!({ "widget": Widget::Text })
        }
      }

      MemberType::Literal(LiteralValue::Str(s)) => {
        serde_json::json!({ "widget": Widget::Select, "options": [s] })
      }

      // List of literals is a multi_select (multiple values from options)
      MemberType::ListOfSum(members) => {
        let literals: Vec<String> = members
          .iter()
          .filter_map(|m| match m.typ(db) {
            MemberType::Literal(LiteralValue::Str(s)) => Some(s),
            _ => None,
          })
          .collect();

        if literals.len() == members.len() && !literals.is_empty() {
          serde_json::json!({ "widget": Widget::MultiSelect, "options": literals })
        } else if members.len() == 1 {
          let inner = member_to_descriptor(db, &members[0].typ(db));
          serde_json::json!({ "widget": Widget::List, "items": inner })
        } else {
          serde_json::json!({ "widget": Widget::Text })
        }
      }

      _ => serde_json::json!({ "widget": Widget::Text }),
    }
  }

  fn simple_type_to_descriptor(db: &TypedownDatabase, typ: &TdTypeEnum) -> serde_json::Value {
    match typ {
      TdTypeEnum::TdStrType(_) => serde_json::json!({ "widget": Widget::Text }),
      TdTypeEnum::TdNumType(_) => serde_json::json!({ "widget": Widget::Number }),
      TdTypeEnum::TdBoolType(_) => serde_json::json!({ "widget": Widget::Checkbox }),
      TdTypeEnum::TdDateType(_) => serde_json::json!({ "widget": Widget::Date }),
      TdTypeEnum::TdDateTimeType(_) => serde_json::json!({ "widget": Widget::Date }),
      TdTypeEnum::TdTimeType(_) => serde_json::json!({ "widget": Widget::Text }),
      TdTypeEnum::TdListType(list) => match list.elem(db) {
        Some(elem) => {
          let inner = simple_type_to_descriptor(db, &elem);
          serde_json::json!({ "widget": Widget::List, "items": inner })
        }
        None => serde_json::json!({ "widget": Widget::List }),
      },
      TdTypeEnum::TdProductType(product) => {
        if let Some(name) = product.name(db) {
          serde_json::json!({ "widget": Widget::Relation, "schema": name })
        } else {
          serde_json::json!({ "widget": Widget::Text })
        }
      }
      _ => serde_json::json!({ "widget": Widget::Text }),
    }
  }
}

fn export_metadata(handle: FileHandle) -> ExportedMetadata {
  let meta = handle.metadata();
  ExportedMetadata {
    mtime: meta.mtime_epoch_secs(),
    ctime: meta.ctime_epoch_secs(),
  }
}

fn export_markdown_body(
  db: &TypedownDatabase,
  project: Project,
  file: File,
  body: &MdBody,
) -> String {
  let mut out = String::new();

  // Separate block elements with blank lines for CommonMark
  let mut first = true;
  for child in body.syntax().children() {
    let kind = child.kind();
    // Skip whitespace/newline tokens between blocks
    if kind == SyntaxKind::Whitespace || kind == SyntaxKind::Newline {
      continue;
    }
    if !first {
      out.push('\n');
    }
    first = false;
    emit_md_block(db, project, file, &child, &mut out);
  }

  if !out.ends_with('\n') {
    out.push('\n');
  }
  out
}

fn emit_md_block(
  db: &TypedownDatabase,
  project: Project,
  file: File,
  node: &RedNode,
  out: &mut String,
) {
  match node.kind() {
    SyntaxKind::MdToggleList => emit_md_toggle_list(db, project, file, node, out),
    _ => emit_md_node(db, project, file, node, out),
  }
}

/// Emit a toggle list as HTML <details><summary>...</summary>...</details>
fn emit_md_toggle_list(
  db: &TypedownDatabase,
  project: Project,
  file: File,
  node: &RedNode,
  out: &mut String,
) {
  let Some(list) = MdToggleList::cast(node.clone()) else {
    return;
  };

  for item in list.items() {
    out.push_str("<details>\n");

    if let Some(summary) = item.summary() {
      out.push_str("<summary>");
      emit_md_node(db, project, file, summary.syntax(), out);
      out.push_str("</summary>\n");
    }

    if let Some(details) = item.details() {
      for block in details.block_elements() {
        let mut block_html = String::new();
        emit_md_block(db, project, file, block.syntax(), &mut block_html);
        let trimmed = block_html.trim_end_matches('\n');
        let is_block_element = trimmed.starts_with('<');
        if is_block_element {
          out.push_str(trimmed);
        } else {
          out.push_str("<div>");
          out.push_str(trimmed);
          out.push_str("</div>");
        }
        out.push('\n');
      }
    }

    out.push_str("</details>\n");
  }
}

/// Emit a node, translating fref interpolations to markdown links
fn emit_md_node(
  db: &TypedownDatabase,
  project: Project,
  file: File,
  node: &RedNode,
  out: &mut String,
) {
  // Leaf token
  if node.as_token().is_some() {
    out.push_str(&node.text());
    return;
  }

  // Interpolation fragment
  if node.kind() == SyntaxKind::InterpFragment {
    let Some(fragment) = InterpFragment::cast(node.clone()) else {
      return;
    };
    let Some(expr) = fragment.expr() else { return };
    let expr_node = expr.syntax().clone();

    // Try to resolve as fref link
    if let Some(link) = try_resolve_fref(db, project, file, &expr_node) {
      out.push_str(&link);
      return;
    }
    // Not a fref: Evaluate and call to_string on the result
    let hir = lower_node(db, project, file, expr_node);
    if let Some(obj) = evaluate_node(db, hir).value(db)
      && let Some(func) = obj.lookup_method(db, "to_string")
    {
      let native_fn = func.func(db).resolve();
      if let Some(result) = native_fn(db, obj, vec![])
        && let Some(str_obj) = result.as_td_str_obj()
      {
        out.push_str(&str_obj.value(db));
      }
    }
    return;
  }

  // Composite node: Recurse into children
  for child in node.children() {
    emit_md_node(db, project, file, &child, out);
  }
}

/// Resolved reference: display name and URL
pub struct ResolvedRef {
  pub name: String,
  pub url: String,
}

/// Resolve a symbol to a display name and URL
pub fn resolve_ref(
  db: &TypedownDatabase,
  project: Project,
  symbol: &Symbol,
) -> Option<ResolvedRef> {
  let name = resolve_display_name(db, project, symbol);

  match symbol.kind(db) {
    SymbolKind::UserDefinedResource(_, target_file)
    | SymbolKind::UserDefinedSchema(_, target_file) => {
      let handle = target_file.handle(db);
      let path = handle.path()?;
      let config = get_vault_config(db, project);
      let content_dir = config.content_dir(db);
      let base_path = config.base_path(db);
      let relative = path.strip_prefix(&content_dir).unwrap_or(path);
      let path_str = relative.to_string_lossy();
      let without_ext = strip_content_extension(&path_str);
      let url = if base_path == "/" {
        format!("/{without_ext}")
      } else {
        format!("{base_path}/{without_ext}")
      };
      Some(ResolvedRef { name, url })
    }
    SymbolKind::Asset(_, _, target_file) => {
      let handle = target_file.handle(db);
      let path = handle.path()?;
      let config = get_vault_config(db, project);
      let content_dir = config.content_dir(db);
      let base_path = config.base_path(db);
      let relative = path.strip_prefix(&content_dir).unwrap_or(path);
      let path_str = relative.to_string_lossy();
      let url = if base_path == "/" {
        format!("/{path_str}")
      } else {
        format!("{base_path}/{path_str}")
      };
      Some(ResolvedRef { name, url })
    }
    _ => None,
  }
}

/// Resolve a fref interpolation to a markdown link
fn try_resolve_fref(
  db: &TypedownDatabase,
  project: Project,
  file: File,
  node: &RedNode,
) -> Option<String> {
  let hir = lower_node(db, project, file, node.clone());
  let target_symbol = referee(db, hir).value(db)?;
  let resolved = resolve_ref(db, project, &target_symbol)?;

  // Images produce a markdown image, other assets produce a link
  if let SymbolKind::Asset(asset_kind, _, _) = target_symbol.kind(db)
    && asset_kind.is_image()
  {
    return Some(format!("![{}]({})", resolved.name, resolved.url));
  }

  Some(format!("[{}]({})", resolved.name, resolved.url))
}

/// Get a display name for a symbol: Try _label, then name field, then file stem
fn resolve_display_name(db: &TypedownDatabase, project: Project, symbol: &Symbol) -> String {
  let kind = symbol.kind(db);

  // Try _label or name from the evaluated resource
  if let SymbolKind::UserDefinedResource(_, target_file) = &kind
    && let Some(target_symbol) = file_symbol(db, project, *target_file).value(db)
    && let Some(obj) = evaluate_resource(db, target_symbol).value(db)
  {
    let label_or_name = obj
      .get_owned_field(db, "_label")
      .or_else(|| obj.get_owned_field(db, "name"));
    if let Some(str_obj) = label_or_name
      .as_ref()
      .and_then(|field| field.as_td_str_obj())
    {
      return str_obj.value(db);
    }
  }

  // Fallback: File stem
  match &kind {
    SymbolKind::UserDefinedResource(_, target_file)
    | SymbolKind::UserDefinedSchema(_, target_file) => target_file
      .handle(db)
      .path()
      .and_then(|path| path.file_stem())
      .and_then(|stem| stem.to_str())
      .unwrap_or("unknown")
      .to_string(),
    _ => symbol.name(db).to_string(),
  }
}

pub(super) fn evaluate_lazy_field(
  db: &TypedownDatabase,
  field: Either<HirValue, TdObjectEnum>,
) -> Option<TdObjectEnum> {
  match field {
    Either::Right(obj) => Some(obj),
    Either::Left(hir) => evaluate_node(db, hir).value(db),
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::db::fixtures::load_vault_fixture;

  #[test]
  fn exports_header_fields() {
    let (db, project, file) = load_vault_fixture("evaluate/my_vault", "content/valid_person.td");
    let result = export_resource(&db, project, file);
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
    let (db, project, file) = load_vault_fixture("evaluate/my_vault", "content/md_with_content.td");
    let result = export_resource(&db, project, file);
    let exported = result.expect("should export");
    assert!(
      exported.content.contains("Hello world"),
      "content should contain body text: {}",
      exported.content
    );
  }

  #[test]
  fn exports_markdown_body() {
    let (db, project, file) = load_vault_fixture("evaluate/my_vault", "content/md_with_content.td");
    let exported = export_resource(&db, project, file).expect("should export");
    assert_eq!(exported.content, "Hello world\n");
  }

  #[test]
  fn exports_all_markdown_elements() {
    let (db, project, file) = load_vault_fixture("evaluate/my_vault", "content/all_md_elements.td");
    let exported = export_resource(&db, project, file).expect("should export");
    let content = &exported.content;
    // Verify key elements are present in the exported content
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
  fn exports_container_with_title() {
    let (db, project, file) = load_vault_fixture("evaluate/my_vault", "content/all_md_elements.td");
    let exported = export_resource(&db, project, file).expect("should export");
    assert!(
      exported.content.contains("::: details Click to expand"),
      "should contain container with title: {}",
      exported.content,
    );
  }

  #[test]
  fn returns_none_for_schema() {
    let (db, project, file) = load_vault_fixture("evaluate/my_vault", "schemas/Person.td");
    let result = export_resource(&db, project, file);
    assert!(result.is_none(), "schema should return None");
  }

  #[test]
  fn exports_list_field_schema() {
    let (db, project, file) = load_vault_fixture("evaluate/my_vault", "schemas/WithListField.td");
    let props =
      export_property_descriptors(&db, project, file).expect("WithListField schema should export");
    assert_eq!(props["tags"]["widget"], "list");
    assert_eq!(props["tags"]["items"]["widget"], "text");
    assert_eq!(props["scores"]["widget"], "list");
    assert_eq!(props["scores"]["items"]["widget"], "number");
  }

  #[test]
  fn exports_schema_properties() {
    let (db, project, file) = load_vault_fixture("evaluate/my_vault", "schemas/Person.td");
    let props =
      export_property_descriptors(&db, project, file).expect("Person schema should export");
    assert_eq!(props["name"]["widget"], "text");
    assert_eq!(props["age"]["widget"], "number");
  }

  #[test]
  fn exports_schema_select_property() {
    let (db, project, file) = load_vault_fixture("evaluate/my_vault", "schemas/Status.td");
    let props =
      export_property_descriptors(&db, project, file).expect("Status schema should export");
    assert_eq!(props["status"]["widget"], "select");
    assert_eq!(
      props["status"]["options"],
      serde_json::json!(["draft", "published", "archived"])
    );
  }

  #[test]
  fn exports_schema_relation_property() {
    let (db, project, file) = load_vault_fixture("evaluate/my_vault", "schemas/Event.td");
    let props =
      export_property_descriptors(&db, project, file).expect("Event schema should export");
    assert_eq!(props["title"]["widget"], "text");
    assert_eq!(props["location"]["widget"], "relation");
    assert_eq!(props["location"]["schema"], "Address");
  }

  #[test]
  fn returns_none_for_non_schema() {
    let (db, project, file) = load_vault_fixture("evaluate/my_vault", "content/valid_person.td");
    let result = export_property_descriptors(&db, project, file);
    assert!(
      result.is_none(),
      "resource file should return None from export_property_descriptors"
    );
  }

  #[test]
  fn exports_asset_as_blob_descriptor() {
    let (db, project, file) = load_vault_fixture("evaluate/my_vault", "content/icon.svg");
    let exported = export_resource(&db, project, file).expect("asset should export");
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

  // fref links use build.base_path from typedown.yaml
  #[test]
  fn fref_uses_base_path() {
    let (db, project, file) =
      load_vault_fixture("evaluate/base_path_vault", "content/with_fref.td");
    let exported = export_resource(&db, project, file).expect("should export");
    assert!(
      exported.content.contains("[Alice](/blog/alice)"),
      "fref should use base_path /blog: {}",
      exported.content
    );
  }

  #[test]
  fn fref_resolves_image_asset_to_markdown_image() {
    let (db, project, file) = load_vault_fixture("evaluate/my_vault", "content/with_asset_fref.td");
    let exported = export_resource(&db, project, file).expect("should export");
    assert!(
      exported.content.contains("![icon](/icon.svg)"),
      "image asset fref should produce markdown image: {}",
      exported.content
    );
  }

  // Default base_path is /
  #[test]
  fn fref_default_base_path() {
    let (db, project, file) = load_vault_fixture("evaluate/my_vault", "content/with_fref.td");
    let exported = export_resource(&db, project, file).expect("should export");
    assert!(
      !exported.content.contains("/blog"),
      "default base_path should not have /blog prefix: {}",
      exported.content
    );
  }

  #[test]
  fn export_separates_blocks_with_blank_lines() {
    let (db, project, file) = load_vault_fixture("evaluate/my_vault", "content/md_with_content.td");
    let exported = export_resource(&db, project, file).expect("should export");
    // Block elements should be separated by blank lines
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
  fn exports_toggle_list_as_details() {
    let (db, project, file) = load_vault_fixture("evaluate/my_vault", "content/all_md_elements.td");
    let exported = export_resource(&db, project, file).expect("should export");
    let content = &exported.content;
    // Toggle list should produce a self-contained HTML block with no blank lines
    let expected = r#"<details>
<summary>Toggle summary</summary>
<div>Toggle details content</div>
</details>
"#;
    assert!(
      content.contains(expected),
      "toggle list should emit:\n{expected}\ngot:\n{content}"
    );
  }

  #[test]
  fn exports_schemaless_file() {
    let (db, project, file) = load_vault_fixture("evaluate/my_vault", "content/schemaless.td");
    let result = export_resource(&db, project, file);
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
}
