use std::path::Path;

use lsp_server::Notification;
use lsp_types::notification::{Notification as _, PublishDiagnostics};
use lsp_types::{Diagnostic, NumberOrString, PublishDiagnosticsParams};
use ropey::Rope;
use typedown_lang::db::TypedownDatabase;
use typedown_lang::db::derived::check_schema_dir::check_schema_dir;
use typedown_lang::db::derived::evaluate::evaluate_resource::evaluate_resource;
use typedown_lang::db::derived::evaluate::evaluate_type::evaluate_type;
use typedown_lang::db::derived::get_vault_config::get_vault_config;
use typedown_lang::db::derived::hir::lower_node;
use typedown_lang::db::derived::name_resolver::file_symbol::file_symbol;
use typedown_lang::db::derived::parse_file::parse_file;
use typedown_lang::db::derived::typechecker::typecheck::typecheck;
use typedown_lang::db::types::{File, Project};
use typedown_lang::db::utils::is_content_file;
use typedown_lang::integrations::lint::lint_markdown;
use typedown_lang::syntax::ast::{AstNode, SourceFile};
use typedown_lang::syntax::diagnostic::Diagnostic as TdDiagnostic;

use crate::core::analysis::Analysis;
use crate::core::utils::position::text_offset_to_lsp_position;
use crate::core::utils::uri::path_to_uri;

use super::to_lsp_diagnostic;

pub fn publish_diagnostics_for_project(analysis: &Analysis) -> Vec<Notification> {
  let db = &analysis.db;
  let project = analysis.project;
  let files = project.files(db);

  let mut notifications = Vec::new();
  for (path, file) in &files {
    if !is_content_file(path) {
      continue;
    }
    let rope = match analysis.file_rope(path) {
      Some(rope) => rope,
      None => continue,
    };
    notifications.push(get_diagnostics_for_file(
      analysis, db, project, path, *file, &rope,
    ));
  }

  // Check for duplicate schema files
  let schema_check = check_schema_dir(db, project);
  for diag in schema_check.diagnostics(db) {
    if let TdDiagnostic::DuplicateSchemaName { ref path, .. } = diag {
      let schema_dir = get_vault_config(db, project).schema_dir(db);
      let full_path = schema_dir.join(path);
      let scheme = analysis
        .scheme_map
        .get(&full_path)
        .map(|s| s.as_str())
        .unwrap_or("file");
      let uri = path_to_uri(&full_path, scheme);
      notifications.push(Notification::new(
        PublishDiagnostics::METHOD.to_string(),
        PublishDiagnosticsParams {
          uri,
          diagnostics: vec![Diagnostic {
            range: lsp_types::Range::default(),
            severity: Some(lsp_types::DiagnosticSeverity::ERROR),
            code: Some(lsp_types::NumberOrString::String(
              diag.code().as_str().into(),
            )),
            source: Some("typedown".into()),
            message: diag.message(),
            ..Default::default()
          }],
          version: None,
        },
      ));
    }
  }

  notifications
}

pub fn publish_diagnostics_for_file(analysis: &Analysis, target: &Path) -> Vec<Notification> {
  if !is_content_file(target) {
    return vec![];
  }

  let db = &analysis.db;
  let project = analysis.project;
  let files = project.files(db);

  let Some(file) = files.get(target) else {
    return vec![];
  };
  let Some(rope) = analysis.file_rope(target) else {
    return vec![];
  };
  vec![get_diagnostics_for_file(
    analysis, db, project, target, *file, &rope,
  )]
}

fn get_diagnostics_for_file(
  analysis: &Analysis,
  db: &TypedownDatabase,
  project: Project,
  path: &Path,
  file: File,
  rope: &Rope,
) -> Notification {
  // Parse errors
  let parse_result = parse_file(db, project, file);
  let mut td_diags: Vec<TdDiagnostic> = parse_result.diagnostics(db).to_vec();

  // Typecheck errors
  let root = parse_result.ast(db);
  let hir = lower_node(db, project, file, root);
  let typecheck_result = typecheck(db, hir);
  td_diags.extend(typecheck_result.diagnostics(db).iter().cloned());

  // Evaluation errors
  if let Some(sym) = file_symbol(db, project, file).value(db) {
    if sym.kind(db).is_schema() {
      let eval_result = evaluate_type(db, sym);
      td_diags.extend(eval_result.diagnostics(db).iter().cloned());
    } else {
      let eval_result = evaluate_resource(db, sym);
      td_diags.extend(eval_result.diagnostics(db).iter().cloned());
    }
  }

  let mut lsp_diags: Vec<Diagnostic> = td_diags
    .iter()
    .filter_map(|diag| to_lsp_diagnostic(diag, rope))
    .collect();

  // Lint warnings (markdown body only)
  if let Some(body) = SourceFile::cast(parse_result.ast(db)).and_then(|sf| sf.body()) {
    for lint in lint_markdown(&body) {
      let start = lint.start_offset.min(rope.len_chars());
      let end = lint.end_offset.min(rope.len_chars());
      lsp_diags.push(Diagnostic {
        range: lsp_types::Range {
          start: text_offset_to_lsp_position(rope, start),
          end: text_offset_to_lsp_position(rope, end),
        },
        severity: Some(lsp_types::DiagnosticSeverity::WARNING),
        code: Some(NumberOrString::String(lint.code.as_str().into())),
        source: Some("typedown".into()),
        message: lint.message,
        ..Default::default()
      });
    }
  }

  let scheme = analysis
    .scheme_map
    .get(path)
    .map(String::as_str)
    .unwrap_or("file");
  let uri = path_to_uri(path, scheme);
  let params = PublishDiagnosticsParams {
    uri,
    diagnostics: lsp_diags,
    version: None,
  };

  Notification::new(PublishDiagnostics::METHOD.to_string(), params)
}

#[cfg(test)]
mod tests {
  use std::collections::HashMap;
  use std::path::PathBuf;
  use std::sync::{Arc, Condvar, Mutex};

  use typedown_lang::db::types::{File, FileHandle, FileMetadata, Project};
  use typedown_lang::db::{QueryStorage, TypedownDatabase};

  use crate::core::analysis::Analysis;

  use super::publish_diagnostics_for_project;

  const VAULT_CONFIG: &str = r#"version: "1"
vault:
  content_dir: content
  schema_dir: schemas
"#;
  const SCHEMA_PERSON: &str = r#"---
_type: schema
properties:
  name:
    type: string
  age:
    type: number
---
"#;

  fn setup(content: &str) -> Analysis {
    let root = PathBuf::from(if cfg!(windows) { "C:\\vault" } else { "/vault" });
    let test_path = root.join("content/file.td");

    let db = TypedownDatabase {
      storage: QueryStorage::default(),
    };

    let config_file = File::new(
      &db,
      FileHandle::Content(
        root.join("typedown.yaml"),
        VAULT_CONFIG.to_string(),
        FileMetadata::default(),
      ),
    );
    let person_file = File::new(
      &db,
      FileHandle::Content(
        root.join("schemas/Person.td"),
        SCHEMA_PERSON.to_string(),
        FileMetadata::default(),
      ),
    );
    let test_file = File::new(
      &db,
      FileHandle::Content(
        test_path.clone(),
        content.to_string(),
        FileMetadata::default(),
      ),
    );

    let files = HashMap::from([
      (root.join("typedown.yaml"), config_file),
      (root.join("schemas/Person.td"), person_file),
      (test_path, test_file),
    ]);

    let project = Project::new(&db, root, files);
    Analysis::new(
      db,
      project,
      Arc::new(HashMap::new()),
      Arc::new(HashMap::new()),
      Arc::new((Mutex::new(1), Condvar::new())),
    )
  }

  #[test]
  fn no_diagnostics_for_valid_file() {
    let analysis = setup(
      r#"---
_type: Person
name: "Alice"
age: 30
---
"#,
    );
    let notifications = publish_diagnostics_for_project(&analysis);
    // Only the content file should have a notification; it must be empty.
    let content_notif = notifications
      .iter()
      .find(|notif| notif.params.to_string().contains("content/file.td"));
    if let Some(notif) = content_notif {
      let params: serde_json::Value = serde_json::from_str(&notif.params.to_string()).unwrap();
      let diags = params["diagnostics"].as_array().unwrap();
      assert!(
        diags.is_empty(),
        "valid file should produce no diagnostics, got: {diags:?}"
      );
    }
  }

  #[test]
  fn unresolved_schema_produces_diagnostic() {
    // _type references a schema that does not exist.
    let analysis = setup(
      r#"---
_type: NonExistent
name: "Alice"
---
"#,
    );
    let notifications = publish_diagnostics_for_project(&analysis);
    let content_notif = notifications
      .iter()
      .find(|notif| notif.params.to_string().contains("content/file.td"));
    let notif = content_notif.expect("expected a notification for the content file");
    let params: serde_json::Value = serde_json::from_str(&notif.params.to_string()).unwrap();
    let diags = params["diagnostics"].as_array().unwrap();
    assert!(
      !diags.is_empty(),
      "unresolved schema should produce at least one diagnostic"
    );
    let codes: Vec<&str> = diags
      .iter()
      .filter_map(|diag| diag["code"].as_str())
      .collect();
    assert!(
      codes.contains(&"unresolved-schema"),
      "expected an unresolved-schema diagnostic, got codes: {codes:?}"
    );
  }

  #[test]
  fn missing_required_field_produces_diagnostic() {
    // Required field 'age' is absent.
    let analysis = setup(
      r#"---
_type: Person
name: "Alice"
---
"#,
    );
    let notifications = publish_diagnostics_for_project(&analysis);
    let content_notif = notifications
      .iter()
      .find(|notif| notif.params.to_string().contains("content/file.td"));
    let notif = content_notif.expect("expected a notification for the content file");
    let params: serde_json::Value = serde_json::from_str(&notif.params.to_string()).unwrap();
    let diags = params["diagnostics"].as_array().unwrap();
    assert!(
      !diags.is_empty(),
      "missing required field should produce at least one diagnostic"
    );
    let codes: Vec<&str> = diags
      .iter()
      .filter_map(|diag| diag["code"].as_str())
      .collect();
    assert!(
      codes.contains(&"missing-required-field"),
      "expected a missing-required-field diagnostic, got codes: {codes:?}"
    );
  }
}
