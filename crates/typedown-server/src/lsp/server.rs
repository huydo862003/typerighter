use std::path::{Path, PathBuf};
use std::sync::Arc;

use anyhow::Error;
use ropey::Rope;
use threadpool::ThreadPool;
use typedown_incremental::Cancelled;
use typedown_lang::db::utils::is_type_file;

use lsp_server::{Connection, Message, Notification, Request, RequestId, Response};
use lsp_types::notification::{
  DidChangeTextDocument, DidChangeWatchedFiles, DidCloseTextDocument, DidOpenTextDocument,
  DidRenameFiles, Notification as NotificationTrait,
};
use lsp_types::request::{ExecuteCommand, RegisterCapability, Request as RequestTrait};
use lsp_types::{
  ClientCapabilities, DidChangeTextDocumentParams, DidChangeWatchedFilesParams,
  DidChangeWatchedFilesRegistrationOptions, DidCloseTextDocumentParams, DidOpenTextDocumentParams,
  FileChangeType, FileSystemWatcher, GlobPattern, Registration, RegistrationParams,
  RenameFilesParams, TextDocumentContentChangeEvent, WatchKind,
};

use crate::core::analysis::Analysis;
use crate::core::analysis_host::AnalysisHost;
use crate::core::multiproject::{Multiproject, ProjectEntry};
use crate::core::utils::lsp::{try_extract_path_from_notification, try_extract_path_from_request};
use crate::core::utils::uri::uri_to_path;
use crate::lsp::notification::diagnostics::{publish_diagnostics, publish_diagnostics_for_file};
use crate::lsp::service::commands::create_linked_resource::{
  self as create_linked, CreateLinkedResourceArgs,
};
use crate::lsp::service::{self, commands};

pub struct Server {
  connection: Connection,
  multiproject: Multiproject,
  client_capabilities: ClientCapabilities,
  thread_pool: ThreadPool,
}

impl Server {
  pub fn new(
    connection: Connection,
    multiproject: Multiproject,
    client_capabilities: ClientCapabilities,
  ) -> Self {
    let num_threads = std::thread::available_parallelism()
      .map(|count| count.get().min(4))
      .unwrap_or(2);
    Self {
      connection,
      multiproject,
      client_capabilities,
      thread_pool: ThreadPool::new(num_threads),
    }
  }

  pub fn save(self) {
    // Wait for in-flight requests and diagnostics to finish
    self.thread_pool.join();
    self.multiproject.save();
  }

  /// Run the server event loop until the client sends a shutdown request.
  pub fn run(&self) -> anyhow::Result<()> {
    self.register_file_watcher()?;
    self.register_file_operations()?;

    for msg in &self.connection.receiver {
      match msg {
        Message::Request(req) => {
          if let Err(err) = self.handle_request(req) {
            log::error!("Failed to handle request: {err}");
          }
        }
        Message::Notification(note) => {
          if let Err(err) = self.handle_notification(note) {
            log::error!("Failed to handle notification: {err}");
          }
        }
        Message::Response(_) => {}
      }
    }
    Ok(())
  }

  fn handle_request(&self, req: Request) -> anyhow::Result<()> {
    if self.connection.handle_shutdown(&req)? {
      return Ok(());
    }

    // executeCommand needs the connection for sending applyEdit/showDocument back
    if req.method == ExecuteCommand::METHOD {
      return self.handle_execute_command(req);
    }

    // Resolve to the owning project
    let uri = try_extract_path_from_request(&req)?;
    let path = uri_to_path(&uri).ok_or_else(|| Error::msg("Failed to convert URI to path"))?;
    let project_entry = self.multiproject.load_nearest_project(&path)?;

    let analysis = project_entry
      .host
      .read()
      .map_err(|_| Error::msg("project_entry.host RwLock is poisoned"))?
      .snapshot();

    // Dispatch to thread pool so the main loop stays responsive
    // Cancelled::catch handles the case where a didChange cancels in-flight queries
    let sender = self.connection.sender.clone();
    let request_id = req.id.clone();
    self.thread_pool.execute(move || {
      let resp = match Cancelled::catch(|| service::dispatch(&analysis, req)) {
        Ok(resp) => resp,
        // A didChange arrived and cancelled this query via the DB's cancelled flag
        Err(_) => Response::new_err(
          request_id,
          lsp_server::ErrorCode::ContentModified as i32,
          "request cancelled: content modified".to_string(),
        ),
      };
      if let Err(err) = sender.send(Message::Response(resp)) {
        log::error!("Failed to send response: {err}");
      }
    });

    Ok(())
  }

  fn handle_execute_command(&self, req: Request) -> anyhow::Result<()> {
    let params: lsp_types::ExecuteCommandParams = serde_json::from_value(req.params)?;

    match params.command.as_str() {
      commands::CREATE_LINKED_RESOURCE => {
        let args: CreateLinkedResourceArgs = serde_json::from_value(
          params
            .arguments
            .into_iter()
            .next()
            .ok_or_else(|| Error::msg("missing arguments"))?,
        )?;

        // Resolve the project from the source URI
        let source_uri: lsp_types::Uri = args
          .source_uri
          .parse()
          .map_err(|_| Error::msg("invalid source URI"))?;
        let path =
          uri_to_path(&source_uri).ok_or_else(|| Error::msg("Failed to convert URI to path"))?;
        let project_entry = self.multiproject.load_nearest_project(&path)?;
        let analysis = project_entry
          .host
          .read()
          .map_err(|_| Error::msg("RwLock poisoned"))?
          .snapshot();

        if let Err(err) = create_linked::execute(&analysis, args, &self.connection) {
          log::error!("createLinkedResource failed: {err}");
        }

        // Send success response
        self
          .connection
          .sender
          .send(Message::Response(Response::new_ok(req.id, ())))?;
      }
      _ => {
        self
          .connection
          .sender
          .send(Message::Response(Response::new_err(
            req.id,
            lsp_server::ErrorCode::InvalidParams as i32,
            format!("unknown command: {}", params.command),
          )))?;
      }
    }

    Ok(())
  }

  fn handle_notification(&self, note: Notification) -> anyhow::Result<()> {
    // DidChangeWatchedFiles can contain changes spanning multiple projects
    // Route each change to its own project independently
    if note.method == DidChangeWatchedFiles::METHOD {
      let params = serde_json::from_value::<DidChangeWatchedFilesParams>(note.params.clone())?;
      // Collect affected projects so we push diagnostics once per project, not per file
      let mut affected_projects = Vec::new();
      for change in params.changes {
        let Some(path) = uri_to_path(&change.uri) else {
          log::warn!(
            "Could not convert watched file URI to path: {}",
            change.uri.as_str()
          );
          continue;
        };
        let project_entry = match self.multiproject.load_nearest_project(&path) {
          Ok(entry) => entry,
          Err(err) => {
            log::warn!(
              "No project found for watched file {}: {err}",
              path.display()
            );
            continue;
          }
        };
        {
          let mut host = project_entry
            .host
            .write()
            .expect("RwLock should not be poisoned");
          match change.typ {
            FileChangeType::CREATED | FileChangeType::CHANGED => {
              host.on_disk_change(path);
            }
            FileChangeType::DELETED => {
              host.on_disk_delete(path);
            }
            _ => {}
          }
        }
        if !affected_projects
          .iter()
          .any(|p: &Arc<ProjectEntry>| p.root_dir == project_entry.root_dir)
        {
          affected_projects.push(project_entry);
        }
      }
      for project_entry in &affected_projects {
        self.send_diagnostics_async(project_entry, None);
      }
      return Ok(());
    }

    // workspace/didRenameFiles: update the project state for each renamed file
    if note.method == DidRenameFiles::METHOD {
      let params = serde_json::from_value::<RenameFilesParams>(note.params.clone())?;
      let mut affected_projects = Vec::new();
      for file_rename in &params.files {
        let old_uri: lsp_types::Uri = match file_rename.old_uri.parse() {
          Ok(uri) => uri,
          Err(_) => continue,
        };
        let new_uri: lsp_types::Uri = match file_rename.new_uri.parse() {
          Ok(uri) => uri,
          Err(_) => continue,
        };
        let (Some(old_path), Some(new_path)) = (uri_to_path(&old_uri), uri_to_path(&new_uri))
        else {
          continue;
        };
        let project_entry = match self.multiproject.load_nearest_project(&old_path) {
          Ok(entry) => entry,
          Err(_) => continue,
        };
        project_entry
          .host
          .write()
          .expect("RwLock should not be poisoned")
          .on_did_rename_file(old_path, new_path);
        if !affected_projects
          .iter()
          .any(|p: &Arc<ProjectEntry>| p.root_dir == project_entry.root_dir)
        {
          affected_projects.push(project_entry);
        }
      }
      for project_entry in &affected_projects {
        self.send_diagnostics_async(project_entry, None);
      }
      return Ok(());
    }

    // For other notifications, extract the document URI and route to a single project
    let uri = try_extract_path_from_notification(&note)?;
    let path = uri_to_path(&uri).ok_or_else(|| Error::msg("Failed to convert URI to path"))?;
    let project_entry = self.multiproject.load_nearest_project(&path)?;

    let method = note.method.clone();

    let analysis = {
      let mut host = project_entry
        .host
        .write()
        .expect("RwLock should not be poisoned");
      handle_text_notification(&mut host, &note)?;
      host.snapshot()
    };

    if method == DidOpenTextDocument::METHOD {
      // didOpen: Full project diagnostics so cross-file errors show immediately
      self.send_diagnostics_with_snapshot(analysis, None);
    } else if method == DidChangeTextDocument::METHOD {
      // didChange: Full diagnostics for schema files (affects all referencing content files), single-file diagnostics for content files for responsiveness
      let is_schema = is_type_file(&path);
      self.send_diagnostics_with_snapshot(analysis, if is_schema { None } else { Some(path) });
    }
    // didClose: No diagnostics needed

    Ok(())
  }

  // Compute and send diagnostics on a worker thread using an existing snapshot
  fn send_diagnostics_with_snapshot(&self, analysis: Analysis, path: Option<PathBuf>) {
    let sender = self.connection.sender.clone();
    self.thread_pool.execute(move || {
      // Silently drop if cancelled by a newer didChange
      let Ok(notifications) = Cancelled::catch(|| match path.as_deref() {
        Some(path) => publish_diagnostics_for_file(&analysis, path),
        None => publish_diagnostics(&analysis),
      }) else {
        return;
      };
      for notif in notifications {
        if let Err(err) = sender.send(Message::Notification(notif)) {
          log::error!("Failed to send diagnostics: {err}");
          break;
        }
      }
    });
  }

  // Take a fresh snapshot and send diagnostics on a worker thread
  fn send_diagnostics_async(&self, project_entry: &ProjectEntry, path: Option<&Path>) {
    let analysis = project_entry
      .host
      .read()
      .expect("RwLock should not be poisoned")
      .snapshot();
    let path = path.map(Path::to_path_buf);
    self.send_diagnostics_with_snapshot(analysis, path);
  }

  /* File watcher */
  fn register_file_watcher(&self) -> anyhow::Result<()> {
    let supports_dynamic = self
      .client_capabilities
      .workspace
      .as_ref()
      .and_then(|workspace| workspace.did_change_watched_files.as_ref())
      .and_then(|cap| cap.dynamic_registration)
      .unwrap_or(false);

    if !supports_dynamic {
      return Ok(());
    }

    // Watch all relevant files
    let watchers = vec![
      FileSystemWatcher {
        glob_pattern: GlobPattern::String("**/*.td".to_string()),
        kind: Some(WatchKind::all()),
      },
      FileSystemWatcher {
        glob_pattern: GlobPattern::String("**/*.md".to_string()),
        kind: Some(WatchKind::all()),
      },
      FileSystemWatcher {
        glob_pattern: GlobPattern::String("**/*.{pdf,svg,png,jpg,jpeg,webp}".to_string()),
        kind: Some(WatchKind::all()),
      },
      FileSystemWatcher {
        glob_pattern: GlobPattern::String("**/typedown.yaml".to_string()),
        kind: Some(WatchKind::all()),
      },
      FileSystemWatcher {
        glob_pattern: GlobPattern::String("**/typedown.yml".to_string()),
        kind: Some(WatchKind::all()),
      },
    ];

    let registration = Registration {
      id: "typedown-file-watcher".to_string(),
      method: DidChangeWatchedFiles::METHOD.to_string(),
      register_options: Some(serde_json::to_value(
        DidChangeWatchedFilesRegistrationOptions { watchers },
      )?),
    };

    let req = Request::new(
      RequestId::from("typedown-register-watcher".to_string()),
      RegisterCapability::METHOD.to_string(),
      RegistrationParams {
        registrations: vec![registration],
      },
    );

    self.connection.sender.send(Message::Request(req))?;
    Ok(())
  }

  /// Dynamically register for workspace/willRenameFiles  because some editors (Vscode) ignore static fileOperations capabilities when dynamicRegistration is true
  fn register_file_operations(&self) -> anyhow::Result<()> {
    let supports_dynamic = self
      .client_capabilities
      .workspace
      .as_ref()
      .and_then(|ws| ws.file_operations.as_ref())
      .and_then(|fo| fo.dynamic_registration)
      .unwrap_or(false);

    if !supports_dynamic {
      return Ok(());
    }

    let file_filter = serde_json::json!({
      "filters": [{
        "pattern": { "glob": "**/*.td" }
      }, {
        "pattern": { "glob": "**/*.md" }
      }]
    });

    let registrations = vec![
      Registration {
        id: "typedown-will-rename".to_string(),
        method: "workspace/willRenameFiles".to_string(),
        register_options: Some(file_filter.clone()),
      },
      Registration {
        id: "typedown-did-rename".to_string(),
        method: "workspace/didRenameFiles".to_string(),
        register_options: Some(file_filter),
      },
    ];

    let req = Request::new(
      RequestId::from("typedown-register-file-ops".to_string()),
      RegisterCapability::METHOD.to_string(),
      RegistrationParams { registrations },
    );

    self.connection.sender.send(Message::Request(req))?;
    Ok(())
  }
}

/// Handle text document notifications (open, change, close)
fn handle_text_notification(host: &mut AnalysisHost, note: &Notification) -> anyhow::Result<()> {
  match note.method.as_str() {
    // Editor opened a file: take ownership of its content from the editor buffer
    DidOpenTextDocument::METHOD => {
      let params = serde_json::from_value::<DidOpenTextDocumentParams>(note.params.clone())?;
      host.on_editor_open_file(&params.text_document.uri, params.text_document.text);
    }
    // Editor sent incremental diffs: apply each change to the in-memory rope
    DidChangeTextDocument::METHOD => {
      let params = serde_json::from_value::<DidChangeTextDocumentParams>(note.params.clone())?;
      let path = uri_to_path(&params.text_document.uri)
        .ok_or_else(|| Error::msg("Failed to convert URI to path"))?;

      let mut rope = host.open_file_content(&path).cloned().unwrap_or_default();
      for change in params.content_changes {
        rope = apply_content_change(rope, change);
      }
      host.on_editor_change_file(path, rope);
    }
    // Editor closed the file: fall back to the on-disk version
    DidCloseTextDocument::METHOD => {
      let params = serde_json::from_value::<DidCloseTextDocumentParams>(note.params.clone())?;
      let path = uri_to_path(&params.text_document.uri)
        .ok_or_else(|| Error::msg("Failed to convert URI to path"))?;
      host.on_close_file(&path);
    }
    _ => {}
  };
  Ok(())
}

/// Apply a single incremental change to a rope
/// If the change has no range it is a full replacement
pub(crate) fn apply_content_change(mut rope: Rope, change: TextDocumentContentChangeEvent) -> Rope {
  let Some(range) = change.range else {
    return Rope::from(change.text);
  };

  let start = rope.line_to_char(range.start.line as usize) + range.start.character as usize;
  let end = rope.line_to_char(range.end.line as usize) + range.end.character as usize;
  rope.remove(start..end);
  rope.insert(start, &change.text);
  rope
}
