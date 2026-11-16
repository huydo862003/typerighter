import {
  globSync, readFileSync,
} from 'node:fs';
import {
  resolve,
} from 'node:path';
import type {
  ViteDevServer,
} from 'vite';
import type {
  TdSidebarItem,
} from '@typerighter/rpc-client';
import {
  invalidateVirtualModule,
} from './utils';
import type {
  TypedownContext,
} from '@/node/lib/typedown-context';
import {
  type ContentTree,
  buildContentTree,
  buildDirectoryListingMap,
  extractFrontmatterBuiltinFields,
  getTdResourceTitle,
  type ContentSummary,
} from '@/shared';

const RESOLVED_ID = '\0@typedown/site-data';

interface SiteData {
  contentItems: ContentSummary[];
  contentTree: ContentTree;
  schemas: Record<string, unknown>;
  directoryListings: ReturnType<typeof buildDirectoryListingMap>;
}

const EMPTY = {
  ready: false,
  contentTree: {
    entries: [],
  },
  schemas: {},
  directoryListings: {},
};

export class VirtualSiteData {
  private data: SiteData | undefined;

  // Seed the sidebar from disk so it renders instantly
  scan (rootDirectory: string): void {
    const files = globSync('**/*.td', {
      cwd: rootDirectory,
      exclude: (name: string) => name.startsWith('_'),
    });

    // First pass: collect directories whose index.td is hidden from sidebar
    const hiddenDirsInSidebar = new Set<string>();

    for (const filepath of files) {
      const stem = filepath.split('/').pop()
        ?.replace(/\.td$/, '');

      if (stem !== 'index') continue;

      let raw: string | undefined;

      try {
        raw = readFileSync(resolve(rootDirectory, filepath), 'utf-8');
      } catch {
        // skip unreadable files
      }

      if (raw && extractFrontmatterBuiltinFields(raw)?.showInSidebar === false) {
        const directory = filepath.split('/').slice(0, -1)
          .join('/');

        hiddenDirsInSidebar.add(directory);
      }
    }

    const isUnderHiddenDirectoryInSidebar = (filepath: string): boolean => {
      if (hiddenDirsInSidebar.size === 0) return false;
      let current = filepath.split('/').slice(0, -1)
        .join('/');

      while (current) {
        if (hiddenDirsInSidebar.has(current)) return true;
        const parent = current.split('/').slice(0, -1)
          .join('/');

        if (parent === current) break;
        current = parent;
      }

      return false;
    };

    const contentItems: ContentSummary[] = [];

    for (const filepath of files) {
      if (isUnderHiddenDirectoryInSidebar(filepath)) continue;

      let raw: string | undefined;

      try {
        raw = readFileSync(resolve(rootDirectory, filepath), 'utf-8');
      } catch {
        // skip unreadable files
      }

      const fields = raw ? extractFrontmatterBuiltinFields(raw) : undefined;

      if (fields?.showInSidebar === false) continue;

      contentItems.push({
        filepath,
        label: fields?.label ?? getTdResourceTitle(filepath),
        header: {},
      });
    }

    const contentTree = buildContentTree(contentItems);

    this.data = {
      contentItems,
      contentTree,
      schemas: {},
      directoryListings: buildDirectoryListingMap(contentTree.entries, ''),
    };
  }

  load (): string {
    if (!this.data) {
      return `export default ${JSON.stringify(EMPTY)}`;
    }

    const payload = {
      ready: true,
      contentTree: this.data.contentTree,
      schemas: this.data.schemas,
      directoryListings: this.data.directoryListings,
    };

    return `export default ${JSON.stringify(payload)}`;
  }

  clear (): void {
    this.data = undefined;
  }

  // Fetch full data from RPC (used on initial load and config changes, not during HMR)
  fetch (context: TypedownContext, server: ViteDevServer): void {
    fetchFromRpc(context)
      .then((result) => {
        this.data = result;
        this.invalidate(server);
      })
      .catch((error) => {
        console.error('[typedown] Failed to fetch site data:', error instanceof Error ? error.message : error);
      });
  }

  // Fetch and block until data is ready (for production builds)
  async fetchSync (context: TypedownContext): Promise<void> {
    this.data = await fetchFromRpc(context);
  }

  // Patch a single sidebar entry, rebuild tree
  upsertSidebarEntry (sidebarItem: TdSidebarItem): void {
    if (!this.data) return;
    const index = this.data.contentItems.findIndex(
      (item) => item.filepath === sidebarItem.filepath,
    );

    const summary = convertSidebarItemToSummary(sidebarItem);

    if (0 <= index) {
      this.data.contentItems[index] = summary;
    } else {
      this.data.contentItems.push(summary);
    }

    this.rebuildTree();
  }

  // Remove a sidebar entry by filepath
  removeSidebarEntry (filepath: string): void {
    if (!this.data) return;
    this.data.contentItems = this.data.contentItems.filter(
      (item) => item.filepath !== filepath,
    );
    this.rebuildTree();
  }

  // Update a schema's properties
  patchSchema (name: string, properties: unknown): void {
    if (!this.data) return;
    this.data.schemas[name] = properties;
  }

  // Remove a schema
  removeSchema (name: string): void {
    if (!this.data) return;
    const {
      [name]: _, ...remaining
    } = this.data.schemas;

    this.data.schemas = remaining;
  }

  private rebuildTree (): void {
    if (!this.data) return;
    const contentTree = buildContentTree(this.data.contentItems);

    this.data.contentTree = contentTree;
    this.data.directoryListings = buildDirectoryListingMap(contentTree.entries, '');
  }

  invalidate (server: ViteDevServer): void {
    invalidateVirtualModule(server, RESOLVED_ID);
  }
}

function convertSidebarItemToSummary (item: TdSidebarItem): ContentSummary {
  return {
    filepath: item.filepath,
    schema: item.schema,
    schemaLabel: item.schemaLabel,
    label: item.label,
    icon: item.icon,
    header: {},
    excerpt: item.excerpt,
    fileMetadata: item.fileMetadata,
  };
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
    contentItems,
    contentTree,
    schemas,
    directoryListings,
  };
}
