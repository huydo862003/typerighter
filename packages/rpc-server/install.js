import { execFileSync } from "node:child_process";
import { mkdirSync, chmodSync, copyFileSync, existsSync } from "node:fs";
import path from "node:path";
import {
  dev,
  isNixOS,
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

// On NixOS, build using Nix derivation
if (isNixOS()) {
  const tag = releaseTag();
  const flakeTarget = existsSync(path.join(repoRoot(), "flake.nix"))
    ? ".#typedown-rpc"
    : `github:huydo862003/typerighter/${tag}#typedown-rpc`;

  console.log(
    `[rpc-server] NixOS detected: building typedown-rpc using Nix derivation (${flakeTarget})`,
  );
  const binDir = path.dirname(bin);
  mkdirSync(binDir, { recursive: true });

  try {
    const outPath = execFileSync(
      "nix",
      [
        "--extra-experimental-features",
        "nix-command flakes",
        "build",
        flakeTarget,
        "--no-link",
        "--print-out-paths",
      ],
      {
        cwd: repoRoot(),
        encoding: "utf8",
      },
    ).trim();

    const builtBin = path.join(outPath, "bin", "typedown-rpc");
    copyFileSync(builtBin, bin);
    if (process.platform !== "win32") {
      chmodSync(bin, 0o755);
    }
    console.log("[rpc-server] typedown-rpc built and installed successfully via Nix");
    process.exit(0);
  } catch {
    console.warn("[rpc-server] nix build failed, falling back to binary download");
  }
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
