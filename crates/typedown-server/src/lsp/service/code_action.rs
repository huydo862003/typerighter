use lsp_types::{
  CodeAction, CodeActionKind, CodeActionParams, CodeActionResponse, Command, TextEdit,
  WorkspaceEdit,
};
use std::collections::HashMap;
use typedown_incremental::StableCompare;
use typedown_lang::db::TypedownDatabase;
use typedown_lang::db::derived::evaluate::evaluate_type::evaluate_type;
use typedown_lang::db::derived::hir::lower_node;
use typedown_lang::db::derived::name_resolver::members::members;
use typedown_lang::db::derived::parse_file::parse_file;
use typedown_lang::db::derived::typechecker::expected_node_type::expected_node_type;
use typedown_lang::db::typecheck::utils::is_nullable;
use typedown_lang::db::types::{
  File, LazyType, LiteralValue, Project, Scope, SymbolKind, TdStaticType, TdTypeEnum,
};
use typedown_lang::syntax::ast::{AstNode, Expr, SourceFile};
use typedown_lang::syntax::red::RedNode;
use typedown_lang::syntax::syntax_kind::SyntaxKind;

use crate::core::analysis::Analysis;
use crate::core::utils::ast::{find_ancestor, is_in_mapping_value_position, node_at_offset};
use crate::core::utils::position::lsp_position_to_text_offset;
use crate::core::utils::uri::uri_to_path;
use crate::lsp::service::commands;
use crate::lsp::service::commands::create_linked_resource::CreateLinkedResourceArgs;

pub fn code_action(analysis: &Analysis, params: CodeActionParams) -> Option<CodeActionResponse> {
  let db = &analysis.db;
  let project = analysis.project;

  let path = uri_to_path(&params.text_document.uri)?;
  let file = *project.files(db).get(&path)?;
  let root = parse_file(db, project, file).ast(db);
  let source = SourceFile::cast(root.clone())?;

  let mut actions: Vec<lsp_types::CodeActionOrCommand> = Vec::new();
  let requested_kinds = &params.context.only;

  // Schema initialization for empty frontmatter
  if !source
    .frontmatter()
    .is_some_and(|fm| fm.mapping().is_some())
  {
    for (name, template) in collect_schemas(db, project) {
      let edit = TextEdit {
        range: params.range,
        new_text: format!("---\n_type: {name}\n{template}---\n"),
      };

      actions.push(lsp_types::CodeActionOrCommand::CodeAction(CodeAction {
        title: format!("Initialize as {name}"),
        kind: Some(CodeActionKind::QUICKFIX),
        edit: Some(WorkspaceEdit {
          changes: Some(HashMap::from([(
            params.text_document.uri.clone(),
            vec![edit],
          )])),
          ..Default::default()
        }),
        ..Default::default()
      }));
    }
  }

  // "Create new {Schema} and link here" for schema-typed fields
  let rope = analysis.file_rope(&path)?;
  let offset = lsp_position_to_text_offset(&rope, params.range.start)?;
  if let Some(node) = node_at_offset(root, offset.saturating_sub(1))
    && let Some(action) = create_linked_action(db, project, file, &node, &params)
  {
    actions.push(lsp_types::CodeActionOrCommand::CodeAction(action));
  }

  // Filter by requested kinds if the client specified any
  if let Some(only) = requested_kinds {
    actions.retain(|action| {
      let kind = match action {
        lsp_types::CodeActionOrCommand::CodeAction(a) => a.kind.as_ref(),
        _ => None,
      };
      kind.is_some_and(|k| only.iter().any(|o| k.as_str().starts_with(o.as_str())))
    });
  }

  if actions.is_empty() {
    return None;
  }
  Some(actions)
}

// Offer "Create new {Schema} and link here" when cursor is on a schema-typed field
fn create_linked_action(
  db: &TypedownDatabase,
  project: Project,
  file: File,
  node: &RedNode,
  params: &CodeActionParams,
) -> Option<CodeAction> {
  // Must be in or near a value position
  let entry = find_ancestor(node, SyntaxKind::YamlMappingEntry)?;
  let entry_key = entry
    .children()
    .find(|c| c.kind() == SyntaxKind::YamlMappingEntryKey)?;
  let key_text = entry_key.text().trim().to_string();

  // Resolve the field's declared type
  let typ = resolve_entry_type(db, project, file, node, &entry)?;

  // Extract the schema name from the type (or nullable wrapper)
  let schema_name = extract_schema_name(db, &typ)?;

  // Detect if the field is a list
  let is_list = typ.is_td_list_type()
    || typ.as_td_sum_type().is_some_and(|s| {
      s.members(db)
        .iter()
        .any(|m| m.resolve(db).is_some_and(|t| t.is_td_list_type()))
    });

  let args = CreateLinkedResourceArgs {
    schema: schema_name.clone(),
    source_uri: params.text_document.uri.as_str().to_string(),
    line: params.range.start.line,
    character: params.range.start.character,
    is_list,
    filename: String::new(),
    prompts: vec![commands::Prompt::Input {
      field: "filename".to_string(),
      prompt: format!("New {schema_name} filename:"),
      default: Some("untitled.td".to_string()),
    }],
  };

  Some(CodeAction {
    title: format!("Create new {schema_name} and link to {key_text}"),
    kind: Some(CodeActionKind::REFACTOR),
    command: Some(Command {
      title: format!("Create new {schema_name}"),
      command: commands::CREATE_LINKED_RESOURCE.to_string(),
      arguments: Some(vec![serde_json::to_value(args).ok()?]),
    }),
    ..Default::default()
  })
}

// Resolve the declared type of a mapping entry's value
fn resolve_entry_type(
  db: &TypedownDatabase,
  project: Project,
  file: File,
  node: &RedNode,
  entry: &RedNode,
) -> Option<TdTypeEnum> {
  // Try via the value expression
  if is_in_mapping_value_position(node) {
    let entry_value = find_ancestor(node, SyntaxKind::YamlMappingEntryValue)?;
    if let Some(value_expr) = entry_value.children().find_map(Expr::cast) {
      let hir = lower_node(db, project, file, value_expr.syntax().clone());
      if let Some(typ) = expected_node_type(db, hir).typ(db) {
        return Some(typ);
      }
    }
  }

  // Fall back to schema field lookup
  let key_text = entry
    .children()
    .find(|c| c.kind() == SyntaxKind::YamlMappingEntryKey)?
    .text()
    .trim()
    .to_string();
  let mapping = find_ancestor(entry, SyntaxKind::YamlMapping)?;
  let schema_name = typedown_lang::db::utils::schema_name_in_mapping(&mapping)?;
  let scope = Scope::project_scope(db, project);
  let symbol = *members(db, scope).members(db).get(&schema_name)?;
  let typ = evaluate_type(db, symbol).typ(db)?;
  let schema = typ.as_td_schema_type()?;
  let prop = schema.fields(db).get(&key_text)?.clone();
  prop.field_type.resolve(db)
}

// Extract the schema name from a type, handling nullable and list wrappers
fn extract_schema_name(db: &TypedownDatabase, typ: &TdTypeEnum) -> Option<String> {
  if typ.is_td_schema_type() {
    return Some(typ.display_name(db));
  }
  // Check nullable: T? -> extract T if T is a schema
  if let Some(sum) = typ.as_td_sum_type() {
    for member in sum.members(db) {
      if let Some(resolved) = member.resolve(db)
        && resolved.is_td_schema_type()
      {
        return Some(resolved.display_name(db));
      }
    }
  }
  // Check list[Schema]
  if let Some(list) = typ.as_td_list_type() {
    let resolved = list.element_type(db)?.resolve(db)?;
    if resolved.is_td_schema_type() {
      return Some(resolved.display_name(db));
    }
  }
  None
}

// Collect all schemas with their field templates
fn collect_schemas(db: &TypedownDatabase, project: Project) -> Vec<(String, String)> {
  let scope = Scope::project_scope(db, project);

  members(db, scope)
    .members(db)
    .iter()
    .filter(|(_, sym)| matches!(sym.kind(db), SymbolKind::UserDefinedSchema(..)))
    .filter_map(|(name, sym)| {
      let typ = evaluate_type(db, *sym).typ(db)?;
      let schema = typ.as_td_schema_type()?;
      let fields = schema.fields(db);

      let mut template = String::new();

      for (field_name, prop_desc) in &fields {
        let default = default_value(db, &prop_desc.field_type);
        let optional = prop_desc
          .field_type
          .resolve(db)
          .is_some_and(|t| is_nullable(db, &t));

        if optional {
          template.push_str(&format!("# {field_name}: {default}\n"));
        } else {
          template.push_str(&format!("{field_name}: {default}\n"));
        }
      }

      Some((name.clone(), template))
    })
    .collect()
}

// Generate a default value string for a schema field type
fn default_value(db: &TypedownDatabase, lazy: &LazyType) -> String {
  let Some(typ) = lazy.resolve(db) else {
    return "\"\"".to_string();
  };
  match typ {
    TdTypeEnum::TdLiteralType(lit) => match lit.value(db) {
      LiteralValue::Str(s) => format!("\"{s}\""),
      _ => "\"\"".to_string(),
    },
    TdTypeEnum::TdStrType(_) => "\"\"".to_string(),
    TdTypeEnum::TdNumType(_) => "0".to_string(),
    TdTypeEnum::TdBoolType(_) => "false".to_string(),
    TdTypeEnum::TdDateType(_) => "\"\"".to_string(),
    TdTypeEnum::TdDateTimeType(_) => "\"\"".to_string(),
    TdTypeEnum::TdTimeType(_) => "\"\"".to_string(),
    TdTypeEnum::TdSchemaType(schema) => {
      format!("fref(\"{}\")", schema.name(db))
    }
    TdTypeEnum::TdListType(_) => "[]".to_string(),

    TdTypeEnum::TdSumType(sum) => {
      let mut members: Vec<_> = sum.members(db).into_iter().collect();
      members.sort_by(|a, b| a.stable_cmp(db, b));
      // Optional type: use non-null member's default
      let non_null: Vec<_> = members
        .iter()
        .filter(|m| !m.resolve(db).is_some_and(|t| t.as_td_null_type().is_some()))
        .collect();
      if non_null.len() == 1 {
        return default_value(db, non_null[0]);
      }
      // Enum: use first literal as default
      members
        .iter()
        .find_map(|m| {
          if let Some(TdTypeEnum::TdLiteralType(lit)) = m.resolve(db)
            && let LiteralValue::Str(s) = lit.value(db)
          {
            return Some(format!("\"{s}\""));
          }
          None
        })
        .unwrap_or_else(|| "\"\"".to_string())
    }
    _ => "\"\"".to_string(),
  }
}

#[cfg(test)]
mod tests {
  use std::collections::HashMap;
  use std::path::PathBuf;
  use std::sync::{Arc, Condvar, Mutex};

  use lsp_types::{CodeActionContext, CodeActionParams, Position, Range, TextDocumentIdentifier};
  use typedown_lang::db::types::{File, FileHandle, FileMetadata, Project};
  use typedown_lang::db::{QueryStorage, TypedownDatabase};

  use crate::core::analysis::Analysis;
  use crate::core::utils::uri::path_to_uri;

  use super::code_action;

  const VAULT_CONFIG: &str = r#"version: "1"
vault:
  root_dir: "."
"#;

  const SCHEMA_TASK: &str = r#"---
_type: schema
properties:
  title:
    type: string
  status:
    type: ['todo', 'in_progress', 'done']
  assignee:
    type: Person
    optional: true
---
"#;

  const SCHEMA_PERSON: &str = r#"---
_type: schema
properties:
  name:
    type: string
  active:
    type: boolean
---
"#;

  fn setup(content: &str) -> (Analysis, lsp_types::Uri) {
    let root = PathBuf::from(if cfg!(windows) { "C:\\vault" } else { "/vault" });
    let type_root = root.join("_types");
    let test_path = root.join("file.td");
    let uri = path_to_uri(&test_path, "file");

    let db = TypedownDatabase {
      storage: QueryStorage::default(),
    };

    let files = HashMap::from([
      (
        root.join("typedown.yaml"),
        File::new(
          &db,
          FileHandle::Content(
            root.join("typedown.yaml"),
            VAULT_CONFIG.to_string(),
            FileMetadata::default(),
          ),
        ),
      ),
      (
        type_root.join("Task.td"),
        File::new(
          &db,
          FileHandle::Content(
            type_root.join("Task.td"),
            SCHEMA_TASK.to_string(),
            FileMetadata::default(),
          ),
        ),
      ),
      (
        type_root.join("Person.td"),
        File::new(
          &db,
          FileHandle::Content(
            type_root.join("Person.td"),
            SCHEMA_PERSON.to_string(),
            FileMetadata::default(),
          ),
        ),
      ),
      (
        test_path.clone(),
        File::new(
          &db,
          FileHandle::Content(test_path, content.to_string(), FileMetadata::default()),
        ),
      ),
    ]);

    let project = Project::new(&db, root, files);
    let analysis = Analysis::new(
      db,
      project,
      Arc::new(HashMap::new()),
      Arc::new(HashMap::new()),
      Arc::new((Mutex::new(1), Condvar::new())),
    );

    (analysis, uri)
  }

  fn make_params(uri: lsp_types::Uri) -> CodeActionParams {
    make_params_at(uri, 0, 0)
  }

  fn make_params_at(uri: lsp_types::Uri, line: u32, character: u32) -> CodeActionParams {
    CodeActionParams {
      text_document: TextDocumentIdentifier { uri },
      range: Range {
        start: Position { line, character },
        end: Position { line, character },
      },
      context: CodeActionContext::default(),
      work_done_progress_params: Default::default(),
      partial_result_params: Default::default(),
    }
  }

  fn make_params_with_only(
    uri: lsp_types::Uri,
    line: u32,
    character: u32,
    only: Vec<lsp_types::CodeActionKind>,
  ) -> CodeActionParams {
    CodeActionParams {
      text_document: TextDocumentIdentifier { uri },
      range: Range {
        start: Position { line, character },
        end: Position { line, character },
      },
      context: CodeActionContext {
        only: Some(only),
        ..Default::default()
      },
      work_done_progress_params: Default::default(),
      partial_result_params: Default::default(),
    }
  }

  #[test]
  fn offers_schema_actions_for_empty_file() {
    let (analysis, uri) = setup("---\n---\n");
    let params = make_params(uri);
    let response = code_action(&analysis, params).expect("should return actions");

    let titles: Vec<&str> = response
      .iter()
      .filter_map(|a| match a {
        lsp_types::CodeActionOrCommand::CodeAction(ca) => Some(ca.title.as_str()),
        _ => None,
      })
      .collect();

    assert!(
      titles.iter().any(|t| t.contains("Task")),
      "should offer Task: {titles:?}"
    );
    assert!(
      titles.iter().any(|t| t.contains("Person")),
      "should offer Person: {titles:?}"
    );
  }

  #[test]
  fn no_init_actions_for_file_with_content() {
    let (analysis, uri) = setup(
      r#"---
_type: Task
title: "hello"
---
"#,
    );
    let params = make_params(uri);
    let response = code_action(&analysis, params);

    let has_init = response.as_ref().is_some_and(|actions| {
      actions.iter().any(|a| match a {
        lsp_types::CodeActionOrCommand::CodeAction(ca) => ca.title.starts_with("Initialize"),
        _ => false,
      })
    });
    assert!(
      !has_init,
      "should not offer init actions for non-empty frontmatter"
    );
  }

  #[test]
  fn create_linked_action_on_schema_typed_field() {
    let (analysis, uri) = setup(
      r#"---
_type: Task
title: "hello"
assignee:
---
"#,
    );
    // Cursor on the assignee line (line 3)
    let params = make_params_at(uri, 3, 5);
    let response = code_action(&analysis, params);

    let has_create = response.as_ref().is_some_and(|actions| {
      actions.iter().any(|a| match a {
        lsp_types::CodeActionOrCommand::CodeAction(ca) => ca.title.contains("Create new Person"),
        _ => false,
      })
    });
    assert!(
      has_create,
      "should offer create linked action on Person-typed field: {:?}",
      response.as_ref().map(|a| a
        .iter()
        .map(|x| match x {
          lsp_types::CodeActionOrCommand::CodeAction(ca) => ca.title.clone(),
          _ => String::new(),
        })
        .collect::<Vec<_>>())
    );
  }

  #[test]
  fn no_create_linked_on_string_field() {
    let (analysis, uri) = setup(
      r#"---
_type: Task
title: "hello"
---
"#,
    );
    // Cursor on the title line (line 2)
    let params = make_params_at(uri, 2, 5);
    let response = code_action(&analysis, params);

    let has_create = response.as_ref().is_some_and(|actions| {
      actions.iter().any(|a| match a {
        lsp_types::CodeActionOrCommand::CodeAction(ca) => ca.title.contains("Create new"),
        _ => false,
      })
    });
    assert!(
      !has_create,
      "should not offer create linked on string field"
    );
  }

  #[test]
  fn context_only_filters_actions() {
    let (analysis, uri) = setup("---\n---\n");
    // Request only refactor actions, should not return quickfix (init) actions
    let params = make_params_with_only(uri, 0, 0, vec![lsp_types::CodeActionKind::REFACTOR]);
    let response = code_action(&analysis, params);

    let has_init = response.as_ref().is_some_and(|actions| {
      actions.iter().any(|a| match a {
        lsp_types::CodeActionOrCommand::CodeAction(ca) => ca.title.starts_with("Initialize"),
        _ => false,
      })
    });
    assert!(
      !has_init,
      "refactor-only request should not include quickfix actions"
    );
  }

  #[test]
  fn generated_template_includes_fields() {
    let (analysis, uri) = setup("---\n---\n");
    let params = make_params(uri);
    let response = code_action(&analysis, params).expect("should return actions");

    let task_action = response
      .iter()
      .filter_map(|a| match a {
        lsp_types::CodeActionOrCommand::CodeAction(ca) => Some(ca),
        _ => None,
      })
      .find(|ca| ca.title.contains("Task"))
      .expect("should have Task action");

    let text = &task_action
      .edit
      .as_ref()
      .and_then(|e| e.changes.as_ref())
      .and_then(|c| c.values().next())
      .expect("should have file edits")[0]
      .new_text;

    assert!(text.contains("_type: Task"), "should contain _type: {text}");
    assert!(text.contains("title:"), "should contain title: {text}");
    assert!(text.contains("status:"), "should contain status: {text}");
    assert!(
      text.contains("\"done\"") || text.contains("\"todo\""),
      "enum should default to first option: {text}"
    );
    assert!(
      text.contains("assignee: fref(\"Person\")"),
      "relation should use fref: {text}"
    );
  }
}
