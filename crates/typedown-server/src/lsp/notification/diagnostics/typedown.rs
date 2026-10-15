use std::path::Path;

use lsp_server::Notification;
use lsp_types::notification::{Notification as _, PublishDiagnostics};
use lsp_types::{Diagnostic, NumberOrString, PublishDiagnosticsParams};
use ropey::Rope;
use typedown_lang::db::TypedownDatabase;
use typedown_lang::db::derived::check_schemas::check_schemas;
use typedown_lang::db::derived::evaluate::evaluate_config::evaluate_config;
use typedown_lang::db::derived::evaluate::evaluate_resource::evaluate_resource;
use typedown_lang::db::derived::evaluate::evaluate_typ::evaluate_typ;
use typedown_lang::db::derived::get_vault_config::get_vault_config;
use typedown_lang::db::derived::name_resolver::file_symbol::file_symbol;
use typedown_lang::db::derived::parse_file::parse_file;
use typedown_lang::db::derived::typechecker::typecheck::typecheck;
use typedown_lang::db::types::{File, Project, ProjectRedNode};
use typedown_lang::db::utils::is_content_file;
use typedown_lang::integrations::lint::lint_markdown;
use typedown_lang::syntax::ast::{AstNode, SourceFile};
use typedown_lang::syntax::diagnostic::{TdDiagnostic, TdDiagnosticKind};

use crate::core::analysis::Analysis;
use crate::core::utils::position::text_offset_to_lsp_position;
use crate::core::utils::uri::path_to_uri;

use super::to_lsp_diagnostic;

pub fn publish_diagnostics_for_project(analysis: &Analysis) -> Vec<Notification> {
  let db = &analysis.db;
  let project = analysis.project;
  let files = project.files(db);

  let mut notifications = Vec::new();
  for (path, file) in &*files {
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

  // Config file diagnostics (typedown.yaml)
  let config_result = evaluate_config(db, project);
  let config_diagnostics = config_result.diagnostics(db);
  if !config_diagnostics.is_empty() {
    let root_dir = project.root_dir(db);
    let config_path = root_dir.join("typedown.yaml");
    let alt_path = root_dir.join("typedown.yml");
    let path = if files.contains_key(&config_path) {
      &config_path
    } else {
      &alt_path
    };
    let scheme = analysis
      .scheme_map
      .get(path)
      .map(|scheme| scheme.as_str())
      .unwrap_or("file");
    let uri = path_to_uri(path, scheme);
    let diagnostics: Vec<Diagnostic> = config_diagnostics
      .iter()
      .map(|diag| Diagnostic {
        range: lsp_types::Range::default(),
        severity: Some(lsp_types::DiagnosticSeverity::ERROR),
        code: Some(NumberOrString::String(diag.code_str().into())),
        source: Some("typedown".into()),
        message: diag.message(),
        ..Default::default()
      })
      .collect();
    notifications.push(Notification::new(
      PublishDiagnostics::METHOD.to_string(),
      PublishDiagnosticsParams {
        uri,
        diagnostics,
        version: None,
      },
    ));
  }

  // Check for duplicate schema files
  let schema_check = check_schemas(db, project);
  for diag in schema_check.diagnostics(db) {
    if let TdDiagnostic {
      kind: TdDiagnosticKind::DuplicateSchemaName { ref path, .. },
      ..
    } = diag
    {
      let root_dir = get_vault_config(db, project).root_dir(db);
      let full_path = root_dir.join(path);
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
            code: Some(lsp_types::NumberOrString::String(diag.code_str().into())),
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
  let root = parse_result.ast(db).node.clone();
  let project_node = ProjectRedNode::new(project, file, root);
  let typecheck_result = typecheck(db, project_node);
  td_diags.extend(typecheck_result.diagnostics(db).iter().cloned());

  // Evaluation errors
  if let Some(sym) = file_symbol(db, project, file).value(db) {
    if sym.kind(db).is_schema() {
      let eval_result = evaluate_typ(db, sym);
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
  let root = parse_result.ast(db).node.clone();
  if let Some(body) = SourceFile::cast(root.clone()).and_then(|sf| sf.body()) {
    for diag in lint_markdown(&body) {
      lsp_diags.push(lint_diag_to_lsp(&diag, rope));
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

fn lint_diag_to_lsp(lint: &TdDiagnostic, rope: &Rope) -> Diagnostic {
  let (start, end) = lint.offsets();
  let start = start.min(rope.len_chars());
  let end = end.min(rope.len_chars());
  Diagnostic {
    range: lsp_types::Range {
      start: text_offset_to_lsp_position(rope, start),
      end: text_offset_to_lsp_position(rope, end),
    },
    severity: Some(lsp_types::DiagnosticSeverity::WARNING),
    code: Some(NumberOrString::String(lint.code_str().into())),
    source: Some("typedown".into()),
    message: lint.message(),
    ..Default::default()
  }
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
  root_dir: "."
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
    let test_path = root.join("file.td");

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
        root.join("_types/Person.td"),
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
      (root.join("_types/Person.td"), person_file),
      (test_path, test_file),
    ]);

    let project = Project::new(&db, root, files, None);
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
    // Only the content file should have a notification; it must be empty
    let content_notif = notifications
      .iter()
      .find(|notif| notif.params.to_string().contains("file.td"));
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
    // _type references a schema that does not exist
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
      .find(|notif| notif.params.to_string().contains("file.td"));
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
    // Required field 'age' is absent
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
      .find(|notif| notif.params.to_string().contains("file.td"));
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

  #[test]
  fn meta_with_wrong_type_produces_diagnostic() {
    let analysis = setup(
      r#"---
_type: Person
_meta:
  title: 42
name: "Alice"
age: 30
---
"#,
    );
    let notifications = publish_diagnostics_for_project(&analysis);
    let content_notif = notifications
      .iter()
      .find(|notif| notif.params.to_string().contains("file.td"));
    let notif = content_notif.expect("expected a notification for the content file");
    let params: serde_json::Value = serde_json::from_str(&notif.params.to_string()).unwrap();
    let diags = params["diagnostics"].as_array().unwrap();
    assert!(
      !diags.is_empty(),
      "_meta.title with wrong type should produce a diagnostic"
    );
  }

  #[test]
  fn valid_meta_produces_no_diagnostic() {
    let analysis = setup(
      r#"---
_type: Person
_meta:
  title: "Custom Title"
  description: "Custom Description"
name: "Alice"
age: 30
---
"#,
    );
    let notifications = publish_diagnostics_for_project(&analysis);
    let content_notif = notifications
      .iter()
      .find(|notif| notif.params.to_string().contains("file.td"));
    if let Some(notif) = content_notif {
      let params: serde_json::Value = serde_json::from_str(&notif.params.to_string()).unwrap();
      let diags = params["diagnostics"].as_array().unwrap();
      assert!(
        diags.is_empty(),
        "valid _meta should produce no diagnostics, got: {diags:?}"
      );
    }
  }

  #[test]
  fn invalid_config_produces_diagnostic() {
    let root = PathBuf::from(if cfg!(windows) { "C:\\vault" } else { "/vault" });
    let db = TypedownDatabase {
      storage: QueryStorage::default(),
    };

    // Config with wrong type for version (number instead of string)
    let config_file = File::new(
      &db,
      FileHandle::Content(
        root.join("typedown.yaml"),
        "version: 42\n".to_string(),
        FileMetadata::default(),
      ),
    );

    let files = HashMap::from([(root.join("typedown.yaml"), config_file)]);
    let project = Project::new(&db, root, files, None);
    let analysis = Analysis::new(
      db,
      project,
      Arc::new(HashMap::new()),
      Arc::new(HashMap::new()),
      Arc::new((Mutex::new(1), Condvar::new())),
    );

    let notifications = publish_diagnostics_for_project(&analysis);
    let config_notif = notifications
      .iter()
      .find(|notif| notif.params.to_string().contains("typedown.yaml"));
    let notif = config_notif.expect("expected a diagnostic for the config file");
    let params: serde_json::Value = serde_json::from_str(&notif.params.to_string()).unwrap();
    let diags = params["diagnostics"].as_array().unwrap();
    assert!(
      !diags.is_empty(),
      "invalid config should produce at least one diagnostic"
    );
  }

  fn setup_with_contract(content: &str, contract_json: &str) -> Analysis {
    let root = PathBuf::from(if cfg!(windows) { "C:\\vault" } else { "/vault" });
    let test_path = root.join("file.td");
    let contract_path = root.join(".typedown/components.json");

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
    let test_file = File::new(
      &db,
      FileHandle::Content(
        test_path.clone(),
        content.to_string(),
        FileMetadata::default(),
      ),
    );
    let contract_file = File::new(
      &db,
      FileHandle::Content(
        contract_path.clone(),
        contract_json.to_string(),
        FileMetadata::default(),
      ),
    );

    let files = HashMap::from([
      (root.join("typedown.yaml"), config_file),
      (test_path, test_file),
      (contract_path, contract_file),
    ]);

    let project = Project::new(&db, root, files, Some(contract_file));
    Analysis::new(
      db,
      project,
      Arc::new(HashMap::new()),
      Arc::new(HashMap::new()),
      Arc::new((Mutex::new(1), Condvar::new())),
    )
  }

  #[test]
  fn unknown_component_produces_diagnostic() {
    let analysis = setup_with_contract(
      ":::my-widget\n:::\n",
      r#"{"KnownWidget":{"source":"./KnownWidget.vue","props":{}}}"#,
    );
    let notifications = publish_diagnostics_for_project(&analysis);
    let notif = notifications
      .iter()
      .find(|n| n.params.to_string().contains("file.td"))
      .expect("expected notification for file.td");
    let params: serde_json::Value = serde_json::from_str(&notif.params.to_string()).unwrap();
    let codes: Vec<&str> = params["diagnostics"]
      .as_array()
      .unwrap()
      .iter()
      .filter_map(|d| d["code"].as_str())
      .collect();
    assert!(
      codes.contains(&"unknown-component"),
      "expected unknown-component diagnostic, got: {codes:?}"
    );
  }

  #[test]
  fn missing_required_prop_produces_diagnostic() {
    let contract =
      r#"{"Chart":{"source":"./Chart.vue","props":{"data":{"schema":{"type":"string"}}}}}"#;
    let analysis = setup_with_contract(
      r#":::chart
:::
"#,
      contract,
    );
    let notifications = publish_diagnostics_for_project(&analysis);
    let notif = notifications
      .iter()
      .find(|n| n.params.to_string().contains("file.td"))
      .expect("expected notification for file.td");
    let params: serde_json::Value = serde_json::from_str(&notif.params.to_string()).unwrap();
    let codes: Vec<&str> = params["diagnostics"]
      .as_array()
      .unwrap()
      .iter()
      .filter_map(|d| d["code"].as_str())
      .collect();
    assert!(
      codes.contains(&"missing-required-prop"),
      "expected missing-required-prop diagnostic, got: {codes:?}"
    );
  }

  #[test]
  fn known_component_produces_no_component_diagnostic() {
    let contract = r#"{"Chart":{"source":"./Chart.vue","props":{}}}"#;
    let analysis = setup_with_contract(":::chart\n:::\n", contract);
    let notifications = publish_diagnostics_for_project(&analysis);
    let notif = notifications
      .iter()
      .find(|n| n.params.to_string().contains("file.td"));
    if let Some(notif) = notif {
      let params: serde_json::Value = serde_json::from_str(&notif.params.to_string()).unwrap();
      let component_codes: Vec<&str> = params["diagnostics"]
        .as_array()
        .unwrap()
        .iter()
        .filter_map(|d| d["code"].as_str())
        .filter(|c| {
          matches!(
            *c,
            "unknown-component" | "missing-required-prop" | "unknown-prop"
          )
        })
        .collect();
      assert!(
        component_codes.is_empty(),
        "valid component usage should produce no component diagnostics, got: {component_codes:?}"
      );
    }
  }

  #[test]
  fn unknown_prop_produces_diagnostic() {
    let contract = r#"{"Chart":{"source":"./Chart.vue","props":{}}}"#;
    let analysis = setup_with_contract(":::chart {extra=\"val\"}\n:::\n", contract);
    let notifications = publish_diagnostics_for_project(&analysis);
    let notif = notifications
      .iter()
      .find(|n| n.params.to_string().contains("file.td"))
      .expect("expected notification for file.td");
    let params: serde_json::Value = serde_json::from_str(&notif.params.to_string()).unwrap();
    let codes: Vec<&str> = params["diagnostics"]
      .as_array()
      .unwrap()
      .iter()
      .filter_map(|d| d["code"].as_str())
      .collect();
    assert!(
      codes.contains(&"unknown-prop"),
      "expected unknown-prop diagnostic, got: {codes:?}"
    );
  }

  #[test]
  fn valid_config_produces_no_config_diagnostic() {
    let analysis = setup(
      r#"---
_type: Person
name: "Alice"
age: 30
---
"#,
    );
    let notifications = publish_diagnostics_for_project(&analysis);
    let config_notif = notifications
      .iter()
      .find(|notif| notif.params.to_string().contains("typedown.yaml"));
    // Config notification should either not exist or have empty diagnostics
    if let Some(notif) = config_notif {
      let params: serde_json::Value = serde_json::from_str(&notif.params.to_string()).unwrap();
      let diags = params["diagnostics"].as_array().unwrap();
      assert!(
        diags.is_empty(),
        "valid config should produce no diagnostics, got: {diags:?}"
      );
    }
  }
}
