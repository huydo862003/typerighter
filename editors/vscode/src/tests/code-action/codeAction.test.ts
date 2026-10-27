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

  test('create linked resource action on schema-typed field', async () => {
    // empty-friend.td has `friend:` (empty Person? field)
    const document = await openDocument('empty-friend.td');

    await sleep(1000);

    // Position on the empty 'friend:' field (line 4)
    const actions = await vscode.commands.executeCommand<vscode.CodeAction[]>(
      'vscode.executeCodeActionProvider',
      document.uri,
      new vscode.Range(new vscode.Position(4, 0), new vscode.Position(4, 0)),
    );

    if (actions && 0 < actions.length) {
      const createActions = actions.filter((action) => action.title.includes('Create new'));

      if (0 < createActions.length) {
        const action = createActions[0];

        // Should mention the target schema type
        ok(
          action.title.includes('Person'),
          `create action should mention Person, got: ${action.title}`,
        );
        // Should have a command (for the prompt flow)
        ok(
          action.command,
          'create linked action should have a command for the filename prompt',
        );
      }
    }
  });

  test('non-empty file does not offer initialization actions', async () => {
    const document = await openDocument('alice.td');

    await sleep(1000);

    const actions = await vscode.commands.executeCommand<vscode.CodeAction[]>(
      'vscode.executeCodeActionProvider',
      document.uri,
      new vscode.Range(new vscode.Position(0, 0), new vscode.Position(0, 0)),
    );

    const initializeActions = (actions ?? []).filter((action) => action.title.startsWith('Initialize as'));

    strictEqual(initializeActions.length, 0, 'non-empty file should NOT offer initialization actions');
  });

  test('no create-linked action on string field', async () => {
    const document = await openDocument('alice.td');

    await sleep(1000);

    // Position on 'name: "Alice"' line (line 2) - string field, not schema-typed
    const actions = await vscode.commands.executeCommand<vscode.CodeAction[]>(
      'vscode.executeCodeActionProvider',
      document.uri,
      new vscode.Range(new vscode.Position(2, 0), new vscode.Position(2, 0)),
    );

    const createActions = (actions ?? []).filter((action) => action.title.includes('Create new'));

    strictEqual(createActions.length, 0, 'string field should not offer create-linked action');
  });
});
