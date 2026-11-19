import {
  type ExtensionContext,
  commands,
  window,
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

  // Create Linked Resource command: triggers code actions filtered to linking actions
  context.subscriptions.push(
    commands.registerCommand('typedown.createLinkedResource', async () => {
      const actions = await commands.executeCommand<any[]>(
        'vscode.executeCodeActionProvider',
        window.activeTextEditor?.document.uri,
        window.activeTextEditor?.selection,
      );

      if (!actions) return;

      const linkActions = actions.filter(
        (action: any) =>
          action.title?.includes('Create new') && action.title?.includes('link')
          || action.title?.includes('linking back'),
      );

      if (linkActions.length === 0) {
        void window.showInformationMessage('No linked resource actions available at this position');

        return;
      }

      if (linkActions.length === 1) {
        // Single action: execute directly
        const action = linkActions[0];

        if (action.command) {
          await commands.executeCommand(action.command.command, ...action.command.arguments);
        }

        return;
      }

      // Multiple actions: let user pick
      const picked = await window.showQuickPick(
        linkActions.map((action: any) => ({
          label: action.title,
          action,
        })),
        {
          placeHolder: 'Select action',
        },
      );

      if (picked?.action?.command) {
        await commands.executeCommand(
          picked.action.command.command,
          ...picked.action.command.arguments,
        );
      }
    }),
  );
}

export function deactivate () {}
