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
  SITE_DATA_EVENT,
} from '../constants';
import type {
  TypedownContext,
} from '@/node/lib/typedown-context';
import {
  type ContentTree,
  buildCascadeFilteredItems,
  buildContentTree,
  buildDirectoryListingMap,
  extractFrontmatterBuiltinFields,
  getTdResourceTitle,
  type ContentSummary,
} from '@/shared';

interface SiteData {
  // Cascade-filtered items for the sidebar nav tree
  contentItems: ContentSummary[];
  // All items including hidden ones, for directory listings
  fullContentItems: ContentSummary[];
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

    const fullContentItems: ContentSummary[] = [];

    for (const filepath of files) {
      let raw: string | undefined;

      try {
        raw = readFileSync(resolve(rootDirectory, filepath), 'utf-8');
      } catch {
        // skip unreadable files
      }

      const fields = raw ? extractFrontmatterBuiltinFields(raw) : undefined;

      fullContentItems.push({
        filepath,
        label: fields?.label ?? getTdResourceTitle(filepath),
        header: {},
        ssg: {
          showInSidebar: fields?.showInSidebar ?? true,
          showInSearch: fields?.showInSearch ?? true,
          crawlable: fields?.crawlable ?? true,
        },
      });
    }

    const contentItems = buildCascadeFilteredItems(fullContentItems);
    const contentTree = buildContentTree(contentItems);
    const fullContentTree = buildContentTree(fullContentItems);

    this.data = {
      contentItems,
      fullContentItems,
      contentTree,
      schemas: {},
      directoryListings: buildDirectoryListingMap(fullContentTree.entries, ''),
    };
  }

  load (): string {
    const payload = this.serializeState();

    return `
import { shallowRef } from 'vue'
export const siteData = shallowRef(${JSON.stringify(payload)})
export default siteData
if (import.meta.hot) {
  import.meta.hot.on(${JSON.stringify(SITE_DATA_EVENT)}, (data) => {
    siteData.value = data
  })
  import.meta.hot.accept()
}`;
  }

  clear (): void {
    this.data = undefined;
  }

  // Fetch full data from RPC (used on initial load and config changes, not during HMR)
  fetch (context: TypedownContext, server: ViteDevServer): void {
    fetchFromRpc(context)
      .then((result) => {
        this.data = result;
        this.notifyHmrClient(server);
      })
      .catch((error) => {
        console.error(
          '[typedown] Failed to fetch site data:',
          error instanceof Error ? error.message : error,
        );
      });
  }

  // Fetch and block until data is ready (for production builds)
  async fetchSync (context: TypedownContext): Promise<void> {
    this.data = await fetchFromRpc(context);
  }

  // Patch a single sidebar entry, rebuild tree
  upsertSidebarEntry (sidebarItem: TdSidebarItem): void {
    if (!this.data) return;
    const summary = convertSidebarItemToSummary(sidebarItem);
    const fullIndex = this.data.fullContentItems.findIndex(
      (item) => item.filepath === sidebarItem.filepath,
    );

    if (0 <= fullIndex) {
      this.data.fullContentItems[fullIndex] = summary;
    } else {
      this.data.fullContentItems.push(summary);
    }

    this.rebuildTree();
  }

  // Remove a sidebar entry by filepath
  removeSidebarEntry (filepath: string): void {
    if (!this.data) return;
    this.data.fullContentItems = this.data.fullContentItems.filter(
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
      [name]: _removed, ...remainingSchemas
    } = this.data.schemas;

    this.data.schemas = remainingSchemas;
  }

  notifyHmrClient (server: ViteDevServer): void {
    server.hot.send({
      type: 'custom',
      event: SITE_DATA_EVENT,
      data: this.serializeState(),
    });
  }

  private rebuildTree (): void {
    if (!this.data) return;
    const contentItems = buildCascadeFilteredItems(this.data.fullContentItems);
    const contentTree = buildContentTree(contentItems);
    const fullContentTree = buildContentTree(this.data.fullContentItems);

    this.data.contentItems = contentItems;
    this.data.contentTree = contentTree;
    this.data.directoryListings = buildDirectoryListingMap(
      fullContentTree.entries,
      '',
    );
  }

  private serializeState (): Record<string, unknown> {
    if (!this.data) return EMPTY;

    return {
      ready: true,
      contentTree: this.data.contentTree,
      schemas: this.data.schemas,
      directoryListings: this.data.directoryListings,
    };
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
    ssg: item.ssg,
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

  // Build content tree from all sidebar items (including hidden)
  const fullContentItems: ContentSummary[] = sidebarItems.map((item) => ({
    ...item,
    header: {},
  }));
  const contentItems = buildCascadeFilteredItems(fullContentItems);
  const contentTree = buildContentTree(contentItems);
  const fullContentTree = buildContentTree(fullContentItems);
  const directoryListings = buildDirectoryListingMap(
    fullContentTree.entries,
    config.siteTitle,
  );

  return {
    contentItems,
    fullContentItems,
    contentTree,
    schemas,
    directoryListings,
  };
}
