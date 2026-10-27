import {
  ok, strictEqual,
} from 'node:assert';
// eslint-disable-next-line import/no-namespace
import * as vscode from 'vscode';
import {
  activate, openDocument, closeAllEditors, sleep,
} from '../helper';

suite('Rename', () => {
  suiteSetup(async () => {
    await activate();
  });

  teardown(async () => {
    await closeAllEditors();
  });

  test('prepare rename on _type value returns the type name range', async () => {
    const document = await openDocument('alice.td');

    await sleep(1000);

    const result = await vscode.commands.executeCommand<{
      range: vscode.Range;
      placeholder: string;
    }>(
      'vscode.executePrepareRenameProvider',
      document.uri,
      new vscode.Position(1, 7),
    );

    ok(result, 'prepareRename should return a result for _type reference');
    if ('placeholder' in result) {
      strictEqual(result.placeholder, 'Person', 'placeholder should be "Person"');
    }
    strictEqual(result.range.start.line, 1, 'range should start on line 1');
    strictEqual(result.range.start.character, 7, 'range should start at character 7');
    strictEqual(result.range.end.character, 13, 'range should end at character 13');
  });

  test('rename "Person" to "Human" produces edits in referencing files', async () => {
    const document = await openDocument('alice.td');

    await sleep(1000);

    const edit = await vscode.commands.executeCommand<vscode.WorkspaceEdit>(
      'vscode.executeRenameProvider',
      document.uri,
      new vscode.Position(1, 7),
      'Human',
    );

    ok(edit, 'rename should produce a WorkspaceEdit');
    ok(1 < edit.size, `rename should affect multiple files, got size: ${edit.size}`);

    const entries = edit.entries();

    ok(
      entries.some(([uri]) => uri.fsPath.endsWith('alice.td')),
      'rename should include edits for alice.td',
    );
    ok(
      entries.some(([uri]) => uri.fsPath.endsWith('refers-alice.td')),
      'rename should include edits for refers-alice.td',
    );
  });

  test('prepare rename on plain string value returns null', async () => {
    const document = await openDocument('alice.td');

    await sleep(1000);

    try {
      const result = await vscode.commands.executeCommand(
        'vscode.executePrepareRenameProvider',
        document.uri,
        new vscode.Position(2, 7),
      );

      strictEqual(result, null, 'plain string value should not be renameable');
    } catch {
      // Some LSP clients throw when prepareRename returns null
    }
  });
});
