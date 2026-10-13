import path from 'node:path';
import {
  readdir,
} from 'node:fs/promises';
import type {
  TdComponentEntry,
} from '@typerighter/rpc-client';

export interface UserComponent {
  name: string;
  absolutePath: string;
}

// Resolve TdComponentEntry list to concrete { name, absolutePath } pairs
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
