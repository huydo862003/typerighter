import {
  computed,
} from 'vue';
import {
  useSiteData, useRoute, useSiteConfig,
} from '../../app';
import {
  getDirectoryUrl, getIndexUrl, getTdContentUrl, getTdResourceTitle, INDEX_FILENAME, path, unslugify,
  type ContentTree, type ContentTreeNode, type ContentSummary,
} from '@/shared';

export interface PreviousNextLink {
  url: string;
  title: string;
}

export function usePreviousNext () {
  const siteData = useSiteData();
  const route = useRoute();
  const {
    withBase,
  } = useSiteConfig();

  const flatPages = computed(() => flattenTree(siteData.contentTree));
  const currentIndex = computed(() => flatPages.value.findIndex((page) => page.url === route.path));
  const previous = computed(() => {
    if (currentIndex.value < 1) return undefined;
    const page = flatPages.value[currentIndex.value - 1];

    return {
      ...page,
      url: withBase(page.url),
    };
  });
  const next = computed(() => {
    if (currentIndex.value === -1 || flatPages.value.length - 1 <= currentIndex.value) return undefined;
    const page = flatPages.value[currentIndex.value + 1];

    return {
      ...page,
      url: withBase(page.url),
    };
  });

  return {
    previous,
    next,
  };
}

function flattenNode (node: ContentTreeNode, pages: PreviousNextLink[], urlPrefix: string) {
  const indexItem = node.items.find((item) => path.filestem(item.filepath) === INDEX_FILENAME);
  const directoryUrl = getDirectoryUrl(urlPrefix, node.name);

  if (indexItem) {
    pages.push(itemToLink(indexItem));
  } else {
    // Virtual index page for directories without index.td
    pages.push({
      url: getIndexUrl(directoryUrl),
      title: unslugify(node.name),
    });
  }

  for (const item of node.items) {
    if (path.filestem(item.filepath) !== INDEX_FILENAME) {
      pages.push(itemToLink(item));
    }
  }

  for (const child of node.children) {
    flattenNode(child, pages, directoryUrl);
  }
}

function flattenTree (tree: ContentTree): PreviousNextLink[] {
  const pages: PreviousNextLink[] = [];

  for (const item of tree.rootItems) {
    pages.push(itemToLink(item));
  }
  for (const child of tree.children) {
    flattenNode(child, pages, '');
  }

  return pages;
}

function itemToLink (item: ContentSummary): PreviousNextLink {
  return {
    url: getTdContentUrl(item.filepath),
    title: getTdResourceTitle(item.header, item.filepath),
  };
}
