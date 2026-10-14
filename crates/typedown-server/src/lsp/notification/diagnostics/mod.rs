pub mod config;
pub mod typedown;

use std::path::Path;

use lsp_server::Notification;
use lsp_types::{Diagnostic, DiagnosticSeverity, NumberOrString};
use ropey::Rope;
use typedown_lang::syntax::diagnostic::TdDiagnostic;

use crate::core::analysis::Analysis;
use crate::core::utils::position::text_offset_to_lsp_position;

pub fn publish_diagnostics(analysis: &Analysis) -> Vec<Notification> {
  let mut notifications = typedown::publish_diagnostics_for_project(analysis);
  notifications.extend(config::publish_diagnostics(analysis));
  notifications
}

pub fn publish_diagnostics_for_file(analysis: &Analysis, path: &Path) -> Vec<Notification> {
  typedown::publish_diagnostics_for_file(analysis, path)
}

// Convert our diagnostics to lsp diagnostics
pub(super) fn to_lsp_diagnostic(diag: &TdDiagnostic, rope: &Rope) -> Option<Diagnostic> {
  let (start_offset, end_offset) = diag.offsets();

  let start_offset = start_offset.min(rope.len_chars());
  let end_offset = end_offset.min(rope.len_chars());

  let range = lsp_types::Range {
    start: text_offset_to_lsp_position(rope, start_offset),
    end: text_offset_to_lsp_position(rope, end_offset),
  };

  Some(Diagnostic {
    range,
    severity: Some(DiagnosticSeverity::ERROR),
    code: Some(NumberOrString::String(diag.code_str().into())),
    source: Some("typedown".into()),
    message: diag.message(),
    ..Default::default()
  })
}
