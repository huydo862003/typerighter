use std::io::BufReader;
use std::net::TcpListener;
use std::sync::{Arc, Mutex};

use crossbeam_channel::bounded;
use lsp_server::{Connection, Message};

pub enum IoHandle {
  Stdio(lsp_server::IoThreads),
  Tcp {
    reader: std::thread::JoinHandle<()>,
    writer: std::thread::JoinHandle<()>,
  },
}

impl IoHandle {
  pub fn join(self) {
    match self {
      Self::Stdio(io) => {
        let _ = io.join();
      }
      Self::Tcp { reader, writer } => {
        let _ = reader.join();
        let _ = writer.join();
      }
    }
  }
}

pub fn connect_stdio() -> (Connection, IoHandle) {
  let (conn, io) = Connection::stdio();
  (conn, IoHandle::Stdio(io))
}

pub type ShutdownSocket = Arc<Mutex<Option<std::net::TcpStream>>>;

/// Bind a TCP listener, print addr:port to stdout, and accept the first connection
/// The listener is returned so the caller can accept subsequent connections via `accept_tcp`
pub fn connect_tcp(
  addr: &str,
  port: u16,
) -> anyhow::Result<(Connection, IoHandle, TcpListener, ShutdownSocket)> {
  let listener = TcpListener::bind(format!("{addr}:{port}"))?;
  let bound_addr = listener.local_addr()?;
  // Clients (editors, rpc-server package) parse this to know where to connect
  println!("{bound_addr}");

  let (conn, io, shutdown) = accept_tcp(&listener)?;
  Ok((conn, io, listener, shutdown))
}

/// Accept the next client connection on an existing listener
pub fn accept_tcp(
  listener: &TcpListener,
) -> anyhow::Result<(Connection, IoHandle, ShutdownSocket)> {
  // Clone the stream so reader and writer threads each own a handle to the same socket
  let (stream, _) = listener.accept()?;
  stream.set_nodelay(true)?; // disable Nagle: small JSON-RPC messages need low latency, not coalescing
  let shutdown_socket: ShutdownSocket = Arc::new(Mutex::new(Some(stream.try_clone()?)));
  let reader_stream = stream.try_clone()?;
  let mut writer_stream = stream;

  let (writer_sender, writer_receiver) = bounded::<Message>(16);
  let (reader_sender, reader_receiver) = bounded::<Message>(16);

  let reader = std::thread::spawn(move || {
    let mut buffer_read = BufReader::new(reader_stream);
    while let Ok(Some(message)) = Message::read(&mut buffer_read) {
      let is_exit =
        matches!(&message, Message::Notification(notification) if notification.method == "exit");
      reader_sender.send(message).unwrap();
      if is_exit {
        break;
      }
    }
  });

  let writer = std::thread::spawn(move || {
    for message in writer_receiver {
      if message.write(&mut writer_stream).is_err() {
        break;
      }
    }
  });

  let connection = Connection {
    sender: writer_sender,
    receiver: reader_receiver,
  };
  Ok((
    connection,
    IoHandle::Tcp { reader, writer },
    shutdown_socket,
  ))
}
