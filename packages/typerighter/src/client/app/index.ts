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
import TdFlashcard from '../theme-default/components/custom/TdFlashcard.vue';
import {
  Content,
} from './components/Content';
import {
  provideTdContent,
} from './composables/useTdContent';
import {
  createRouter, routerSymbol, type Router,
} from './router';
import type {
  PageModule,
  ContentTree,
  SchemaDefinition,
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
}

export interface TypedownSiteData {
  /** Content files as a recursive directory tree */
  contentTree: ContentTree;
  /** Serialized MiniSearch index for full-text search */
  searchIndex?: string;
  /** Schema definitions keyed by schema name */
  schemas: Record<string, SchemaDefinition>;
}

type PageLoader = (path: string) => Promise<PageModule | undefined>;

const siteConfigSymbol: InjectionKey<TypedownSiteConfig> = Symbol('typedown-site-config');
const siteDataSymbol: InjectionKey<TypedownSiteData> = Symbol('typedown-site-data');
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
}> {
  const siteConfig: TypedownSiteConfig = {
    title: config.title ?? '',
    description: config.description ?? '',
  };

  const siteData: TypedownSiteData = {
    contentTree: data.contentTree ?? {
      rootItems: [],
      children: [],
    },
    schemas: data.schemas ?? {},
  };

  const router = createRouter(loadPageModule);

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
  app.component('Flashcard', TdFlashcard);
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
  };
}

export function usePageLoader (): PageLoader | undefined {
  return inject(pageLoaderSymbol, undefined);
}

// Returns a reactive ref that updates on HMR
export function useSearchIndex (): ShallowRef<string | undefined> {
  return inject(searchIndexSymbol, shallowRef(undefined));
}

export function useSiteConfig (): TypedownSiteConfig {
  return inject(siteConfigSymbol, {
    title: '',
    description: '',
  });
}

export function useSiteData (): TypedownSiteData {
  return inject(siteDataSymbol, {
    contentTree: {
      rootItems: [],
      children: [],
    },
    schemas: {},
  });
}
