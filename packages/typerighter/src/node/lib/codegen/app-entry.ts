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
    rootDir: rootDirectory === '.' ? undefined : rootDirectory,
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
import 'typerighter/math.css';
import { createTypedownApp } from 'typerighter/client';
import { shallowRef, watch } from 'vue';
import theme from '${ids.theme}';
import { pages as initialPages } from '${ids.pages}';
import { siteData as siteDataRef } from '${ids.siteData}';
import { searchIndex as searchIndexRef } from '${ids.searchIndex}';

const siteData = shallowRef({ ...siteDataRef.value, pages: initialPages });
watch(siteDataRef, (data) => { siteData.value = { ...data, pages: siteData.value.pages }; });

const { app, router } = await createTypedownApp({ ...${siteConfig}, theme }, { siteData, searchIndex: searchIndexRef });
${componentRegistrations}
app.mount('#app');

// Pages module changes on file add/delete (rare) - accept to avoid full page reload
if (import.meta.hot) {
  import.meta.hot.accept('${ids.pages}', (m) => {
    if (m) siteData.value = { ...siteData.value, pages: m.pages };
  });
}
`;
}
