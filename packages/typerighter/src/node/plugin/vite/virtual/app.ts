import type {
  ViteDevServer,
} from 'vite';
import {
  RESOLVED_VIRTUAL_APP_ID,
  PAGES_ID,
  SITE_DATA_ID,
  SEARCH_INDEX_ID,
} from '../constants';
import {
  invalidateVirtualModule,
  type VirtualModule,
} from './utils';
import type {
  TypedownContext,
} from '@/node/lib/typedown-context';
import {
  CONTENT_EXTENSIONS,
} from '@/shared';

export interface AppEntryOptions {
  rootDir?: string;
  basePath?: string;
  siteTitle: string;
  siteDescription: string;
  repo?: string;
  author?: string;
  license?: string;
  nav?: {
    title: string;
    link: string;
    icon?: string;
  }[];
}

export class VirtualApp implements VirtualModule {
  async load (context: TypedownContext): Promise<string> {
    const config = await context.getConfig();

    return generate({
      ...config,
      rootDir: config.rootDir,
    });
  }

  invalidate (server: ViteDevServer): void {
    invalidateVirtualModule(server, RESOLVED_VIRTUAL_APP_ID);
  }
}

// Also used by SSG build to generate the client entry file
export function generate (options: AppEntryOptions): string {
  const {
    rootDir: rootDirectory = '.',
  } = options;

  const siteConfig = JSON.stringify({
    title: options.siteTitle,
    description: options.siteDescription,
    basePath: options.basePath ?? '/',
    repo: options.repo,
    author: options.author,
    license: options.license,
    nav: options.nav,
  });

  return `
import 'typerighter/style.css';
import 'typerighter/fonts.css';
import('typerighter/math.css');
import { createTypedownApp } from 'typerighter/client';
import { TdDirectoryIndex, TdGlossaryIndex } from 'typerighter/client/theme-default';
import { isIndexUrl, getDirectoryFromPageUrl } from 'typerighter/shared';
import { h } from 'vue';
import theme from 'typerighter/client/theme-default';
import { pages as initialPages } from '${PAGES_ID}';
import initialSiteData from '${SITE_DATA_ID}';
let pages = initialPages;
const contentExts = ${JSON.stringify(CONTENT_EXTENSIONS)};

function findPage(base) {
  for (const ext of contentExts) {
    const key = base + ext;
    if (pages[key]) return pages[key];
  }
}

async function loadPageModule(pagePath) {
  const base = ('/${rootDirectory}/' + pagePath).replace(/\\/+/g, '/').replace(/\\/$/, '');
  const loader = findPage(base);
  if (loader) return loader();

  if (isIndexUrl(pagePath)) {
    const dirPath = getDirectoryFromPageUrl(pagePath);
    const dir = siteData.value.directoryListings[dirPath];
    if (dir) return {
      default: { name: 'DirectoryIndex', render() { return h(TdDirectoryIndex); } },
      __pageData: { frontmatter: {}, headings: [], title: dir.title },
    };
  }

  return undefined;
}

const { app, searchIndex: searchIndexRef, siteData } = await createTypedownApp(loadPageModule, theme.Layout, ${siteConfig}, initialSiteData);
app.mount('#app');

// Load search index in the background after the app is mounted
import('${SEARCH_INDEX_ID}').then((m) => { searchIndexRef.value = m.default; });

// Accept HMR so modules update without a full page reload
if (import.meta.hot) {
  import.meta.hot.accept('${PAGES_ID}', (m) => {
    if (m) pages = m.pages;
  });

  import.meta.hot.accept('${SEARCH_INDEX_ID}', (m) => {
    if (m) searchIndexRef.value = m.default;
  });

  import.meta.hot.accept('${SITE_DATA_ID}', (m) => {
    if (m) siteData.value = m.default;
  });
}
`;
}
