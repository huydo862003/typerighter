use std::io::BufReader;
use std::net::TcpListener;

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
        io.join().unwrap();
      }
      Self::Tcp { reader, writer } => {
        reader.join().unwrap();
        writer.join().unwrap();
      }
    }
  }
}

pub fn connect_stdio() -> (Connection, IoHandle) {
  let (conn, io) = Connection::stdio();
  (conn, IoHandle::Stdio(io))
}

/// Bind a TCP listener & print addr:port to stdout
pub fn connect_tcp(addr: &str, port: u16) -> anyhow::Result<(Connection, IoHandle)> {
  let listener = TcpListener::bind(format!("{addr}:{port}"))?;
  let bound_addr = listener.local_addr()?;
  // Clients (editors, rpc-server package) parse this to know where to connect
  println!("{bound_addr}");

  // Clone the stream so reader and writer threads each own a handle to the same socket
  let (stream, _) = listener.accept()?;
  let reader_stream = stream.try_clone()?;
  let mut writer_stream = stream;

  let (writer_sender, writer_receiver) = bounded::<Message>(16);
  let (reader_sender, reader_receiver) = bounded::<Message>(16);

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

  let connection = Connection {
    sender: writer_sender,
    receiver: reader_receiver,
  };
  Ok((connection, IoHandle::Tcp { reader, writer }))
}
