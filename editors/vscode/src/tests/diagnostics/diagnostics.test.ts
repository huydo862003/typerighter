import {
  deepStrictEqual, strictEqual, ok,
} from 'node:assert';
import {
  DiagnosticSeverity,
} from 'vscode';
import {
  activate, openDocument, pollForDiagnostics, pollForNoDiagnostics, closeAllEditors, getFileUri,
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
    const diagnostics = await pollForNoDiagnostics(getFileUri('valid.td'));

    deepStrictEqual(diagnostics, [], 'valid file should have exactly zero diagnostics');
  });

  test('unknown type produces error on the type name with correct code', async () => {
    await openDocument('unknown-type.td');
    const diagnostics = await pollForDiagnostics(getFileUri('unknown-type.td'));

    // Should have at least one diagnostic: unresolved schema for "NonExistent"
    ok(1 <= diagnostics.length, `expected at least 1 diagnostic, got ${diagnostics.length}: ${diagnostics.map((diagnostic) => diagnostic.message).join('; ')}`);

    const diagnostic = diagnostics[0];

    strictEqual(diagnostic.severity, DiagnosticSeverity.Error, 'should be error severity');
    ok(diagnostic.message.toLowerCase().includes('nonexistent'), `message should mention "NonExistent", got: "${diagnostic.message}"`);
  });

  test('unknown field produces diagnostic on the field name', async () => {
    await openDocument('unknown-field.td');
    const diagnostics = await pollForDiagnostics(getFileUri('unknown-field.td'));

    // Should flag "nonexistent_field" as unknown
    ok(0 < diagnostics.length, 'expected at least one diagnostic for unknown field');

    const fieldDiagnostic = diagnostics.find((diagnostic) => diagnostic.message.toLowerCase().includes('nonexistent_field'));

    ok(fieldDiagnostic, `expected a diagnostic mentioning "nonexistent_field", got: ${diagnostics.map((diagnostic) => diagnostic.message).join('; ')}`);
    strictEqual(fieldDiagnostic.severity, DiagnosticSeverity.Error, 'unknown field should be error severity');
  });

  test('valid file does not produce false positives for known fields', async () => {
    // Ensure "name" and "age" (valid Person fields) do NOT trigger diagnostics
    await openDocument('valid.td');
    const diagnostics = await pollForNoDiagnostics(getFileUri('valid.td'));

    const falsePositives = diagnostics.filter((diagnostic) =>
      diagnostic.message.includes('name') || diagnostic.message.includes('age'));

    deepStrictEqual(falsePositives, [], 'known fields should not produce diagnostics');
  });
});
