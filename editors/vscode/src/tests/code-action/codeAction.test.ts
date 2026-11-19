import {
  ok, strictEqual,
} from 'node:assert';
// eslint-disable-next-line import/no-namespace
import * as vscode from 'vscode';
import {
  activate, openDocument, closeAllEditors, pauseMs,
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

    await pauseMs(1000);

    // Position on the empty 'friend:' field (line 4)
    const actions = await vscode.commands.executeCommand<vscode.CodeAction[]>(
      'vscode.executeCodeActionProvider',
      document.uri,
      new vscode.Range(new vscode.Position(4, 0), new vscode.Position(4, 0)),
    );

    const createActions = (actions ?? []).filter((action) => action.title.includes('Create new'));

    ok(0 < createActions.length, 'should offer at least one create action on schema-typed field');

    const actionWithCommand = createActions.find((action) => action.command);

    ok(actionWithCommand, 'at least one create action should have a command for the prompt flow');
  });

  test('non-empty file does not offer initialization actions', async () => {
    const document = await openDocument('alice.td');

    await pauseMs(1000);

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

    await pauseMs(1000);

    // Position on 'name: "Alice"' line (line 2) - string field, not schema-typed
    const actions = await vscode.commands.executeCommand<vscode.CodeAction[]>(
      'vscode.executeCodeActionProvider',
      document.uri,
      new vscode.Range(new vscode.Position(2, 0), new vscode.Position(2, 0)),
    );

    // Filter for schema-typed field actions specifically (not backward links)
    const typedFieldActions = (actions ?? []).filter((action) => action.title.includes('and link to'));

    strictEqual(typedFieldActions.length, 0, 'string field should not offer typed-field create-linked action');
  });
});
