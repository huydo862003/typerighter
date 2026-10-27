import {
  ok, strictEqual,
} from 'node:assert';
// eslint-disable-next-line import/no-namespace
import * as vscode from 'vscode';
import {
  activate, openDocument, closeAllEditors, sleep,
} from '../helper';

suite('Syntax Highlighting', () => {
  suiteSetup(async () => {
    await activate();
  });

  teardown(async () => {
    await closeAllEditors();
  });

  test('semantic tokens include type reference on _type line', async () => {
    const document = await openDocument('alice.td');

    await sleep(1000);

    const tokens = await vscode.commands.executeCommand<vscode.SemanticTokens>(
      'vscode.provideDocumentSemanticTokens',
      document.uri,
    );

    ok(tokens, 'should provide semantic tokens');
    ok(5 <= tokens.data.length, `should have at least one semantic token (5 ints), got ${tokens.data.length} ints`);
  });

  test('typedown language id is correctly assigned', async () => {
    const document = await openDocument('alice.td');

    strictEqual(document.languageId, 'typedown', '.td files should have languageId "typedown"');
  });

  test('non-td files do not get typedown language id', async () => {
    const yamlDocument = await vscode.workspace.openTextDocument(
      vscode.Uri.joinPath(vscode.workspace.workspaceFolders![0].uri, 'typedown.yaml'),
    );

    ok(yamlDocument.languageId !== 'typedown', `typedown.yaml should not have languageId "typedown", got: ${yamlDocument.languageId}`);
  });
});
