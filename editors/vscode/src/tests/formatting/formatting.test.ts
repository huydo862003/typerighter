import {
  ok,
} from 'node:assert';
// eslint-disable-next-line import/no-namespace
import * as vscode from 'vscode';
import {
  activate, openDocument, closeAllEditors, sleep,
} from '../helper';

suite('Formatting', () => {
  suiteSetup(async () => {
    await activate();
  });

  teardown(async () => {
    await closeAllEditors();
  });

  test('format provider responds for a typedown document', async () => {
    const document = await openDocument('alice.td');

    await sleep(1000);

    const edits = await vscode.commands.executeCommand<vscode.TextEdit[]>(
      'vscode.executeFormatDocumentProvider',
      document.uri,
      {
        tabSize: 2,
        insertSpaces: true,
      } as vscode.FormattingOptions,
    );

    ok(edits !== undefined && edits !== null, 'format provider should respond');
    ok(Array.isArray(edits), 'formatting result should be an array of TextEdits');
  });
});
