import { existsSync } from "node:fs";
import { execFileSync, spawn } from "node:child_process";
import { createInterface } from "node:readline";
import { EventEmitter } from "node:events";
import { binPath } from "./platform.js";

function resolveBin() {
  // Prefer local binary (dev build or downloaded release)
  const local = binPath();
  if (existsSync(local)) return local;

  // Fall back to system binary
  try {
    const system = execFileSync("which", ["typedown-rpc"], { encoding: "utf-8" }).trim();
    if (system) return system;
  } catch {
    // ignore
  }

  throw new Error(
    `typedown-rpc binary not found. Run "pnpm install" to download it, ` +
      `install via Nix, or build manually with "cargo build --release -p typedown-server".`,
  );
}

const bin = resolveBin();

export class RpcServer extends EventEmitter {
  constructor({ root, addr, port } = {}) {
    super();
    this._root = root ?? process.cwd();
    this._addr = addr ?? "127.0.0.1";
    this._port = port ?? 0;
    this._process = undefined;
    this._listening = false;
    this._resolvedAddress = undefined;
    this._resolvedPort = undefined;
  }

  get address() {
    if (!this._listening) return undefined;
    return this._resolvedAddress;
  }

  get host() {
    if (!this._listening) return undefined;
    return this._addr;
  }

  get port() {
    if (!this._listening) return undefined;
    return this._resolvedPort;
  }

  get listening() {
    return this._listening;
  }

  listen(callback) {
    if (this._process) {
      throw new Error("Server is already running");
    }

    const child = spawn(bin, [], {
      cwd: this._root,
      env: {
        ...process.env,
        TYPEDOWN_RPC_ROOT: this._root,
        TYPEDOWN_RPC_ADDR: this._addr,
        TYPEDOWN_RPC_PORT: String(this._port),
      },
      stdio: ["ignore", "pipe", "inherit"],
      // Detach so the server can save its cache after Node exits
      detached: true,
    });

    this._process = child;

    const stdout = child.stdout;
    if (!stdout) {
      this.emit("error", new Error("Failed to capture typedown-rpc stdout"));
      return this;
    }

    // The server prints addr:port as the first line to stdout
    const reader = createInterface({ input: stdout });
    reader.once("line", (line) => {
      reader.close();
      const address = line.trim();
      this._resolvedAddress = address;
      const colonIndex = address.lastIndexOf(":");
      this._resolvedPort = Number(address.slice(colonIndex + 1));
      this._listening = true;
      this.emit("listening");
      if (callback) callback();
    });

    child.on("error", (error) => {
      this.emit("error", error);
    });

    child.on("exit", (code, signal) => {
      this._listening = false;
      this._process = undefined;
      this._resolvedAddress = undefined;
      this._resolvedPort = undefined;
      this.emit("close", code, signal);
    });

    return this;
  }

  // Unref the child process so it does not keep the Node event loop alive
  unref() {
    if (this._process) {
      this._process.unref();
    }
    return this;
  }

  // Let the server detect the socket close and save cache in the background
  // The process is already unref'd so it won't block Node from exiting
  close() {
    return this;
  }
}
