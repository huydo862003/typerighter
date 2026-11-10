import {
  createApp,
  defineComponent,
  h,
  inject,
  ref,
  shallowRef,
  watch,
  type App,
  type Component,
  type InjectionKey,
  type Ref,
  type ShallowRef,
} from 'vue';
import defaultTheme from '../theme-default';
import TdDirectoryIndex from '../theme-default/components/custom/TdDirectoryIndex.vue';
import TdFlashcard from '../theme-default/components/custom/TdFlashcard.vue';
import TdGrid from '../theme-default/components/custom/TdGrid.vue';
import TdGridCell from '../theme-default/components/custom/TdGridCell.vue';
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
  CONTENT_EXTENSIONS,
  isIndexUrl,
  getDirectoryFromPageUrl,
  getParentUrl,
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
export {
  useTheme,
  type Theme,
} from '../theme-default/composables/client-preferences/useTheme';

export interface TypedownSiteConfig {
  /** Site title */
  title: string;
  /** Site description */
  description: string;
  /** URL base path (e.g. "/" or "/blog") */
  basePath: string;
  /** Absolute site URL (e.g. "https://example.com") */
  origin?: string;
  /** HTML lang attribute */
  lang?: string;
  /** Repository config */
  repo?: {
    url: string;
    root: string;
  };
  /** Site author */
  author?: string;
  /** License name */
  license?: string;
  /** Vault root directory relative to the Vite project root (e.g. "vault" or "api-reference/vault") */
  rootDir?: string;
  /** Navigation links */
  nav?: {
    title: string;
    link: string;
    icon?: string;
  }[];
  /** Active theme */
  theme: TypedownTheme;
}

export interface TypedownSiteData {
  /** Whether the data has been fetched from the server */
  ready: boolean;
  /** Page loaders keyed by vault-relative path (e.g. "/vault/notes/intro.td") */
  pages: Record<string, () => Promise<PageModule>>;
  /** Content files as a recursive directory tree */
  contentTree: ContentTree;
  /** Serialized MiniSearch index for full-text search */
  searchIndex?: string;
  /** Schema definitions keyed by schema name */
  schemas: Record<string, SchemaDefinition>;
  /** Directory listings keyed by URL path */
  directoryListings: Record<string, DirectoryListing>;
}

export interface TypedownTheme {
  /** Root layout component */
  Layout: Component;
  /** Optional 404 not-found component */
  NotFound?: Component;
  /** Called after app creation, before mount */
  enhanceApp?: (context: {
    app: App;
    router: Router;
    siteConfig: TypedownSiteConfig;
    siteData: Ref<TypedownSiteData>;
  }) => void | Promise<void>;
}

type PageLoader = (path: string) => Promise<PageModule | undefined>;

const siteConfigSymbol: InjectionKey<TypedownSiteConfig> = Symbol(
  'typedown-site-config',
);
const siteDataSymbol: InjectionKey<Ref<TypedownSiteData>> =
  Symbol('typedown-site-data');
const pageLoaderSymbol: InjectionKey<PageLoader> = Symbol(
  'typedown-page-loader',
);
const searchIndexSymbol: InjectionKey<ShallowRef<string | undefined>> = Symbol(
  'typedown-search-index',
);

export async function createTypedownApp (
  siteConfig: TypedownSiteConfig,
  siteData: Ref<TypedownSiteData>,
): Promise<{
  app: App;
  router: Router;
  searchIndex: ShallowRef<string | undefined>;
}> {
  const {
    theme,
  } = siteConfig;

  // Strip the vault root prefix from glob keys to match against route paths exactly
  const rootPrefix = '/' + (siteConfig.rootDir ?? '');

  function findPage (pagePath: string): (() => Promise<PageModule>) | undefined {
    for (const extension of CONTENT_EXTENSIONS) {
      const target = pagePath + extension;

      for (const key of Object.keys(siteData.value.pages)) {
        if (!key.startsWith(rootPrefix)) continue;
        const relative = key.slice(rootPrefix.length);

        if (relative === target) return siteData.value.pages[key];
      }
    }
  }

  async function loadPageModule (
    pagePath: string,
  ): Promise<PageModule | undefined> {
    const loader = findPage(pagePath);

    if (loader) return loader();

    if (isIndexUrl(pagePath)) {
      const directoryPath = getDirectoryFromPageUrl(pagePath);
      const directory = siteData.value.directoryListings[directoryPath];

      if (directory)
        return {
          default: {
            name: 'DirectoryIndex',
            render () {
              return h(TdDirectoryIndex);
            },
          },
          pageData: {
            frontmatter: {},
            headings: [],
            title: directory.title,
            titleHtml: directory.title,
            filepath: '',
          },
        };
    }
  }

  if (typeof document !== 'undefined' && siteConfig.lang) {
    document.documentElement.lang = siteConfig.lang;
  }

  const router = createRouter(loadPageModule, {
    basePath: siteConfig.basePath,
    fallbackComponent: theme.NotFound,
  });

  // When pages change, navigate away from removed pages or into newly added ones
  watch(siteData, (next, previous) => {
    if (previous.pages === next.pages) return;

    const routePath = router.route.path;
    const hadPage = CONTENT_EXTENSIONS.some((extension) =>
      Object.keys(previous.pages).some((key) => key.endsWith(routePath + extension)));
    const hasPage = !!findPage(routePath);

    if (!hadPage && hasPage) {
      router.go(routePath);

      return;
    }

    if (hadPage && !hasPage) {
      router.go(getParentUrl(routePath));
    }
  });

  const TypedownApp = defineComponent({
    name: 'TypedownApp',
    setup () {
      provideTdContent(router.route);

      return () => h(theme.Layout);
    },
  });

  const app = createApp(TypedownApp);

  app.provide(routerSymbol, router);
  app.provide(siteConfigSymbol, siteConfig);
  app.provide(siteDataSymbol, siteData);
  app.provide(pageLoaderSymbol, loadPageModule);
  const searchIndex = shallowRef(siteData.value.searchIndex);

  app.provide(searchIndexSymbol, searchIndex);
  app.component('TypedownContent', Content);

  // custom components
  /* eslint-disable vue/multi-word-component-names */
  app.component('DirectoryIndex', TdDirectoryIndex);
  app.component('GlossaryIndex', TdGlossaryIndex);
  app.component('Flashcard', TdFlashcard);
  app.component('Grid', TdGrid);
  app.component('GridCell', TdGridCell);
  app.component('LucideIcon', TdLucideIcon);
  /* eslint-enable vue/multi-word-component-names */

  if (theme.enhanceApp) {
    await theme.enhanceApp({
      app,
      router,
      siteConfig,
      siteData,
    });
  }

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
    theme: defaultTheme,
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

const defaultSiteData = ref<TypedownSiteData>({
  ready: false,
  pages: {},
  contentTree: {
    entries: [],
  },
  schemas: {},
  directoryListings: {},
});

export function useSiteData (): Ref<TypedownSiteData> {
  return inject(siteDataSymbol, defaultSiteData);
}
