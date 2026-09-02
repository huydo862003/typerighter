import type {
  Disposable,
} from 'vscode';
import {
  workspace,
} from 'vscode';
import type {
  LanguageClientOptions,
  ServerOptions,
} from 'vscode-languageclient/node';
import {
  LanguageClient,
  RevealOutputChannelOn,
  TransportKind,
} from 'vscode-languageclient/node';
import {
  resolvePromptsAndExecute,
} from './command';
import {
  LogManager,
} from './log';

export class LspManager implements Disposable {
  private static instance: LspManager | undefined;
  private readonly client: LanguageClient;

  private constructor (client: LanguageClient) {
    this.client = client;
  }

  static getInstance (binaryPath: string): LspManager {
    if (!LspManager.instance) {
      const serverOptions: ServerOptions = {
        command: binaryPath,
        transport: TransportKind.stdio,
      };

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
              await resolvePromptsAndExecute(client, command, args);

              return;
            }

            return next(command, args);
          },
        },
      };

      const client = new LanguageClient(
        'typedown-lsp',
        'Typedown LSP',
        serverOptions,
        clientOptions,
      );

      client.start();
      LspManager.instance = new LspManager(client);
    }

    return LspManager.instance;
  }

  dispose (): Thenable<void> {
    return this.client.stop();
  }
}
