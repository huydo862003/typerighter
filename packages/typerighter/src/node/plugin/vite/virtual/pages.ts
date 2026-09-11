import type {
  ViteDevServer,
} from 'vite';
import {
  RESOLVED_PAGES_ID,
  PAGE_DATA_PREFIX, RESOLVED_PAGE_DATA_PREFIX,
} from '../constants';
import {
  invalidateVirtualModule,
  type VirtualModule,
} from './utils';
import type {
  TypedownContext,
} from '@/node/lib/typedown-context';
import {
  buildPageData,
} from '@/node/lib/render/vue';

export class VirtualPages implements VirtualModule {
  async load (context: TypedownContext): Promise<string> {
    const config = await context.getConfig();
    const rootDirectory = config.rootDir ?? '.';
    const glob = rootDirectory === '.' ? '/**/*.{td,md}' : `/${rootDirectory}/**/*.{td,md}`;

    return `export const pages = import.meta.glob('${glob}');`;
  }

  resolvePageData (id: string): string | undefined {
    if (id.startsWith(PAGE_DATA_PREFIX)) {
      return '\0' + id;
    }
  }

  isPageDataModule (resolvedId: string): boolean {
    return resolvedId.startsWith(RESOLVED_PAGE_DATA_PREFIX);
  }

  async loadPageData (resolvedId: string, context: TypedownContext): Promise<string> {
    const filepath = decodeURIComponent(resolvedId.slice(RESOLVED_PAGE_DATA_PREFIX.length));
    const resource = await context.getFile(filepath);
    const pageData = await buildPageData(context, resource, filepath);

    return `export const pageData = ${JSON.stringify(pageData)}`;
  }

  invalidatePageData (server: ViteDevServer, filepath: string): void {
    invalidateVirtualModule(server, RESOLVED_PAGE_DATA_PREFIX + encodeURIComponent(filepath));
  }

  invalidate (server: ViteDevServer): void {
    invalidateVirtualModule(server, RESOLVED_PAGES_ID);
  }
}
