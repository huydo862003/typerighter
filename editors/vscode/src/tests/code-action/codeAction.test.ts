import {
  ok, strictEqual,
} from 'node:assert';
// eslint-disable-next-line import/no-namespace
import * as vscode from 'vscode';
import {
  activate, openDocument, closeAllEditors, sleep,
} from '../helper';

suite('Code Actions', () => {
  suiteSetup(async () => {
    await activate();
  });

  teardown(async () => {
    await closeAllEditors();
  });

  test('empty file offers schema initialization actions', async () => {
    const document = await openDocument('empty.td');

    await sleep(1000);

    const actions = await vscode.commands.executeCommand<vscode.CodeAction[]>(
      'vscode.executeCodeActionProvider',
      document.uri,
      new vscode.Range(new vscode.Position(0, 0), new vscode.Position(0, 0)),
    );

    ok(actions && 0 < actions.length, 'should offer code actions for empty file');

    const initializeActions = actions.filter((action) => action.title.startsWith('Initialize as'));

    ok(0 < initializeActions.length, `should have "Initialize as" actions, got: ${actions.map((action) => action.title).join(', ')}`);
    // Each init action should have a workspace edit attached
    for (const action of initializeActions) {
      ok(action.edit || action.command, `"${action.title}" should have an edit or command`);
    }
  });

  test('non-empty file does not offer initialization actions', async () => {
    // empty-vault only has empty.td, but let's verify the typedown.yaml
    // doesn't get offered init actions (it's not a .td file, so no actions)
    const document = await openDocument('empty.td');

    await sleep(1000);

    // First, write some content to make it non-empty
    const editor = await vscode.window.showTextDocument(document);

    await editor.edit((builder) => {
      builder.insert(new vscode.Position(0, 0), '---\n_type: schema\n---\n');
    });

    await sleep(1000);

    const actions = await vscode.commands.executeCommand<vscode.CodeAction[]>(
      'vscode.executeCodeActionProvider',
      document.uri,
      new vscode.Range(new vscode.Position(0, 0), new vscode.Position(0, 0)),
    );

    const initializeActions = (actions ?? []).filter((action) => action.title.startsWith('Initialize as'));

    strictEqual(initializeActions.length, 0, 'non-empty file should NOT offer initialization actions');

    // Revert the edit
    await vscode.commands.executeCommand('workbench.action.files.revert');
  });
});
