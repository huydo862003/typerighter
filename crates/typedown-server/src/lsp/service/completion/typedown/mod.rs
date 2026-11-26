//! Completion for `.td` files
//!
//! Read the flow top to bottom:
//! 1. `CursorContext` (shared, `lsp/service/utils/cursor`) resolves the request to file, offset and node
//! 2. `position` classifies where the cursor is into exactly one `CursorPosition`
//! 3. `providers` builds the items for that position (`providers::complete` is the dispatch)
//!
//! Providers that need "what type is expected here" use `lsp/service/utils/expected_typ`
//!
//! To add a completion: add a `CursorPosition` variant, detect it in `position::classify`, handle it in `providers::complete`

mod items;
mod position;
mod providers;
#[cfg(test)]
mod tests;

use lsp_types::{CompletionParams, CompletionResponse};

use crate::core::analysis::Analysis;
use crate::lsp::service::utils::cursor::CursorContext;

pub fn resolve_completion(
  analysis: &Analysis,
  params: CompletionParams,
) -> Option<CompletionResponse> {
  let position_params = &params.text_document_position;
  let ctx = CursorContext::new(
    analysis,
    &position_params.text_document.uri,
    position_params.position,
  )?;
  let position = position::classify(&ctx)?;
  providers::complete(&ctx, position).map(CompletionResponse::Array)
}
