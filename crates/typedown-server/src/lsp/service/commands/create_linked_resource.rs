//! Creates a new .td file with templated frontmatter and inserts fref() at the source location
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
use typedown_lang::db::derived::evaluate::evaluate_type::evaluate_type;
use typedown_lang::db::derived::get_vault_config::get_vault_config;
use typedown_lang::db::derived::name_resolver::file_symbol::file_symbol;
use typedown_lang::db::derived::name_resolver::members::members;
use typedown_lang::db::derived::typechecker::get_symbol_type::get_symbol_type;
use typedown_lang::db::types::derived::object_system::TdStaticType;
use typedown_lang::db::types::{Project, Scope, SymbolKind};

use typedown_types::path::normalize_path;

use crate::core::analysis::Analysis;
use crate::core::utils::uri::{path_to_uri, uri_to_path};

static REQUEST_COUNTER: AtomicU64 = AtomicU64::new(1);

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateLinkedResourceArgs {
  pub schema: String,
  pub source_uri: String,
  pub line: u32,
  pub character: u32,
  pub is_list: bool,
  // Filled by the editor after prompting
  #[serde(default)]
  pub filename: String,
  // Editor resolves these before sending the command
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
  // Ensure .td extension
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
  let relative_path = if let Some(dir) = &target_dir {
    format!(
      "{}/{}",
      normalize_path(dir.strip_prefix(&root_dir).unwrap_or(dir)),
      filename
    )
  } else {
    // No existing files of this schema, create next to the source file
    let source_dir = source_path.parent().unwrap_or(&root_dir);
    let rel_dir = normalize_path(source_dir.strip_prefix(&root_dir).unwrap_or(source_dir));
    if rel_dir.is_empty() {
      filename.clone()
    } else {
      format!("{rel_dir}/{filename}")
    }
  };
  let absolute_path = root_dir.join(&relative_path);

  // Build the templated frontmatter
  let frontmatter = build_template(db, project, &args.schema);

  // Resolve URIs
  let scheme = analysis
    .scheme_map
    .get(&absolute_path)
    .map(String::as_str)
    .unwrap_or("file");
  let new_file_uri = path_to_uri(&absolute_path, scheme);

  // Build the fref text to insert
  let fref_text = if args.is_list {
    format!("\n  - fref(\"{}\")", relative_path)
  } else {
    format!("fref(\"{}\")", relative_path)
  };

  // Create workspace edit: create file + write frontmatter + insert fref at source
  let document_changes = DocumentChanges::Operations(vec![
    // Create the new file
    DocumentChangeOperation::Op(ResourceOp::Create(CreateFile {
      uri: new_file_uri.clone(),
      options: None,
      annotation_id: None,
    })),
    // Write the frontmatter to the new file
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
    // Insert fref at the source cursor
    DocumentChangeOperation::Edit(TextDocumentEdit {
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
    }),
  ]);

  let edit_params = ApplyWorkspaceEditParams {
    label: Some(format!("Create {}", args.schema)),
    edit: WorkspaceEdit {
      document_changes: Some(document_changes),
      ..Default::default()
    },
  };

  let edit_request = lsp_server::Request::new(
    next_request_id(),
    "workspace/applyEdit".to_string(),
    edit_params,
  );
  connection.sender.send(edit_request.into())?;

  let show_params = ShowDocumentParams {
    uri: new_file_uri,
    external: Some(false),
    take_focus: Some(true),
    selection: None,
  };
  let show_request = lsp_server::Request::new(
    next_request_id(),
    "window/showDocument".to_string(),
    show_params,
  );
  connection.sender.send(show_request.into())?;

  Ok(())
}

// Unique request ID for server-to-client requests
fn next_request_id() -> lsp_server::RequestId {
  lsp_server::RequestId::from(REQUEST_COUNTER.fetch_add(1, Ordering::Relaxed) as i32)
}

// Find the directory with existing files of this schema nearest to the source file
fn find_nearest_schema_directory(
  db: &TypedownDatabase,
  project: Project,
  schema_name: &str,
  root_dir: &Path,
  source_path: &Path,
) -> Option<PathBuf> {
  let mut best: Option<std::path::PathBuf> = None;
  let mut best_prefix_len = 0;

  for (path, file) in project.files(db).iter() {
    if !path.starts_with(root_dir) {
      continue;
    }
    let Some(sym) = file_symbol(db, project, *file).value(db) else {
      continue;
    };
    if !matches!(sym.kind(db), SymbolKind::UserDefinedResource(_, _)) {
      continue;
    }
    if get_symbol_type(db, sym)
      .typ(db)
      .is_none_or(|t| t.display_name(db) != schema_name)
    {
      continue;
    }
    let Some(dir) = path.parent() else { continue };
    let prefix_len = common_prefix_len(dir, source_path);
    if prefix_len > best_prefix_len {
      best_prefix_len = prefix_len;
      best = Some(dir.to_path_buf());
    }
  }
  best
}

// Count how many path components two paths share from the root
fn common_prefix_len(a: &Path, b: &Path) -> usize {
  a.components()
    .zip(b.components())
    .take_while(|(x, y)| x == y)
    .count()
}

// Build a frontmatter template with all fields from the schema
fn build_template(db: &TypedownDatabase, project: Project, schema_name: &str) -> String {
  let mut template = format!("---\n_type: {schema_name}\n");

  let typ = members(db, Scope::project_scope(db, project))
    .members(db)
    .get(schema_name)
    .copied()
    .and_then(|sym| evaluate_type(db, sym).typ(db));

  if let Some(schema) = typ.as_ref().and_then(|t| t.as_td_schema_type()) {
    for (field_name, _) in schema.fields(db) {
      template.push_str(&format!("{field_name}: \n"));
    }
  }

  template.push_str("---\n");
  template
}
