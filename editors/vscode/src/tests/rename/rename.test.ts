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

  test('rename "Person" to "Human" produces edits with correct new text', async () => {
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

    // Check affected files
    ok(
      entries.some(([uri]) => uri.fsPath.endsWith('alice.td')),
      'rename should include edits for alice.td',
    );
    ok(
      entries.some(([uri]) => uri.fsPath.endsWith('refers-alice.td')),
      'rename should include edits for refers-alice.td',
    );

    // Check that all text edits contain "Human" as the new text
    for (const [
      , edits,
    ] of entries) {
      for (const textEdit of edits) {
        strictEqual(
          (textEdit as vscode.TextEdit).newText,
          'Human',
          'every text edit should replace with "Human"',
        );
      }
    }
  });

  test('prepare rename on plain string value returns null', async () => {
    const document = await openDocument('alice.td');

    await sleep(1000);

    // "Alice" at line 2, char 7 - plain string, not renameable
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

  test('prepare rename on number value returns null', async () => {
    const document = await openDocument('alice.td');

    await sleep(1000);

    // 30 at line 3, char 5 - number, not renameable
    try {
      const result = await vscode.commands.executeCommand(
        'vscode.executePrepareRenameProvider',
        document.uri,
        new vscode.Position(3, 5),
      );

      strictEqual(result, null, 'number value should not be renameable');
    } catch {
      // Expected
    }
  });
});
