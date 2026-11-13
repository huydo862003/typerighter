import {
  execSync,
} from 'node:child_process';
import {
  log,
} from '@clack/prompts';
import {
  VSCODE_EXTENSION_ID,
  detectVscodeBinary,
} from '../setup-editor/vscode';

// Finds a real VS Code binary and re-runs --install-extension, which also updates
export function updateVscode (): void {
  const bin = detectVscodeBinary();

  if (!bin) return;

  log.info(`Updating VS Code extension via ${bin}...`);

  try {
    execSync(`${bin} --install-extension ${VSCODE_EXTENSION_ID}`, {
      stdio: 'inherit',
    });

    log.success('VS Code extension updated');
  } catch {
    log.warn(
      `Failed to update VS Code extension via \`${bin}\` - update it from the extensions panel`,
    );
  }
}
