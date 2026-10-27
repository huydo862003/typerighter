import {
  ok, strictEqual,
} from 'node:assert';
// eslint-disable-next-line import/no-namespace
import * as vscode from 'vscode';
import {
  activate, openDocument, closeAllEditors, sleep,
} from '../helper';

suite('Go to Definition', () => {
  suiteSetup(async () => {
    await activate();
  });

  teardown(async () => {
    await closeAllEditors();
  });

  test('_type: Person resolves to _types/Person.td', async () => {
    const document = await openDocument('alice.td');

    await sleep(1000);

    const definitions = await vscode.commands.executeCommand<vscode.Location[]>(
      'vscode.executeDefinitionProvider',
      document.uri,
      new vscode.Position(1, 7),
    );

    ok(definitions && 0 < definitions.length, 'should resolve _type to at least one definition');
    strictEqual(definitions.length, 1, `should resolve to exactly one definition, got ${definitions.length}`);
    ok(
      definitions[0].uri.fsPath.endsWith('_types/Person.td'),
      `should point to _types/Person.td, got: ${definitions[0].uri.fsPath}`,
    );
  });

  test('fref("alice.td") resolves to alice.td', async () => {
    const document = await openDocument('with-fref.td');

    await sleep(1000);

    const definitions = await vscode.commands.executeCommand<vscode.Location[]>(
      'vscode.executeDefinitionProvider',
      document.uri,
      new vscode.Position(4, 18),
    );

    ok(definitions && 0 < definitions.length, 'should resolve fref to a definition');
    strictEqual(definitions.length, 1, `should resolve to exactly one file, got ${definitions.length}`);
    ok(
      definitions[0].uri.fsPath.endsWith('alice.td') && !definitions[0].uri.fsPath.includes('with-fref'),
      `should point to alice.td (not with-fref.td), got: ${definitions[0].uri.fsPath}`,
    );
  });

  test('no definition on plain string value', async () => {
    const document = await openDocument('alice.td');

    await sleep(1000);

    const definitions = await vscode.commands.executeCommand<vscode.Location[]>(
      'vscode.executeDefinitionProvider',
      document.uri,
      new vscode.Position(2, 7),
    );

    strictEqual(definitions?.length ?? 0, 0, 'plain string value should not resolve to a definition');
  });
});
