import type {
  ViteDevServer,
} from 'vite';
import type {
  TypedownContext,
} from '@/node/lib/typedown-context';

export interface VirtualModule {
  load (context: TypedownContext): string | Promise<string>;
  invalidate (server: ViteDevServer): void;
}

// PITFALL: \0 virtual modules are /@id/__x00__ in browser, HMR paths must match
// Ref: https://vite.dev/guide/api-plugin.html#virtual-modules-convention
function encodeVirtualUrl (url: string): string {
  return url.startsWith('\0')
    ? '/@id/__x00__' + url.slice(1)
    : url;
}

// Invalidate a virtual module and push an HMR update to the client
export function invalidateVirtualModule (server: ViteDevServer, resolvedId: string): void {
  const module_ = server.moduleGraph.getModuleById(resolvedId);

  if (!module_) return;
  server.moduleGraph.invalidateModule(module_);

  const depUrl = encodeVirtualUrl(module_.url);

  for (const importer of module_.importers) {
    server.hot.send({
      type: 'update',
      updates: [
        {
          type: 'js-update',
          path: encodeVirtualUrl(importer.url),
          acceptedPath: depUrl,
          timestamp: Date.now(),
        },
      ],
    });
  }
}
