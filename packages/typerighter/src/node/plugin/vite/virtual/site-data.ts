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
import {
  type ContentTree,
  buildContentTree,
  buildDirectoryListingMap,
  type ContentSummary,
} from '@/shared';

const RESOLVED_ID = '\0@typedown/site-data';

interface SiteData {
  contentTree: ContentTree;
  schemas: Record<string, unknown>;
  directoryListings: ReturnType<typeof buildDirectoryListingMap>;
}

const EMPTY: SiteData = {
  contentTree: {
    entries: [],
  },
  schemas: {},
  directoryListings: {},
};

export class VirtualSiteData implements VirtualModule {
  private data: SiteData | undefined;

  load (_context: TypedownContext): string {
    return `export default ${JSON.stringify(this.data ?? EMPTY)}`;
  }

  clear (): void {
    this.data = undefined;
  }

  // Fetch in background, push to client via HMR when ready
  fetch (context: TypedownContext, server?: ViteDevServer): void {
    fetchFromRpc(context)
      .then((result) => {
        this.data = result;
        if (server) this.invalidate(server);
      })
      .catch((error) => {
        console.error('[typedown] Failed to fetch site data:', error instanceof Error ? error.message : error);
      });
  }

  invalidate (server: ViteDevServer): void {
    invalidateVirtualModule(server, RESOLVED_ID);
  }
}

async function fetchFromRpc (context: TypedownContext): Promise<SiteData> {
  const [
    config,
    sidebarItems,
    schemaNames,
  ] = await Promise.all([
    context.getConfig(),
    context.listSidebar(),
    context.listSchemas(),
  ]);

  const schemaEntries = await Promise.all(
    schemaNames.map(async (name) => {
      const info = await context.getSchema(name);

      return [
        name,
        info.properties,
      ] as const;
    }),
  );
  const schemas = Object.fromEntries(schemaEntries);

  // Build content tree from lightweight sidebar items
  const contentItems: ContentSummary[] = sidebarItems.map((item) => ({
    ...item,
    header: {},
  }));
  const contentTree = buildContentTree(contentItems);
  const directoryListings = buildDirectoryListingMap(contentTree.entries, config.siteTitle);

  return {
    contentTree,
    schemas,
    directoryListings,
  };
}
