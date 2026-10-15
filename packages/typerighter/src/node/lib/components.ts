import fs from 'node:fs';
import path from 'node:path';
import {
  readdir,
} from 'node:fs/promises';
import {
  createRequire,
} from 'node:module';
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

  // When node_modules is symlinked (e.g. pnpm workspace or temp dirs), TS cannot resolve vue through the symlink. Resolve the real path and add it via paths
  const vuePaths = resolveVuePaths(projectRoot);

  return createCheckerByJson(projectRoot, {
    compilerOptions: {
      strict: false,
      jsx: 'preserve',
      jsxImportSource: 'vue',
      ...(vuePaths && {
        paths: vuePaths,
      }),
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
      let entries: fs.Dirent[];

      try {
        entries = await readdir(directoryPath, {
          withFileTypes: true,
        });
      } catch {
        continue;
      }

      for (const child of entries) {
        // Direct .vue file: ComponentName.vue
        if (child.isFile() && child.name.endsWith('.vue')) {
          components.push({
            name: child.name.slice(0, -4),
            absolutePath: path.join(directoryPath, child.name),
          });
          continue;
        }

        // Subdirectory with index.vue: ComponentName/index.vue
        if (child.isDirectory()) {
          const indexPath = path.join(directoryPath, child.name, 'index.vue');

          if (fs.existsSync(indexPath)) {
            components.push({
              name: child.name,
              absolutePath: indexPath,
            });
          }
        }
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

function resolveVuePaths (projectRoot: string): Record<string, string[]> | undefined {
  try {
    const require = createRequire(path.join(projectRoot, 'package.json'));
    const vueDirectory = path.dirname(require.resolve('vue/package.json'));

    return {
      vue: [vueDirectory],
      'vue/*': [path.join(vueDirectory, '*')],
    };
  } catch {
    return undefined;
  }
}
