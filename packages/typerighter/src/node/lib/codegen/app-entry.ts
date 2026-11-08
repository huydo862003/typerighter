import type {
  UserComponent,
} from '@/node/lib/components';

export interface AppEntryOptions {
  rootDir?: string;
  // Path within the host Vite project where typedown is mounted (e.g. 'api-reference')
  mountPath?: string;
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
    mountPath = '',
    userComponents = [],
  } = options;

  // Full path from Vite project root to the vault directory
  const fullRootDirectory = [
    mountPath,
    rootDirectory === '.' ? '' : rootDirectory,
  ].filter(Boolean).join('/');

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
import { path } from 'typerighter/shared';
import { shallowRef } from 'vue';
import theme from '${ids.theme}';
import { pages as initialPages } from '${ids.pages}';
import initialSiteData from '${ids.siteData}';

const siteData = shallowRef({ ...initialSiteData, pages: initialPages });
const { app, router, searchIndex: searchIndexRef } = await createTypedownApp({ ...${siteConfig}, theme }, siteData);
${componentRegistrations}
app.mount('#app');

// Load search index in the background after the app is mounted
import('${ids.searchIndex}').then((m) => { searchIndexRef.value = m.default; });

// Accept HMR so modules update without a full page reload
if (import.meta.hot) {
  import.meta.hot.accept('${ids.pages}', (m) => {
    if (m) siteData.value = { ...siteData.value, pages: m.pages };
  });

  import.meta.hot.accept('${ids.searchIndex}', (m) => {
    if (m) searchIndexRef.value = m.default;
  });

  import.meta.hot.accept('${ids.siteData}', (m) => {
    if (m) siteData.value = { ...m.default, pages: siteData.value.pages };
  });

  // Update route.data directly when content changes - Vue SFC setup only re-runs on script block changes
  import.meta.hot.on('${ids.pageDataEvent}', ({ filepath, pageData }) => {
    const eventBase = path.join('/', '${fullRootDirectory}', path.stripExtension(filepath));
    const routeBase = path.join('/', '${fullRootDirectory}', router.route.path);
    if (eventBase === routeBase) {
      router.route.data = pageData;
    }
  });
}
`;
}
