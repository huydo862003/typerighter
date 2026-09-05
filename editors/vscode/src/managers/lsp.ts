import type {
  Disposable,
} from 'vscode';
import {
  window,
  workspace,
} from 'vscode';
import type {
  LanguageClientOptions,
  ServerOptions,
  StreamInfo,
} from 'vscode-languageclient/node';
import {
  LanguageClient,
  RevealOutputChannelOn,
} from 'vscode-languageclient/node';
import {
  createConnection,
} from 'net';
import {
  spawn,
} from 'child_process';
import {
  createInterface,
} from 'readline';
import {
  resolvePromptsAndExecute,
} from './command';
import {
  ensureBinary,
} from './ensureBinary';
import {
  LogManager,
} from './log';

// Spawn the LSP binary and read the TCP port it prints to stdout
function spawnAndConnect (binaryPath: string): Promise<StreamInfo> {
  return new Promise((resolve, reject) => {
    const proc = spawn(binaryPath, [], { stdio: ['ignore', 'pipe', 'inherit'] });

    proc.on('error', reject);

    const rl = createInterface({ input: proc.stdout! });

    rl.once('line', (line) => {
      rl.close();
      const port = Number(line.trim());

      if (!port) {
        reject(new Error(`Invalid port from LSP binary: ${line}`));

        return;
      }

      const socket = createConnection(port, '127.0.0.1', () => {
        resolve({ reader: socket, writer: socket });
      });
      socket.on('error', reject);
    });
  });
}

export class LspManager implements Disposable {
  private static instance: LspManager | undefined;
  private client: LanguageClient | undefined;
  private disposed = false;

  private constructor () {}

  // Resolves the binary and starts the LSP client
  // Never throws, errors are shown to the user via notification
  async start (): Promise<void> {
    if (this.client || this.disposed) return;

    let binaryPath: string;

    try {
      binaryPath = await ensureBinary();
    } catch (error) {
      window.showErrorMessage(`Typerighter: failed to set up LSP binary. ${error}`);

      return;
    }

    // Extension was deactivated while downloading
    if (this.disposed) return;

    const serverOptions: ServerOptions = () => spawnAndConnect(binaryPath);

    const clientOptions: LanguageClientOptions = {
      documentSelector: [
        {
          scheme: 'file',
          language: 'typedown',
        },
      ],
      workspaceFolder: workspace.workspaceFolders?.[0],
      outputChannel: LogManager.getInstance().mainChannel,
      revealOutputChannelOn: RevealOutputChannelOn.Error,
      middleware: {
        executeCommand: async (command, args, next) => {
          if (command.startsWith('_typerighter.')) {
            await resolvePromptsAndExecute(this.client!, command, args);

            return;
          }

          return next(command, args);
        },
      },
    };

    this.client = new LanguageClient(
      'typedown-lsp',
      'Typedown LSP',
      serverOptions,
      clientOptions,
    );

    await this.client.start();
  }

  static getInstance (): LspManager {
    if (!LspManager.instance) {
      LspManager.instance = new LspManager();
    }

    return LspManager.instance;
  }

  dispose (): Thenable<void> {
    this.disposed = true;

    return this.client?.stop() ?? Promise.resolve();
  }
}
