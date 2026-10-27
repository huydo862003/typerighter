import {
  ok, strictEqual,
} from 'node:assert';
// eslint-disable-next-line import/no-namespace
import * as vscode from 'vscode';
import {
  activate, openDocument, closeAllEditors, sleep,
} from '../helper';

suite('Hover', () => {
  suiteSetup(async () => {
    await activate();
  });

  teardown(async () => {
    await closeAllEditors();
  });

  test('hover on _type value shows schema type information', async () => {
    const document = await openDocument('alice.td');

    await sleep(1000);

    const hovers = await vscode.commands.executeCommand<vscode.Hover[]>(
      'vscode.executeHoverProvider',
      document.uri,
      new vscode.Position(1, 7),
    );

    ok(hovers && 0 < hovers.length, 'should provide hover for type reference');

    const content = hovers[0].contents
      .map((item) => (typeof item === 'string' ? item : (item as vscode.MarkdownString).value))
      .join('\n');

    ok(content.includes('Person'), `hover should mention "Person", got: ${content}`);
  });

  test('no hover on frontmatter delimiter', async () => {
    const document = await openDocument('alice.td');

    await sleep(1000);

    const hovers = await vscode.commands.executeCommand<vscode.Hover[]>(
      'vscode.executeHoverProvider',
      document.uri,
      new vscode.Position(0, 1),
    );

    strictEqual(hovers?.length ?? 0, 0, 'should not provide hover on "---"');
  });
});
