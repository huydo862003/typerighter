use std::path::{Path, PathBuf};

use jsonrpsee::server::Server;
use typedown_incremental::Cancelled;
use typedown_server::rpc::contract::TdBuildRpcServer;
use typedown_server::rpc::server::RpcServer;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
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
  let port: u16 = std::env::var("TYPEDOWN_RPC_PORT")
    .unwrap_or_else(|_| "4747".to_string())
    .parse()
    .expect("TYPEDOWN_RPC_PORT must be a valid port number");
  let rpc_server = RpcServer::new(root_dir)?;
  let module = rpc_server.into_rpc();

  let server = Server::builder().build(format!("{addr}:{port}")).await?;
  let addr = server.local_addr()?;
  let handle = server.start(module);

  println!("ws://{addr}");

  handle.stopped().await;

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
