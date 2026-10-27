import {
  ok, strictEqual,
} from 'node:assert';
// eslint-disable-next-line import/no-namespace
import * as vscode from 'vscode';
import {
  activate, openDocument, closeAllEditors, sleep,
} from '../helper';

function getLabels (completions: vscode.CompletionList): string[] {
  return completions.items.map((item) =>
    typeof item.label === 'string' ? item.label : item.label.label);
}

suite('Completion', () => {
  suiteSetup(async () => {
    await activate();
  });

  teardown(async () => {
    await closeAllEditors();
  });

  test('suggests remaining schema fields (excludes already-declared ones)', async () => {
    const document = await openDocument('alice.td');

    await sleep(1000);

    // alice.td has _type, name, age declared. Person schema has name, age, friend
    // Completions at line 4 (after age:30, before ---) should suggest "friend" but NOT "name" or "age"
    const completions = await vscode.commands.executeCommand<vscode.CompletionList>(
      'vscode.executeCompletionItemProvider',
      document.uri,
      new vscode.Position(4, 0),
    );

    ok(completions, 'should return completions');
    const labels = getLabels(completions);

    ok(labels.includes('friend'), `should suggest "friend" (undeclared field), got: ${labels.join(', ')}`);
    ok(!labels.includes('name'), '"name" is already declared and should NOT be suggested');
    ok(!labels.includes('age'), '"age" is already declared and should NOT be suggested');
  });

  test('fref completion suggests vault-relative paths', async () => {
    const document = await openDocument('with-fref.td');

    await sleep(1000);

    // Position inside fref("alice.td") - character 14 is inside the string
    const completions = await vscode.commands.executeCommand<vscode.CompletionList>(
      'vscode.executeCompletionItemProvider',
      document.uri,
      new vscode.Position(4, 14),
    );

    ok(completions, 'should return completions');
    const labels = getLabels(completions);

    ok(
      labels.some((label) => label.includes('alice')),
      `should suggest alice.td as fref target, got: ${labels.join(', ')}`,
    );
  });

  test('no field completions without _type declaration', async () => {
    // with-fref.td has _type: Person - fields ARE suggested
    // An empty file without _type should NOT suggest schema fields
    // (we test this implicitly: completions at line 0 of alice.td before _type)
    const document = await openDocument('alice.td');

    await sleep(1000);

    // Line 0 is "---" - no field completions expected here
    const completions = await vscode.commands.executeCommand<vscode.CompletionList>(
      'vscode.executeCompletionItemProvider',
      document.uri,
      new vscode.Position(0, 0),
    );

    if (completions) {
      const schemaFields = completions.items.filter((item) =>
        item.kind === vscode.CompletionItemKind.Field
        && [
          'name',
          'age',
          'friend',
        ].includes(typeof item.label === 'string' ? item.label : item.label.label));

      strictEqual(schemaFields.length, 0, `should not suggest schema fields on "---" line, got: ${schemaFields.map((item) => typeof item.label === 'string' ? item.label : item.label.label).join(', ')}`);
    }
  });
});
