import type {
  ViteDevServer,
} from 'vite';
import {
  invalidateVirtualModule,
  type VirtualModule,
} from './utils';
import type {
  TypedownContext,
} from '@/node/lib/typedown-context';

const RESOLVED_ID = '\0@typedown/pages';

export class VirtualPages implements VirtualModule {
  async load (context: TypedownContext): Promise<string> {
    const config = await context.getConfig();
    const rootDirectory = config.rootDir ?? '.';
    const glob = rootDirectory === '.' ? '/**/*.{td,md}' : `/${rootDirectory}/**/*.{td,md}`;

    return `export const pages = import.meta.glob('${glob}');`;
  }

  invalidate (server: ViteDevServer): void {
    invalidateVirtualModule(server, RESOLVED_ID);
  }
}
