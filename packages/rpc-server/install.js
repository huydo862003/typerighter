import { execFileSync } from "node:child_process";
import { mkdirSync, chmodSync } from "node:fs";
import path from "node:path";
import {
  dev,
  osArch,
  releaseTag,
  artifactName,
  artifactUrl,
  binPath,
  repoRoot,
} from "./platform.js";

const bin = binPath();

// In dev mode, we compile and `bin` will just automatically point to the artifact
if (dev()) {
  console.log("[rpc-server] Development mode: building typedown-rpc with cargo");
  try {
    execFileSync("cargo", ["build", "-p", "typedown-server"], {
      cwd: repoRoot(),
      stdio: "inherit",
    });
  } catch {
    console.error("[rpc-server] cargo build failed");
    process.exit(1);
  }
  process.exit(0);
}

// In non dev mode, fetch the artifacts
const arch = osArch();
const tag = releaseTag();
const artifact = artifactName(arch);
const url = artifactUrl(tag, artifact);

const binDir = path.dirname(bin);
mkdirSync(binDir, { recursive: true });

console.log(`[rpc-server] Downloading typedown-rpc from ${url}`);

try {
  execFileSync("curl", ["-fsSL", "-o", bin, url], { stdio: "inherit" });
} catch {
  console.error(`[rpc-server] Failed to download typedown-rpc from ${url}`);
  console.error(
    "[rpc-server] You can build it manually: cargo build --release -p typedown-server",
  );
  process.exit(1);
}

if (process.platform !== "win32") {
  chmodSync(bin, 0o755);
}

console.log("[rpc-server] typedown-rpc installed successfully");
