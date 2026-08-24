import {
  computed, type ComputedRef,
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

interface SiblingGroup {
  pages: PreviousNextLink[];
  groupName: string;
}

export function usePreviousNext (): {
  previous: ComputedRef<PreviousNextLink | undefined>;
  next: ComputedRef<PreviousNextLink | undefined>;
  groupName: ComputedRef<string | undefined>;
} {
  const siteData = useSiteData();
  const route = useRoute();
  const {
    withBase,
  } = useSiteConfig();

  const result = computed(() => {
    const siblings = findSiblings(siteData.contentTree, route.path);

    if (siblings === undefined) return {};

    const idx = siblings.pages.findIndex((page) => page.url === route.path);

    if (idx === -1) return { groupName: siblings.groupName };

    const prev = 0 < idx ? siblings.pages[idx - 1] : undefined;
    const next = idx < siblings.pages.length - 1 ? siblings.pages[idx + 1] : undefined;

    return {
      previous: prev ? { ...prev, url: withBase(prev.url) } : undefined,
      next: next ? { ...next, url: withBase(next.url) } : undefined,
      groupName: siblings.groupName,
    };
  });

  return {
    previous: computed(() => result.value.previous),
    next: computed(() => result.value.next),
    groupName: computed(() => result.value.groupName),
  };
}

// Find all sibling pages in the same directory as the current route
function findSiblings (tree: ContentTree, currentUrl: string): SiblingGroup | undefined {
  if (tree.rootItems.some((item) => getTdContentUrl(item.filepath) === currentUrl)) {
    const pages: PreviousNextLink[] = [];

    for (const item of tree.rootItems) {
      pages.push(getItemLink(item));
    }
    for (const child of tree.children) {
      pages.push(getNodeIndexLink(child, ''));
    }

    return { pages, groupName: '' };
  }

  for (const child of tree.children) {
    const found = findSiblingsInNode(child, currentUrl, '');

    if (found !== undefined) return found;
  }

  return undefined;
}

function findSiblingsInNode (node: ContentTreeNode, currentUrl: string, urlPrefix: string): SiblingGroup | undefined {
  const directoryUrl = getDirectoryUrl(urlPrefix, node.name);
  const isDirectChild = node.items.some((item) => getTdContentUrl(item.filepath) === currentUrl);

  if (isDirectChild) {
    const pages: PreviousNextLink[] = [];
    const indexItem = node.items.find((item) => path.filestem(item.filepath) === INDEX_FILENAME);

    if (indexItem) {
      pages.push(getItemLink(indexItem));
    }

    for (const item of node.items) {
      if (path.filestem(item.filepath) !== INDEX_FILENAME) {
        pages.push(getItemLink(item));
      }
    }

    for (const child of node.children) {
      pages.push(getNodeIndexLink(child, directoryUrl));
    }

    return { pages, groupName: unslugify(node.name) };
  }

  for (const child of node.children) {
    const found = findSiblingsInNode(child, currentUrl, directoryUrl);

    if (found !== undefined) return found;
  }

  return undefined;
}

function getItemLink (item: ContentSummary): PreviousNextLink {
  return {
    url: getTdContentUrl(item.filepath),
    title: getTdResourceTitle(item.header, item.filepath),
  };
}

function getNodeIndexLink (node: ContentTreeNode, urlPrefix: string): PreviousNextLink {
  const directoryUrl = getDirectoryUrl(urlPrefix, node.name);
  const indexItem = node.items.find((item) => path.filestem(item.filepath) === INDEX_FILENAME);

  if (indexItem) {
    return getItemLink(indexItem);
  }

  return {
    url: getIndexUrl(directoryUrl),
    title: unslugify(node.name),
  };
}
