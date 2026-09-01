use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use jsonrpsee::core::{RpcResult, async_trait};
use jsonrpsee::types::ErrorObjectOwned;
use jsonrpsee::types::error::INVALID_PARAMS_CODE;
use jsonrpsee::{PendingSubscriptionSink, SubscriptionMessage, SubscriptionSink};
use notify::{Event, EventKind, RecommendedWatcher, RecursiveMode, Watcher};
use ropey::Rope;
use serde::Serialize;
use tokio::sync::broadcast;
use tokio::time::{Duration, sleep};
use typedown_incremental::{Cancelled, QueryStorage};
use typedown_lang::db::TypedownDatabase;
use typedown_lang::db::derived::check_schemas::check_schemas;
use typedown_lang::db::derived::evaluate::evaluate_resource::evaluate_resource;
use typedown_lang::db::derived::evaluate::evaluate_type::evaluate_type;
use typedown_lang::db::derived::get_vault_config::get_vault_config;
use typedown_lang::db::derived::hir::lower_node;
use typedown_lang::db::derived::name_resolver::file_symbol::file_symbol;
use typedown_lang::db::derived::name_resolver::members::schema_members;
use typedown_lang::db::derived::name_resolver::resolve::resolve;
use typedown_lang::db::derived::parse_file::parse_file;
use typedown_lang::db::derived::typechecker::typecheck::typecheck;
use typedown_lang::db::types::{File, Project, SymbolKind};
use typedown_lang::db::utils::{is_content_file, is_internal_file, is_type_file};
use typedown_lang::integrations::export::{
  export_property_descriptors, export_resource, export_resource_meta, resolve_schema_label,
};
use typedown_lang::integrations::format::format_markdown;
use typedown_lang::integrations::lint::lint_markdown;
use typedown_lang::syntax::ast::{AstNode, SourceFile};
use typedown_lang::syntax::diagnostic::Diagnostic as TdDiagnostic;

use typedown_types::path::normalize_path;

use crate::core::analysis_host::AnalysisHost;
use crate::core::utils::fs::{is_asset_file, is_vault_config};

use super::contract::{
  CANCELLED_ERROR_CODE, TdBuildRpcServer, TdBuiltResource, TdContentNotification, TdContentSummary,
  TdDiagnosticItem, TdDiagnosticReport, TdFileMetadata, TdFilePath, TdFormatResult, TdIcon,
  TdRpcSubscriptionCloseResponse, TdSchemaInfo, TdSchemaNotification, TdSidebarItem, TdSiteConfig,
};

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

  let root_dir_rel = normalize_path(root_dir.strip_prefix(&root).unwrap_or(&root_dir));

  TdSiteConfig {
    version: config.version(db).to_string(),
    base_path: base_path.to_string(),
    root_dir: root_dir_rel,
    site_title: config.site_title(db).to_string(),
    site_description: config.site_description(db).to_string(),
    repo: config.repo(db).clone(),
    public_dir: config.public_dir(db).to_string(),
  }
}

/// RPC build server that holds a single project and serves build requests
// TIL: Use tokio::sync::RwLock in async contexts, not std::sync::RwLock as std::sync::RwLock blocks the OS thread while waiting, which can deadlock the tokio runtime if the lock is held across an .await point
pub struct RpcServer {
  host: Arc<tokio::sync::RwLock<AnalysisHost>>,
  events: Arc<FsEventBus>,
}

// Shared state for the FS watcher task
struct FsEventBus {
  // Content events
  content_changed_tx: broadcast::Sender<TdContentNotification>,
  content_created_tx: broadcast::Sender<TdContentNotification>,
  content_deleted_tx: broadcast::Sender<TdContentNotification>,
  // Schema events
  schema_changed_tx: broadcast::Sender<TdSchemaNotification>,
  schema_created_tx: broadcast::Sender<TdSchemaNotification>,
  schema_deleted_tx: broadcast::Sender<TdSchemaNotification>,
  // Config events
  config_changed_tx: broadcast::Sender<TdSiteConfig>,
  // Held to keep the watcher alive for the lifetime of the server
  _watcher: RecommendedWatcher,
}

impl RpcServer {
  pub fn new(root_dir: PathBuf) -> anyhow::Result<Self> {
    let cache_dir = root_dir.join(".typedown/.local/cache");
    let (_session, serialized) = typedown_incremental::CacheSession::open(&cache_dir)
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

    let (content_changed_tx, _) = broadcast::channel(64);
    let (content_created_tx, _) = broadcast::channel(64);
    let (content_deleted_tx, _) = broadcast::channel(64);
    let (schema_changed_tx, _) = broadcast::channel(64);
    let (schema_created_tx, _) = broadcast::channel(64);
    let (schema_deleted_tx, _) = broadcast::channel(64);
    let (config_changed_tx, _) = broadcast::channel(64);

    let (fs_tx, fs_rx) = tokio::sync::mpsc::unbounded_channel();
    let _watcher = Self::setup_watcher(&root_dir, &vault_root, fs_tx)?;

    let host = Arc::new(tokio::sync::RwLock::new(host));
    let events = Arc::new(FsEventBus {
      content_changed_tx,
      content_created_tx,
      content_deleted_tx,
      schema_changed_tx,
      schema_created_tx,
      schema_deleted_tx,
      config_changed_tx,
      _watcher,
    });

    Self::spawn_fs_watcher_task(Arc::clone(&host), Arc::clone(&events), fs_rx);

    Ok(Self { host, events })
  }

  /// Set up the file watcher
  fn setup_watcher(
    root_dir: &Path,
    vault_root: &Path,
    fs_tx: tokio::sync::mpsc::UnboundedSender<FsEvent>,
  ) -> anyhow::Result<RecommendedWatcher> {
    let vault_root = vault_root.to_path_buf();
    let mut watcher = notify::recommended_watcher(move |result: Result<Event, notify::Error>| {
      let Ok(event) = result else { return };
      for path in &event.paths {
        // Only watch vault config at project root and content/assets inside the vault
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

  fn spawn_fs_watcher_task(
    host: Arc<tokio::sync::RwLock<AnalysisHost>>,
    events: Arc<FsEventBus>,
    mut fs_rx: tokio::sync::mpsc::UnboundedReceiver<FsEvent>,
  ) {
    tokio::spawn(async move {
      loop {
        let Some(first) = fs_rx.recv().await else {
          break;
        };
        let mut pending: HashMap<PathBuf, FsEvent> = HashMap::new();
        pending.insert(first.path.clone(), first);

        // Drain additional events within 50ms so rapid editor saves batch together
        let deadline = sleep(Duration::from_millis(50));
        tokio::pin!(deadline);
        loop {
          tokio::select! {
            event = fs_rx.recv() => match event {
              Some(ev) => { pending.insert(ev.path.clone(), ev); }
              None => return,
            },
            _ = &mut deadline => break,
          }
        }

        let mut host_guard = host.write().await;
        for event in pending.values() {
          match event.kind {
            FsEventKind::Created | FsEventKind::Modified => {
              host_guard.on_disk_change(event.path.clone())
            }
            FsEventKind::Removed => host_guard.on_disk_delete(event.path.clone()),
          }
        }
        let analysis = host_guard.snapshot();
        drop(host_guard);

        // If a concurrent write cancels these queries, skip and let the next batch retry
        let Ok(()) = Cancelled::catch(|| {
          let db = &analysis.db;
          let project = analysis.project;
          let config = get_vault_config(db, project);
          let root_dir = config.root_dir(db);

          // Notify subscribers if any pending event is a config file change
          if pending.values().any(|event| is_vault_config(&event.path)) {
            let _ = events
              .config_changed_tx
              .send(build_site_config(db, project));
          }

          for event in pending.into_values() {
            if event.path.starts_with(&root_dir) && !is_type_file(&event.path) {
              let relative =
                normalize_path(event.path.strip_prefix(&root_dir).unwrap_or(&event.path));
              let notification = TdContentNotification { content: relative };
              let sender = match event.kind {
                FsEventKind::Created => &events.content_created_tx,
                FsEventKind::Modified => &events.content_changed_tx,
                FsEventKind::Removed => &events.content_deleted_tx,
              };
              let _ = sender.send(notification);
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
              let sender = match event.kind {
                FsEventKind::Created => &events.schema_created_tx,
                FsEventKind::Modified => &events.schema_changed_tx,
                FsEventKind::Removed => &events.schema_deleted_tx,
              };
              let _ = sender.send(notification);
            }
          }
        }) else {
          continue;
        };
      }
    });
  }

  async fn build_file_impl(&self, file_path: &TdFilePath) -> RpcResult<TdBuiltResource> {
    let mut results = self
      .build_files_impl(std::slice::from_ref(file_path))
      .await?;
    Ok(results.swap_remove(0))
  }

  async fn build_files_impl(&self, file_paths: &[TdFilePath]) -> RpcResult<Vec<TdBuiltResource>> {
    let analysis = self.host.read().await.snapshot();
    let file_paths: Vec<_> = file_paths.iter().map(|fp| fp.0.clone()).collect();
    catch_cancelled(move || {
      let db = &analysis.db;
      let project = analysis.project;

      let config = get_vault_config(db, project);
      let root_dir = config.root_dir(db);
      let files = project.files(db);

      let mut results = Vec::with_capacity(file_paths.len());
      for file_path in &file_paths {
        let path = root_dir.join(file_path);
        let file = files.get(&path).ok_or_else(|| {
          ErrorObjectOwned::owned(
            INVALID_PARAMS_CODE,
            format!("File not found: {file_path}"),
            None::<()>,
          )
        })?;

        let exported = export_resource(db, project, *file).ok_or_else(|| {
          ErrorObjectOwned::owned(
            INVALID_PARAMS_CODE,
            format!("File is not a resource: {file_path}"),
            None::<()>,
          )
        })?;

        let schema_label = get_schema_label(db, project, exported.schema.as_deref());

        results.push(TdBuiltResource {
          schema: exported.schema,
          schema_label,
          label: exported.label,
          icon: exported.icon.map(|i| TdIcon { name: i.name }),
          header: exported.header,
          content: exported.content,
          metadata: TdFileMetadata {
            mtime: exported.metadata.mtime,
            ctime: exported.metadata.ctime,
          },
        });
      }

      Ok(results)
    })
  }

  async fn list_vault_impl(&self) -> RpcResult<Vec<String>> {
    let analysis = self.host.read().await.snapshot();
    catch_cancelled(move || {
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
    })
  }

  async fn list_files_grouped_by_schema_impl(
    &self,
  ) -> RpcResult<HashMap<String, Vec<TdContentSummary>>> {
    let analysis = self.host.read().await.snapshot();
    catch_cancelled(move || {
      let db = &analysis.db;
      let project = analysis.project;

      let config = get_vault_config(db, project);
      let root_dir = config.root_dir(db);
      let files = project.files(db);

      let mut groups: HashMap<String, Vec<TdContentSummary>> = HashMap::new();
      for (path, file) in files.iter() {
        if !path.starts_with(&root_dir) {
          continue;
        }
        if !is_content_file(path) || is_internal_file(path) {
          continue;
        }
        let relative = normalize_path(path.strip_prefix(&root_dir).unwrap_or(path));
        if let Some(exported) = export_resource(db, project, *file) {
          let group_key = exported.schema.clone().unwrap_or_default();
          let schema_label = get_schema_label(db, project, exported.schema.as_deref());
          let excerpt = extract_excerpt(&exported.content);
          groups.entry(group_key).or_default().push(TdContentSummary {
            filepath: relative,
            schema: exported.schema,
            schema_label,
            label: exported.label,
            icon: exported.icon.map(|i| TdIcon { name: i.name }),
            header: exported.header,
            excerpt,
            metadata: TdFileMetadata {
              mtime: exported.metadata.mtime,
              ctime: exported.metadata.ctime,
            },
          });
        }
      }

      Ok(groups)
    })
  }

  async fn list_sidebar_impl(&self) -> RpcResult<Vec<TdSidebarItem>> {
    let analysis = self.host.read().await.snapshot();
    catch_cancelled(move || {
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
            icon: meta.icon.map(|i| TdIcon { name: i.name }),
            metadata: TdFileMetadata {
              mtime: meta.metadata.mtime,
              ctime: meta.metadata.ctime,
            },
          });
        }
      }

      Ok(items)
    })
  }

  async fn get_config_impl(&self) -> RpcResult<TdSiteConfig> {
    let analysis = self.host.read().await.snapshot();
    catch_cancelled(move || {
      let db = &analysis.db;
      let project = analysis.project;

      Ok(build_site_config(db, project))
    })
  }

  async fn list_schemas_impl(&self) -> RpcResult<Vec<String>> {
    let analysis = self.host.read().await.snapshot();
    catch_cancelled(move || {
      let db = &analysis.db;
      let project = analysis.project;

      let config = get_vault_config(db, project);
      let root_dir = config.root_dir(db);
      let files = project.files(db);

      let mut schemas = Vec::new();
      for (path, file) in &files {
        if !path.starts_with(&root_dir) || !is_type_file(path) {
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
    })
  }

  async fn get_schema_impl(&self, schema: &str) -> RpcResult<TdSchemaInfo> {
    let analysis = self.host.read().await.snapshot();
    let schema = schema.to_string();
    catch_cancelled(move || {
      let db = &analysis.db;
      let project = analysis.project;

      let schema_members = schema_members(db, project);
      let sym = schema_members
        .members(db)
        .get(&schema)
        .copied()
        .ok_or_else(|| {
          ErrorObjectOwned::owned(INVALID_PARAMS_CODE, "Schema not found", None::<()>)
        })?;
      let SymbolKind::UserDefinedSchema(_, file) = sym.kind(db) else {
        return Err(ErrorObjectOwned::owned(
          INVALID_PARAMS_CODE,
          "Schema not found",
          None::<()>,
        ));
      };

      let label = resolve_schema_label(db, project, file);
      let properties = export_property_descriptors(db, project, file)
        .unwrap_or_else(|| serde_json::Value::Object(Default::default()));

      Ok(TdSchemaInfo {
        schema,
        label,
        properties,
      })
    })
  }

  async fn check_vault_impl(&self) -> RpcResult<TdDiagnosticReport> {
    let analysis = self.host.read().await.snapshot();
    tokio::task::block_in_place(|| {
      catch_cancelled(move || {
        let db = &analysis.db;
        let project = analysis.project;

        let config = get_vault_config(db, project);
        let root_dir = config.root_dir(db);
        let files = project.files(db);

        let mut all_diagnostics = Vec::new();
        let mut file_count: u32 = 0;

        for (path, &file) in &files {
          if !path.starts_with(&root_dir) || !is_content_file(path) || is_type_file(path) {
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
      })
    }) // block_in_place
  }

  async fn format_file_impl(&self, file_path: &TdFilePath) -> RpcResult<TdFormatResult> {
    let analysis = self.host.read().await.snapshot();
    let fp = file_path.0.clone();
    tokio::task::block_in_place(|| {
      catch_cancelled(move || {
        let db = &analysis.db;
        let project = analysis.project;

        let config = get_vault_config(db, project);
        let root_dir = config.root_dir(db);
        let path = root_dir.join(&fp);

        let file = *project.files(db).get(&path).ok_or_else(|| {
          ErrorObjectOwned::owned(
            INVALID_PARAMS_CODE,
            format!("File not found: {}", fp),
            None::<()>,
          )
        })?;

        let root = parse_file(db, project, file).ast(db);
        let source_file = SourceFile::cast(root.clone()).ok_or_else(|| {
          ErrorObjectOwned::owned(INVALID_PARAMS_CODE, "Failed to parse file", None::<()>)
        })?;

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
      })
    }) // block_in_place
  }
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
  let root = parse_result.ast(db);
  let hir = lower_node(db, project, file, root);
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
  if let Some(body) = SourceFile::cast(parse_result.ast(db)).and_then(|sf| sf.body()) {
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

/// Wrap a closure that runs derived queries, catching Cancelled panics and
/// converting them to an RPC error so the client can retry
fn catch_cancelled<T>(f: impl FnOnce() -> RpcResult<T>) -> RpcResult<T> {
  match Cancelled::catch(f) {
    Ok(result) => result,
    Err(_) => Err(ErrorObjectOwned::owned(
      CANCELLED_ERROR_CODE,
      "Request cancelled: content modified",
      None::<()>,
    )),
  }
}

/// Accept a subscription and forward broadcast messages to the client
async fn run_subscription<T: Serialize + Clone>(
  pending: PendingSubscriptionSink,
  mut rx: broadcast::Receiver<T>,
) -> TdRpcSubscriptionCloseResponse {
  let Ok(sink) = pending.accept().await else {
    return TdRpcSubscriptionCloseResponse::Err("Failed to accept subscription".into());
  };
  while let Ok(notification) = rx.recv().await {
    if !forward(&sink, &notification).await {
      break;
    }
  }
  TdRpcSubscriptionCloseResponse::Ok
}

async fn forward<T: Serialize>(sink: &SubscriptionSink, value: &T) -> bool {
  let Ok(msg) = SubscriptionMessage::new(sink.method_name(), sink.subscription_id(), value) else {
    return true; // Skip unserializable notification, keep subscription alive
  };
  sink.send(msg).await.is_ok()
}

#[async_trait]
impl TdBuildRpcServer<(), ()> for RpcServer {
  async fn request_file(&self, file_path: TdFilePath) -> RpcResult<TdBuiltResource> {
    self.build_file_impl(&file_path).await
  }

  async fn request_files(&self, file_paths: Vec<TdFilePath>) -> RpcResult<Vec<TdBuiltResource>> {
    self.build_files_impl(&file_paths).await
  }

  async fn list_vault(&self) -> RpcResult<Vec<String>> {
    self.list_vault_impl().await
  }

  async fn list_files_grouped_by_schema(
    &self,
  ) -> RpcResult<HashMap<String, Vec<TdContentSummary>>> {
    self.list_files_grouped_by_schema_impl().await
  }

  async fn list_sidebar(&self) -> RpcResult<Vec<TdSidebarItem>> {
    self.list_sidebar_impl().await
  }

  async fn list_schemas(&self) -> RpcResult<Vec<String>> {
    self.list_schemas_impl().await
  }

  async fn get_schema(&self, schema: String) -> RpcResult<TdSchemaInfo> {
    self.get_schema_impl(&schema).await
  }

  async fn get_config(&self) -> RpcResult<TdSiteConfig> {
    self.get_config_impl().await
  }

  async fn check_vault(&self) -> RpcResult<TdDiagnosticReport> {
    self.check_vault_impl().await
  }

  async fn format_file(&self, file_path: TdFilePath) -> RpcResult<TdFormatResult> {
    self.format_file_impl(&file_path).await
  }

  async fn subscribe_content_changed(
    &self,
    pending: PendingSubscriptionSink,
  ) -> TdRpcSubscriptionCloseResponse {
    run_subscription(pending, self.events.content_changed_tx.subscribe()).await
  }

  async fn subscribe_content_created(
    &self,
    pending: PendingSubscriptionSink,
  ) -> TdRpcSubscriptionCloseResponse {
    run_subscription(pending, self.events.content_created_tx.subscribe()).await
  }

  async fn subscribe_content_deleted(
    &self,
    pending: PendingSubscriptionSink,
  ) -> TdRpcSubscriptionCloseResponse {
    run_subscription(pending, self.events.content_deleted_tx.subscribe()).await
  }

  async fn subscribe_schema_changed(
    &self,
    pending: PendingSubscriptionSink,
  ) -> TdRpcSubscriptionCloseResponse {
    run_subscription(pending, self.events.schema_changed_tx.subscribe()).await
  }

  async fn subscribe_schema_created(
    &self,
    pending: PendingSubscriptionSink,
  ) -> TdRpcSubscriptionCloseResponse {
    run_subscription(pending, self.events.schema_created_tx.subscribe()).await
  }

  async fn subscribe_schema_deleted(
    &self,
    pending: PendingSubscriptionSink,
  ) -> TdRpcSubscriptionCloseResponse {
    run_subscription(pending, self.events.schema_deleted_tx.subscribe()).await
  }

  async fn subscribe_config_changed(
    &self,
    pending: PendingSubscriptionSink,
  ) -> TdRpcSubscriptionCloseResponse {
    run_subscription(pending, self.events.config_changed_tx.subscribe()).await
  }
}

// Extract the first non-empty paragraph from markdown content as a plain text excerpt
fn extract_excerpt(content: &str) -> Option<String> {
  for line in content.lines() {
    let trimmed = line.trim();
    // Skip blank lines, headings, lists, code fences, HTML, containers
    if trimmed.is_empty()
      || trimmed.starts_with('#')
      || trimmed.starts_with('-')
      || trimmed.starts_with('*')
      || trimmed.starts_with('>')
      || trimmed.starts_with("```")
      || trimmed.starts_with(":::")
      || trimmed.starts_with('<')
      || trimmed.starts_with('|')
      || trimmed.starts_with("[[")
    {
      continue;
    }
    // Strip inline markdown: bold, italic, links, code
    let plain = trimmed
      .replace("**", "")
      .replace("__", "")
      .replace('*', "")
      .replace('_', " ");
    // Strip markdown links [text](url) -> text
    let plain = regex_replace_links(&plain);
    let plain = plain.trim().to_string();
    if plain.is_empty() {
      continue;
    }
    return Some(plain);
  }
  None
}

// Replace [text](url) with text
fn regex_replace_links(s: &str) -> String {
  let mut result = String::with_capacity(s.len());
  let mut chars = s.chars().peekable();
  while let Some(c) = chars.next() {
    if c == '[' {
      let mut text = String::new();
      let mut found_close = false;
      for inner in chars.by_ref() {
        if inner == ']' {
          found_close = true;
          break;
        }
        text.push(inner);
      }
      if found_close && chars.peek() == Some(&'(') {
        chars.next(); // skip (
        let mut depth = 1;
        for inner in chars.by_ref() {
          if inner == '(' {
            depth += 1;
          }
          if inner == ')' {
            depth -= 1;
            if depth == 0 {
              break;
            }
          }
        }
        result.push_str(&text);
      } else {
        result.push('[');
        result.push_str(&text);
        if found_close {
          result.push(']');
        }
      }
    } else {
      result.push(c);
    }
  }
  result
}
