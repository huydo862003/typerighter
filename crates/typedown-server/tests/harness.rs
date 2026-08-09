//! Test fixture for integration-testing the LSP server over an in-memory connection.
//!
//! Usage:
//!   let server = Server::new()
//!     .file("typedown.yaml", r#"version: "1"\nvault:\n  ..."#)
//!     .file("schemas/Person.td", "---\n...")
//!     .start();
//!
//!   server.open("content/file.td", "---\n_type: Person\n---\n");
//!   let resp = server.request::<Completion>(params);

use std::cell::Cell;
use std::str::FromStr;
use std::time::Duration;

use lsp_server::{Connection, Message, Request, Response};
use lsp_types::{
  ClientCapabilities, DidOpenTextDocumentParams, InitializeParams, ServerCapabilities,
  TextDocumentItem, TextDocumentSyncCapability, TextDocumentSyncKind, Uri, WorkspaceFolder,
  notification::{DidOpenTextDocument, Notification as _},
};
use serde::Serialize;
use serde_json::Value;
use tempfile::TempDir;
use typedown_server::core::multiproject::Multiproject;
use typedown_server::lsp::server::Server as LspServer;

const TIMEOUT: Duration = Duration::from_secs(5);

/// Builder for a test server backed by an in-memory vault written to a TempDir.
pub struct ServerBuilder {
  files: Vec<(String, String)>,
}

impl ServerBuilder {
  pub fn new() -> Self {
    Self { files: vec![] }
  }

  /// Add a file to the vault. Path is relative to the vault root.
  pub fn file(mut self, path: &str, content: &str) -> Self {
    self.files.push((path.to_string(), content.to_string()));
    self
  }

  /// Write files to a TempDir and start the LSP server thread.
  pub fn start(self) -> Server {
    let dir = TempDir::new().expect("failed to create tempdir");

    // Write vault files to disk.
    for (rel_path, content) in &self.files {
      let abs_path = dir.path().join(rel_path);
      std::fs::create_dir_all(abs_path.parent().unwrap()).unwrap();
      std::fs::write(&abs_path, content).unwrap();
    }

    let root = dir.path().to_path_buf();

    // In-memory LSP connection: server side + client side.
    let (server_conn, client_conn) = Connection::memory();

    let root_clone = root.clone();
    std::thread::spawn(move || {
      let multiproject = Multiproject::default();
      multiproject.load_nearest_project(&root_clone).unwrap();

      // Perform the LSP initialize handshake using the server-side connection.
      let capabilities = ServerCapabilities {
        text_document_sync: Some(TextDocumentSyncCapability::Kind(
          TextDocumentSyncKind::INCREMENTAL,
        )),
        ..Default::default()
      };
      // Read the initialize request sent by the client below, reply with capabilities.
      let (init_id, _init_params) = server_conn
        .initialize_start()
        .expect("initialize_start failed");
      server_conn
        .initialize_finish(init_id, serde_json::json!({ "capabilities": capabilities }))
        .expect("initialize_finish failed");

      LspServer::new(server_conn, multiproject, ClientCapabilities::default())
        .run()
        .ok();
    });

    // Client side: send initialize.
    let workspace_uri = Uri::from_str(&format!("file://{}", root.display())).unwrap();
    let init_params = InitializeParams {
      capabilities: ClientCapabilities::default(),
      workspace_folders: Some(vec![WorkspaceFolder {
        uri: workspace_uri,
        name: "vault".to_string(),
      }]),
      ..Default::default()
    };
    client_conn
      .initialize(serde_json::to_value(init_params).unwrap())
      .expect("initialize failed");

    Server {
      conn: client_conn,
      req_id: Cell::new(1),
      _dir: dir,
    }
  }
}

impl Default for ServerBuilder {
  fn default() -> Self {
    Self::new()
  }
}

/// A running LSP test server connected over an in-memory channel.
pub struct Server {
  conn: Connection,
  req_id: Cell<i32>,
  _dir: TempDir,
}

impl Server {
  pub fn new() -> ServerBuilder {
    ServerBuilder::new()
  }

  /// Send a `textDocument/didOpen` notification for the given vault-relative path.
  pub fn open(&self, rel_path: &str, content: &str) {
    let uri = self.file_uri(rel_path);
    let params = DidOpenTextDocumentParams {
      text_document: TextDocumentItem {
        uri,
        language_id: "typedown".to_string(),
        version: 1,
        text: content.to_string(),
      },
    };
    let notif = lsp_server::Notification::new(
      DidOpenTextDocument::METHOD.to_string(),
      serde_json::to_value(params).unwrap(),
    );
    self.conn.sender.send(Message::Notification(notif)).unwrap();
    // Drain any diagnostics notifications the server pushes back.
    self.drain_notifications();
  }

  /// Send an LSP request and return the raw JSON response value.
  pub fn request<R>(&self, params: R::Params) -> Value
  where
    R: lsp_types::request::Request,
    R::Params: Serialize,
  {
    let id = self.req_id.get();
    self.req_id.set(id + 1);

    let req = Request::new(id.into(), R::METHOD.to_string(), params);
    self.conn.sender.send(Message::Request(req)).unwrap();

    // Read messages until we get the response with our id.
    loop {
      let msg = self
        .conn
        .receiver
        .recv_timeout(TIMEOUT)
        .expect("timeout waiting for LSP response");
      match msg {
        Message::Response(Response {
          id: resp_id,
          result,
          error,
          ..
        }) if resp_id == id.into() => {
          if let Some(err) = error {
            panic!("LSP error response: {err:?}");
          }
          return result.unwrap_or(Value::Null);
        }
        // Notifications (e.g. diagnostics) may arrive before the response.
        Message::Notification(_) => continue,
        other => panic!("unexpected message: {other:?}"),
      }
    }
  }

  /// Return the `file://` URI for a vault-relative path.
  pub fn file_uri(&self, rel_path: &str) -> Uri {
    let abs = self._dir.path().join(rel_path);
    Uri::from_str(&format!("file://{}", abs.display())).unwrap()
  }

  /// Receive and discard any queued notifications (e.g. diagnostics pushes).
  fn drain_notifications(&self) {
    while let Ok(msg) = self.conn.receiver.recv_timeout(Duration::from_millis(50)) {
      match msg {
        Message::Notification(_) => {}
        other => panic!("unexpected message while draining: {other:?}"),
      }
    }
  }
}
