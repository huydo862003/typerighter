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
use crate::db::derived::name_resolver::scope::get_file_runtime_scope;
use crate::db::derived::parse_file::parse_file;
use crate::db::types::derived::object_system::TdStaticType;
use crate::db::types::{
  File, FileHandle, HirValue, LazyType, LiteralValue, Project, Symbol, SymbolKind, TdBlobType,
  TdObjectEnum, TdRuntimeObject, TdTypeEnum,
};
use crate::db::utils::strip_content_extension;

use crate::syntax::ast::{AstNode, InterpFragment, MdBody, SourceFile};
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

  let (schema_type, header_obj) = if let Some(schema_obj) = obj.as_td_schema_obj() {
    (schema_obj.schema(db), obj.clone())
  } else if let Some(product) = obj.as_td_product_obj() {
    (product.product_type(db), obj.clone())
  } else if obj.as_td_dict_obj().is_some() {
    // Schemaless files may evaluate to a DictObj
    (get_schemaless_type(db).into(), obj.clone())
  } else {
    return None;
  };
  let _ = &header_obj; // suppress unused warning
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
    map.retain(|_, v| !v.is_null());
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

  let schema = typ.as_td_schema_type()?;
  let fields = schema.fields(db);

  let mut properties = serde_json::Map::new();

  for (name, prop_desc) in fields {
    let mut prop_json = lazy_to_descriptor(db, &prop_desc.field_type);
    if let Some(ref def_obj) = prop_desc.default_value
      && let Ok(def_json) = json::to_json(db, project, def_obj)
      && let serde_json::Value::Object(ref mut map) = prop_json
    {
      map.insert("default".to_string(), def_json);
    }
    properties.insert(name, prop_json);
  }

  return Some(serde_json::Value::Object(properties));

  // Map a LazyType to a property descriptor with a widget hint
  fn lazy_to_descriptor(db: &TypedownDatabase, lazy: &LazyType) -> serde_json::Value {
    let Some(typ) = lazy.resolve(db) else {
      return serde_json::json!({ "type": "string" });
    };

    // Sum of string literals is a select (filter out TdNullType for nullable `T?` types)
    if let Some(sum) = typ.as_td_sum_type() {
      let members = sum.members(db);
      let non_null_members: Vec<LazyType> = members
        .iter()
        .filter(|m| {
          if let Some(m_typ) = m.resolve(db) {
            !m_typ.is_td_null_type()
          } else {
            true
          }
        })
        .cloned()
        .collect();

      if non_null_members.len() == 1 {
        return lazy_to_descriptor(db, &non_null_members[0]);
      }

      let mut literals: Vec<String> = non_null_members
        .iter()
        .filter_map(|m| {
          if let Some(TdTypeEnum::TdLiteralType(lit)) = m.resolve(db)
            && let LiteralValue::Str(s) = lit.value(db)
          {
            Some(s)
          } else {
            None
          }
        })
        .collect();
      literals.sort();

      if !literals.is_empty() && literals.len() == non_null_members.len() {
        return serde_json::json!({ "widget": Widget::Select, "options": literals });
      }
      return serde_json::json!({ "widget": Widget::Text });
    }

    // List type: check if elem is a sum of string literals (multi_select)
    if let Some(list) = typ.as_td_list_type() {
      if let Some(elem_lazy) = list.elem(db)
        && let Some(elem_typ) = elem_lazy.resolve(db)
      {
        if let Some(sum) = elem_typ.as_td_sum_type() {
          let members = sum.members(db);
          let mut literals: Vec<String> = members
            .iter()
            .filter_map(|m| {
              if let Some(TdTypeEnum::TdLiteralType(lit)) = m.resolve(db)
                && let LiteralValue::Str(s) = lit.value(db)
              {
                Some(s)
              } else {
                None
              }
            })
            .collect();
          literals.sort();

          if literals.len() == members.len() && !literals.is_empty() {
            return serde_json::json!({ "widget": Widget::MultiSelect, "options": literals });
          }
          if members.len() == 1 {
            let first_member = members.iter().next().unwrap();
            let inner = lazy_to_descriptor(db, first_member);
            return serde_json::json!({ "widget": Widget::List, "items": inner });
          }
        } else {
          let inner = lazy_to_descriptor(db, &elem_lazy);
          return serde_json::json!({ "widget": Widget::List, "items": inner });
        }
      }
      return serde_json::json!({ "widget": Widget::Text });
    }

    simple_type_to_descriptor(db, &typ)
  }

  fn simple_type_to_descriptor(db: &TypedownDatabase, typ: &TdTypeEnum) -> serde_json::Value {
    match typ {
      TdTypeEnum::TdStrType(_) => serde_json::json!({ "widget": Widget::Text }),
      TdTypeEnum::TdNumType(_) => serde_json::json!({ "widget": Widget::Number }),
      TdTypeEnum::TdBoolType(_) => serde_json::json!({ "widget": Widget::Checkbox }),
      TdTypeEnum::TdDateType(_) => serde_json::json!({ "widget": Widget::Date }),
      TdTypeEnum::TdDateTimeType(_) => serde_json::json!({ "widget": Widget::Date }),
      TdTypeEnum::TdTimeType(_) => serde_json::json!({ "widget": Widget::Text }),
      TdTypeEnum::TdListType(list) => match list.elem(db).and_then(|e| e.resolve(db)) {
        Some(elem) => {
          let inner = simple_type_to_descriptor(db, &elem);
          serde_json::json!({ "widget": Widget::List, "items": inner })
        }
        None => serde_json::json!({ "widget": Widget::List }),
      },
      TdTypeEnum::TdSchemaType(schema) => {
        serde_json::json!({ "widget": Widget::Relation, "schema": schema.name(db) })
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
  let mut emitter = MarkdownExporter::new(db, project, file);

  emitter.emit_body(body);
  emitter.finish()
}

struct MarkdownExporter<'a> {
  db: &'a TypedownDatabase,
  project: Project,
  file: File,
  out: String,
  prefix: String,
  at_line_start: bool,
}

impl<'a> MarkdownExporter<'a> {
  fn new(db: &'a TypedownDatabase, project: Project, file: File) -> Self {
    Self {
      db,
      project,
      file,
      out: String::new(),
      prefix: String::new(),
      at_line_start: true,
    }
  }

  fn finish(mut self) -> String {
    if !self.out.ends_with('\n') {
      self.out.push('\n');
    }
    self.out
  }

  fn write(&mut self, text: &str) {
    for ch in text.chars() {
      if ch == '\n' {
        self.out.push('\n');
        self.at_line_start = true;
      } else {
        if self.at_line_start {
          self.out.push_str(&self.prefix);
          self.at_line_start = false;
        }
        self.out.push(ch);
      }
    }
  }

  fn newline(&mut self) {
    self.out.push('\n');
    self.at_line_start = true;
  }

  fn emit_body(&mut self, body: &MdBody) {
    let mut first = true;
    for child in body.syntax().children() {
      if child.kind() == SyntaxKind::Whitespace || child.kind() == SyntaxKind::Newline {
        continue;
      }
      if !first {
        self.newline();
      }
      first = false;
      self.emit_block(&child);
    }
  }

  fn emit_block(&mut self, node: &RedNode) {
    match node.kind() {
      SyntaxKind::MdHeading => self.emit_heading(node),
      SyntaxKind::MdHorizontalRule => self.emit_horizontal_rule(node),
      SyntaxKind::MdParagraph => self.emit_paragraph(node),
      SyntaxKind::MdBlockquote => self.emit_blockquote(node),
      SyntaxKind::MdBulletList => self.emit_list(node),
      SyntaxKind::MdOrderedList => self.emit_list(node),
      SyntaxKind::MdContainerBlock => self.emit_container(node),
      SyntaxKind::MdContainerShorthand => self.emit_container_shorthand(node),
      SyntaxKind::MdTable => self.emit_passthrough(node),
      SyntaxKind::CodeBlock | SyntaxKind::MathBlock => self.emit_passthrough(node),
      _ => self.emit_passthrough(node),
    }
  }

  fn emit_child_blocks(&mut self, node: &RedNode) {
    let mut first = true;
    for child in node.children() {
      let kind = child.kind();
      if kind == SyntaxKind::Whitespace || kind == SyntaxKind::Newline {
        continue;
      }
      if !first {
        self.newline();
      }
      first = false;
      self.emit_block(&child);
    }
  }

  fn emit_heading(&mut self, node: &RedNode) {
    self.emit_inline_children(node);
    self.newline();
  }

  fn emit_horizontal_rule(&mut self, node: &RedNode) {
    self.write(&node.text());
    self.newline();
  }

  fn emit_paragraph(&mut self, node: &RedNode) {
    self.emit_inline_children(node);
    self.newline();
  }

  fn emit_blockquote(&mut self, node: &RedNode) {
    let old_prefix = self.prefix.clone();
    self.prefix.push_str("> ");

    let mut first = true;
    for child in node.children() {
      let kind = child.kind();
      if kind == SyntaxKind::MdSymbol
        || kind == SyntaxKind::Whitespace
        || kind == SyntaxKind::Newline
      {
        continue;
      }
      if !first {
        self.newline();
      }
      first = false;
      self.emit_block(&child);
    }

    self.prefix = old_prefix;
  }

  fn emit_list(&mut self, node: &RedNode) {
    for child in node.children() {
      match child.kind() {
        SyntaxKind::MdBulletListItem | SyntaxKind::MdTaskListItem => {
          self.emit_list_item(&child, "- ");
        }
        SyntaxKind::MdOrderedListItem => {
          let marker = self.extract_ordered_marker(&child);
          self.emit_list_item(&child, &marker);
        }
        _ => {}
      }
    }
  }

  fn extract_ordered_marker(&self, node: &RedNode) -> String {
    let mut num = String::new();
    for child in node.children() {
      match child.kind() {
        SyntaxKind::MdNumber => num = child.text().to_string(),
        SyntaxKind::MdSymbol if child.text() == "." => {
          return format!("{num}. ");
        }
        _ => {
          if !num.is_empty() {
            break;
          }
        }
      }
    }
    "1. ".to_string()
  }

  fn emit_list_item(&mut self, node: &RedNode, marker: &str) {
    let old_prefix = self.prefix.clone();
    let continuation = " ".repeat(marker.len());

    self.write(marker);
    self.prefix.push_str(&continuation);

    if node.kind() == SyntaxKind::MdTaskListItem {
      for child in node.children() {
        if child.kind() == SyntaxKind::MdCheckbox {
          self.write(&child.text());
          self.write(" ");
          break;
        }
      }
    }

    let mut first = true;
    for child in node.children() {
      let kind = child.kind();
      if kind == SyntaxKind::MdSymbol
        || kind == SyntaxKind::MdNumber
        || kind == SyntaxKind::Whitespace
        || kind == SyntaxKind::Newline
        || kind == SyntaxKind::MdCheckbox
      {
        continue;
      }
      if !first {
        self.newline();
      }
      first = false;
      self.emit_block(&child);
    }

    self.prefix = old_prefix;
  }

  fn emit_container(&mut self, node: &RedNode) {
    self.write(":::");
    let mut seen_opening = false;
    for child in node.children() {
      if child.kind() == SyntaxKind::MdSymbol && child.text() == ":::" && !seen_opening {
        seen_opening = true;
        continue;
      }
      if child.kind() == SyntaxKind::Newline {
        break;
      }
      if seen_opening {
        self.emit_inline(&child);
      }
    }
    self.newline();

    for child in node.children() {
      match child.kind() {
        SyntaxKind::MdContainerSlot => {
          self.emit_child_blocks(&child);
        }
        SyntaxKind::MdContainerSlotSeparator => {
          self.write(&child.text());
          self.newline();
        }
        _ => {}
      }
    }

    self.write(":::");
    self.newline();
  }

  fn emit_container_shorthand(&mut self, node: &RedNode) {
    let mut label = String::new();
    let mut props = String::new();

    for child in node.children() {
      match child.kind() {
        SyntaxKind::Ident => label.push_str(&child.text()),
        SyntaxKind::MdSymbol if child.text() == "-" => label.push('-'),
        SyntaxKind::MdContainerPropBlock => props = child.text().trim().to_string(),
        _ => {}
      }
    }

    self.write("::: ");
    self.write(&label);
    if !props.is_empty() {
      self.write(" ");
      self.write(&props);
    }
    self.newline();
    self.write(":::");
    self.newline();
  }

  fn emit_passthrough(&mut self, node: &RedNode) {
    let text = node.text().to_string();

    let mut min_indent = usize::MAX;
    for line in text.lines() {
      if !line.trim().is_empty() {
        let indent = line.len() - line.trim_start().len();

        if indent < min_indent {
          min_indent = indent;
        }
      }
    }
    if min_indent == usize::MAX {
      min_indent = 0;
    }

    for (index, line) in text.lines().enumerate() {
      if index > 0 {
        self.newline();
      }
      if line.len() >= min_indent {
        self.write(&line[min_indent..]);
      } else {
        self.write(line.trim_start());
      }
    }
    self.newline();
  }

  fn emit_inline_children(&mut self, node: &RedNode) {
    let mut started = false;

    self.emit_inline_children_inner(node, &mut started);
  }

  fn emit_inline_children_inner(&mut self, node: &RedNode, started: &mut bool) {
    for child in node.children() {
      let kind = child.kind();
      if kind == SyntaxKind::Newline {
        continue;
      }
      if !*started && kind == SyntaxKind::Whitespace {
        continue;
      }
      if !*started && child.as_token().is_none() && kind != SyntaxKind::InterpFragment {
        self.emit_inline_children_inner(&child, started);
        continue;
      }
      *started = true;
      self.emit_inline(&child);
    }
  }

  fn emit_inline(&mut self, node: &RedNode) {
    if node.as_token().is_some() {
      self.write(&node.text());
      return;
    }

    if node.kind() == SyntaxKind::InterpFragment {
      let Some(fragment) = InterpFragment::cast(node.clone()) else {
        return;
      };
      let Some(expr) = fragment.expr() else { return };
      let expr_node = expr.syntax().clone();

      if let Some(link) = try_resolve_fref(self.db, self.project, self.file, &expr_node) {
        self.write(&link);
        return;
      }
      let hir = lower_node(self.db, self.project, self.file, expr_node);
      let scope = get_file_runtime_scope(self.db, self.project, self.file);
      if let Some(obj) = evaluate_node(self.db, hir, scope).value(self.db)
        && let Some(func) = obj.lookup_method(self.db, "to_string")
        && let Ok(result) = func.call(self.db, Some(obj), vec![])
        && let Some(str_obj) = result.as_td_str_obj()
      {
        self.write(&str_obj.value(self.db));
      }
      return;
    }

    for child in node.children() {
      self.emit_inline(&child);
    }
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
      let root_dir = config.root_dir(db);
      let base_path = config.base_path(db);
      let relative = path.strip_prefix(&root_dir).unwrap_or(path);
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
      let root_dir = config.root_dir(db);
      let base_path = config.base_path(db);
      let relative = path.strip_prefix(&root_dir).unwrap_or(path);
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

/// Get a display name for a symbol: Try _label, then file stem
fn resolve_display_name(db: &TypedownDatabase, project: Project, symbol: &Symbol) -> String {
  let kind = symbol.kind(db);

  // Try _label or name from the evaluated resource
  if let SymbolKind::UserDefinedResource(_, target_file) = &kind
    && let Some(target_symbol) = file_symbol(db, project, *target_file).value(db)
    && let Some(obj) = evaluate_resource(db, target_symbol).value(db)
  {
    let label_or_name = obj.get_owned_field(db, "_label");
    if let Some(str_obj) = label_or_name
      .as_ref()
      .and_then(|field| field.as_td_str_obj())
    {
      return str_obj.value(db);
    }
  }

  // Fallback: file stem, or parent directory name for index files
  match &kind {
    SymbolKind::UserDefinedResource(_, target_file)
    | SymbolKind::UserDefinedSchema(_, target_file) => {
      let handle = target_file.handle(db);
      let path = handle.path();
      let stem = path.and_then(|p| p.file_stem()).and_then(|s| s.to_str());

      match stem {
        Some("index") => path
          .and_then(|p| p.parent())
          .and_then(|p| p.file_name())
          .and_then(|n| n.to_str())
          .unwrap_or("index")
          .to_string(),
        Some(name) => name.to_string(),
        None => "unknown".to_string(),
      }
    }
    _ => symbol.name(db).to_string(),
  }
}

pub(super) fn evaluate_lazy_field(
  db: &TypedownDatabase,
  field: Either<HirValue, TdObjectEnum>,
) -> Option<TdObjectEnum> {
  match field {
    Either::Right(obj) => Some(obj),
    Either::Left(hir) => {
      let file_scope = get_file_runtime_scope(db, hir.project(db), hir.file(db));
      evaluate_node(db, hir, file_scope).value(db)
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::db::fixtures::load_vault_fixture;

  #[test]
  fn exports_header_fields() {
    let (db, project, file) = load_vault_fixture("evaluate/my_vault", "valid_person.td");
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
    let (db, project, file) = load_vault_fixture("evaluate/my_vault", "md_with_content.td");
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
    let (db, project, file) = load_vault_fixture("evaluate/my_vault", "md_with_content.td");
    let exported = export_resource(&db, project, file).expect("should export");
    assert_eq!(exported.content, "Hello world\n");
  }

  #[test]
  fn exports_all_markdown_elements() {
    let (db, project, file) = load_vault_fixture("evaluate/my_vault", "all_md_elements.td");
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
  fn exports_container_shorthand() {
    let (db, project, file) = load_vault_fixture("evaluate/my_vault", "md_container_shorthand.td");
    let exported = export_resource(&db, project, file).expect("should export");
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

  // Unresolved interpolation in markdown silently produces empty output
  #[test]
  fn exports_unresolved_interpolation_as_empty() {
    let (db, project, file) = load_vault_fixture("evaluate/my_vault", "md_unresolved_interp.td");
    let exported = export_resource(&db, project, file).expect("should export");
    let content = &exported.content;
    // ${question} resolves to nothing because `question` is not a field on Person
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
    let exported = export_resource(&db, project, file).expect("should export");
    let content = &exported.content;

    // No line should have 4+ leading spaces (which markdown-it treats as a code block)
    for line in content.lines() {
      let indent = line.len() - line.trim_start().len();

      assert!(
        indent < 4 || line.trim_start().is_empty(),
        "line has {indent} spaces of indentation (would become code block): '{line}'\nfull content:\n{content}",
      );
    }

    // Nested container content should not be indented
    assert!(
      content.contains("nested paragraph\n"),
      "nested paragraph should not have leading whitespace: {content}",
    );

    // Blockquote content should use > prefix
    assert!(
      content.contains("> blockquote content\n"),
      "blockquote should use > prefix: {content}",
    );

    // List item content
    assert!(
      content.contains("- item one\n"),
      "list item should use - marker: {content}",
    );
  }

  #[test]
  fn exports_container_with_title() {
    let (db, project, file) = load_vault_fixture("evaluate/my_vault", "all_md_elements.td");
    let exported = export_resource(&db, project, file).expect("should export");
    assert!(
      exported.content.contains("::: details Click to expand"),
      "should contain container with title: {}",
      exported.content,
    );
  }

  #[test]
  fn returns_none_for_schema() {
    let (db, project, file) = load_vault_fixture("evaluate/my_vault", "_types/Person.td");
    let result = export_resource(&db, project, file);
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
    let (db, project, file) = load_vault_fixture("evaluate/base_path_vault", "with_fref.td");
    let exported = export_resource(&db, project, file).expect("should export");
    assert!(
      exported.content.contains("[Alice](/blog/alice)"),
      "fref should use base_path /blog: {}",
      exported.content
    );
  }

  #[test]
  fn fref_resolves_image_asset_to_markdown_image() {
    let (db, project, file) = load_vault_fixture("evaluate/my_vault", "with_asset_fref.td");
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
    let (db, project, file) = load_vault_fixture("evaluate/my_vault", "with_fref.td");
    let exported = export_resource(&db, project, file).expect("should export");
    assert!(
      !exported.content.contains("/blog"),
      "default base_path should not have /blog prefix: {}",
      exported.content
    );
  }

  #[test]
  fn export_separates_blocks_with_blank_lines() {
    let (db, project, file) = load_vault_fixture("evaluate/my_vault", "md_with_content.td");
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
  fn exports_schemaless_file() {
    let (db, project, file) = load_vault_fixture("evaluate/my_vault", "schemaless.td");
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
