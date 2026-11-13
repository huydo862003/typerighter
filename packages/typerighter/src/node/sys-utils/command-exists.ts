import {
  execSync,
} from 'node:child_process';

/**
 * Check whether a command is available on the system PATH
 * Uses `where` on Windows and `which` on Unix
 */
export function commandExists (command: string): boolean {
  if (!/^[\w.+-]+$/u.test(command)) return false;

  try {
    const lookup = process.platform === 'win32' ? 'where' : 'which';

    execSync(`${lookup} ${command}`, {
      stdio: 'ignore',
    });

    return true;
  } catch {
    return false;
  }
}
