use lsp_types::{
  CodeAction, CodeActionKind, CodeActionParams, CodeActionResponse, Command, TextEdit,
  WorkspaceEdit,
};
use ropey::Rope;
use std::collections::HashMap;
use typedown_lang::db::TypedownDatabase;
use typedown_lang::db::derived::evaluate::evaluate_typ::evaluate_typ;
use typedown_lang::db::derived::name_resolver::file_symbol::file_symbol;
use typedown_lang::db::derived::name_resolver::members::members;
use typedown_lang::db::derived::parse_file::parse_file;
use typedown_lang::db::derived::typechecker::expected_node_typ::expected_node_typ;
use typedown_lang::db::types::{File, Project, ProjectRedNode, SymbolKind, TdStaticTyp, TdTypEnum};
use typedown_lang::db::types::{Scope, ScopeKind};
use typedown_lang::db::utils::get_mapping_schema_name;
use typedown_lang::syntax::ast::{AstNode, Expr, SourceFile};
use typedown_lang::syntax::red::RedNode;
use typedown_lang::syntax::syntax_kind::SyntaxKind;

use crate::core::analysis::Analysis;
use crate::core::utils::ast::{find_ancestor, is_in_mapping_value_position, node_at_offset};
use crate::core::utils::position::lsp_position_to_byte_offset;
use crate::core::utils::uri::uri_to_path;
use crate::lsp::service::commands;
use crate::lsp::service::commands::create_linked_resource::{
  CreateLinkedResourceArgs, InsertContext,
};
use crate::lsp::service::template::{collect_schema_names, collect_schemas, slugify};

pub fn resolve_code_action(
  analysis: &Analysis,
  params: CodeActionParams,
) -> Option<CodeActionResponse> {
  let db = &analysis.db;
  let project = analysis.project;

  let path = uri_to_path(&params.text_document.uri)?;
  let file = *project.files(db).get(&path)?;
  let root = parse_file(db, project, file).ast(db).node.clone();
  let source = SourceFile::cast(root.clone())?;

  let mut actions: Vec<lsp_types::CodeActionOrCommand> = Vec::new();
  let requested_kinds = &params.context.only;

  let rope = analysis.file_rope(&path)?;

  // Schema initialization for empty frontmatter
  if source
    .frontmatter()
    .is_none_or(|frontmatter| frontmatter.mapping().is_none())
  {
    // Check actual file content for --- delimiters
    // (the parser creates a YamlFrontmatter node even for files without them)
    let first_line = rope.line(0).to_string();
    let has_delimiters = first_line.starts_with("---");

    // When delimiters exist, insert content between them; find the closing --- line
    let insert_range = if has_delimiters {
      let closing_line = (1..rope.len_lines())
        .find(|&line| rope.line(line).to_string().starts_with("---"))
        .unwrap_or(1) as u32;
      lsp_types::Range {
        start: lsp_types::Position {
          line: 1,
          character: 0,
        },
        end: lsp_types::Position {
          line: closing_line,
          character: 0,
        },
      }
    } else {
      // No delimiters: insert from start of file
      lsp_types::Range {
        start: lsp_types::Position {
          line: 0,
          character: 0,
        },
        end: lsp_types::Position {
          line: 0,
          character: 0,
        },
      }
    };

    for (name, template) in collect_schemas(db, project) {
      let new_text = if has_delimiters {
        format!("_type: {name}\n{template}")
      } else {
        format!("---\n_type: {name}\n{template}---\n")
      };
      let edit = TextEdit {
        range: insert_range,
        new_text,
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
  let offset = lsp_position_to_byte_offset(&rope, params.range.start)?;
  if let Some(node) = node_at_offset(root, offset.saturating_sub(1)) {
    // 1. Schema-typed frontmatter field
    if let Some(action) = create_linked_for_typed_field(db, project, file, &node, &params) {
      actions.push(lsp_types::CodeActionOrCommand::CodeAction(action));
    }

    // 2. Markdown body
    if let Some(action) = create_linked_for_markdown_body(db, project, &node, &params, &rope) {
      actions.push(lsp_types::CodeActionOrCommand::CodeAction(action));
    }

    // 3. Backward link (from frontmatter or markdown body)
    if let Some(action) = create_backward_link(db, project, file, &node, &params) {
      actions.push(lsp_types::CodeActionOrCommand::CodeAction(action));
    }
  }

  // Filter by requested kinds
  if let Some(only_kind) = requested_kinds {
    actions.retain(|action| {
      let kind = match action {
        lsp_types::CodeActionOrCommand::CodeAction(action) => action.kind.as_ref(),
        _ => None,
      };
      kind.is_some_and(|kind| {
        only_kind
          .iter()
          .any(|only_kind| kind.as_str().starts_with(only_kind.as_str()))
      })
    });
  }

  if actions.is_empty() {
    return None;
  }
  Some(actions)
}

// Trigger on schema-typed frontmatter fields (e.g., assignee: on a Task with type Person)
fn create_linked_for_typed_field(
  db: &TypedownDatabase,
  project: Project,
  file: File,
  node: &RedNode,
  params: &CodeActionParams,
) -> Option<CodeAction> {
  let entry = find_ancestor(node, SyntaxKind::YamlMappingEntry)?;
  let entry_key = entry
    .children()
    .find(|child| child.kind() == SyntaxKind::YamlMappingEntryKey)?;
  let key_text = entry_key.text().trim().to_string();

  let typ = resolve_entry_type(db, project, file, node, &entry)?;
  let schema_name = extract_schema_name(db, &typ)?;

  let insert_context = if is_list_typed(db, &typ) {
    InsertContext::FrontmatterListItem
  } else {
    InsertContext::FrontmatterValue
  };

  let args = CreateLinkedResourceArgs {
    schema: schema_name.clone(),
    source_uri: params.text_document.uri.as_str().to_string(),
    line: params.range.start.line,
    character: params.range.start.character,
    insert_context,
    filename: String::new(),
    prompts: vec![commands::Prompt::Input {
      field: "filename".to_string(),
      prompt: format!("New {schema_name} filename:"),
      default: Some("untitled".to_string()),
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

// Trigger when cursor is in markdown body
fn create_linked_for_markdown_body(
  db: &TypedownDatabase,
  project: Project,
  node: &RedNode,
  params: &CodeActionParams,
  rope: &Rope,
) -> Option<CodeAction> {
  // Must be in markdown body, not frontmatter, not inside ${} interpolation
  find_ancestor(node, SyntaxKind::MdBody)?;
  if find_ancestor(node, SyntaxKind::YamlFrontmatter).is_some() {
    return None;
  }
  if find_ancestor(node, SyntaxKind::InterpFragment).is_some() {
    return None;
  }

  let schema_names = collect_schema_names(db, project);
  if schema_names.is_empty() {
    return None;
  }

  // Use selected text as filename slug default
  let has_selection = params.range.start != params.range.end;
  let selected_text = if has_selection {
    extract_text_from_rope(rope, &params.range)
  } else {
    String::new()
  };
  let default_filename = if selected_text.is_empty() {
    "untitled".to_string()
  } else {
    slugify(&selected_text)
  };

  let args = CreateLinkedResourceArgs {
    schema: String::new(),
    source_uri: params.text_document.uri.as_str().to_string(),
    line: params.range.start.line,
    character: params.range.start.character,
    insert_context: InsertContext::MarkdownBody,
    filename: String::new(),
    prompts: vec![
      commands::Prompt::Select {
        field: "schema".to_string(),
        prompt: "Schema:".to_string(),
        choices: schema_names,
      },
      commands::Prompt::Input {
        field: "filename".to_string(),
        prompt: "Filename:".to_string(),
        default: Some(default_filename),
      },
    ],
  };

  Some(CodeAction {
    title: "Create new file and link here".to_string(),
    kind: Some(CodeActionKind::REFACTOR),
    command: Some(Command {
      title: "Create linked resource".to_string(),
      command: commands::CREATE_LINKED_RESOURCE.to_string(),
      arguments: Some(vec![serde_json::to_value(args).ok()?]),
    }),
    ..Default::default()
  })
}

// Trigger when current file has a schema and other schemas reference it
fn create_backward_link(
  db: &TypedownDatabase,
  project: Project,
  file: File,
  node: &RedNode,
  params: &CodeActionParams,
) -> Option<CodeAction> {
  let source_schema_name = get_file_schema_name(db, project, file)?;
  let candidates = find_schemas_with_field_typed_as(db, project, &source_schema_name);
  if candidates.is_empty() {
    return None;
  }

  // Also trigger from markdown body (#5)
  let in_frontmatter = find_ancestor(node, SyntaxKind::YamlFrontmatter).is_some();
  let in_body = find_ancestor(node, SyntaxKind::MdBody).is_some();
  if !in_frontmatter && !in_body {
    return None;
  }

  let mut prompts = vec![];

  let (schema, field) = if candidates.len() == 1 {
    candidates[0].clone()
  } else {
    let choices: Vec<String> = candidates
      .iter()
      .map(|(schema, _)| schema.clone())
      .collect();
    prompts.push(commands::Prompt::Select {
      field: "schema".to_string(),
      prompt: "Schema to create:".to_string(),
      choices,
    });
    // Field will be resolved at execution time from the chosen schema
    (String::new(), String::new())
  };

  prompts.push(commands::Prompt::Input {
    field: "filename".to_string(),
    prompt: "Filename:".to_string(),
    default: Some("untitled".to_string()),
  });

  let title = if candidates.len() == 1 {
    format!("Create new {} linking back here", candidates[0].0)
  } else {
    "Create new file linking back here".to_string()
  };

  let args = CreateLinkedResourceArgs {
    schema,
    source_uri: params.text_document.uri.as_str().to_string(),
    line: params.range.start.line,
    character: params.range.start.character,
    insert_context: InsertContext::BackwardLink {
      field,
      source_schema: source_schema_name,
    },
    filename: String::new(),
    prompts,
  };

  Some(CodeAction {
    title,
    kind: Some(CodeActionKind::REFACTOR),
    command: Some(Command {
      title: "Create backward link".to_string(),
      command: commands::CREATE_LINKED_RESOURCE.to_string(),
      arguments: Some(vec![serde_json::to_value(args).ok()?]),
    }),
    ..Default::default()
  })
}

// Resolve the declared type of a mapping entry's value
fn resolve_entry_type<'db>(
  db: &'db TypedownDatabase,
  project: Project,
  file: File,
  node: &RedNode,
  entry: &RedNode,
) -> Option<TdTypEnum<'db>> {
  if is_in_mapping_value_position(node) {
    let entry_value = find_ancestor(node, SyntaxKind::YamlMappingEntryValue)?;
    if let Some(value_expr) = entry_value.children().find_map(Expr::cast) {
      let project_node = ProjectRedNode::new(project, file, value_expr.syntax().clone());
      if let Some(typ) = expected_node_typ(db, project_node).typ(db) {
        return Some(typ);
      }
    }
  }

  // Fall back to schema field lookup
  let key_text = entry
    .children()
    .find(|child| child.kind() == SyntaxKind::YamlMappingEntryKey)?
    .text()
    .trim()
    .to_string();
  let mapping = find_ancestor(entry, SyntaxKind::YamlMapping)?;
  let schema_name = get_mapping_schema_name(&mapping)?;
  let scope = Scope::new(db, ScopeKind::Project(project));
  let symbol = *members(db, scope).members(db).get(&schema_name)?;
  let typ = evaluate_typ(db, symbol).typ(db)?;
  let schema = typ.as_td_schema_typ()?;
  let prop = schema.fields(db).get(&key_text)?.clone();
  prop.field_typ.resolve(db)
}

// Extract schema name from a type, handling nullable and list wrappers
fn extract_schema_name(db: &TypedownDatabase, typ: &TdTypEnum) -> Option<String> {
  if typ.is_td_schema_typ() {
    return Some(typ.display_name(db));
  }
  if let Some(sum) = typ.as_td_sum_typ() {
    for member in sum.members(db) {
      if let Some(resolved) = member.resolve(db)
        && resolved.is_td_schema_typ()
      {
        return Some(resolved.display_name(db));
      }
    }
  }
  if let Some(list) = typ.as_td_list_typ() {
    let resolved = list.element_typ(db)?.resolve(db)?;
    if resolved.is_td_schema_typ() {
      return Some(resolved.display_name(db));
    }
  }
  None
}

fn is_list_typed(db: &TypedownDatabase, typ: &TdTypEnum) -> bool {
  typ.is_td_list_typ()
    || typ.as_td_sum_typ().is_some_and(|sum| {
      sum
        .members(db)
        .iter()
        .any(|member| member.resolve(db).is_some_and(|typ| typ.is_td_list_typ()))
    })
}

// Get the _type schema name of the current file
fn get_file_schema_name(db: &TypedownDatabase, project: Project, file: File) -> Option<String> {
  let symbol = file_symbol(db, project, file).value(db)?;
  if !matches!(symbol.kind(db), SymbolKind::UserDefinedResource(_, _)) {
    return None;
  }
  let root = parse_file(db, project, file).ast(db).node.clone();
  let mapping = SourceFile::cast(root)?.frontmatter()?.mapping()?;
  let type_value = mapping
    .entries()
    .find(|(key, _)| key == "_type")
    .map(|(_, value)| value)?;
  Some(type_value.syntax().text().trim().to_string())
}

// Find schemas with a field typed as the given schema name
// Returns (target_schema_name, field_name) pairs
fn find_schemas_with_field_typed_as(
  db: &TypedownDatabase,
  project: Project,
  source_schema: &str,
) -> Vec<(String, String)> {
  let scope = Scope::new(db, ScopeKind::Project(project));
  let mut results = vec![];

  for (name, symbol) in members(db, scope).members(db).iter() {
    if !matches!(symbol.kind(db), SymbolKind::UserDefinedSchema(..)) {
      continue;
    }
    let Some(typ) = evaluate_typ(db, *symbol).typ(db) else {
      continue;
    };
    let Some(schema) = typ.as_td_schema_typ() else {
      continue;
    };

    for (field_name, descriptor) in schema.fields(db) {
      let field_references_source = descriptor
        .field_typ
        .resolve(db)
        .is_some_and(|field_typ| references_schema(db, &field_typ, source_schema));
      if field_references_source {
        results.push((name.clone(), field_name.clone()));
      }
    }
  }

  results
}

// Check if a type references a schema by name (handles nullable and list wrappers)
fn references_schema(db: &TypedownDatabase, typ: &TdTypEnum, schema_name: &str) -> bool {
  if typ.is_td_schema_typ() && typ.display_name(db) == schema_name {
    return true;
  }
  if let Some(sum) = typ.as_td_sum_typ() {
    return sum.members(db).iter().any(|member| {
      member
        .resolve(db)
        .is_some_and(|typ| references_schema(db, &typ, schema_name))
    });
  }
  if let Some(list) = typ.as_td_list_typ() {
    return list
      .element_typ(db)
      .and_then(|lazy| lazy.resolve(db))
      .is_some_and(|typ| references_schema(db, &typ, schema_name));
  }
  false
}

fn extract_text_from_rope(rope: &Rope, range: &lsp_types::Range) -> String {
  let start_line = range.start.line as usize;
  let end_line = range.end.line as usize;
  if start_line >= rope.len_lines() || end_line >= rope.len_lines() {
    return String::new();
  }
  let start_offset = rope.line_to_char(start_line) + range.start.character as usize;
  let end_offset = rope.line_to_char(end_line) + range.end.character as usize;
  if start_offset >= rope.len_chars() || end_offset > rope.len_chars() {
    return String::new();
  }
  rope.slice(start_offset..end_offset).to_string()
}

#[cfg(test)]
mod tests {
  use super::resolve_code_action as code_action;
  use crate::core::analysis::Analysis;
  use crate::core::utils::uri::path_to_uri;
  use lsp_types::{CodeActionContext, CodeActionParams, Position, Range, TextDocumentIdentifier};
  use std::collections::HashMap;
  use std::path::PathBuf;
  use std::sync::{Arc, Condvar, Mutex};
  use typedown_lang::db::types::{File, FileHandle, FileMetadata, Project};
  use typedown_lang::db::{QueryStorage, TypedownDatabase};

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
    type: Person?
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

  // Pet references Person via owner field
  const SCHEMA_PET: &str = r#"---
_type: schema
properties:
  name:
    type: string
  owner:
    type: Person
---
"#;

  fn setup(content: &str) -> (Analysis, lsp_types::Uri) {
    setup_with_schemas(content, &[])
  }

  fn setup_with_schemas(
    content: &str,
    extra_schemas: &[(&str, &str)],
  ) -> (Analysis, lsp_types::Uri) {
    let root = PathBuf::from(if cfg!(windows) { "C:\\vault" } else { "/vault" });
    let typ_root = root.join("_types");
    let test_path = root.join("file.td");
    let uri = path_to_uri(&test_path, "file");

    let db = TypedownDatabase {
      storage: QueryStorage::default(),
    };

    let mut files = HashMap::from([
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
        typ_root.join("Task.td"),
        File::new(
          &db,
          FileHandle::Content(
            typ_root.join("Task.td"),
            SCHEMA_TASK.to_string(),
            FileMetadata::default(),
          ),
        ),
      ),
      (
        typ_root.join("Person.td"),
        File::new(
          &db,
          FileHandle::Content(
            typ_root.join("Person.td"),
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

    for (name, schema_content) in extra_schemas {
      let schema_path = typ_root.join(format!("{name}.td"));
      files.insert(
        schema_path.clone(),
        File::new(
          &db,
          FileHandle::Content(
            schema_path,
            schema_content.to_string(),
            FileMetadata::default(),
          ),
        ),
      );
    }

    let project = Project::new(&db, root, files, None);
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

  fn action_titles(response: &Option<Vec<lsp_types::CodeActionOrCommand>>) -> Vec<String> {
    response
      .as_ref()
      .map(|actions| {
        actions
          .iter()
          .filter_map(|action| match action {
            lsp_types::CodeActionOrCommand::CodeAction(code_action) => {
              Some(code_action.title.clone())
            }
            _ => None,
          })
          .collect()
      })
      .unwrap_or_default()
  }

  #[test]
  fn offers_schema_actions_for_empty_file() {
    let (analysis, uri) = setup("---\n---\n");
    let params = make_params(uri);
    let response = code_action(&analysis, params).expect("should return actions");

    let titles: Vec<&str> = response
      .iter()
      .filter_map(|action| match action {
        lsp_types::CodeActionOrCommand::CodeAction(code_action) => Some(code_action.title.as_str()),
        _ => None,
      })
      .collect();

    assert!(
      titles.iter().any(|title| title.contains("Task")),
      "should offer Task: {titles:?}"
    );
    assert!(
      titles.iter().any(|title| title.contains("Person")),
      "should offer Person: {titles:?}"
    );
  }

  #[test]
  fn no_init_actions_for_file_with_content() {
    let (analysis, uri) = setup("---\n_type: Task\ntitle: \"hello\"\n---\n");
    let params = make_params(uri);
    let response = code_action(&analysis, params);

    let has_init = response.as_ref().is_some_and(|actions| {
      actions.iter().any(|action| match action {
        lsp_types::CodeActionOrCommand::CodeAction(code_action) => {
          code_action.title.starts_with("Initialize")
        }
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
    let (analysis, uri) = setup("---\n_type: Task\ntitle: \"hello\"\nassignee:\n---\n");
    let params = make_params_at(uri, 3, 5);
    let response = code_action(&analysis, params);
    let titles = action_titles(&response);
    assert!(
      titles
        .iter()
        .any(|title| title.contains("Create new Person")),
      "should offer create linked on Person-typed field: {titles:?}"
    );
  }

  #[test]
  fn no_create_linked_on_string_field() {
    let (analysis, uri) = setup("---\n_type: Task\ntitle: \"hello\"\n---\n");
    let params = make_params_at(uri, 2, 5);
    let response = code_action(&analysis, params);
    let titles = action_titles(&response);
    assert!(
      !titles.iter().any(|title| title.contains("Create new")),
      "should not offer create linked on string field: {titles:?}"
    );
  }

  #[test]
  fn create_linked_action_in_markdown_body() {
    let (analysis, uri) = setup("---\n_type: Task\ntitle: \"hello\"\n---\nSome prose here\n");
    let params = make_params_at(uri, 4, 5);
    let response = code_action(&analysis, params);
    let titles = action_titles(&response);
    assert!(
      titles
        .iter()
        .any(|title| title.contains("Create new file and link here")),
      "should offer create linked in markdown body: {titles:?}"
    );
  }

  #[test]
  fn no_create_linked_in_frontmatter_body() {
    let (analysis, uri) = setup("---\n_type: Task\ntitle: \"hello\"\n---\n");
    // Cursor on _type line, inside frontmatter
    let params = make_params_at(uri, 1, 2);
    let response = code_action(&analysis, params);
    let titles = action_titles(&response);
    assert!(
      !titles
        .iter()
        .any(|title| title.contains("Create new file and link here")),
      "should not offer markdown body action in frontmatter: {titles:?}"
    );
  }

  #[test]
  fn backward_link_action_when_referencing_schema_exists() {
    let (analysis, uri) = setup_with_schemas(
      "---\n_type: Person\nname: \"Alice\"\n---\n",
      &[("Pet", SCHEMA_PET)],
    );
    let params = make_params_at(uri, 2, 0);
    let response = code_action(&analysis, params);
    let titles = action_titles(&response);
    assert!(
      titles
        .iter()
        .any(|title| title.contains("linking back here")),
      "should offer backward link when Pet references Person: {titles:?}"
    );
  }

  #[test]
  fn no_backward_link_when_no_referencing_schemas() {
    let (analysis, uri) = setup("---\n_type: Task\ntitle: \"hello\"\n---\n");
    let params = make_params_at(uri, 2, 0);
    let response = code_action(&analysis, params);
    let titles = action_titles(&response);
    assert!(
      !titles
        .iter()
        .any(|title| title.contains("linking back here")),
      "should not offer backward link when no schemas reference Task: {titles:?}"
    );
  }

  #[test]
  fn context_only_filters_actions() {
    let (analysis, uri) = setup("---\n---\n");
    let params = make_params_with_only(uri, 0, 0, vec![lsp_types::CodeActionKind::REFACTOR]);
    let response = code_action(&analysis, params);

    let has_init = response.as_ref().is_some_and(|actions| {
      actions.iter().any(|action| match action {
        lsp_types::CodeActionOrCommand::CodeAction(code_action) => {
          code_action.title.starts_with("Initialize")
        }
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
    let (analysis, uri) = setup("---\n---");
    let params = make_params(uri);
    let response = code_action(&analysis, params).expect("should return actions");

    let task_action = response
      .iter()
      .filter_map(|action| match action {
        lsp_types::CodeActionOrCommand::CodeAction(code_action) => Some(code_action),
        _ => None,
      })
      .find(|code_action| code_action.title.contains("Task"))
      .expect("should have Task action");

    let text = &task_action
      .edit
      .as_ref()
      .and_then(|edit| edit.changes.as_ref())
      .and_then(|changes| changes.values().next())
      .expect("should have file edits")[0]
      .new_text;

    assert!(text.contains("_type: Task"), "should contain _type: {text}");
    assert!(text.contains("title:"), "should contain title: {text}");
    assert!(text.contains("status:"), "should contain status: {text}");
    assert!(
      text.contains(r#""done""#) || text.contains(r#""todo""#),
      "enum should default to first option: {text}"
    );
  }

  #[test]
  fn markdown_body_action_has_schema_select_prompt() {
    let (analysis, uri) = setup("---\n_type: Task\ntitle: \"hello\"\n---\nSome text\n");
    let params = make_params_at(uri, 4, 5);
    let response = code_action(&analysis, params);

    let action = response
      .as_ref()
      .and_then(|actions| {
        actions.iter().find_map(|action| match action {
          lsp_types::CodeActionOrCommand::CodeAction(code_action)
            if code_action.title.contains("Create new file") =>
          {
            Some(code_action)
          }
          _ => None,
        })
      })
      .expect("should have markdown body action");

    let args_json = action
      .command
      .as_ref()
      .and_then(|cmd| cmd.arguments.as_ref())
      .and_then(|args| args.first())
      .expect("should have command args");

    let prompts = args_json["prompts"]
      .as_array()
      .expect("should have prompts");
    assert!(
      prompts.iter().any(|prompt| prompt["kind"] == "select"),
      "should have a select prompt for schema: {prompts:?}"
    );
  }

  fn get_init_edit(
    response: &[lsp_types::CodeActionOrCommand],
    name: &str,
  ) -> Option<lsp_types::TextEdit> {
    response.iter().find_map(|action| match action {
      lsp_types::CodeActionOrCommand::CodeAction(code_action)
        if code_action.title.contains(name) =>
      {
        code_action
          .edit
          .as_ref()
          .and_then(|edit| edit.changes.as_ref())
          .and_then(|changes| changes.values().next())
          .map(|edits| edits[0].clone())
      }
      _ => None,
    })
  }

  // Empty file (no delimiters): edit must include opening and closing ---
  #[test]
  fn init_on_empty_file_produces_delimiters() {
    let (analysis, uri) = setup("");
    let params = make_params(uri);
    let response = code_action(&analysis, params).expect("should return actions");
    let edit = get_init_edit(&response, "Task").expect("should have Task action");

    assert!(
      edit.new_text.starts_with("---\n"),
      "new_text must start with ---: {:?}",
      edit.new_text
    );
    assert!(
      edit.new_text.trim_end().ends_with("---"),
      "new_text must end with ---: {:?}",
      edit.new_text
    );
    // Range must be at start of file so the frontmatter is inserted there
    assert_eq!(
      edit.range.start,
      Position {
        line: 0,
        character: 0
      },
      "range start must be at start of file"
    );
  }

  // File with content but no frontmatter: insert full frontmatter at start
  #[test]
  fn init_on_body_only_file_inserts_frontmatter_at_start() {
    let (analysis, uri) = setup(
      "
Some prose content here.
",
    );
    let params = make_params(uri);
    let response = code_action(&analysis, params).expect("should return actions");
    let edit = get_init_edit(&response, "Task").expect("should have Task action");

    assert!(
      edit.new_text.starts_with("---\n"),
      "new_text must start with ---: {:?}",
      edit.new_text
    );
    assert_eq!(
      edit.range.start,
      Position {
        line: 0,
        character: 0
      },
      "range start must be at start of file"
    );
  }

  // File with content and a --- horizontal rule (not frontmatter): insert frontmatter at start
  #[test]
  fn init_on_file_with_body_separator_inserts_frontmatter_at_start() {
    let (analysis, uri) = setup(
      "Some content
---
More content
",
    );
    let params = make_params(uri);
    let response = code_action(&analysis, params).expect("should return actions");
    let edit = get_init_edit(&response, "Task").expect("should have Task action");

    assert!(
      edit.new_text.starts_with("---\n"),
      "new_text must start with --- when no frontmatter: {:?}",
      edit.new_text
    );
    assert_eq!(
      edit.range.start,
      Position {
        line: 0,
        character: 0
      },
      "range start must be at start of file"
    );
  }

  // File with only an opening --- (no closing delimiter): insert content after the opening line
  #[test]
  fn init_on_file_with_only_opening_delimiter_inserts_after_it() {
    let (analysis, uri) = setup(
      "---
",
    );
    let params = make_params(uri);
    let response = code_action(&analysis, params).expect("should return actions");
    let edit = get_init_edit(&response, "Task").expect("should have Task action");

    // has_delimiters=true so no extra --- in new_text
    assert!(
      !edit.new_text.starts_with("---"),
      "new_text must NOT start with --- when opening delimiter already exists: {:?}",
      edit.new_text
    );
    // Insert at or after line 1 (after the opening ---)
    assert!(
      edit.range.start.line >= 1,
      "range must start after the opening ---, got line {}",
      edit.range.start.line
    );
  }

  // File with empty delimiters: edit must NOT add extra ---, content goes inside existing fences
  #[test]
  fn init_on_empty_delimiters_inserts_inside_fences() {
    let (analysis, uri) = setup(
      "---
---
",
    );
    let params = make_params(uri);
    let response = code_action(&analysis, params).expect("should return actions");
    let edit = get_init_edit(&response, "Task").expect("should have Task action");

    assert!(
      !edit.new_text.contains("---"),
      "new_text must NOT contain --- (delimiters already exist): {:?}",
      edit.new_text
    );
    // Range must be positioned inside the existing delimiters (after line 0)
    assert!(
      edit.range.start.line >= 1,
      "range must start inside the existing delimiters (line >= 1), got line {}",
      edit.range.start.line
    );
  }
}
