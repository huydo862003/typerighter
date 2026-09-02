import type {
  ExtensionContext,
} from 'vscode';
import {
  ExtensionContextManager,
  LogManager,
  LspManager,
  SchemaTemplateManager,
  PasteHandlerManager,
} from './managers';

export function activate (context: ExtensionContext) {
  ExtensionContextManager.initialize(context);

  // logManager pushed first so it is disposed last (VSCode uses LIFO order)
  // The LSP client must be stopped before its output channel is torn down
  context.subscriptions.push(LogManager.getInstance());

  const lspManager = LspManager.getInstance();

  context.subscriptions.push(lspManager);

  // Binary download and LSP start run in the background
  // Non-LSP features work immediately
  void lspManager.start();

  context.subscriptions.push(SchemaTemplateManager.getInstance());
  context.subscriptions.push(PasteHandlerManager.getInstance());
}

export function deactivate () {}
