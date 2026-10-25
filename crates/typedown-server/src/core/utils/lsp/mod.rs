use lsp_server::{Notification, Request};
use lsp_types::{TextDocumentIdentifier, Uri};
use serde::{Deserialize, Serialize};

/// Standard requests use this: Prepare rename, Go to definition, ..
#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct HasDocumentUri {
  pub text_document: TextDocumentIdentifier,
}

/// Some request, like willRenameFile
#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct HasFileRenames {
  pub files: Vec<FileRenameEntry>,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct FileRenameEntry {
  pub old_uri: String,
}

fn extract_uri(params: &serde_json::Value) -> anyhow::Result<Uri> {
  // Try textDocument.uri first, then files[0].oldUri
  if let Ok(doc) = serde_json::from_value::<HasDocumentUri>(params.clone()) {
    return Ok(doc.text_document.uri);
  }
  if let Ok(renames) = serde_json::from_value::<HasFileRenames>(params.clone())
    && let Some(first) = renames.files.first()
  {
    return Ok(first.old_uri.parse()?);
  }
  anyhow::bail!("cannot extract URI from params")
}

pub fn try_extract_path_from_request(req: &Request) -> anyhow::Result<Uri> {
  extract_uri(&req.params)
}

pub fn try_extract_path_from_notification(note: &Notification) -> anyhow::Result<Uri> {
  extract_uri(&note.params)
}
