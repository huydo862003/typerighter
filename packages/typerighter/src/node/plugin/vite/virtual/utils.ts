import type {
  ViteDevServer,
} from 'vite';
import type {
  TypedownContext,
} from '@/node/lib/typedown-context';

export interface VirtualModule {
  load(context: TypedownContext): string | Promise<string>;
  invalidate(server: ViteDevServer): void;
}

// Invalidate a virtual module and push an HMR update to the client
export function invalidateVirtualModule (
  server: ViteDevServer,
  resolvedId: string,
): void {
  const module_ = server.moduleGraph.getModuleById(resolvedId);

  if (!module_) return;
  server.moduleGraph.invalidateModule(module_);

  // PITFALL:
  //  - Vite 8.3 removed normalizeHmrUrl from import analysis, so `hot.accept` deps are stored as raw '\0...' module URLs instead of the browser-encoded '/@id/__x00__...' form used in Vite 8.2
  //  - Send module_.url directly so the acceptedPath matches what the browser registered in hotModulesMap
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
