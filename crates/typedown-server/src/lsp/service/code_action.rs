use lsp_types::{
  CodeAction, CodeActionKind, CodeActionParams, CodeActionResponse, TextEdit, WorkspaceEdit,
};
use std::collections::HashMap;
use typedown_lang::db::TypedownDatabase;
use typedown_lang::db::derived::evaluate::evaluate_type::evaluate_type;
use typedown_lang::db::derived::name_resolver::members::members;
use typedown_lang::db::derived::parse_file::parse_file;
use typedown_lang::db::types::{
  LiteralValue, MemberType, Project, Scope, SymbolKind, TdTypeEnum, TypeMemberDescriptors,
};
use typedown_lang::syntax::ast::{AstNode, SourceFile};

use crate::core::analysis::Analysis;
use crate::core::utils::uri::uri_to_path;

pub fn code_action(analysis: &Analysis, params: CodeActionParams) -> Option<CodeActionResponse> {
  let db = &analysis.db;
  let project = analysis.project;

  let path = uri_to_path(&params.text_document.uri)?;
  let file = *project.files(db).get(&path)?;
  let root = parse_file(db, project, file).ast(db);
  let source = SourceFile::cast(root)?;

  // Only offer schema initialization when frontmatter has no mapping entries
  if source
    .frontmatter()
    .is_some_and(|fm| fm.mapping().is_some())
  {
    return None;
  }

  let schemas = collect_schemas(db, project);
  if schemas.is_empty() {
    return None;
  }

  let mut actions = Vec::new();

  for (name, template) in schemas {
    let edit = TextEdit {
      range: params.range,
      new_text: format!("---\n_type: {name}\n{template}---\n"),
    };

    #[allow(clippy::mutable_key_type)]
    let changes = HashMap::from([(params.text_document.uri.clone(), vec![edit])]);

    actions.push(CodeAction {
      title: format!("Initialize as {name}"),
      kind: Some(CodeActionKind::QUICKFIX),
      edit: Some(WorkspaceEdit {
        changes: Some(changes),
        ..Default::default()
      }),
      ..Default::default()
    });
  }

  Some(
    actions
      .into_iter()
      .map(lsp_types::CodeActionOrCommand::CodeAction)
      .collect(),
  )
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
      let product = typ.as_td_product_type()?;
      let fields = product.fields(db);

      let mut template = String::new();

      for (field_name, member) in &fields {
        let default = default_value(db, &member.typ(db));
        let optional = member
          .descriptors(db)
          .contains(TypeMemberDescriptors::OPTIONAL);

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

// Generate a default value string for a member type
fn default_value(db: &TypedownDatabase, member: &MemberType) -> String {
  match member {
    MemberType::Simple(lazy) => {
      let Some(typ) = lazy.resolve(db) else {
        return "\"\"".to_string();
      };
      match typ {
        TdTypeEnum::TdStrType(_) => "\"\"".to_string(),
        TdTypeEnum::TdNumType(_) => "0".to_string(),
        TdTypeEnum::TdBoolType(_) => "false".to_string(),
        TdTypeEnum::TdDateType(_) => "\"\"".to_string(),
        TdTypeEnum::TdDateTimeType(_) => "\"\"".to_string(),
        TdTypeEnum::TdTimeType(_) => "\"\"".to_string(),
        TdTypeEnum::TdProductType(product) => match product.name(db) {
          Some(schema) => format!("fref(\"{schema}\")"),
          None => "\"\"".to_string(),
        },
        TdTypeEnum::TdListType(_) => "[]".to_string(),
        _ => "\"\"".to_string(),
      }
    }
    MemberType::Sum(members) => {
      // Enum: use first literal as default
      members
        .first()
        .and_then(|m| match m.typ(db) {
          MemberType::Literal(LiteralValue::Str(s)) => Some(format!("\"{s}\"")),
          _ => None,
        })
        .unwrap_or_else(|| "\"\"".to_string())
    }
    MemberType::Literal(LiteralValue::Str(s)) => format!("\"{s}\""),
    MemberType::ListOfSum(_) => "[]".to_string(),
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
  content_dir: content
  schema_dir: schemas
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
    let content_root = root.join("content");
    let schema_root = root.join("schemas");
    let test_path = content_root.join("file.td");
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
        schema_root.join("Task.td"),
        File::new(
          &db,
          FileHandle::Content(
            schema_root.join("Task.td"),
            SCHEMA_TASK.to_string(),
            FileMetadata::default(),
          ),
        ),
      ),
      (
        schema_root.join("Person.td"),
        File::new(
          &db,
          FileHandle::Content(
            schema_root.join("Person.td"),
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
    CodeActionParams {
      text_document: TextDocumentIdentifier { uri },
      range: Range {
        start: Position {
          line: 0,
          character: 0,
        },
        end: Position {
          line: 0,
          character: 0,
        },
      },
      context: CodeActionContext::default(),
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
  fn no_actions_for_file_with_content() {
    let (analysis, uri) = setup("---\n_type: Task\ntitle: \"hello\"\n---\n");
    let params = make_params(uri);
    let response = code_action(&analysis, params);

    assert!(
      response.is_none(),
      "should not offer actions for non-empty frontmatter"
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

    let edit = task_action.edit.as_ref().expect("should have edit");
    let changes = edit.changes.as_ref().expect("should have changes");
    let edits = changes.values().next().expect("should have file edits");
    let text = &edits[0].new_text;

    assert!(text.contains("_type: Task"), "should contain _type: {text}");
    assert!(text.contains("title:"), "should contain title: {text}");
    assert!(text.contains("status:"), "should contain status: {text}");
    assert!(
      text.contains("\"todo\""),
      "enum should default to first option: {text}"
    );
    assert!(
      text.contains("# assignee:"),
      "optional field should be commented: {text}"
    );
    assert!(
      text.contains("fref(\"Person\")"),
      "relation should use fref: {text}"
    );
  }
}
