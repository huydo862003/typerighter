import {
  existsSync,
  createWriteStream,
} from 'node:fs';
import {
  chmod,
  mkdir,
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

// Returns the path to the LSP binary, downloading it if missing
export async function ensureBinary (): Promise<string> {
  const context = ExtensionContextManager.context;
  const version = context.extension.packageJSON.version as string;
  const binName = platform === 'win32' ? 'typedown-lsp.exe' : 'typedown-lsp';
  const storageDirectory = context.globalStorageUri.fsPath;
  const versionDirectory = join(storageDirectory, version);
  const binPath = join(versionDirectory, binName);

  if (existsSync(binPath)) return binPath;

  const platformKey = `${platform}-${arch === 'arm64' ? 'arm64' : 'x64'}`;
  const osArch = OS_ARCH_MAP[platformKey];

  if (osArch === undefined) {
    throw new Error(`Unsupported platform: ${platformKey}`);
  }

  const extension = platform === 'win32' ? '.exe' : '';
  const releaseTag = version.includes('-') ? `staging/v${version}` : `v${version}`;
  const artifact = `typedown-lsp-${version}-${osArch}${extension}`;
  const url = `https://github.com/huydo862003/typerighter/releases/download/${encodeURIComponent(releaseTag)}/${artifact}`;

  await window.withProgress(
    {
      location: {
        viewId: 'workbench.view.explorer',
      },
      title: `Downloading typedown-lsp ${version}...`,
    },
    async () => {
      await mkdir(versionDirectory, {
        recursive: true,
      });

      const result = await fetch(url);

      if (!result.ok) {
        throw new Error(`Failed to download LSP binary: HTTP ${result.status} from ${url}`);
      }

      if (!result.body) {
        throw new Error('Response body is empty');
      }

      const temporaryPath = `${binPath}.tmp`;

      try {
        await pipeline(
          Readable.fromWeb(result.body as ReadableStream),
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
    },
  );

  return binPath;
}
