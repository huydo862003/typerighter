import {
  ok,
} from 'node:assert';
// eslint-disable-next-line import/no-namespace
import * as vscode from 'vscode';
import {
  activate, openDocument, closeAllEditors, sleep,
} from '../helper';

suite('Find References', () => {
  suiteSetup(async () => {
    await activate();
  });

  teardown(async () => {
    await closeAllEditors();
  });

  test('schema type references found in content files', async () => {
    const document = await openDocument('_types/Person.td');

    await sleep(1000);

    const references = await vscode.commands.executeCommand<vscode.Location[]>(
      'vscode.executeReferenceProvider',
      document.uri,
      new vscode.Position(0, 0),
    );

    ok(references, 'should return references');
    ok(2 <= references.length, `should find at least 2 references to Person, got: ${references.length}`);

    const paths = references.map((ref) => ref.uri.fsPath);

    ok(paths.some((path) => path.endsWith('alice.td')), 'alice.td should reference Person');
    ok(paths.some((path) => path.endsWith('with-fref.td')), 'with-fref.td should reference Person');
  });

  test('content file references found via fref', async () => {
    const document = await openDocument('alice.td');

    await sleep(1000);

    const references = await vscode.commands.executeCommand<vscode.Location[]>(
      'vscode.executeReferenceProvider',
      document.uri,
      new vscode.Position(0, 0),
    );

    ok(references, 'should return references');
    ok(1 <= references.length, `should find at least 1 reference to alice.td, got: ${references.length}`);

    const paths = references.map((ref) => ref.uri.fsPath);

    ok(paths.some((path) => path.endsWith('with-fref.td')), 'with-fref.td should reference alice.td via fref');
  });
});
