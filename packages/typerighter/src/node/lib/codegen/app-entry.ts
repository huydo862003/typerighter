import type {
  UserComponent,
} from './components';
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

// Virtual module IDs injected by the caller
export interface VirtualModuleIds {
  theme: string;
  pages: string;
  siteData: string;
  searchIndex: string;
  pageDataEvent: string;
}

// Generate the client app entry module source code
export function generateAppEntry (options: AppEntryOptions, ids: VirtualModuleIds): string {
  const {
    rootDir: rootDirectory = '.',
    userComponents = [],
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
import theme from '${ids.theme}';
import { pages as initialPages } from '${ids.pages}';
import initialSiteData from '${ids.siteData}';
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

const { app, router, searchIndex: searchIndexRef, siteData } = await createTypedownApp(loadPageModule, theme, ${siteConfig}, initialSiteData);
${componentRegistrations}
app.mount('#app');

// Load search index in the background after the app is mounted
import('${ids.searchIndex}').then((m) => { searchIndexRef.value = m.default; });

// Accept HMR so modules update without a full page reload
if (import.meta.hot) {
  import.meta.hot.accept('${ids.pages}', (m) => {
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

  import.meta.hot.accept('${ids.searchIndex}', (m) => {
    if (m) searchIndexRef.value = m.default;
  });

  import.meta.hot.accept('${ids.siteData}', (m) => {
    if (m) siteData.value = m.default;
  });

  // Update route.data directly when content changes - Vue SFC setup only re-runs on script block changes
  import.meta.hot.on('${ids.pageDataEvent}', ({ filepath, pageData }) => {
    const eventBase = path.join('/', '${rootDirectory}', path.stripExtension(filepath));
    const routeBase = path.join('/', '${rootDirectory}', router.route.path);
    if (eventBase === routeBase) {
      router.route.data = pageData;
    }
  });
}
`;
}
