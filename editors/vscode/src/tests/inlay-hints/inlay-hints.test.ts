import {
  ok,
} from 'node:assert';
// eslint-disable-next-line import/no-namespace
import * as vscode from 'vscode';
import {
  activate, openDocument, closeAllEditors, sleep,
} from '../helper';

suite('Inlay Hints', () => {
  suiteSetup(async () => {
    await activate();
  });

  teardown(async () => {
    await closeAllEditors();
  });

  test('inlay hint provider responds for a document range', async () => {
    const document = await openDocument('alice.td');

    await sleep(1000);

    const hints = await vscode.commands.executeCommand<vscode.InlayHint[]>(
      'vscode.executeInlayHintProvider',
      document.uri,
      new vscode.Range(new vscode.Position(0, 0), new vscode.Position(document.lineCount, 0)),
    );

    ok(hints !== undefined && hints !== null, 'inlay hint provider should respond');
    ok(Array.isArray(hints), 'inlay hints result should be an array');
  });
});
