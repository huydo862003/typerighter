// Self-contained builtin contract builder.
// No @/ aliases so vite.config.ts can import this directly.
import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import { createCheckerByJson } from 'vue-component-meta';
import { buildContractWithChecker } from '../lib/prop-schema';
import type { ContractEntry } from '../lib/prop-schema';
import { pascalToKebab } from '../../shared/utils/format';

const __dirname = fileURLToPath(new URL('.', import.meta.url));
const PACKAGE_ROOT = path.resolve(__dirname, '../../..');
const CUSTOM_DIR = path.resolve(PACKAGE_ROOT, 'src/client/theme-default/components/custom');

// Auto-discover builtin components: Td*.vue files in custom/ and its immediate subdirectories
function discoverBuiltinComponents (): Array<{ name: string; absolutePath: string }> {
  const dirs = [
    CUSTOM_DIR,
    ...fs.readdirSync(CUSTOM_DIR, { withFileTypes: true })
      .filter((entry) => entry.isDirectory())
      .map((entry) => path.join(CUSTOM_DIR, entry.name)),
  ];

  return dirs.flatMap((dir) =>
    fs.readdirSync(dir)
      .filter((file) => file.startsWith('Td') && file.endsWith('.vue'))
      .map((file) => ({
        name: pascalToKebab(file.replace(/^Td/, '').replace(/\.vue$/, '')),
        absolutePath: path.join(dir, file),
      })),
  );
}

export function buildBuiltinContract (): Record<string, ContractEntry> {
  // Use minimal checker options to get simple type strings (no T | undefined unions from strict mode)
  const checker = createCheckerByJson(PACKAGE_ROOT, {
    compilerOptions: {
      strict: false,
      jsx: 'preserve',
      jsxImportSource: 'vue',
    },
  });

  return buildContractWithChecker(discoverBuiltinComponents(), PACKAGE_ROOT, checker);
}
