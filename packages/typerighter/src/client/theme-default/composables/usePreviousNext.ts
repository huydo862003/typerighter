import {
  computed, type ComputedRef,
} from 'vue';
import {
  useSiteData, useRoute, useSiteConfig,
} from '../../app';
import {
  getDirectoryUrl, getIndexUrl, getNodeIndexItem, getTdContentUrl, getTdIndexTitle, getTdResourceTitle, isIndexFile, unslugify,
  type ContentTree, type ContentTreeNode, type ContentSummary,
} from '@/shared';

export interface PreviousNextLink {
  url: string;
  title: string;
}

export function usePreviousNext (): {
  previous: ComputedRef<PreviousNextLink | undefined>;
  next: ComputedRef<PreviousNextLink | undefined>;
} {
  const siteData = useSiteData();
  const route = useRoute();
  const {
    withBase, title: siteTitle,
  } = useSiteConfig();

  const result = computed(() => {
    const allPages = flattenTree(siteData.value.contentTree, siteTitle);
    let index = allPages.findIndex((page) => page.url === route.path);

    // Page is SSG-hidden: use fullContentTree scoped to its directory
    if (index === -1 && siteData.value.fullContentTree) {
      const directory = route.path.split('/').slice(0, -1)
        .join('/');
      const fullPages = flattenTree(siteData.value.fullContentTree, siteTitle);
      const scopedPages = fullPages.filter((page) => page.url.startsWith(directory + '/'));

      index = scopedPages.findIndex((page) => page.url === route.path);

      if (index !== -1) {
        return buildResult(scopedPages, index);
      }
    }

    if (index === -1) return {};

    return buildResult(allPages, index);
  });

  function buildResult (pages: PreviousNextLink[], index: number) {
    const previous = 0 < index ? pages[index - 1] : undefined;
    const next = index < pages.length - 1 ? pages[index + 1] : undefined;

    return {
      previous: previous
        ? {
          ...previous,
          url: withBase(previous.url),
        }
        : undefined,
      next: next
        ? {
          ...next,
          url: withBase(next.url),
        }
        : undefined,
    };
  }

  return {
    previous: computed(() => result.value.previous),
    next: computed(() => result.value.next),
  };
}

function flattenNode (node: ContentTreeNode, urlPrefix: string, pages: PreviousNextLink[]) {
  const directoryUrl = getDirectoryUrl(urlPrefix, node.name);
  const indexItem = getNodeIndexItem(node);

  if (indexItem) {
    pages.push(getItemLink(indexItem));
  } else {
    // Virtual index page for directories without index.td
    pages.push({
      url: getIndexUrl(directoryUrl),
      title: unslugify(node.name),
    });
  }

  for (const entry of node.entries) {
    if (entry.kind === 'file' && !isIndexFile(entry.item.filepath)) {
      pages.push(getItemLink(entry.item));
    } else if (entry.kind === 'dir') {
      flattenNode(entry.node, directoryUrl, pages);
    }
  }
}

function flattenTree (tree: ContentTree, siteTitle: string): PreviousNextLink[] {
  const pages: PreviousNextLink[] = [];
  const indexEntry = tree.entries.find(
    (entry) => entry.kind === 'file' && isIndexFile(entry.item.filepath),
  );

  if (indexEntry && indexEntry.kind === 'file') {
    pages.push(getItemLink(indexEntry.item, siteTitle));
  } else {
    pages.push({
      url: '/index',
      title: siteTitle,
    });
  }

  for (const entry of tree.entries) {
    if (entry.kind === 'file' && !isIndexFile(entry.item.filepath)) {
      pages.push(getItemLink(entry.item));
    } else if (entry.kind === 'dir') {
      flattenNode(entry.node, '', pages);
    }
  }

  return pages;
}

function getItemLink (item: ContentSummary, siteTitle?: string): PreviousNextLink {
  const title = item.label !== undefined
    ? item.label
    : siteTitle !== undefined
      ? getTdIndexTitle(item.filepath, siteTitle)
      : getTdResourceTitle(item.filepath);

  return {
    url: getTdContentUrl(item.filepath),
    title,
  };
}
