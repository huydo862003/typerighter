import type {
  Component, InjectionKey,
} from 'vue';
import {
  inject, markRaw, nextTick, reactive, readonly,
} from 'vue';
import {
  resolveRootUrl, stripTrailingSlash,
  type PageData, type PageModule,
} from '@/shared';

const isInBrowser = typeof window !== 'undefined';

export interface Route {
  path: string;
  hash: string;
  query: string;
  contentSfc: Component | undefined;
  data: PageData;
}

export interface Router {
  route: Route;
  go: (to: string, options?: {
    replace?: boolean;
    initialLoad?: boolean;
  }) => Promise<void>;
  onBeforeRouteChange?: (to: string) => Promise<boolean | undefined> | boolean | undefined;
  onAfterRouteChange?: (to: string) => Promise<undefined> | undefined;
}

export const routerSymbol: InjectionKey<Router> = Symbol('typedown-router');

const notFoundPageData: PageData = {
  frontmatter: {},
  headings: [],
  title: 'Not Found',
};

// Create a client-side router that loads .td page modules on navigation
export interface RouterOptions {
  basePath?: string;
  fallbackComponent?: Component;
}

export function createRouter (
  loadPageModule: (path: string) => Promise<PageModule | undefined>,
  options: RouterOptions = {},
): Router {
  const base = stripTrailingSlash(options.basePath ?? '/');
  const route = reactive<Route>({
    path: '/',
    hash: '',
    query: '',
    contentSfc: undefined,
    data: notFoundPageData,
  });

  const router: Router = {
    route,
    async go (href, options) {
      href = normalizeHref(href, base);

      if ((await router.onBeforeRouteChange?.(href)) === false) return;

      if (!isInBrowser || options?.initialLoad || changeRoute(href, base, options)) {
        await loadPage(href);
      }

      syncRouteQueryAndHash();
      await router.onAfterRouteChange?.(href);
    },
  };

  let latestPendingPath: string | undefined;

  // Fetch and apply a page module, guarding against stale loads
  async function loadPage (href: string): Promise<void> {
    const targetLocation = new URL(href, 'http://a.com');
    const pendingPath = (latestPendingPath = targetLocation.pathname);

    try {
      const page = await loadPageModule(pendingPath);

      if (!page) throw new Error(`Page not found: ${pendingPath}`);

      if (latestPendingPath === pendingPath) {
        latestPendingPath = undefined;
        const {
          default: comp, __pageData,
        } = page;

        route.path = pendingPath;
        route.contentSfc = markRaw(comp);
        route.data = import.meta.env.PROD
          ? markRaw(__pageData)
          : (readonly(__pageData) as PageData);

        syncRouteQueryAndHash(targetLocation);

        if (isInBrowser) {
          nextTick(() => {
            if (targetLocation.hash) {
              scrollToHash(targetLocation.hash);
            } else {
              window.scrollTo(0, 0);
            }
          });
        }
      }
    } catch {
      if (latestPendingPath === pendingPath) {
        latestPendingPath = undefined;
        route.path = pendingPath;
        route.contentSfc = options.fallbackComponent ? markRaw(options.fallbackComponent) : undefined;
        route.data = notFoundPageData;
        syncRouteQueryAndHash(targetLocation);
      }
    }
  }

  // Sync the route's query and hash from the current location
  function syncRouteQueryAndHash (
    location_: {
      search: string;
      hash: string;
    } = isInBrowser
      ? location
      : {
        search: '',
        hash: '',
      },
  ): void {
    route.query = location_.search;
    route.hash = decodeURIComponent(location_.hash);
  }

  if (isInBrowser) {
    if (history.state == undefined) history.replaceState({}, '');

    window.addEventListener('click', (event) => {
      if (
        event.defaultPrevented
        || !(event.target instanceof Element)
        || event.button !== 0
        || event.ctrlKey || event.shiftKey || event.altKey || event.metaKey
      ) return;

      const link = event.target.closest<HTMLAnchorElement>('a');

      if (!link || link.hasAttribute('download') || link.hasAttribute('target')) return;

      const linkHref = link.getAttribute('href');

      if (linkHref === null) return;

      const {
        href, origin, pathname,
      } = new URL(linkHref, link.baseURI);

      if (origin === location.origin && !pathname.match(/\.\w+$/)) {
        event.preventDefault();
        router.go(href);
      }
    }, {
      capture: true,
    });

    window.addEventListener('popstate', async (event) => {
      if (event.state == undefined) return;

      const href = normalizeHref(location.href, base);

      await loadPage(href);
      syncRouteQueryAndHash();
      await router.onAfterRouteChange?.(href);
    });

    window.addEventListener('hashchange', (event) => {
      event.preventDefault();
      syncRouteQueryAndHash();
    });
  }

  return router;
}

// Scroll to the element matching a URL hash fragment
export function scrollToHash (hash: string): void {
  if (!hash) return;

  let target: HTMLElement | undefined;

  try {
    target = document.getElementById(decodeURIComponent(hash).slice(1)) ?? undefined;
  } catch {
    return;
  }

  target?.scrollIntoView({
    block: 'start',
  });
}

// Inject the current reactive route from the nearest router provider
export function useRoute (): Route {
  return useRouter().route;
}

// Inject the router instance from the nearest provider
export function useRouter (): Router {
  const router = inject(routerSymbol);

  if (!router) throw new Error('useRouter() called outside of Typedown app');

  return router;
}

// Push or replace the browser history entry, returning true if the pathname changed
function changeRoute (
  href: string,
  base: string,
  {
    replace = false,
  } = {},
): boolean {
  const location_ = normalizeHref(location.href, base);

  if (href === location_) {
    scrollToHash(new URL(href, 'http://a.com').hash);

    return false;
  }

  if (replace) {
    history.replaceState({}, '', href);
  } else {
    history.replaceState({
      scrollPosition: window.scrollY,
    }, '');
    history.pushState({}, '', href);
  }

  const nextUrl = new URL(href, location.origin);
  const currentUrl = new URL(location_, location.origin);

  if (nextUrl.pathname === currentUrl.pathname) {
    scrollToHash(nextUrl.hash);

    return false;
  }

  return true;
}

// Strip trailing .html, strip base path, and return pathname + search + hash
function normalizeHref (href: string, base: string): string {
  const url = new URL(href, 'http://a.com');

  let pathname = url.pathname.replace(/\.html$/, '');

  if (base && pathname.startsWith(base)) {
    pathname = pathname.slice(base.length) || '/';
  }

  url.pathname = resolveRootUrl(pathname);

  return url.pathname + url.search + url.hash;
}
