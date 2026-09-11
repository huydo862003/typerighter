use std::collections::{BTreeMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;

use crossbeam_channel::Sender;
use lsp_server::{Connection, Message, Notification, Request, Response};
use notify::{Event, EventKind, RecommendedWatcher, RecursiveMode, Watcher};
use ropey::Rope;
use threadpool::ThreadPool;
use typedown_incremental::{Cancelled, QueryStorage, SerializableQueryDatabase};
use typedown_lang::db::TypedownDatabase;
use typedown_lang::db::derived::check_schemas::check_schemas;
use typedown_lang::db::derived::evaluate::evaluate_resource::evaluate_resource;
use typedown_lang::db::derived::evaluate::evaluate_type::evaluate_type;
use typedown_lang::db::derived::get_vault_config::get_vault_config;
use typedown_lang::db::derived::hir::lower_node;
use typedown_lang::db::derived::name_resolver::file_symbol::file_symbol;
use typedown_lang::db::derived::name_resolver::members::schema_members;
use typedown_lang::db::derived::name_resolver::resolution_index::references;
use typedown_lang::db::derived::name_resolver::resolve::resolve;
use typedown_lang::db::derived::parse_file::parse_file;
use typedown_lang::db::derived::typechecker::typecheck::typecheck;
use typedown_lang::db::types::{File, FileRedNode, Project, SymbolKind};
use typedown_lang::db::utils::{is_content_file, is_internal_file, is_type_file};
use typedown_lang::integrations::export::{
  export_property_descriptors, export_resource_html, export_resource_meta, export_resource_summary,
  resolve_schema_label,
};
use typedown_lang::integrations::format::format_markdown;
use typedown_lang::integrations::lint::lint_markdown;
use typedown_lang::syntax::ast::{AstNode, SourceFile};
use typedown_lang::syntax::diagnostic::Diagnostic as TdDiagnostic;

use typedown_types::path::normalize_path;

use crate::core::analysis::Analysis;
use crate::core::analysis_host::AnalysisHost;
use crate::core::utils::fs::{is_asset_file, is_vault_config};

use super::contract::*;

#[derive(PartialEq)]
enum FsEventKind {
  Created,
  Modified,
  Removed,
}

struct FsEvent {
  path: PathBuf,
  kind: FsEventKind,
}

fn get_schema_label(
  db: &TypedownDatabase,
  project: Project,
  schema: Option<&str>,
) -> Option<String> {
  let name = schema?;
  let members = schema_members(db, project);
  let sym = members.members(db).get(name).copied()?;
  if let SymbolKind::UserDefinedSchema(_, file) = sym.kind(db) {
    Some(resolve_schema_label(db, project, file))
  } else {
    None
  }
}

// Build a TdSiteConfig from the current project state
fn build_site_config(db: &TypedownDatabase, project: Project) -> TdSiteConfig {
  let config = get_vault_config(db, project);
  let root = project.root_dir(db);
  let root_dir = config.root_dir(db);
  let base_path = config.base_path(db);

  let root_dir_rel = normalize_path(root_dir.strip_prefix(&*root).unwrap_or(&root_dir));

  TdSiteConfig {
    version: config.version(db).to_string(),
    base_path: base_path.to_string(),
    root_dir: root_dir_rel,
    site_title: config.site_title(db).to_string(),
    site_description: config.site_description(db).to_string(),
    repo: config.repo(db).clone(),
    author: config.author(db).clone(),
    license: config.license(db).clone(),
    public_dir: config.public_dir(db).to_string(),
    nav: config
      .nav_items(db)
      .iter()
      .map(|(title, link, icon)| TdNavItem {
        title: title.clone(),
        link: link.clone(),
        icon: icon.clone(),
      })
      .collect(),
  }
}

/// RPC build server over lsp-server Connection
pub struct RpcServer {
  connection: Connection,
  host: Arc<std::sync::RwLock<AnalysisHost>>,
  thread_pool: ThreadPool,
  cache_session: typedown_incremental::CacheSession,
  // Held to keep the watcher alive
  _watcher: RecommendedWatcher,
  fs_thread: std::thread::JoinHandle<()>,
}

impl RpcServer {
  pub fn new(connection: Connection, root_dir: PathBuf) -> anyhow::Result<Self> {
    let cache_dir = root_dir.join(".typedown/.local/cache");
    let (cache_session, serialized) = typedown_incremental::CacheSession::open(&cache_dir)
      .unwrap_or_else(|_| (typedown_incremental::CacheSession::empty(), None));

    let storage = match serialized {
      Some(data) => {
        match std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
          let arc = QueryStorage::from_serialized(data);
          std::sync::Arc::try_unwrap(arc).unwrap_or_else(|arc| (*arc).clone())
        })) {
          Ok(storage) => storage,
          Err(_) => {
            let _ = std::fs::remove_dir_all(&cache_dir);
            QueryStorage::default()
          }
        }
      }
      None => QueryStorage::default(),
    };

    let db = TypedownDatabase { storage };
    let host = AnalysisHost::new(db, root_dir.clone())?;

    // Resolve the vault root so the watcher only monitors vault files
    let vault_root = {
      let snapshot = host.snapshot();
      let config = get_vault_config(&snapshot.db, snapshot.project);
      config.root_dir(&snapshot.db)
    };

    let host = Arc::new(std::sync::RwLock::new(host));
    let sender = connection.sender.clone();

    let (fs_tx, fs_rx) = crossbeam_channel::unbounded::<FsEvent>();
    let watcher = Self::setup_watcher(&root_dir, &vault_root, fs_tx)?;

    let num_threads = std::thread::available_parallelism()
      .map(|count| count.get().min(4))
      .unwrap_or(2);

    let fs_host = Arc::clone(&host);
    let fs_thread = std::thread::spawn(move || {
      Self::fs_watcher_loop(fs_host, sender, fs_rx);
    });

    Ok(Self {
      connection,
      host,
      thread_pool: ThreadPool::new(num_threads),
      cache_session,
      _watcher: watcher,
      fs_thread,
    })
  }

  pub fn shutdown(self) {
    self.thread_pool.join();
    drop(self._watcher);
    let _ = self.fs_thread.join();

    match Arc::try_unwrap(self.host) {
      Err(_) => log::warn!("Skipping cache save: host still referenced"),
      Ok(host) => {
        let db = host
          .into_inner()
          .expect("RwLock should not be poisoned at shutdown")
          .into_db();
        let revision = db
          .storage
          .revision
          .load(std::sync::atomic::Ordering::Acquire) as u64;

        let dump_timeout = std::time::Duration::from_secs(120);
        let (tx, rx) = std::sync::mpsc::channel();
        let dump_thread = std::thread::spawn(move || {
          let serialized = db.dump();
          let _ = tx.send(serialized);
        });

        match rx.recv_timeout(dump_timeout) {
          Ok(serialized) => {
            if let Err(err) = self.cache_session.finalize(&serialized, revision) {
              log::error!("Failed to save incremental cache: {err}");
            }
          }
          Err(_) => {
            log::warn!("Cache dump timed out after {dump_timeout:?}, skipping save");
            drop(dump_thread);
          }
        }
      }
    }
  }

  pub fn run(&self) -> anyhow::Result<()> {
    for msg in &self.connection.receiver {
      match msg {
        Message::Request(req) => {
          if self.connection.handle_shutdown(&req)? {
            return Ok(());
          }
          self.handle_request(req);
        }
        Message::Notification(_) | Message::Response(_) => {}
      }
    }
    Ok(())
  }

  fn handle_request(&self, req: Request) {
    let host = Arc::clone(&self.host);
    let sender = self.connection.sender.clone();
    let request_id = req.id.clone();
    let method = req.method.clone();

    self.thread_pool.execute(move || {
      let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let analysis = host.read().unwrap().snapshot();
        dispatch_request(&method, req.params, &analysis)
      }));

      let response = match result {
        Ok(Ok(value)) => Response::new_ok(request_id, value),
        Ok(Err(err)) => Response::new_err(request_id, err.code, err.message),
        Err(payload) => {
          if payload.downcast_ref::<Cancelled>().is_some() {
            Response::new_err(
              request_id,
              CANCELLED_ERROR_CODE,
              "Request cancelled: content modified".into(),
            )
          } else {
            Response::new_err(
              request_id,
              lsp_server::ErrorCode::InternalError as i32,
              "Internal error".into(),
            )
          }
        }
      };

      let _ = sender.send(Message::Response(response));
    });
  }

  /// Set up the file watcher
  fn setup_watcher(
    root_dir: &Path,
    vault_root: &Path,
    fs_tx: crossbeam_channel::Sender<FsEvent>,
  ) -> anyhow::Result<RecommendedWatcher> {
    let vault_root = vault_root.to_path_buf();
    let mut watcher = notify::recommended_watcher(move |result: Result<Event, notify::Error>| {
      let Ok(event) = result else { return };
      // Only watch vault config at project root and content/assets inside the vault
      for path in &event.paths {
        if is_vault_config(path) {
          // Accept config file changes regardless of location
        } else if path.starts_with(&vault_root) && (is_content_file(path) || is_asset_file(path)) {
          // Accept vault content and asset changes
        } else {
          continue;
        }
        let kind = match event.kind {
          EventKind::Create(_) => FsEventKind::Created,
          EventKind::Modify(_) => FsEventKind::Modified,
          EventKind::Remove(_) => FsEventKind::Removed,
          _ => continue,
        };
        let _ = fs_tx.send(FsEvent {
          path: path.clone(),
          kind,
        });
      }
    })?;

    watcher.watch(root_dir, RecursiveMode::Recursive)?;
    Ok(watcher)
  }

  fn fs_watcher_loop(
    host: Arc<std::sync::RwLock<AnalysisHost>>,
    sender: Sender<Message>,
    fs_rx: crossbeam_channel::Receiver<FsEvent>,
  ) {
    while let Ok(first) = fs_rx.recv() {
      let mut pending: BTreeMap<PathBuf, FsEvent> = BTreeMap::new();
      pending.insert(first.path.clone(), first);

      // Drain additional events within 50ms for batching
      let deadline = std::time::Instant::now() + Duration::from_millis(50);
      loop {
        match fs_rx.recv_timeout(deadline.saturating_duration_since(std::time::Instant::now())) {
          Ok(event) => {
            pending.insert(event.path.clone(), event);
          }
          Err(crossbeam_channel::RecvTimeoutError::Timeout) => break,
          Err(crossbeam_channel::RecvTimeoutError::Disconnected) => return,
        }
      }

      // Apply changes to the host
      {
        let mut host_guard = host.write().unwrap();
        for event in pending.values() {
          match event.kind {
            FsEventKind::Created | FsEventKind::Modified => {
              host_guard.on_disk_change(event.path.clone())
            }
            FsEventKind::Removed => host_guard.on_disk_delete(event.path.clone()),
          }
        }
      }

      // If a concurrent write cancels these queries, skip and let the next batch retry
      let Ok(()) = Cancelled::catch(|| {
        let analysis = host.read().unwrap().snapshot();
        let db = &analysis.db;
        let project = analysis.project;
        let config = get_vault_config(db, project);
        let root_dir = config.root_dir(db);

        // Notify subscribers if any pending event is a config file change
        if pending.values().any(|event| is_vault_config(&event.path)) {
          let config = build_site_config(db, project);
          send_notification(&sender, NOTIF_CONFIG_CHANGED, &config);
        }

        for event in pending.into_values() {
          if event.path.starts_with(&root_dir) && !is_type_file(&event.path) {
            let relative =
              normalize_path(event.path.strip_prefix(&root_dir).unwrap_or(&event.path));
            let method = match event.kind {
              FsEventKind::Created => NOTIF_CONTENT_CREATED,
              FsEventKind::Modified => NOTIF_CONTENT_CHANGED,
              FsEventKind::Removed => NOTIF_CONTENT_DELETED,
            };

            let affected_files = if event.kind == FsEventKind::Modified {
              collect_affected_files(db, project, &event.path, &root_dir)
            } else {
              vec![]
            };

            let notification = TdContentNotification {
              content: relative,
              affected_files,
            };
            send_notification(&sender, method, &notification);
          } else if is_type_file(&event.path) {
            let Some(name) = event
              .path
              .file_stem()
              .and_then(|s| s.to_str())
              .map(str::to_string)
            else {
              continue;
            };
            let notification = TdSchemaNotification { schema: name };
            let method = match event.kind {
              FsEventKind::Created => NOTIF_SCHEMA_CREATED,
              FsEventKind::Modified => NOTIF_SCHEMA_CHANGED,
              FsEventKind::Removed => NOTIF_SCHEMA_DELETED,
            };
            send_notification(&sender, method, &notification);
          }
        }
      }) else {
        continue;
      };
    }
  }
}

fn send_notification<T: serde::Serialize>(sender: &Sender<Message>, method: &str, params: &T) {
  let notif = Notification::new(method.to_string(), serde_json::to_value(params).unwrap());
  let _ = sender.send(Message::Notification(notif));
}

fn parse_params<T: serde::de::DeserializeOwned>(params: serde_json::Value) -> RpcResult<T> {
  serde_json::from_value(params).map_err(|err| RpcError::invalid_params(err.to_string()))
}

fn dispatch_request(
  method: &str,
  params: serde_json::Value,
  analysis: &Analysis,
) -> RpcResult<serde_json::Value> {
  #[derive(serde::Deserialize)]
  #[serde(rename_all = "camelCase")]
  struct FilePathParams {
    file_path: String,
  }

  #[derive(serde::Deserialize)]
  #[serde(rename_all = "camelCase")]
  struct FilePathsParams {
    file_paths: Vec<String>,
  }

  #[derive(serde::Deserialize)]
  struct SchemaParams {
    schema: String,
  }

  match method {
    METHOD_REQUEST_FILE => {
      let params: FilePathParams = parse_params(params)?;
      Ok(serde_json::to_value(build_file(analysis, &params.file_path)?).unwrap())
    }
    METHOD_REQUEST_FILES => {
      let params: FilePathsParams = parse_params(params)?;
      Ok(serde_json::to_value(build_files(analysis, &params.file_paths)?).unwrap())
    }
    METHOD_LIST_VAULT => Ok(serde_json::to_value(list_vault(analysis)?).unwrap()),
    METHOD_LIST_FILES_GROUPED_BY_SCHEMA => {
      Ok(serde_json::to_value(list_files_grouped_by_schema(analysis)?).unwrap())
    }
    METHOD_LIST_SIDEBAR => Ok(serde_json::to_value(list_sidebar(analysis)?).unwrap()),
    METHOD_LIST_SCHEMAS => Ok(serde_json::to_value(list_schemas(analysis)?).unwrap()),
    METHOD_GET_SCHEMA => {
      let params: SchemaParams = parse_params(params)?;
      Ok(serde_json::to_value(get_schema(analysis, &params.schema)?).unwrap())
    }
    METHOD_GET_VERSION => Ok(
      serde_json::to_value(format!(
        "{} (built {})",
        env!("CARGO_PKG_VERSION"),
        env!("BUILD_TIMESTAMP"),
      ))
      .unwrap(),
    ),
    METHOD_GET_CONFIG => Ok(serde_json::to_value(get_config(analysis)?).unwrap()),
    METHOD_CHECK_VAULT => Ok(serde_json::to_value(check_vault(analysis)?).unwrap()),
    METHOD_FORMAT_FILE => {
      let params: FilePathParams = parse_params(params)?;
      Ok(serde_json::to_value(format_file(analysis, &params.file_path)?).unwrap())
    }
    _ => Err(RpcError {
      code: lsp_server::ErrorCode::MethodNotFound as i32,
      message: format!("Method not found: {method}"),
    }),
  }
}

const MAX_AFFECTED_DEPTH: usize = 5;

// Collect vault-relative paths of files that transitively reference the given file
fn collect_affected_files(
  db: &TypedownDatabase,
  project: Project,
  changed_path: &Path,
  root_dir: &Path,
) -> Vec<String> {
  let changed_file = match project.files(db).get(changed_path) {
    Some(f) => *f,
    None => return vec![],
  };
  let symbol = match file_symbol(db, project, changed_file).value(db) {
    Some(s) => s,
    None => return vec![],
  };

  let mut affected = HashSet::new();
  let mut current_level = vec![symbol];

  for _ in 0..MAX_AFFECTED_DEPTH {
    let mut next_level = vec![];
    for sym in &current_level {
      for reference in references(db, project, *sym) {
        let ref_file = reference.hir.node(db).owner_file;
        let ref_path = match ref_file.handle(db).path() {
          Some(p) => p.clone(),
          None => continue,
        };
        if ref_path == changed_path {
          continue;
        }
        if affected.insert(ref_path.clone()) {
          if let Some(ref_sym) = file_symbol(db, project, ref_file).value(db) {
            next_level.push(ref_sym);
          }
        }
      }
    }
    if next_level.is_empty() {
      break;
    }
    current_level = next_level;
  }

  affected
    .into_iter()
    .filter_map(|p| p.strip_prefix(root_dir).ok().map(|r| normalize_path(r)))
    .collect()
}

/* Request implementations */

fn build_file(analysis: &Analysis, file_path: &str) -> RpcResult<TdBuiltResource> {
  let mut results = build_files(analysis, &[file_path.to_string()])?;
  Ok(results.swap_remove(0))
}

fn build_files(analysis: &Analysis, file_paths: &[String]) -> RpcResult<Vec<TdBuiltResource>> {
  let db = &analysis.db;
  let project = analysis.project;

  let config = get_vault_config(db, project);
  let root_dir = config.root_dir(db);
  let files = project.files(db);

  let mut results = Vec::with_capacity(file_paths.len());
  for entry in file_paths {
    let path = root_dir.join(entry);
    let file = files
      .get(&path)
      .ok_or_else(|| RpcError::invalid_params(format!("File not found: {entry}")))?;

    let exported = export_resource_html(db, project, *file)
      .ok_or_else(|| RpcError::invalid_params(format!("File is not a resource: {entry}")))?;

    let schema_label = get_schema_label(db, project, exported.schema.as_deref());

    results.push(TdBuiltResource {
      schema: exported.schema,
      schema_label,
      label: exported.label,
      icon: exported.icon.map(|icon| TdIcon { name: icon.name }),
      header: exported.header,
      content: exported.content,
      headings: exported
        .headings
        .into_iter()
        .map(|h| TdHeading {
          level: h.level,
          title: h.title,
          slug: h.slug,
        })
        .collect(),
      title: exported.title,
      metadata: TdFileMetadata {
        mtime: exported.metadata.mtime,
        ctime: exported.metadata.ctime,
      },
    });
  }

  Ok(results)
}

fn list_vault(analysis: &Analysis) -> RpcResult<Vec<String>> {
  let db = &analysis.db;
  let project = analysis.project;

  let config = get_vault_config(db, project);
  let root_dir = config.root_dir(db);
  let files = project.files(db);

  let mut result = Vec::new();
  for path in files.keys() {
    if !path.starts_with(&root_dir) {
      continue;
    }
    if !is_content_file(path) || is_internal_file(path) {
      continue;
    }
    let relative = path.strip_prefix(&root_dir).unwrap_or(path);
    result.push(normalize_path(relative));
  }

  Ok(result)
}

fn list_files_grouped_by_schema(
  analysis: &Analysis,
) -> RpcResult<BTreeMap<String, Vec<TdContentSummary>>> {
  let db = &analysis.db;
  let project = analysis.project;

  let config = get_vault_config(db, project);
  let root_dir = config.root_dir(db);
  let files = project.files(db);

  let mut groups: BTreeMap<String, Vec<TdContentSummary>> = BTreeMap::new();
  for (path, file) in files.iter() {
    if !path.starts_with(&root_dir) {
      continue;
    }
    if !is_content_file(path) || is_internal_file(path) {
      continue;
    }
    let relative = normalize_path(path.strip_prefix(&root_dir).unwrap_or(path));
    if let Some(summary) = export_resource_summary(db, project, *file) {
      let group_key = summary.schema.clone().unwrap_or_default();
      let schema_label = get_schema_label(db, project, summary.schema.as_deref());
      groups.entry(group_key).or_default().push(TdContentSummary {
        filepath: relative,
        schema: summary.schema,
        schema_label,
        label: summary.label,
        icon: summary.icon.map(|icon| TdIcon { name: icon.name }),
        header: summary.header,
        excerpt: summary.excerpt,
        metadata: TdFileMetadata {
          mtime: summary.metadata.mtime,
          ctime: summary.metadata.ctime,
        },
      });
    }
  }

  Ok(groups)
}

fn list_sidebar(analysis: &Analysis) -> RpcResult<Vec<TdSidebarItem>> {
  let db = &analysis.db;
  let project = analysis.project;

  let config = get_vault_config(db, project);
  let root_dir = config.root_dir(db);
  let files = project.files(db);

  let mut items = Vec::new();
  for (path, file) in files.iter() {
    if !path.starts_with(&root_dir) || !is_content_file(path) || is_internal_file(path) {
      continue;
    }
    let relative = normalize_path(path.strip_prefix(&root_dir).unwrap_or(path));
    if let Some(meta) = export_resource_meta(db, project, *file) {
      let schema_label = get_schema_label(db, project, meta.schema.as_deref());
      items.push(TdSidebarItem {
        filepath: relative,
        schema: meta.schema,
        schema_label,
        label: meta.label,
        icon: meta.icon.map(|icon| TdIcon { name: icon.name }),
        metadata: TdFileMetadata {
          mtime: meta.metadata.mtime,
          ctime: meta.metadata.ctime,
        },
      });
    }
  }

  Ok(items)
}

fn get_config(analysis: &Analysis) -> RpcResult<TdSiteConfig> {
  let db = &analysis.db;
  let project = analysis.project;
  Ok(build_site_config(db, project))
}

fn list_schemas(analysis: &Analysis) -> RpcResult<Vec<String>> {
  let db = &analysis.db;
  let project = analysis.project;

  let config = get_vault_config(db, project);
  let root_dir = config.root_dir(db);
  let files = project.files(db);

  let mut schemas = Vec::new();
  for (path, file) in &*files {
    if !path.starts_with(&*root_dir) || !is_type_file(path) {
      continue;
    }
    let Some(symbol) = file_symbol(db, project, *file).value(db) else {
      continue;
    };
    if !matches!(symbol.kind(db), SymbolKind::UserDefinedSchema(..)) {
      continue;
    }
    schemas.push(symbol.name(db).to_string());
  }

  Ok(schemas)
}

fn get_schema(analysis: &Analysis, schema: &str) -> RpcResult<TdSchemaInfo> {
  let db = &analysis.db;
  let project = analysis.project;

  let schema_members = schema_members(db, project);
  let sym = schema_members
    .members(db)
    .get(schema)
    .copied()
    .ok_or_else(|| RpcError::invalid_params("Schema not found"))?;
  let SymbolKind::UserDefinedSchema(_, file) = sym.kind(db) else {
    return Err(RpcError::invalid_params("Schema not found"));
  };

  let label = resolve_schema_label(db, project, file);
  let properties = export_property_descriptors(db, project, file)
    .unwrap_or_else(|| serde_json::Value::Object(Default::default()));

  Ok(TdSchemaInfo {
    schema: schema.to_string(),
    label,
    properties,
  })
}

fn check_vault(analysis: &Analysis) -> RpcResult<TdDiagnosticReport> {
  let db = &analysis.db;
  let project = analysis.project;

  let config = get_vault_config(db, project);
  let root_dir = config.root_dir(db);
  let files = project.files(db);

  let mut all_diagnostics = Vec::new();
  let mut file_count: u32 = 0;

  for (path, &file) in &*files {
    if !path.starts_with(&*root_dir) || !is_content_file(path) || is_type_file(path) {
      continue;
    }
    file_count += 1;

    let relative_path = normalize_path(path.strip_prefix(&root_dir).unwrap_or(path));
    let rope = match analysis.file_rope(path) {
      Some(r) => r,
      None => continue,
    };

    let items = collect_file_diagnostics(db, project, file, &relative_path, &rope);
    all_diagnostics.extend(items);
  }

  // Check for duplicate schema files
  let schema_check = check_schemas(db, project);
  for diag in schema_check.diagnostics(db) {
    let filepath = if let TdDiagnostic::DuplicateSchemaName { ref path, .. } = diag {
      path.clone()
    } else {
      String::new()
    };
    all_diagnostics.push(TdDiagnosticItem {
      filepath,
      line: 0,
      column: 0,
      severity: "error".to_string(),
      code: diag.code().as_str().to_string(),
      message: diag.message(),
    });
  }

  let error_count = all_diagnostics
    .iter()
    .filter(|d| d.severity == "error")
    .count() as u32;
  let warning_count = all_diagnostics
    .iter()
    .filter(|d| d.severity == "warning")
    .count() as u32;

  Ok(TdDiagnosticReport {
    diagnostics: all_diagnostics,
    file_count,
    error_count,
    warning_count,
  })
}

fn format_file(analysis: &Analysis, file_path: &str) -> RpcResult<TdFormatResult> {
  let db = &analysis.db;
  let project = analysis.project;

  let config = get_vault_config(db, project);
  let root_dir = config.root_dir(db);
  let path = root_dir.join(file_path);

  let file = *project
    .files(db)
    .get(&path)
    .ok_or_else(|| RpcError::invalid_params(format!("File not found: {file_path}")))?;

  let root = parse_file(db, project, file).ast(db).node.clone();
  let source_file = SourceFile::cast(root.clone())
    .ok_or_else(|| RpcError::invalid_params("Failed to parse file"))?;

  let original = root.text();
  let formatted = match source_file.body() {
    Some(body) => {
      let frontmatter = original[..body.syntax().offset()].to_string();
      let body_formatted = format_markdown(&body);
      frontmatter + &body_formatted
    }
    None => original.to_string(),
  };

  let changed = formatted != original;
  Ok(TdFormatResult {
    content: formatted,
    changed,
  })
}

/// Collect all diagnostics for a single content file
fn collect_file_diagnostics(
  db: &TypedownDatabase,
  project: Project,
  file: File,
  filepath: &str,
  rope: &Rope,
) -> Vec<TdDiagnosticItem> {
  let mut items = Vec::new();

  // Parse errors
  let parse_result = parse_file(db, project, file);
  let mut td_diags: Vec<TdDiagnostic> = parse_result.diagnostics(db).to_vec();

  // Typecheck and name resolution errors
  let root = parse_result.ast(db).node.clone();
  let hir = lower_node(db, project, FileRedNode::new(file, root));
  let typecheck_result = typecheck(db, hir);
  td_diags.extend(typecheck_result.diagnostics(db).iter().cloned());
  let resolve_result = resolve(db, hir);
  td_diags.extend(resolve_result.diagnostics(db).iter().cloned());

  // Evaluation errors
  if let Some(sym) = file_symbol(db, project, file).value(db) {
    if sym.kind(db).is_schema() {
      let eval_result = evaluate_type(db, sym);
      td_diags.extend(eval_result.diagnostics(db).iter().cloned());
    } else {
      let eval_result = evaluate_resource(db, sym);
      td_diags.extend(eval_result.diagnostics(db).iter().cloned());
    }
  }

  // Deduplicate diagnostics by (code, line, column)
  let mut seen = std::collections::HashSet::new();
  for diag in &td_diags {
    let (line, column) = if let Some((start, _)) = diag.offsets() {
      let start = start.min(rope.len_chars());
      let l = rope.char_to_line(start);
      let c = start - rope.line_to_char(l);
      (l as u32 + 1, c as u32 + 1)
    } else {
      (1, 1)
    };

    let code = diag.code().as_str().to_string();
    let key = (code.clone(), line, column);
    if !seen.insert(key) {
      continue;
    }

    items.push(TdDiagnosticItem {
      filepath: filepath.to_string(),
      line,
      column,
      severity: "error".to_string(),
      code,
      message: diag.message(),
    });
  }

  // Lint warnings
  if let Some(body) = SourceFile::cast(parse_result.ast(db).node.clone()).and_then(|sf| sf.body()) {
    for lint in lint_markdown(&body) {
      let start = lint.start_offset.min(rope.len_chars());
      let l = rope.char_to_line(start);
      let c = start - rope.line_to_char(l);

      items.push(TdDiagnosticItem {
        filepath: filepath.to_string(),
        line: l as u32 + 1,
        column: c as u32 + 1,
        severity: "warning".to_string(),
        code: lint.code.as_str().to_string(),
        message: lint.message,
      });
    }
  }

  items
}
