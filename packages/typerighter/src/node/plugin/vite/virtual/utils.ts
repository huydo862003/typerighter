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

// Invalidate a virtual module and push an HMR update to the client
export function invalidateVirtualModule (server: ViteDevServer, resolvedId: string): void {
  const module_ = server.moduleGraph.getModuleById(resolvedId);

  if (!module_) return;
  server.moduleGraph.invalidateModule(module_);
  server.hot.send({
    type: 'update',
    updates: [
      {
        type: 'js-update',
        path: module_.url,
        acceptedPath: module_.url,
        timestamp: Date.now(),
      },
    ],
  });
}
