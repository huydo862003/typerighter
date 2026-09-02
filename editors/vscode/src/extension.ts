import type {
  ExtensionContext,
} from 'vscode';
import {
  window,
} from 'vscode';
import {
  ExtensionContextManager,
  ensureBinary,
  LogManager,
  LspManager,
  SchemaTemplateManager,
  PasteHandlerManager,
} from './managers';

export async function activate (context: ExtensionContext) {
  ExtensionContextManager.initialize(context);

  let binaryPath: string;

  try {
    binaryPath = await ensureBinary();
  } catch (error) {
    window.showErrorMessage(`Typerighter: failed to set up LSP binary. ${error}`);

    return;
  }

  // logManager pushed first so it is disposed last (VSCode uses LIFO order)
  // The LSP client must be stopped before its output channel is torn down
  context.subscriptions.push(LogManager.getInstance());
  context.subscriptions.push(LspManager.getInstance(binaryPath));
  context.subscriptions.push(SchemaTemplateManager.getInstance());
  context.subscriptions.push(PasteHandlerManager.getInstance());
}

export function deactivate () {}
