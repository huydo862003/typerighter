pub mod code_action;
pub mod commands;
pub mod completion;
pub mod definition;
pub mod formatting;
pub mod hover;
pub mod inlay_hint;
pub mod references;
pub mod rename_symbol;
pub mod semantic_tokens;
pub mod utils;

use lsp_server::{ErrorCode, Request, Response};
use lsp_types::request::{
  CodeActionRequest, Completion, Formatting, GotoDefinition, HoverRequest, InlayHintRequest,
  PrepareRenameRequest, References, Rename, Request as _, SemanticTokensFullRequest,
  WillRenameFiles,
};
use serde::Serialize;
use serde::de::DeserializeOwned;
use serde_json::Value;

use crate::core::analysis::Analysis;

/// Dispatch an LSP request to the appropriate service handler
pub fn dispatch(analysis: &Analysis, request: Request) -> Response {
  match request.method.as_str() {
    CodeActionRequest::METHOD => try_handle(&request, |params| {
      code_action::resolve_code_action(analysis, params)
    }),
    HoverRequest::METHOD => try_handle(&request, |params| hover::resolve_hover(analysis, params)),
    Completion::METHOD => try_handle(&request, |params| {
      completion::resolve_completion(analysis, params)
    }),
    GotoDefinition::METHOD => try_handle(&request, |params| {
      definition::resolve_definition(analysis, params)
    }),
    References::METHOD => try_handle(&request, |params| {
      references::find_references(analysis, params)
    }),
    SemanticTokensFullRequest::METHOD => try_handle(&request, |params| {
      semantic_tokens::resolve_semantic_tokens_full(analysis, params)
    }),
    PrepareRenameRequest::METHOD => try_handle(&request, |params| {
      rename_symbol::prepare_rename(analysis, params)
    }),
    Rename::METHOD => try_handle(&request, |params| rename_symbol::rename(analysis, params)),
    Formatting::METHOD => try_handle(&request, |params| {
      formatting::format_document(analysis, params)
    }),
    InlayHintRequest::METHOD => try_handle(&request, |params| {
      inlay_hint::resolve_inlay_hints(analysis, params)
    }),
    WillRenameFiles::METHOD => try_handle(&request, |params| {
      rename_symbol::will_rename_files(analysis, params)
    }),
    _ => Response::new_err(
      request.id,
      ErrorCode::MethodNotFound as i32,
      format!("unhandled method: {}", request.method),
    ),
  }
}

// Deserialize params and call the handler
// Returns null on deserialization failure so the client always gets a valid reply
fn try_handle<P: DeserializeOwned, R: Serialize>(
  req: &Request,
  handler: impl FnOnce(P) -> Option<R>,
) -> Response {
  match serde_json::from_value::<P>(req.params.clone()) {
    Ok(params) => Response::new_ok(req.id.clone(), handler(params)),
    Err(err) => {
      log::warn!("Failed to deserialize {} params: {err}", req.method);
      Response::new_ok(req.id.clone(), Value::Null)
    }
  }
}
