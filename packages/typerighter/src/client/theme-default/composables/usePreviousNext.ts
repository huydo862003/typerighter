import {
  computed,
} from 'vue';
import {
  useSiteData, useRoute,
} from '../../app';
import {
  getTdContentUrl, getTdResourceTitle, INDEX_FILENAME, path,
  type ContentTree, type ContentTreeNode, type ContentSummary,
} from '@/shared';

export interface PreviousNextLink {
  url: string;
  title: string;
}

export function usePreviousNext () {
  const siteData = useSiteData();
  const route = useRoute();

  const flatPages = computed(() => flattenTree(siteData.contentTree));
  const currentIndex = computed(() => flatPages.value.findIndex((page) => page.url === route.path));
  const previous = computed(() => currentIndex.value < 1 ? undefined : flatPages.value[currentIndex.value - 1]);
  const next = computed(() => currentIndex.value === -1 || flatPages.value.length - 1 <= currentIndex.value ? undefined : flatPages.value[currentIndex.value + 1]);

  return {
    previous,
    next,
  };
}

function flattenNode (node: ContentTreeNode, pages: PreviousNextLink[]) {
  const indexItem = node.items.find((item) => path.filestem(item.filepath) === INDEX_FILENAME);

  if (indexItem) {
    pages.push(itemToLink(indexItem));
  }

  for (const item of node.items) {
    if (path.filestem(item.filepath) !== INDEX_FILENAME) {
      pages.push(itemToLink(item));
    }
  }

  for (const child of node.children) {
    flattenNode(child, pages);
  }
}

function flattenTree (tree: ContentTree): PreviousNextLink[] {
  const pages: PreviousNextLink[] = [];

  for (const item of tree.rootItems) {
    pages.push(itemToLink(item));
  }
  for (const child of tree.children) {
    flattenNode(child, pages);
  }

  return pages;
}

function itemToLink (item: ContentSummary): PreviousNextLink {
  return {
    url: getTdContentUrl(item.filepath),
    title: getTdResourceTitle(item.header, item.filepath),
  };
}
