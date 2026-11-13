import {
  execSync,
} from 'node:child_process';
import {
  log, outro,
} from '@clack/prompts';

export const VSCODE_EXTENSION_ID = 'huydo862003.typedown';

// All known VS Code-compatible CLI binaries, in preference order
export const VSCODE_BINARIES = [
  'code',
  'codium',
  'code-insiders',
  'code-oss',
];

// Installs the Typedown extension via the detected VS Code CLI binary
export function setupVscode (bin: string): void {
  log.info(`Installing VS Code extension via ${bin}...`);

  try {
    execSync(`${bin} --install-extension ${VSCODE_EXTENSION_ID}`, {
      stdio: 'inherit',
    });

    log.success('VS Code extension installed');
    outro('Restart VS Code if it is already open.');
  } catch {
    log.error(`Failed to install via \`${bin} --install-extension\``);
    log.info(
      'Install manually: search for "typedown" in the VS Code extensions panel',
    );
    process.exit(1);
  }
}
