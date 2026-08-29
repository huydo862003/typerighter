use lsp_server::Connection;
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
use typedown_server::lsp::server::Server;
use typedown_server::lsp::service::{commands, semantic_tokens};

// The entrypoint
pub fn main() -> anyhow::Result<()> {
  let (connection, io_thread) = Connection::stdio();

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
    completion_provider: Some(CompletionOptions::default()),
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
      version: Some(env!("CARGO_PKG_VERSION").to_string()),
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

  io_thread.join()?;
  Ok(())
}
