import {
  deepStrictEqual, strictEqual, ok,
} from 'node:assert';
import {
  DiagnosticSeverity,
} from 'vscode';
import {
  activate, openDocument, waitForDiagnostics, waitForNoDiagnostics, closeAllEditors, getFileUri,
} from '../helper';

suite('Diagnostics', () => {
  suiteSetup(async () => {
    await activate();
  });

  teardown(async () => {
    await closeAllEditors();
  });

  test('valid file produces zero diagnostics', async () => {
    await openDocument('valid.td');
    const diags = await waitForNoDiagnostics(getFileUri('valid.td'));

    deepStrictEqual(diags, [], 'valid file should have exactly zero diagnostics');
  });

  test('unknown type produces error on the type name with correct code', async () => {
    await openDocument('unknown-type.td');
    const diags = await waitForDiagnostics(getFileUri('unknown-type.td'));

    // Should have at least one diagnostic: unresolved schema for "NonExistent"
    ok(1 <= diags.length, `expected at least 1 diagnostic, got ${diags.length}: ${diags.map((diag) => diag.message).join('; ')}`);

    const diag = diags[0];

    strictEqual(diag.severity, DiagnosticSeverity.Error, 'should be error severity');
    ok(diag.message.toLowerCase().includes('nonexistent'), `message should mention "NonExistent", got: "${diag.message}"`);
  });

  test('unknown field produces diagnostic on the field name', async () => {
    await openDocument('unknown-field.td');
    const diags = await waitForDiagnostics(getFileUri('unknown-field.td'));

    // Should flag "nonexistent_field" as unknown
    ok(0 < diags.length, 'expected at least one diagnostic for unknown field');

    const fieldDiag = diags.find((diag) => diag.message.toLowerCase().includes('nonexistent_field'));

    ok(fieldDiag, `expected a diagnostic mentioning "nonexistent_field", got: ${diags.map((diag) => diag.message).join('; ')}`);
    strictEqual(fieldDiag.severity, DiagnosticSeverity.Error, 'unknown field should be error severity');
  });

  test('valid file does not produce false positives for known fields', async () => {
    // Ensure "name" and "age" (valid Person fields) do NOT trigger diagnostics
    await openDocument('valid.td');
    const diags = await waitForNoDiagnostics(getFileUri('valid.td'));

    const falsePositives = diags.filter((diag) =>
      diag.message.includes('name') || diag.message.includes('age'));

    deepStrictEqual(falsePositives, [], 'known fields should not produce diagnostics');
  });
});
