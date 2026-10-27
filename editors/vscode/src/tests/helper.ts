// eslint-disable-next-line import/no-namespace
import * as vscode from 'vscode';

const EXTENSION_ID = 'huydo862003.typedown-vscode';

/**
 * Activate the extension and wait for the LSP client to be ready.
 * Call once in suiteSetup
 */
export async function activate (): Promise<void> {
  const extension = vscode.extensions.getExtension(EXTENSION_ID);

  if (!extension) {
    throw new Error(`Extension ${EXTENSION_ID} not found`);
  }
  if (!extension.isActive) {
    await extension.activate();
  }
  // Give the LSP client time to start (binary spawn + TCP connect + initialize handshake)
  await sleep(3000);
}

/**
 * Close all open editors to prevent state leakage between tests
 */
export async function closeAllEditors (): Promise<void> {
  await vscode.commands.executeCommand('workbench.action.closeAllEditors');
}

/**
 * Get the URI for a file in the current workspace
 */
export function fileUri (relativePath: string): vscode.Uri {
  const workspaceFolder = vscode.workspace.workspaceFolders?.[0];

  if (!workspaceFolder) {
    throw new Error('No workspace folder open');
  }

  return vscode.Uri.joinPath(workspaceFolder.uri, relativePath);
}

/**
 * Open a document from the current workspace folder
 */
export async function openDocument (relativePath: string): Promise<vscode.TextDocument> {
  const workspaceFolder = vscode.workspace.workspaceFolders?.[0];

  if (!workspaceFolder) {
    throw new Error('No workspace folder open');
  }
  const uri = vscode.Uri.joinPath(workspaceFolder.uri, relativePath);
  const document = await vscode.workspace.openTextDocument(uri);

  await vscode.window.showTextDocument(document);

  return document;
}

export function sleep (ms: number): Promise<void> {
  return new Promise((resolve) => setTimeout(resolve, ms));
}

/**
 * Wait until diagnostics are published for a URI, or timeout.
 * Returns the diagnostics array
 */
export async function waitForDiagnostics (
  uri: vscode.Uri,
  timeout = 10000,
): Promise<vscode.Diagnostic[]> {
  const existing = vscode.languages.getDiagnostics(uri);

  if (0 < existing.length) return existing;

  return new Promise((resolve) => {
    const timer = setTimeout(() => {
      listener.dispose();
      resolve(vscode.languages.getDiagnostics(uri));
    }, timeout);

    const listener = vscode.languages.onDidChangeDiagnostics((event) => {
      if (event.uris.some((eventUri) => eventUri.toString() === uri.toString())) {
        const diags = vscode.languages.getDiagnostics(uri);

        if (0 < diags.length) {
          clearTimeout(timer);
          listener.dispose();
          resolve(diags);
        }
      }
    });
  });
}

/**
 * Wait until diagnostics for a URI become empty (no errors), or timeout
 */
export async function waitForNoDiagnostics (
  uri: vscode.Uri,
  timeout = 10000,
): Promise<vscode.Diagnostic[]> {
  const existing = vscode.languages.getDiagnostics(uri);

  if (existing.length === 0) return existing;

  return new Promise((resolve) => {
    const timer = setTimeout(() => {
      listener.dispose();
      resolve(vscode.languages.getDiagnostics(uri));
    }, timeout);

    const listener = vscode.languages.onDidChangeDiagnostics((event) => {
      if (event.uris.some((eventUri) => eventUri.toString() === uri.toString())) {
        const diags = vscode.languages.getDiagnostics(uri);

        if (diags.length === 0) {
          clearTimeout(timer);
          listener.dispose();
          resolve(diags);
        }
      }
    });
  });
}
