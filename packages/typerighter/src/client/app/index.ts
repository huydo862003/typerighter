import {
  createApp,
  defineComponent,
  h,
  inject,
  shallowRef,
  type App,
  type Component,
  type InjectionKey,
  type ShallowRef,
} from 'vue';
import TdDirectoryIndex from '../theme-default/components/custom/TdDirectoryIndex.vue';
import TdFlashcard from '../theme-default/components/custom/TdFlashcard.vue';
import TdGlossaryIndex from '../theme-default/components/custom/glossary/TdGlossaryIndex.vue';
import TdLucideIcon from '../theme-default/components/TdLucideIcon.vue';
import {
  Content,
} from './components/Content';
import {
  provideTdContent,
} from './composables/useTdContent';
import {
  createRouter, routerSymbol, type Router,
} from './router';
import {
  stripTrailingSlash,
  type PageModule,
  type ContentTree,
  type SchemaDefinition,
  type DirectoryListing,
} from '@/shared';

export {
  useTdContent,
} from './composables/useTdContent';
export {
  useRouter, useRoute,
} from './router';
export {
  Content,
} from './components/Content';

export interface TypedownSiteConfig {
  /** Site title */
  title: string;
  /** Site description */
  description: string;
  /** URL base path (e.g. "/" or "/blog") */
  basePath: string;
  /** Repository URL */
  repo?: string;
  /** Site author */
  author?: string;
  /** License name */
  license?: string;
  /** Navigation links */
  nav?: {
    title: string;
    link: string;
    icon?: string;
  }[];
}

export interface TypedownSiteData {
  /** Content files as a recursive directory tree */
  contentTree: ContentTree;
  /** Serialized MiniSearch index for full-text search */
  searchIndex?: string;
  /** Schema definitions keyed by schema name */
  schemas: Record<string, SchemaDefinition>;
  /** Directory listings keyed by URL path */
  directoryListings: Record<string, DirectoryListing>;
}

type PageLoader = (path: string) => Promise<PageModule | undefined>;

const siteConfigSymbol: InjectionKey<TypedownSiteConfig> = Symbol('typedown-site-config');
const siteDataSymbol: InjectionKey<ShallowRef<TypedownSiteData>> = Symbol('typedown-site-data');
const pageLoaderSymbol: InjectionKey<PageLoader> = Symbol('typedown-page-loader');
const searchIndexSymbol: InjectionKey<ShallowRef<string | undefined>> = Symbol('typedown-search-index');

export async function createTypedownApp (
  loadPageModule: (path: string) => Promise<PageModule | undefined>,
  Layout: Component,
  config: Partial<TypedownSiteConfig> = {},
  data: Partial<TypedownSiteData> = {},
): Promise<{
  app: App;
  router: Router;
  searchIndex: ShallowRef<string | undefined>;
  siteData: ShallowRef<TypedownSiteData>;
}> {
  const siteConfig: TypedownSiteConfig = {
    title: config.title ?? '',
    description: config.description ?? '',
    basePath: config.basePath ?? '/',
    repo: config.repo,
    author: config.author,
    license: config.license,
    nav: config.nav,
  };

  const siteData = shallowRef<TypedownSiteData>({
    contentTree: data.contentTree ?? {
      entries: [],
    },
    schemas: data.schemas ?? {},
    directoryListings: data.directoryListings ?? {},
  });

  const router = createRouter(loadPageModule, {
    basePath: siteConfig.basePath,
  });

  const TypedownApp = defineComponent({
    name: 'TypedownApp',
    setup () {
      provideTdContent(router.route);

      return () => h(Layout);
    },
  });

  const app = createApp(TypedownApp);

  app.provide(routerSymbol, router);
  app.provide(siteConfigSymbol, siteConfig);
  app.provide(siteDataSymbol, siteData);
  app.provide(pageLoaderSymbol, loadPageModule);
  const searchIndex = shallowRef(data.searchIndex);

  app.provide(searchIndexSymbol, searchIndex);
  app.component('TypedownContent', Content);

  // custom components
  /* eslint-disable vue/multi-word-component-names */
  app.component('DirectoryIndex', TdDirectoryIndex);
  app.component('GlossaryIndex', TdGlossaryIndex);
  app.component('Flashcard', TdFlashcard);
  app.component('LucideIcon', TdLucideIcon);
  /* eslint-enable vue/multi-word-component-names */

  if (typeof window !== 'undefined') {
    await router.go(location.href, {
      replace: true,
      initialLoad: true,
    });
  }

  return {
    app,
    router,
    searchIndex,
    siteData,
  };
}

export function usePageLoader (): PageLoader | undefined {
  return inject(pageLoaderSymbol, undefined);
}

// Returns a reactive ref that updates on HMR
export function useSearchIndex (): ShallowRef<string | undefined> {
  return inject(searchIndexSymbol, shallowRef(undefined));
}

export function useSiteConfig () {
  const config = inject(siteConfigSymbol, {
    title: '',
    description: '',
    basePath: '/',
  });
  const base = stripTrailingSlash(config.basePath);

  return {
    ...config,
    withBase (path: string): string {
      if (base === '/') return path;

      const normalized = path.startsWith('/') ? path : '/' + path;

      return base + normalized;
    },
  };
}

const defaultSiteData = shallowRef<TypedownSiteData>({
  contentTree: {
    entries: [],
  },
  schemas: {},
  directoryListings: {},
});

export function useSiteData (): ShallowRef<TypedownSiteData> {
  return inject(siteDataSymbol, defaultSiteData);
}
