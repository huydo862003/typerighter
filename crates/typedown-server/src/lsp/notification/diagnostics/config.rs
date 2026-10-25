use lsp_server::Notification;
use lsp_types::notification::{Notification as _, PublishDiagnostics};
use lsp_types::{Diagnostic, DiagnosticSeverity, NumberOrString, PublishDiagnosticsParams};
use ropey::Rope;
use typedown_lang::db::derived::get_vault_config::get_vault_config;
use typedown_lang::syntax::diagnostic::TdDiagnostic;

use crate::core::analysis::Analysis;
use crate::core::utils::position::text_offset_to_lsp_position;
use crate::core::utils::uri::path_to_uri;

pub fn publish_diagnostics(analysis: &Analysis) -> Vec<Notification> {
  let db = &analysis.db;
  let project = analysis.project;
  let root = project.root_dir(db);

  // Find the vault config file path
  let config_path = [root.join("typedown.yaml"), root.join("typedown.yml")]
    .into_iter()
    .find(|path| project.files(db).contains_key(path));

  let Some(config_path) = config_path else {
    return vec![];
  };

  let rope = match analysis.file_rope(&config_path) {
    Some(rope) => rope,
    None => return vec![],
  };

  let config_result = get_vault_config(db, project);
  let lsp_diagnostics = config_result
    .diagnostics(db)
    .iter()
    .map(|diagnostic| to_lsp_diagnostic(diagnostic, &rope))
    .collect();

  let scheme = analysis
    .scheme_map
    .get(&config_path)
    .map(String::as_str)
    .unwrap_or("file");
  let uri = path_to_uri(&config_path, scheme);

  vec![Notification::new(
    PublishDiagnostics::METHOD.to_string(),
    PublishDiagnosticsParams {
      uri,
      diagnostics: lsp_diagnostics,
      version: None,
    },
  )]
}

fn to_lsp_diagnostic(diag: &TdDiagnostic, rope: &Rope) -> Diagnostic {
  let (start_offset, end_offset) = diag.offsets();

  let start_offset = start_offset.min(rope.len_chars());
  let end_offset = end_offset.min(rope.len_chars());

  let range = lsp_types::Range {
    start: text_offset_to_lsp_position(rope, start_offset),
    end: text_offset_to_lsp_position(rope, end_offset),
  };

  Diagnostic {
    range,
    severity: Some(DiagnosticSeverity::ERROR),
    code: Some(NumberOrString::String(diag.code_str().into())),
    source: Some("typedown".into()),
    message: diag.message(),
    ..Default::default()
  }
}
