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

/// Dispatch an LSP request to the appropriate service handler.
pub fn dispatch(analysis: &Analysis, req: Request) -> Response {
  match req.method.as_str() {
    CodeActionRequest::METHOD => try_handle(&req, |p| code_action::code_action(analysis, p)),
    HoverRequest::METHOD => try_handle(&req, |p| hover::hover(analysis, p)),
    Completion::METHOD => try_handle(&req, |p| completion::completion(analysis, p)),
    GotoDefinition::METHOD => try_handle(&req, |p| definition::definition(analysis, p)),
    References::METHOD => try_handle(&req, |p| references::find_references(analysis, p)),
    SemanticTokensFullRequest::METHOD => {
      try_handle(&req, |p| semantic_tokens::semantic_tokens_full(analysis, p))
    }
    PrepareRenameRequest::METHOD => {
      try_handle(&req, |p| rename_symbol::prepare_rename(analysis, p))
    }
    Rename::METHOD => try_handle(&req, |p| rename_symbol::rename(analysis, p)),
    Formatting::METHOD => try_handle(&req, |p| formatting::formatting(analysis, p)),
    InlayHintRequest::METHOD => try_handle(&req, |p| inlay_hint::inlay_hints(analysis, p)),
    WillRenameFiles::METHOD => try_handle(&req, |p| rename_symbol::will_rename_files(analysis, p)),
    _ => Response::new_err(
      req.id,
      ErrorCode::MethodNotFound as i32,
      format!("unhandled method: {}", req.method),
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
