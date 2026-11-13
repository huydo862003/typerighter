import fs from 'node:fs';
import path from 'node:path';
import {
  spawn,
} from 'node:child_process';
import pc from 'picocolors';
import {
  detectPackageManager,
} from '../../sys-utils/detect-package-manager';

const CACHE_DIRECTORY = path.join(
  process.env.HOME ?? process.env.USERPROFILE ?? '.',
  '.typedown',
  '.local',
);
const CACHE_FILE = path.join(CACHE_DIRECTORY, 'update-check.json');
const TTL_MS = 24 * 60 * 60 * 1000; // 24 hours
const REGISTRY_URL = 'https://registry.npmjs.org/typerighter/latest';

/**
 * Read the cached update check result
 * - If the cache is fresh and a newer version exists, print a notice
 * - If the cache is stale or missing, spawn a background process to refresh it
 */
export function checkForUpdates (): void {
  try {
    const cache = readUpdateCheckCache();

    if (cache && Date.now() - cache.checkedAt < TTL_MS) {
      // Cache is fresh - schedule the notice for after the command finishes
      if (isNewer(cache.latest, __VERSION__)) {
        scheduleNotice(cache.latest);
      }
    } else {
      // Cache is stale or missing - refresh in background
      spawnBackgroundCheck();
    }
  } catch {
    // Never fail the CLI over an update check
  }
}

interface CacheData {
  latest: string;
  checkedAt: number;
}

/**
 * Simple semver comparison: returns true if `left` is newer than `right`
 * Only handles major.minor.patch (no pre-release tags)
 */
function isNewer (left: string, right: string): boolean {
  const partsLeft = left.split('.').map(Number);
  const partsRight = right.split('.').map(Number);

  for (let index = 0; index < 3; index++) {
    const versionLeft = partsLeft[index] ?? 0;
    const versionRight = partsRight[index] ?? 0;

    if (versionLeft < versionRight) return false;

    if (versionRight < versionLeft) return true;
  }

  return false;
}

function readUpdateCheckCache (): CacheData | undefined {
  try {
    const raw = fs.readFileSync(CACHE_FILE, 'utf-8');
    const data = JSON.parse(raw) as CacheData;

    if (typeof data.latest === 'string' && typeof data.checkedAt === 'number') {
      return data;
    }
  } catch {
    // No cache or invalid
  }

  return undefined;
}

function scheduleNotice (latest: string): void {
  process.on('exit', () => {
    const border = pc.cyan('|');
    const current = pc.dim(__VERSION__);
    const next = pc.green(latest);
    const packageManager = detectPackageManager('.');
    const installCommand =
      packageManager === 'npm'
        ? 'npm install -g typerighter@latest'
        : `${packageManager} add -g typerighter@latest`;

    console.error('');
    console.error(`  ${border}`);
    console.error(`  ${border}  Update available: ${current} -> ${next}`);
    console.error(`  ${border}  Run ${pc.cyan(installCommand)} to update`);
    console.error(`  ${border}`);
    console.error('');
  });
}

function spawnBackgroundCheck (): void {
  const script = `
import fs from "node:fs";

const response = await fetch(${JSON.stringify(REGISTRY_URL)});
if (!response.ok) process.exit(0);
const data = await response.json();
if (typeof data.version !== "string") process.exit(0);

fs.mkdirSync(${JSON.stringify(CACHE_DIRECTORY)}, { recursive: true });
fs.writeFileSync(${JSON.stringify(CACHE_FILE)}, JSON.stringify({ latest: data.version, checkedAt: Date.now() }));
`;

  const child = spawn(
    process.execPath,
    [
      '--input-type=module',
      '--eval',
      script,
    ],
    {
      detached: true,
      stdio: 'ignore',
    },
  );

  child.unref();
}
