import {
  execSync,
} from 'node:child_process';
import {
  log, outro,
} from '@clack/prompts';
import {
  commandExists,
} from '../../sys-utils/command-exists';

export const VSCODE_EXTENSION_ID = 'huydo862003.typedown';

// All known VS Code-compatible CLI binaries, in preference order
export const VSCODE_BINARIES = [
  'code',
  'codium',
  'code-insiders',
  'code-oss',
];

// Finds the first VS Code binary in PATH that is actually VS Code, not an alias to something else
// Verifies by running --version and checking that the first line is a semver (VS Code format)
export function detectVscodeBinary (): string | undefined {
  return VSCODE_BINARIES.find((bin) => {
    if (!commandExists(bin)) return false;

    try {
      const output = execSync(`${bin} --version`, {
        encoding: 'utf-8',
        stdio: [
          'ignore',
          'pipe',
          'ignore',
        ],
      });

      return /^\d+\.\d+\.\d+/.test(output.trim().split('\n')[0]);
    } catch {
      return false;
    }
  });
}

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
