import { createRequire } from "node:module";
import { existsSync, readFileSync } from "node:fs";
import path from "node:path";
import process from "node:process";
import dotenv from "dotenv";

const require = createRequire(import.meta.url);
const pkg = require("./package.json");

const env = dotenv.config({ path: new URL(".env", import.meta.url) });
const isDev =
  process.env.DEV !== undefined
    ? process.env.DEV === "true"
    : env.parsed?.DEV === "true";

export function dev() {
  return isDev;
}

export function isNixOS() {
  if (process.platform !== "linux") {
    return false;
  }
  if (existsSync("/etc/NIXOS") || existsSync("/etc/nixos")) {
    return true;
  }
  try {
    const osRelease = readFileSync("/etc/os-release", "utf8");
    return /ID(?:_LIKE)?=["']?nixos["']?/i.test(osRelease);
  } catch {
    return false;
  }
}

const PLATFORM_MAP = {
  "linux-x64": "linux-x86_64",
  "darwin-x64": "darwin-x86_64",
  "darwin-arm64": "darwin-aarch64",
  "win32-x64": "windows-x86_64",
};

export function osArch() {
  const key = `${process.platform}-${process.arch}`;
  const mapped = PLATFORM_MAP[key];
  if (!mapped) {
    throw new Error(
      `Unsupported platform: ${process.platform} ${process.arch}`,
    );
  }
  return mapped;
}

export function releaseTag() {
  const version = pkg.version;
  if (version.includes("-")) {
    return `staging/v${version}`;
  }
  return `v${version}`;
}

export function artifactName(osArchStr) {
  const ext = process.platform === "win32" ? ".exe" : "";
  return `typedown-rpc-${pkg.version}-${osArchStr}${ext}`;
}

export function artifactUrl(tag, artifact) {
  return `https://github.com/huydo862003/typerighter/releases/download/${tag}/${artifact}`;
}

export function repoRoot() {
  return path.resolve(path.dirname(import.meta.filename), "..", "..");
}

export function binPath() {
  const ext = process.platform === "win32" ? ".exe" : "";
  if (isDev) {
    return path.join(repoRoot(), "target", "debug", `typedown-rpc${ext}`);
  }
  return path.join(path.dirname(import.meta.filename), "bin", `typedown-rpc${ext}`);
}
