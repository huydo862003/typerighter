use std::io::BufReader;
use std::net::TcpListener;

use crossbeam_channel::bounded;
use lsp_server::{Connection, Message};
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

enum IoHandle {
  Stdio(lsp_server::IoThreads),
  Tcp {
    reader: std::thread::JoinHandle<()>,
    writer: std::thread::JoinHandle<()>,
  },
}

impl IoHandle {
  fn join(self) {
    match self {
      Self::Stdio(io) => { io.join().unwrap(); }
      Self::Tcp { reader, writer } => {
        reader.join().unwrap();
        writer.join().unwrap();
      }
    }
  }
}

fn connect_tcp() -> anyhow::Result<(Connection, IoHandle)> {
  let port = std::env::var("TYPEDOWN_LSP_PORT")
    .ok()
    .and_then(|p| p.parse::<u16>().ok())
    .unwrap_or(0);
  let listener = TcpListener::bind(format!("127.0.0.1:{port}"))?;
  let bound_port = listener.local_addr()?.port();
  // Print the port so clients can read it from stdout
  println!("{bound_port}");

  let (stream, _) = listener.accept()?;
  let reader_stream = stream.try_clone()?;
  let mut writer_stream = stream;

  let (writer_sender, writer_receiver) = bounded::<Message>(0);
  let (reader_sender, reader_receiver) = bounded::<Message>(0);

  let reader = std::thread::spawn(move || {
    let mut buf_read = BufReader::new(reader_stream);
    while let Some(msg) = Message::read(&mut buf_read).unwrap() {
      let is_exit = matches!(&msg, Message::Notification(n) if n.method == "exit");
      reader_sender.send(msg).unwrap();
      if is_exit {
        break;
      }
    }
  });

  let writer = std::thread::spawn(move || {
    writer_receiver.into_iter().for_each(|msg| {
      msg.write(&mut writer_stream).unwrap();
    });
  });

  let connection = Connection { sender: writer_sender, receiver: reader_receiver };
  Ok((connection, IoHandle::Tcp { reader, writer }))
}

// The entrypoint
pub fn main() -> anyhow::Result<()> {
  let use_stdio = std::env::args().any(|arg| arg == "--stdio");
  let (connection, io_handle) = if use_stdio {
    let (conn, io) = Connection::stdio();
    (conn, IoHandle::Stdio(io))
  } else {
    connect_tcp()?
  };

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
      version: Some(format!("{} (built {})", env!("CARGO_PKG_VERSION"), env!("BUILD_TIMESTAMP"))),
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
