import {
  existsSync,
} from 'node:fs';
import path from 'node:path';

/**
 * Detect which package manager the user is using
 * First checks the npm_config_user_agent env var (set by npx, pnpm dlx, etc.), then falls back to checking for lockfiles in the project root
 */
export function detectPackageManager (root: string): string {
  const userAgent = process.env.npm_config_user_agent ?? '';
  const agentName = userAgent.split('/')[0];

  if (agentName === 'pnpm') return 'pnpm';
  if (agentName === 'yarn') return 'yarn';
  if (agentName === 'bun') return 'bun';
  if (agentName === 'deno') return 'deno';

  if (existsSync(path.join(root, 'pnpm-lock.yaml'))) return 'pnpm';
  if (existsSync(path.join(root, 'yarn.lock'))) return 'yarn';
  if (existsSync(path.join(root, 'bun.lock')) || existsSync(path.join(root, 'bun.lockb'))) return 'bun';
  if (existsSync(path.join(root, 'deno.lock'))) return 'deno';

  return 'npm';
}
