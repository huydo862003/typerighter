// Creates a new .td file with templated frontmatter and inserts fref() at the source location

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

use lsp_server::Connection;
use lsp_types::{
  ApplyWorkspaceEditParams, CreateFile, DocumentChangeOperation, DocumentChanges, OneOf,
  OptionalVersionedTextDocumentIdentifier, Position, ResourceOp, ShowDocumentParams,
  TextDocumentEdit, TextEdit, Uri, WorkspaceEdit,
};
use serde::{Deserialize, Serialize};
use typedown_lang::db::TypedownDatabase;
use typedown_lang::db::derived::evaluate::evaluate_typ::evaluate_typ;
use typedown_lang::db::derived::get_vault_config::get_vault_config;
use typedown_lang::db::derived::name_resolver::file_symbol::file_symbol;
use typedown_lang::db::derived::name_resolver::members::members;
use typedown_lang::db::derived::typechecker::get_symbol_typ::get_symbol_typ;
use typedown_lang::db::types::derived::obj_system::TdStaticTyp;
use typedown_lang::db::types::{Project, Scope, ScopeKind, SymbolKind};

use typedown_types::path::normalize_path;

use crate::core::analysis::Analysis;
use crate::core::utils::uri::{path_to_uri, uri_to_path};
use crate::lsp::service::template::build_template;

static REQUEST_COUNTER: AtomicU64 = AtomicU64::new(1);

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", tag = "kind")]
pub enum InsertContext {
  FrontmatterValue,
  FrontmatterListItem,
  MarkdownBody,
  BackwardLink {
    field: String,
    source_schema: String,
  },
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateLinkedResourceArgs {
  pub schema: String,
  pub source_uri: String,
  pub line: u32,
  pub character: u32,
  pub insert_context: InsertContext,
  #[serde(default)]
  pub filename: String,
  #[serde(default, skip_serializing_if = "Vec::is_empty")]
  pub prompts: Vec<super::Prompt>,
}

pub fn execute(
  analysis: &Analysis,
  args: CreateLinkedResourceArgs,
  connection: &Connection,
) -> anyhow::Result<()> {
  if args.filename.is_empty() {
    anyhow::bail!("filename is required");
  }

  // 1. Resolve paths
  let filename = if args.filename.ends_with(".td") {
    args.filename.clone()
  } else {
    format!("{}.td", args.filename)
  };

  let source_uri: Uri = args
    .source_uri
    .parse()
    .map_err(|_| anyhow::anyhow!("invalid source URI"))?;
  let source_path =
    uri_to_path(&source_uri).ok_or_else(|| anyhow::anyhow!("invalid source path"))?;

  let db = &analysis.db;
  let project = analysis.project;
  let config = get_vault_config(db, project);
  let root_dir = config.root_dir(db);

  let target_dir =
    find_nearest_schema_directory(db, project, &args.schema, &root_dir, &source_path);
  let relative_path = resolve_relative_path(&target_dir, &root_dir, &source_path, &filename);
  let absolute_path = root_dir.join(&relative_path);
  let fref_path = format!("/{relative_path}");

  // 2. Build the new file's frontmatter
  let backward_link = match &args.insert_context {
    InsertContext::BackwardLink {
      field,
      source_schema,
    } => {
      let source_relative =
        normalize_path(source_path.strip_prefix(&root_dir).unwrap_or(&source_path));
      // Resolve field at runtime when empty (multi-candidate case)
      let resolved_field = if field.is_empty() {
        resolve_backward_link_field(db, project, &args.schema, source_schema).unwrap_or_default()
      } else {
        field.clone()
      };
      Some((resolved_field, format!("/{source_relative}")))
    }
    _ => None,
  };
  let frontmatter = build_template(
    db,
    project,
    &args.schema,
    backward_link
      .as_ref()
      .map(|(field, path)| (field.as_str(), path.as_str())),
  );

  // 3. Build the fref text to insert at source cursor
  let fref_text = match &args.insert_context {
    InsertContext::FrontmatterValue => format!(r#"fref("{fref_path}")"#),
    InsertContext::FrontmatterListItem => format!("\n  - fref(\"{fref_path}\")"),
    InsertContext::MarkdownBody => format!(r#"${{fref("{fref_path}")}}"#),
    InsertContext::BackwardLink { .. } => String::new(),
  };

  // 4. Build workspace edit
  let scheme = analysis
    .scheme_map
    .get(&absolute_path)
    .map(String::as_str)
    .unwrap_or("file");
  let new_file_uri = path_to_uri(&absolute_path, scheme);

  let mut operations = vec![
    // Create the new file
    DocumentChangeOperation::Op(ResourceOp::Create(CreateFile {
      uri: new_file_uri.clone(),
      options: None,
      annotation_id: None,
    })),
    // Write frontmatter to the new file
    DocumentChangeOperation::Edit(TextDocumentEdit {
      text_document: OptionalVersionedTextDocumentIdentifier {
        uri: new_file_uri.clone(),
        version: None,
      },
      edits: vec![OneOf::Left(TextEdit {
        range: lsp_types::Range::default(),
        new_text: frontmatter,
      })],
    }),
  ];

  // Insert fref at source cursor (skip for backward links)
  if !fref_text.is_empty() {
    operations.push(DocumentChangeOperation::Edit(TextDocumentEdit {
      text_document: OptionalVersionedTextDocumentIdentifier {
        uri: source_uri,
        version: None,
      },
      edits: vec![OneOf::Left(TextEdit {
        range: lsp_types::Range {
          start: Position {
            line: args.line,
            character: args.character,
          },
          end: Position {
            line: args.line,
            character: args.character,
          },
        },
        new_text: fref_text,
      })],
    }));
  }

  // 5. Apply edit and open new file
  let edit_params = ApplyWorkspaceEditParams {
    label: Some(format!("Create {}", args.schema)),
    edit: WorkspaceEdit {
      document_changes: Some(DocumentChanges::Operations(operations)),
      ..Default::default()
    },
  };

  let edit_request = lsp_server::Request::new(
    next_request_id(),
    "workspace/applyEdit".to_string(),
    edit_params,
  );
  connection.sender.send(edit_request.into())?;

  let show_request = lsp_server::Request::new(
    next_request_id(),
    "window/showDocument".to_string(),
    ShowDocumentParams {
      uri: new_file_uri,
      external: Some(false),
      take_focus: Some(true),
      selection: None,
    },
  );
  connection.sender.send(show_request.into())?;

  Ok(())
}

fn next_request_id() -> lsp_server::RequestId {
  lsp_server::RequestId::from(REQUEST_COUNTER.fetch_add(1, Ordering::Relaxed) as i32)
}

fn resolve_relative_path(
  target_dir: &Option<PathBuf>,
  root_dir: &Path,
  source_path: &Path,
  filename: &str,
) -> String {
  if let Some(dir) = target_dir {
    format!(
      "{}/{}",
      normalize_path(dir.strip_prefix(root_dir).unwrap_or(dir)),
      filename
    )
  } else {
    let source_dir = source_path.parent().unwrap_or(root_dir);
    let rel_dir = normalize_path(source_dir.strip_prefix(root_dir).unwrap_or(source_dir));
    if rel_dir.is_empty() {
      filename.to_string()
    } else {
      format!("{rel_dir}/{filename}")
    }
  }
}

// Find the directory with existing files of this schema nearest to the source file
fn find_nearest_schema_directory(
  db: &TypedownDatabase,
  project: Project,
  schema_name: &str,
  root_dir: &Path,
  source_path: &Path,
) -> Option<PathBuf> {
  let mut best: Option<PathBuf> = None;
  let mut best_prefix_length = 0;

  for (path, file) in project.files(db).iter() {
    if !path.starts_with(root_dir) {
      continue;
    }
    let Some(symbol) = file_symbol(db, project, *file).value(db) else {
      continue;
    };
    if !matches!(symbol.kind(db), SymbolKind::UserDefinedResource(_, _)) {
      continue;
    }
    if get_symbol_typ(db, symbol)
      .typ(db)
      .is_none_or(|typ| typ.display_name(db) != schema_name)
    {
      continue;
    }
    let Some(dir) = path.parent() else { continue };
    let prefix_length = common_prefix_len(dir, source_path);
    if prefix_length > best_prefix_length {
      best_prefix_length = prefix_length;
      best = Some(dir.to_path_buf());
    }
  }
  best
}

fn common_prefix_len(left_path: &Path, right_path: &Path) -> usize {
  left_path
    .components()
    .zip(right_path.components())
    .take_while(|(left_component, right_component)| left_component == right_component)
    .count()
}

// Find the field on target_schema that references source_schema
fn resolve_backward_link_field(
  db: &TypedownDatabase,
  project: Project,
  target_schema: &str,
  source_schema: &str,
) -> Option<String> {
  let scope = Scope::new(db, ScopeKind::Project(project));
  let symbol = *members(db, scope).members(db).get(target_schema)?;
  let typ = evaluate_typ(db, symbol).typ(db)?;
  let schema = typ.as_td_schema_typ()?;

  for (field_name, descriptor) in schema.fields(db) {
    let matches = descriptor
      .field_typ
      .resolve(db)
      .is_some_and(|field_typ| field_typ.display_name(db).contains(source_schema));
    if matches {
      return Some(field_name);
    }
  }
  None
}
