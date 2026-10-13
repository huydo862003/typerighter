import path from 'node:path';
import {
  readdir,
} from 'node:fs/promises';
import type {
  ViteDevServer,
} from 'vite';
import type {
  TdComponentEntry,
} from '@typerighter/rpc-client';
import {
  RESOLVED_VIRTUAL_APP_ID,
  PAGES_ID,
  SITE_DATA_ID,
  SEARCH_INDEX_ID,
  PAGE_DATA_EVENT,
} from '../constants';
import {
  invalidateVirtualModule,
} from './utils';
import type {
  AppContext,
} from '@/node/context';
import {
  CONTENT_EXTENSIONS,
} from '@/shared';

export interface AppEntryOptions {
  rootDir?: string;
  basePath?: string;
  origin?: string;
  lang?: string;
  siteTitle: string;
  siteDescription: string;
  repo?: {
    url: string;
    root: string;
  };
  author?: string;
  license?: string;
  nav?: {
    title: string;
    link: string;
    icon?: string;
  }[];
  userComponents?: UserComponent[];
}

export interface UserComponent {
  name: string;
  absolutePath: string;
}

export class VirtualApp {
  async load (context: AppContext): Promise<string> {
    const tdContext = await context.getTdContext();
    const config = await tdContext.getConfig();
    const userComponents = await resolveUserComponents(config.components, context.root);

    return generate({
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
    });
  }

  invalidate (server: ViteDevServer): void {
    invalidateVirtualModule(server, RESOLVED_VIRTUAL_APP_ID);
  }
}

// Also used by SSG build to generate the client entry file
export function generate (options: AppEntryOptions): string {
  const {
    rootDir: rootDirectory = '.', userComponents = [],
  } = options;

  const componentImports = userComponents
    .map((component) => `import ${component.name} from ${JSON.stringify(component.absolutePath)};`)
    .join('\n');

  const componentRegistrations = userComponents
    .map((component) => `app.component(${JSON.stringify(component.name)}, ${component.name});`)
    .join('\n');

  const siteConfig = JSON.stringify({
    title: options.siteTitle,
    description: options.siteDescription,
    basePath: options.basePath ?? '/',
    origin: options.origin,
    lang: options.lang,
    repo: options.repo,
    author: options.author,
    license: options.license,
    rootDir: rootDirectory,
    nav: options.nav,
  });

  const header = [
    componentImports,
    'import \'typerighter/style.css\';',
  ]
    .filter(Boolean)
    .join('\n');

  return `
${header}
import 'typerighter/fonts.css';
import('typerighter/math.css');
import { createTypedownApp } from 'typerighter/client';
import { TdDirectoryIndex, TdGlossaryIndex } from 'typerighter/client/theme-default';
import { isIndexUrl, getDirectoryFromPageUrl, getParentUrl, path } from 'typerighter/shared';
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
  const loader = findPage(path.join('/', '${rootDirectory}', pagePath));
  if (loader) return loader();

  if (isIndexUrl(pagePath)) {
    const dirPath = getDirectoryFromPageUrl(pagePath);
    const dir = siteData.value.directoryListings[dirPath];
    if (dir) return {
      default: { name: 'DirectoryIndex', render() { return h(TdDirectoryIndex); } },
      pageData: { frontmatter: {}, headings: [], title: dir.title },
    };
  }

  return undefined;
}

const { app, router, searchIndex: searchIndexRef, siteData } = await createTypedownApp(loadPageModule, theme.Layout, ${siteConfig}, initialSiteData);
${componentRegistrations}
app.mount('#app');

// Load search index in the background after the app is mounted
import('${SEARCH_INDEX_ID}').then((m) => { searchIndexRef.value = m.default; });

// Accept HMR so modules update without a full page reload
if (import.meta.hot) {
  import.meta.hot.accept('${PAGES_ID}', (m) => {
    if (!m) return;
    const base = path.join('/', '${rootDirectory}', router.route.path);
    const hadPage = !!findPage(base);
    pages = m.pages;
    const hasPage = !!findPage(base);
    // Current page was removed (deleted or renamed away)
    if (hadPage && !hasPage) router.go(getParentUrl(router.route.path));
    // A real page appeared where a directory index was shown
    if (!hadPage && hasPage) router.go(router.route.path);
  });

  import.meta.hot.accept('${SEARCH_INDEX_ID}', (m) => {
    if (m) searchIndexRef.value = m.default;
  });

  import.meta.hot.accept('${SITE_DATA_ID}', (m) => {
    if (m) siteData.value = m.default;
  });

  // Update route.data directly when content changes - Vue SFC setup only re-runs on script block changes
  import.meta.hot.on('${PAGE_DATA_EVENT}', ({ filepath, pageData }) => {
    const eventBase = path.join('/', '${rootDirectory}', path.stripExtension(filepath));
    const routeBase = path.join('/', '${rootDirectory}', router.route.path);
    if (eventBase === routeBase) {
      router.route.data = pageData;
    }
  });
}
`;
}

// Resolve TdComponentEntry list to concrete { name, absolutePath } pairs
export async function resolveUserComponents (
  entries: TdComponentEntry[],
  projectRoot: string,
): Promise<UserComponent[]> {
  const components: UserComponent[] = [];

  for (const entry of entries) {
    if (entry.kind === 'dir') {
      const directoryPath = path.resolve(projectRoot, entry.path);
      let files: string[];

      try {
        files = await readdir(directoryPath);
      } catch {
        continue;
      }

      for (const file of files) {
        if (!file.endsWith('.vue')) continue;

        const name = file.slice(0, -4);

        components.push({
          name,
          absolutePath: path.join(directoryPath, file),
        });
      }
    } else {
      components.push({
        name: entry.name,
        absolutePath: path.resolve(projectRoot, entry.source),
      });
    }
  }

  return components;
}
