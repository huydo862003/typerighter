import {
  ok,
} from 'node:assert';
// eslint-disable-next-line import/no-namespace
import * as vscode from 'vscode';
import {
  activate, openDocument, closeAllEditors, sleep,
} from '../helper';

suite('Snippets', () => {
  suiteSetup(async () => {
    await activate();
  });

  teardown(async () => {
    await closeAllEditors();
  });

  test('code block snippet with ``` prefix is available in markdown body', async () => {
    const document = await openDocument('alice.td');

    await sleep(1000);

    // Line 5 is after the closing "---" - markdown body area
    const completions = await vscode.commands.executeCommand<vscode.CompletionList>(
      'vscode.executeCompletionItemProvider',
      document.uri,
      new vscode.Position(5, 0),
    );

    ok(completions, 'should return completions');
    const snippets = completions.items.filter((item) => item.kind === vscode.CompletionItemKind.Snippet);
    const labels = snippets.map((item) =>
      typeof item.label === 'string' ? item.label : item.label.label);

    ok(
      labels.some((label) => label.includes('```') || label.toLowerCase().includes('code')),
      `should include code block snippet, got snippets: ${labels.join(', ')}`,
    );

    // Verify the snippet body contains the expected structure
    const codeSnippet = snippets.find((item) => {
      const label = typeof item.label === 'string' ? item.label : item.label.label;

      return label.includes('```') || label.toLowerCase().includes('code');
    });

    ok(codeSnippet, 'code block snippet should exist');
    if (codeSnippet?.insertText && typeof codeSnippet.insertText !== 'string') {
      ok(codeSnippet.insertText.value.includes('```'), 'snippet body should contain ```');
    }
  });

  test('math block snippet with $$ prefix is available', async () => {
    const document = await openDocument('alice.td');

    await sleep(1000);

    const completions = await vscode.commands.executeCommand<vscode.CompletionList>(
      'vscode.executeCompletionItemProvider',
      document.uri,
      new vscode.Position(5, 0),
    );

    ok(completions, 'should return completions');
    const snippets = completions.items.filter((item) => item.kind === vscode.CompletionItemKind.Snippet);
    const labels = snippets.map((item) =>
      typeof item.label === 'string' ? item.label : item.label.label);

    ok(
      labels.some((label) => label.includes('$$') || label.toLowerCase().includes('math')),
      `should include math block snippet, got snippets: ${labels.join(', ')}`,
    );
  });
});
