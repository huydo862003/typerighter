import type {
  ViteDevServer,
} from 'vite';
import {
  RESOLVED_PAGES_ID,
} from '../constants';
import type {
  TypedownContext,
} from '@/node/lib/typedown-context';

export class VirtualPages {
  // Path within the host Vite project where typedown is mounted (empty when typedown is at the Vite root)
  constructor (private readonly mountPath: string = '') { }

  async load (context: TypedownContext): Promise<string> {
    const config = await context.getConfig();
    const rootDirectory = config.rootDir ?? '.';
    const segments = [
      this.mountPath,
      rootDirectory === '.' ? '' : rootDirectory,
    ].filter(Boolean);
    const prefix = 0 < segments.length ? '/' + segments.join('/') : '';
    const glob = prefix ? `${prefix}/**/*.{td,md}` : '/**/*.{td,md}';

    // Strip the vault prefix from glob keys so pages are keyed by route path
    return [
      `const raw = import.meta.glob('${glob}');`,
      'export const pages = Object.fromEntries(',
      `  Object.entries(raw).map(([k, v]) => [k.slice(${prefix.length}), v])`,
      ');',
    ].join('\n');
  }

  // Pages uses module invalidation because import.meta.glob is a compile-time transform that Vite must re-evaluate when files are added or deleted
  invalidate (server: ViteDevServer): void {
    const module_ = server.moduleGraph.getModuleById(RESOLVED_PAGES_ID);

    if (!module_) return;
    server.moduleGraph.invalidateModule(module_);

    for (const importer of module_.importers) {
      server.hot.send({
        type: 'update',
        updates: [
          {
            type: 'js-update',
            path: importer.url,
            acceptedPath: module_.url,
            timestamp: Date.now(),
          },
        ],
      });
    }
  }
}
