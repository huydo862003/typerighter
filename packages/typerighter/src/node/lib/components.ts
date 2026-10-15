import fs from 'node:fs';
import path from 'node:path';
import {
  readdir,
} from 'node:fs/promises';
import {
  createChecker, createCheckerByJson,
} from 'vue-component-meta';
import type {
  TdComponentEntry,
} from '@typerighter/rpc-client';

export interface UserComponent {
  name: string;
  absolutePath: string;
}

// Use project tsconfig if available, otherwise synthesize a minimal one for vue-component-meta
export function createComponentChecker (projectRoot: string) {
  const tsconfig = path.join(projectRoot, 'tsconfig.json');

  if (fs.existsSync(tsconfig)) {
    return createChecker(tsconfig);
  }

  return createCheckerByJson(projectRoot, {
    compilerOptions: {
      strict: false,
      jsx: 'preserve',
      jsxImportSource: 'vue',
    },
  });
}

// Expand dir entries to individual .vue files, resolve explicit entries
export async function resolveUserComponents (
  entries: TdComponentEntry[],
  projectRoot: string,
): Promise<UserComponent[]> {
  const components: UserComponent[] = [];

  for (const entry of entries) {
    if (entry.kind === 'dir') {
      const directoryPath = path.resolve(projectRoot, entry.path);
      let files: string[];

      try {
        files = await readdir(directoryPath);
      } catch {
        continue;
      }

      for (const file of files) {
        if (!file.endsWith('.vue')) continue;

        const name = file.slice(0, -4);

        components.push({
          name,
          absolutePath: path.join(directoryPath, file),
        });
      }
    } else {
      components.push({
        name: entry.name,
        absolutePath: path.resolve(projectRoot, entry.source),
      });
    }
  }

  return components;
}
