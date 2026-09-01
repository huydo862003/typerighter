import {
  computed, type ComputedRef,
} from 'vue';
import {
  useSiteData, useRoute, useSiteConfig,
} from '../../app';
import {
  getDirectoryUrl, getIndexUrl, getNodeIndexItem, getTdContentUrl, getTdResourceTitle, isIndexFile, unslugify,
  type ContentTree, type ContentTreeEntry, type ContentTreeNode, type ContentSummary,
} from '@/shared';

export interface PreviousNextLink {
  url: string;
  title: string;
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
    const siblings = findSiblings(siteData.value.contentTree, route.path);

    if (siblings === undefined) return {};

    const index = siblings.pages.findIndex((page) => page.url === route.path);

    if (index === -1) return {
      groupName: siblings.groupName,
    };

    const previous = 0 < index ? siblings.pages[index - 1] : undefined;
    const next = index < siblings.pages.length - 1 ? siblings.pages[index + 1] : undefined;

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
      groupName: siblings.groupName,
    };
  });

  return {
    previous: computed(() => result.value.previous),
    next: computed(() => result.value.next),
    groupName: computed(() => result.value.groupName),
  };
}

interface SiblingGroup {
  pages: PreviousNextLink[];
  groupName: string;
}

function entriesToLinks (entries: ContentTreeEntry[], urlPrefix: string): PreviousNextLink[] {
  const pages: PreviousNextLink[] = [];

  for (const entry of entries) {
    if (entry.kind === 'file') {
      pages.push(getItemLink(entry.item));
    } else {
      pages.push(getNodeIndexLink(entry.node, urlPrefix));
    }
  }

  return pages;
}

// Find all sibling pages in the same directory as the current route
function findSiblings (tree: ContentTree, currentUrl: string): SiblingGroup | undefined {
  if (tree.entries.some((entry) => entry.kind === 'file' && getTdContentUrl(entry.item.filepath) === currentUrl)) {
    const pages = entriesToLinks(tree.entries, '');

    return {
      pages,
      groupName: '',
    };
  }

  for (const entry of tree.entries) {
    if (entry.kind !== 'dir') continue;

    const found = findSiblingsInNode(entry.node, currentUrl, '');

    if (found !== undefined) return found;
  }

  return undefined;
}

function findSiblingsInNode (node: ContentTreeNode, currentUrl: string, urlPrefix: string): SiblingGroup | undefined {
  const directoryUrl = getDirectoryUrl(urlPrefix, node.name);
  const isDirectChild = node.entries.some(
    (entry) => entry.kind === 'file' && getTdContentUrl(entry.item.filepath) === currentUrl,
  );

  if (isDirectChild) {
    const pages: PreviousNextLink[] = [];
    const indexItem = getNodeIndexItem(node);

    if (indexItem) {
      pages.push(getItemLink(indexItem));
    }

    for (const entry of node.entries) {
      if (entry.kind === 'file') {
        if (!isIndexFile(entry.item.filepath)) {
          pages.push(getItemLink(entry.item));
        }
      } else {
        pages.push(getNodeIndexLink(entry.node, directoryUrl));
      }
    }

    return {
      pages,
      groupName: unslugify(node.name),
    };
  }

  for (const entry of node.entries) {
    if (entry.kind !== 'dir') continue;

    const found = findSiblingsInNode(entry.node, currentUrl, directoryUrl);

    if (found !== undefined) return found;
  }

  return undefined;
}

function getItemLink (item: ContentSummary): PreviousNextLink {
  return {
    url: getTdContentUrl(item.filepath),
    title: getTdResourceTitle(item.filepath, item.label),
  };
}

function getNodeIndexLink (node: ContentTreeNode, urlPrefix: string): PreviousNextLink {
  const directoryUrl = getDirectoryUrl(urlPrefix, node.name);
  const indexItem = getNodeIndexItem(node);

  if (indexItem) {
    return getItemLink(indexItem);
  }

  return {
    url: getIndexUrl(directoryUrl),
    title: unslugify(node.name),
  };
}
