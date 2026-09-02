import {
  constants,
  createWriteStream,
} from 'node:fs';
import {
  access,
  chmod,
  mkdir,
  readdir,
  rename,
  rm,
} from 'node:fs/promises';
import {
  join,
} from 'node:path';
import {
  Readable,
} from 'node:stream';
import {
  pipeline,
} from 'node:stream/promises';
import {
  arch,
  platform,
} from 'node:process';
import {
  ProgressLocation,
  window,
} from 'vscode';
import {
  ExtensionContextManager,
} from './extensionContext';

const OS_ARCH_MAP: Record<string, string> = {
  'linux-x64': 'linux-x86_64',
  'darwin-x64': 'darwin-x86_64',
  'darwin-arm64': 'darwin-aarch64',
  'win32-x64': 'windows-x86_64',
};

const DOWNLOAD_TIMEOUT_MS = 60_000;

// Returns the path to the LSP binary, downloading it if missing or corrupted
// Checks for a local dev binary first, placed by setup-binary.mjs during development
export async function ensureBinary (): Promise<string> {
  const context = ExtensionContextManager.context;
  const binName = platform === 'win32' ? 'typedown-lsp.exe' : 'typedown-lsp';

  // Prefer local binary from bin/ if present
  const localBinary = join(context.extensionPath, 'bin', binName);

  if (await isExecutable(localBinary)) return localBinary;

  // Otherwise download to globalStorageUri/{version}/
  const version = context.extension.packageJSON.version;

  if (typeof version !== 'string') {
    throw new Error('Extension version is missing from package.json');
  }

  const storageDirectory = context.globalStorageUri.fsPath;
  const versionDirectory = join(storageDirectory, version);
  const binPath = join(versionDirectory, binName);

  if (await isExecutable(binPath)) return binPath;

  const platformKey = `${platform}-${arch === 'arm64' ? 'arm64' : 'x64'}`;
  const osArch = OS_ARCH_MAP[platformKey];

  if (osArch === undefined) {
    throw new Error(`Unsupported platform: ${platformKey}`);
  }

  const binaryExtension = platform === 'win32' ? '.exe' : '';
  const releaseTag = version.includes('-') ? `staging/v${version}` : `v${version}`;
  const artifact = `typedown-lsp-${version}-${osArch}${binaryExtension}`;
  const url = `https://github.com/huydo862003/typerighter/releases/download/${encodeURIComponent(releaseTag)}/${artifact}`;

  await window.withProgress(
    {
      location: ProgressLocation.Notification,
      title: `Downloading typedown-lsp ${version}`,
      cancellable: false,
    },
    async () => {
      await mkdir(versionDirectory, {
        recursive: true,
      });

      const controller = new AbortController();
      const timeout = setTimeout(() => controller.abort(), DOWNLOAD_TIMEOUT_MS);

      try {
        const result = await fetch(url, {
          signal: controller.signal,
        });

        if (!result.ok) {
          throw new Error(`Failed to download LSP binary: HTTP ${result.status} from ${url}`);
        }

        if (!result.body) {
          throw new Error('Response body is empty');
        }

        const temporaryPath = `${binPath}.tmp`;

        try {
          await pipeline(
            Readable.fromWeb(result.body as ReadableStream, {
              signal: controller.signal,
            }),
            createWriteStream(temporaryPath),
          );

          if (platform !== 'win32') await chmod(temporaryPath, 0o755);
          await rename(temporaryPath, binPath);
        } catch (error) {
          await rm(temporaryPath, {
            force: true,
          });
          throw error;
        }
      } finally {
        clearTimeout(timeout);
      }
    },
  );

  // Clean up old version directories
  void cleanOldVersions(storageDirectory, version);

  return binPath;
}

// Removes old version directories from the storage, keeping only the current version
async function cleanOldVersions (storageDirectory: string, currentVersion: string): Promise<void> {
  try {
    for (const entry of await readdir(storageDirectory)) {
      if (entry !== currentVersion) {
        await rm(join(storageDirectory, entry), {
          recursive: true,
          force: true,
        });
      }
    }
  } catch {
    // Best-effort cleanup
  }
}

async function isExecutable (filePath: string): Promise<boolean> {
  try {
    await access(filePath, platform === 'win32' ? constants.F_OK : constants.X_OK);

    return true;
  } catch {
    return false;
  }
}
