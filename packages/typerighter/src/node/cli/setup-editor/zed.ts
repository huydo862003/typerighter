import {
  execSync,
} from 'node:child_process';
import {
  log, outro,
} from '@clack/prompts';
import {
  commandExists,
} from '../../sys-utils/command-exists';

// Verifies `zed` is in PATH and that it is actually Zed, not an alias to something else
export function detectZedBinary (): string | undefined {
  if (!commandExists('zed')) return undefined;

  try {
    const output = execSync('zed --version', {
      encoding: 'utf-8',
      stdio: [
        'ignore',
        'pipe',
        'ignore',
      ],
    });

    return /\bZed\b/i.test(output) ? 'zed' : undefined;
  } catch {
    return undefined;
  }
}

// Prints instructions for installing the Typedown extension from the Zed marketplace
export function setupZed (): void {
  log.info('Zed extensions are installed from the Zed marketplace');
  log.info('Open Zed and run: "zed: extensions" from the command palette');
  log.info('Search for "typedown" and click Install');

  outro('Follow the instructions above to install in Zed.');
}
