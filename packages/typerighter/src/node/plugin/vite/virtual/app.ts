import type {
  ViteDevServer,
} from 'vite';
import {
  RESOLVED_VIRTUAL_APP_ID,
  PAGES_ID,
  SITE_DATA_ID,
  SEARCH_INDEX_ID,
  THEME_ID,
  PAGE_DATA_EVENT,
} from '../constants';
import {
  invalidateVirtualModule,
} from './utils';
import type {
  AppContext,
} from '@/node/context';
import {
  generateAppEntry, resolveUserComponents,
} from '@/node/lib/codegen';

export class VirtualApp {
  async load (context: AppContext): Promise<string> {
    const tdContext = await context.getTdContext();
    const config = await tdContext.getConfig();
    const userComponents = await resolveUserComponents(config.components, context.root);

    return generateAppEntry({
      rootDir: config.rootDir,
      basePath: config.basePath,
      origin: config.origin ?? undefined,
      lang: config.lang,
      siteTitle: config.siteTitle,
      siteDescription: config.siteDescription,
      repo: config.repo ?? undefined,
      author: config.author ?? undefined,
      license: config.license ?? undefined,
      nav: config.nav,
      userComponents,
    }, {
      theme: THEME_ID,
      pages: PAGES_ID,
      siteData: SITE_DATA_ID,
      searchIndex: SEARCH_INDEX_ID,
      pageDataEvent: PAGE_DATA_EVENT,
    });
  }

  invalidate (server: ViteDevServer): void {
    invalidateVirtualModule(server, RESOLVED_VIRTUAL_APP_ID);
  }
}
