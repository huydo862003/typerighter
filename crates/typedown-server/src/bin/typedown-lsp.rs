use lsp_types::{
  CompletionOptions, FileOperationFilter, FileOperationPattern, FileOperationRegistrationOptions,
  HoverProviderCapability, InitializeParams, InitializeResult, OneOf, RenameOptions,
  SemanticTokensFullOptions, SemanticTokensLegend, SemanticTokensOptions,
  SemanticTokensServerCapabilities, ServerCapabilities, ServerInfo, TextDocumentSyncCapability,
  TextDocumentSyncKind, WorkDoneProgressOptions, WorkspaceFileOperationsServerCapabilities,
  WorkspaceServerCapabilities,
};
use typedown_server::core::logger;
use typedown_server::core::multiproject::Multiproject;
use typedown_server::core::transport;
use typedown_server::lsp::server::Server;
use typedown_server::lsp::service::{commands, semantic_tokens};

pub fn main() -> anyhow::Result<()> {
  let mut args = pico_args::Arguments::from_env();

  if args.contains("--help") {
    eprintln!("Usage: typedown-lsp [OPTIONS]");
    eprintln!();
    eprintln!("Options:");
    eprintln!("  --stdio             Use stdio transport (default: TCP)");
    eprintln!("  --addr <addr>       Listen address (default: 127.0.0.1)");
    eprintln!("  --port <port>       Listen port (default: 0, auto-assign)");
    eprintln!("  --fresh          Start fresh, ignore cache from previous session");
    eprintln!("  --help              Show this help");
    return Ok(());
  }

  let use_stdio = args.contains("--stdio");

  let addr: String = args
    .opt_value_from_str("--addr")?
    .or_else(|| std::env::var("TYPEDOWN_LSP_ADDR").ok())
    .unwrap_or_else(|| "127.0.0.1".to_string());

  let port: u16 = args
    .opt_value_from_str("--port")?
    .or_else(|| {
      std::env::var("TYPEDOWN_LSP_PORT")
        .ok()
        .and_then(|s| s.parse().ok())
    })
    .unwrap_or(0);

  let fresh = args.contains("--fresh")
    || std::env::var("TYPEDOWN_NO_CACHE").is_ok_and(|v| v == "1" || v == "true");

  let remaining = args.finish();
  if !remaining.is_empty() {
    eprintln!(
      "warning: unknown arguments: {:?}",
      remaining
        .iter()
        .map(|s| s.to_string_lossy())
        .collect::<Vec<_>>()
    );
  }

  if fresh {
    // SAFETY: single-threaded at this point, before any server threads are spawned
    unsafe { std::env::set_var("TYPEDOWN_NO_CACHE", "1") };
  }

  let (connection, io_handle) = if use_stdio {
    Ok(transport::connect_stdio())
  } else {
    transport::connect_tcp(&addr, port).map(|(conn, io, _)| (conn, io))
  }?;

  // File logger available immediately, before handshake
  logger::init_file();

  let capabilities = ServerCapabilities {
    rename_provider: Some(OneOf::Right(RenameOptions {
      prepare_provider: Some(true),
      work_done_progress_options: WorkDoneProgressOptions {
        work_done_progress: None,
      },
    })),
    text_document_sync: Some(TextDocumentSyncCapability::Kind(
      TextDocumentSyncKind::INCREMENTAL,
    )),
    hover_provider: Some(HoverProviderCapability::Simple(true)),
    completion_provider: Some(CompletionOptions {
      trigger_characters: Some(
        [".", "(", ",", "$", "/", "_"]
          .iter()
          .map(|s| s.to_string())
          .collect(),
      ),
      ..Default::default()
    }),
    definition_provider: Some(OneOf::Left(true)),
    references_provider: Some(OneOf::Left(true)),
    code_action_provider: Some(lsp_types::CodeActionProviderCapability::Simple(true)),
    document_formatting_provider: Some(OneOf::Left(true)),
    execute_command_provider: Some(lsp_types::ExecuteCommandOptions {
      commands: commands::command_ids(),
      ..Default::default()
    }),
    inlay_hint_provider: Some(lsp_types::OneOf::Left(true)),
    semantic_tokens_provider: Some(SemanticTokensServerCapabilities::SemanticTokensOptions(
      SemanticTokensOptions {
        legend: SemanticTokensLegend {
          token_types: semantic_tokens::token_types(),
          token_modifiers: semantic_tokens::token_modifiers(),
        },
        full: Some(SemanticTokensFullOptions::Bool(true)),
        ..Default::default()
      },
    )),
    workspace: Some(WorkspaceServerCapabilities {
      file_operations: Some(WorkspaceFileOperationsServerCapabilities {
        will_rename: Some(FileOperationRegistrationOptions {
          filters: vec![FileOperationFilter {
            scheme: None,
            pattern: FileOperationPattern {
              glob: "**/*.td".to_string(),
              ..Default::default()
            },
          }],
        }),
        did_rename: Some(FileOperationRegistrationOptions {
          filters: vec![FileOperationFilter {
            scheme: None,
            pattern: FileOperationPattern {
              glob: "**/*.td".to_string(),
              ..Default::default()
            },
          }],
        }),
        ..Default::default()
      }),
      ..Default::default()
    }),
    ..Default::default()
  };

  let multiproject = Multiproject::default();

  // connection.initialize wraps its arg in { "capabilities": ... }, so we use initialize_start/initialize_finish to also include serverInfo
  let (init_id, init_params) = connection.initialize_start()?;
  let init_data = serde_json::to_value(InitializeResult {
    capabilities,
    server_info: Some(ServerInfo {
      name: "typedown-lsp".to_string(),
      version: Some(format!(
        "{} (built {})",
        env!("CARGO_PKG_VERSION"),
        env!("BUILD_TIMESTAMP")
      )),
    }),
  })?;
  connection.initialize_finish(init_id, init_data)?;
  let init_params: InitializeParams = serde_json::from_value(init_params)?;

  // Upgrade logger to also send window/logMessage after handshake
  logger::set_lsp_sender(connection.sender.clone());

  // Projects are loaded lazily on first didOpen/request via load_nearest_project
  log::info!("Typedown LSP server started");

  let server = Server::new(connection, multiproject, init_params.capabilities);

  server.run()?;

  log::info!("Shutting down, saving cache");
  server.save();

  io_handle.join();
  Ok(())
}
