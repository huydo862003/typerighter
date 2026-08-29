use std::fs::File;
use std::io::Write;
use std::path::PathBuf;
use std::sync::{Mutex, OnceLock};

use crossbeam_channel::Sender;
use log::{Level, Log, Metadata, Record};
use lsp_server::{Message, Notification};
use lsp_types::notification::{LogMessage, Notification as _};
use lsp_types::{LogMessageParams, MessageType};

// The LSP sender is set after the handshake completes
static LSP_SENDER: OnceLock<Sender<Message>> = OnceLock::new();

// Sends log messages to both a local file and the LSP channel
// File logging starts immediately, LSP logging starts after set_lsp_sender
struct Logger {
  file: Option<Mutex<File>>,
}

impl Log for Logger {
  fn enabled(&self, _metadata: &Metadata) -> bool {
    true
  }

  fn log(&self, record: &Record) {
    let message = format!("{}", record.args());
    let level = match record.level() {
      Level::Error => "ERROR",
      Level::Warn => "WARN",
      Level::Info => "INFO",
      Level::Debug => "DEBUG",
      Level::Trace => "TRACE",
    };

    // Write to log file
    if let Some(file) = &self.file
      && let Ok(mut file) = file.lock()
    {
      let secs = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
      let time_of_day = secs % 86400;
      let hours = time_of_day / 3600;
      let minutes = (time_of_day % 3600) / 60;
      let seconds = time_of_day % 60;
      let _ = writeln!(
        file,
        "[{hours:02}:{minutes:02}:{seconds:02}] [{level}] {message}"
      );
      // Flush on every write so logs survive crashes
      let _ = file.flush();
    }

    // Send to LSP client if available
    if let Some(sender) = LSP_SENDER.get() {
      let typ = match record.level() {
        Level::Error => MessageType::ERROR,
        Level::Warn => MessageType::WARNING,
        Level::Info => MessageType::INFO,
        Level::Debug | Level::Trace => MessageType::LOG,
      };
      let params = LogMessageParams { typ, message };
      let notification = Notification::new(LogMessage::METHOD.to_string(), params);
      let _ = sender.try_send(Message::Notification(notification));
    }
  }

  fn flush(&self) {}
}

/// Initialize file-only logging.
/// Call early so messages before the LSP handshake are captured.
pub fn init_file() {
  let file = open_log_file();
  let logger = Box::leak(Box::new(Logger { file }));
  log::set_max_level(log::LevelFilter::Info);
  let _ = log::set_logger(logger);
}

/// Start sending log messages to the LSP client as well.
/// Call after the LSP handshake completes.
pub fn set_lsp_sender(sender: Sender<Message>) {
  let _ = LSP_SENDER.set(sender);
}

const MAX_LOG_AGE: std::time::Duration = std::time::Duration::from_secs(7 * 24 * 60 * 60); // 7 days

fn open_log_file() -> Option<Mutex<File>> {
  let log_dir = log_dir()?;
  std::fs::create_dir_all(&log_dir).ok()?;

  // Delete log files older than 7 days
  let log_path = log_dir.join("typedown-lsp.log");
  if let Ok(meta) = std::fs::metadata(&log_path) {
    let too_old = meta
      .modified()
      .ok()
      .and_then(|mtime| mtime.elapsed().ok())
      .is_some_and(|age| age > MAX_LOG_AGE);
    if too_old {
      let _ = std::fs::remove_file(&log_path);
    }
  }

  let file = std::fs::OpenOptions::new()
    .create(true)
    .append(true)
    .open(&log_path)
    .ok()?;
  Some(Mutex::new(file))
}

fn log_dir() -> Option<PathBuf> {
  if let Ok(dir) = std::env::var("XDG_STATE_HOME") {
    return Some(PathBuf::from(dir));
  }
  let home = std::env::var("HOME").ok()?;
  Some(PathBuf::from(home).join(".local/state"))
}
