import fs from 'node:fs';
import {
  fileURLToPath,
} from 'node:url';
import path from 'node:path';
import {
  normalizePath, type Plugin, type ViteDevServer,
} from 'vite';
import type {
  TdComponentEntry,
} from '@typerighter/rpc-client';
import {
  resolveUserComponents,
  createComponentChecker,
  type UserComponent,
} from '@/node/lib/components';
import {
  buildContractWithChecker,
} from '@/node/lib/prop-schema';
import type {
  AppContext,
} from '@/node/context';

const CONTRACT_PATH = '.typedown/components.json';

export function componentContractPlugin (
  getContext: () => AppContext | undefined,
): Plugin {
  return {
    name: 'vite-plugin-typedown-component-contract',

    async buildStart () {
      const context = getContext();

      if (!context) return;

      const tdContext = await context.getTdContext();
      const config = await tdContext.getConfig();
      const components = await resolveUserComponents(
        config.components,
        context.root,
      );

      writeContract(context.root, components);
    },

    configureServer (server) {
      const context = getContext();

      if (!context) return;

      context
        .getTdContext()
        .then(async (tdContext) => {
          const config = await tdContext.getConfig();
          const components = await resolveUserComponents(
            config.components,
            context.root,
          );

          writeContract(context.root, components);
          addWatcher(server, config.components, context.root);
        })
        .catch(() => { });
    },
  };
}

function addWatcher (
  server: ViteDevServer,
  entries: TdComponentEntry[],
  projectRoot: string,
): void {
  // Absolute paths for explicit component files
  const explicitPaths = new Set(
    entries
      .filter(
        (
          entry,
        ): entry is Extract<
          typeof entry,
          {
            kind: 'explicit';
          }
        > => entry.kind === 'explicit',
      )
      .map((entry) => normalizePath(path.resolve(projectRoot, entry.source))),
  );

  // Absolute paths for component directories
  const watchedDirs = new Set(
    entries
      .filter(
        (
          entry,
        ): entry is Extract<
          typeof entry,
          {
            kind: 'dir';
          }
        > => entry.kind === 'dir',
      )
      .map((entry) => normalizePath(path.resolve(projectRoot, entry.path))),
  );

  for (const directory of watchedDirs) {
    server.watcher.add(directory);
  }

  const onChange = (file: string) => {
    if (!isComponentFile(file, explicitPaths, watchedDirs)) return;

    server.config.logger.info(
      '[typedown] component changed, rebuilding contract...',
    );

    // Re-resolve to pick up any added/removed files in dir entries
    resolveUserComponents(entries, projectRoot)
      .then((components) => writeContract(projectRoot, components))
      .catch(() => { });
  };

  server.watcher.on('change', onChange);
  server.watcher.on('add', onChange);
  server.watcher.on('unlink', onChange);
}

function isComponentFile (
  file: string,
  explicitPaths: Set<string>,
  watchedDirs: Set<string>,
): boolean {
  if (!file.endsWith('.vue')) return false;

  const normalized = normalizePath(file);

  if (explicitPaths.has(normalized)) return true;

  for (const directory of watchedDirs) {
    if (normalized.startsWith(directory + '/')) return true;
  }

  return false;
}

// Read builtin-components.json bundled alongside this plugin in dist/
function loadBuiltinContract (): Record<string, unknown> {
  try {
    const contractPath = path.resolve(path.dirname(fileURLToPath(import.meta.url)), 'builtin-components.json');

    return JSON.parse(fs.readFileSync(contractPath, 'utf-8'));
  } catch {
    return {};
  }
}

const BUILTIN_CONTRACT = loadBuiltinContract();

function writeContract (projectRoot: string, components: UserComponent[]): void {
  const hasBuiltins = 0 < Object.keys(BUILTIN_CONTRACT).length;

  // Skip when neither builtins nor user components exist
  if (!hasBuiltins && components.length === 0) return;

  const userContract = components.length > 0
    ? buildContractWithChecker(components, projectRoot, createComponentChecker(projectRoot))
    : {};
  // User entries win over builtins on name collision
  const contract = {
    ...BUILTIN_CONTRACT,
    ...userContract,
  };
  const outputPath = path.join(projectRoot, CONTRACT_PATH);

  fs.mkdirSync(path.dirname(outputPath), {
    recursive: true,
  });
  fs.writeFileSync(outputPath, JSON.stringify(contract, null, 2));
}
