import {
  execSync,
} from 'node:child_process';
import {
  log,
} from '@clack/prompts';
import {
  detectPackageManager,
} from '../../sys-utils/detect-package-manager';

const PACKAGE_NAME = 'typerighter';

// Maps each package manager to its command for upgrading a dev dependency to @latest
const UPGRADE_COMMANDS: Record<string, string> = {
  bun: `bun add -d ${PACKAGE_NAME}@latest`,
  deno: `deno add --dev npm:${PACKAGE_NAME}@latest`,
  npm: `npm install --save-dev ${PACKAGE_NAME}@latest`,
  pnpm: `pnpm add -D ${PACKAGE_NAME}@latest`,
  yarn: `yarn add --dev ${PACKAGE_NAME}@latest`,
};

// Detects the package manager used in the vault and upgrades typerighter to @latest
export function updateVault (root: string): void {
  const packageManager = detectPackageManager(root);
  const command = UPGRADE_COMMANDS[packageManager] ?? UPGRADE_COMMANDS.npm;

  log.info(`Updating ${PACKAGE_NAME} in vault (${packageManager})...`);

  try {
    execSync(command, {
      cwd: root,
      stdio: 'inherit',
    });

    log.success(`${PACKAGE_NAME} updated in vault`);
  } catch {
    log.error(`Failed to update ${PACKAGE_NAME}: ${command}`);
    log.info(`Run manually: ${command}`);
  }
}
