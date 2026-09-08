use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use typedown_incremental::Cancelled;
use typedown_server::core::transport;
use typedown_server::rpc::server::RpcServer;

pub fn main() -> anyhow::Result<()> {
  // Cancelled panics are expected control flow in the incremental query engine
  // Suppress them so they don't print "Box<dyn Any>" to stderr
  let default_hook = std::panic::take_hook();
  std::panic::set_hook(Box::new(move |info| {
    if info.payload().downcast_ref::<Cancelled>().is_some() {
      return;
    }
    default_hook(info);
  }));

  let start = std::env::var("TYPEDOWN_RPC_ROOT")
    .map(PathBuf::from)
    .unwrap_or_else(|_| std::env::current_dir().expect("failed to get current directory"));

  let root_dir = find_vault_root(&start)?;

  let addr = std::env::var("TYPEDOWN_RPC_ADDR").unwrap_or_else(|_| "127.0.0.1".to_string());
  let port = std::env::var("TYPEDOWN_RPC_PORT")
    .ok()
    .and_then(|port_str| port_str.parse::<u16>().ok())
    .unwrap_or(0);
  let (connection, io_handle, shutdown_socket) = transport::connect_tcp(&addr, port)?;

  // On SIGINT, shut down the TCP socket so the reader thread unblocks and the server exits cleanly
  let interrupted = Arc::new(AtomicBool::new(false));
  {
    let interrupted = Arc::clone(&interrupted);
    ctrlc::set_handler(move || {
      interrupted.store(true, Ordering::Relaxed);
      if let Some(socket) = shutdown_socket.lock().unwrap().take() {
        let _ = socket.shutdown(std::net::Shutdown::Both);
      }
    })
    .ok();
  }

  let server = RpcServer::new(connection, root_dir)?;
  server.run()?;
  server.shutdown();

  if !interrupted.load(Ordering::Relaxed) {
    io_handle.join();
  }

  Ok(())
}

fn find_vault_root(start: &Path) -> anyhow::Result<PathBuf> {
  let mut current = start.to_path_buf();
  loop {
    if current.join("typedown.yaml").exists() || current.join("typedown.yml").exists() {
      return Ok(current);
    }
    if !current.pop() {
      anyhow::bail!(
        "no typedown.yaml or typedown.yml found in {} or any parent",
        start.display()
      );
    }
  }
}
