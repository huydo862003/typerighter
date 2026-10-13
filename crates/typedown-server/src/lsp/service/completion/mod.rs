pub mod config;
pub mod typedown;

use lsp_types::{CompletionParams, CompletionResponse};

use crate::core::analysis::Analysis;
use crate::core::utils::uri::uri_to_path;

pub fn resolve_completion(
  analysis: &Analysis,
  params: CompletionParams,
) -> Option<CompletionResponse> {
  let path = uri_to_path(&params.text_document_position.text_document.uri)?;

  if path
    .file_name()
    .is_some_and(|name| name == "typedown.yaml" || name == "typedown.yml")
  {
    return config::resolve_completion(analysis, params);
  }

  typedown::resolve_completion(analysis, params)
}
