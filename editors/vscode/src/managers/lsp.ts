import {
  join,
} from 'node:path';
import {
  platform,
} from 'node:process';
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
  ExtensionContextManager,
} from './extensionContext';
import {
  LogManager,
} from './log';

export class LspManager implements Disposable {
  private static instance: LspManager | undefined;
  private readonly client: LanguageClient;

  private constructor (client: LanguageClient) {
    this.client = client;
  }

  static getInstance (): LspManager {
    if (!LspManager.instance) {
      const context = ExtensionContextManager.context;
      const binName = platform === 'win32' ? 'typedown-lsp.exe' : 'typedown-lsp';

      const serverOptions: ServerOptions = {
        command: context.asAbsolutePath(join('bin', binName)),
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
