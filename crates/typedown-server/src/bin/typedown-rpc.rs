use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

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

  let mut args = pico_args::Arguments::from_env();

  if args.contains("--help") {
    eprintln!("Usage: typedown-rpc [OPTIONS]");
    eprintln!();
    eprintln!("Options:");
    eprintln!("  --root <path>       Vault root directory (default: cwd)");
    eprintln!("  --addr <addr>       Listen address (default: 127.0.0.1)");
    eprintln!("  --port <port>       Listen port (default: 0, auto-assign)");
    eprintln!("  --fresh          Start fresh, ignore cache from previous session");
    eprintln!("  --help              Show this help");
    return Ok(());
  }

  // CLI flags take precedence over env vars
  let root: PathBuf = args
    .opt_value_from_str("--root")?
    .or_else(|| std::env::var("TYPEDOWN_RPC_ROOT").ok().map(PathBuf::from))
    .unwrap_or_else(|| std::env::current_dir().expect("failed to get current directory"));

  let addr: String = args
    .opt_value_from_str("--addr")?
    .or_else(|| std::env::var("TYPEDOWN_RPC_ADDR").ok())
    .unwrap_or_else(|| "127.0.0.1".to_string());

  let port: u16 = args
    .opt_value_from_str("--port")?
    .or_else(|| {
      std::env::var("TYPEDOWN_RPC_PORT")
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

  let root_dir = find_vault_root(&root)?;

  if fresh {
    // SAFETY: single-threaded at this point, before any server threads are spawned
    unsafe { std::env::set_var("TYPEDOWN_NO_CACHE", "1") };
  }

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
