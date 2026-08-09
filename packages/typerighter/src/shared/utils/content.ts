import {
  INDEX_FILENAME,
} from '../constants';
import type {
  ContentSummary, ContentTree, ContentTreeNode, DirectoryListing,
} from '../types/content';
import {
  parseNumericPrefix, unslugify,
} from './format';
import {
  getParentUrl, getTdContentUrl,
} from './url';
import {
  basename, filestem, dirname,
} from './path';

// Build a recursive tree from a flat list of content summaries, grouped by directory
export function buildContentTree (items: ContentSummary[]): ContentTree {
  const root: ContentTreeNode = {
    name: '',
    children: [],
    items: [],
  };

  for (const item of items) {
    const parts = item.filepath.split('/');
    const directoryParts = parts.slice(0, -1);

    let current = root;

    for (const part of directoryParts) {
      let child = current.children.find((node) => node.name === part);

      if (!child) {
        child = {
          name: part,
          children: [],
          items: [],
        };
        current.children.push(child);
      }

      current = child;
    }

    current.items.push(item);
  }

  sortTree(root);

  return {
    rootItems: root.items,
    children: root.children,
  };
}

// Build a map of directory paths to their directory listing data
export function buildDirectoryListingMap (tree: ContentTreeNode[], rootTitle: string): Record<string, DirectoryListing> {
  const map: Record<string, DirectoryListing> = {};

  function countDescendants (node: ContentTreeNode): number {
    let count = node.items.length;

    for (const child of node.children) {
      count += countDescendants(child);
    }

    return count;
  }

  function getChildUrl (child: ContentTreeNode, urlPrefix: string): string {
    const first = child.items[0] ?? child.children[0]?.items[0];

    if (!first) return `${urlPrefix}/${child.name}`;

    const contentUrl = getTdContentUrl(first.filepath);

    return filestem(first.filepath) === INDEX_FILENAME
      ? contentUrl
      : getParentUrl(contentUrl);
  }

  function walk (nodes: ContentTreeNode[], urlPrefix: string) {
    for (const node of nodes) {
      const directoryUrl = urlPrefix ? `${urlPrefix}/${node.name}` : `/${node.name}`;

      map[directoryUrl] = {
        title: unslugify(node.name),
        url: directoryUrl,
        subdirectories: node.children
          .map((child) => ({
            name: unslugify(child.name),
            url: getChildUrl(child, directoryUrl),
            count: countDescendants(child),
          })),
        items: node.items
          .map((item) => ({
            name: getTdResourceTitle(item.header, item.filepath),
            url: getTdContentUrl(item.filepath),
          })),
      };

      walk(node.children, directoryUrl);
    }
  }

  // Root directory
  map['/'] = {
    title: rootTitle,
    url: '/',
    subdirectories: tree
      .map((child) => ({
        name: unslugify(child.name),
        url: getChildUrl(child, ''),
        count: countDescendants(child),
      })),
    items: [],
  };

  walk(tree, '');

  return map;
}

export function getTdIndexTitle (filepath: string, siteTitle: string): string {
  const parent = basename(dirname(filepath));

  return parent ? unslugify(parent) : siteTitle;
}

// Resolve a display title from frontmatter _label, name, or the file path
export function getTdResourceTitle (header: Record<string, unknown>, filepath: string): string {
  if (header._label !== undefined) return String(header._label);
  if (header.name !== undefined) return String(header.name);

  return unslugify(filestem(filepath));
}

// Sort by numeric prefix first, then alphabetically as fallback
function sortTree (node: ContentTreeNode) {
  node.children.sort((left, right) => {
    const leftPrefix = parseNumericPrefix(left.name);
    const rightPrefix = parseNumericPrefix(right.name);

    if (leftPrefix.order !== rightPrefix.order) return leftPrefix.order - rightPrefix.order;

    return leftPrefix.rest.localeCompare(rightPrefix.rest);
  });

  node.items.sort((left, right) => {
    const leftPrefix = parseNumericPrefix(filestem(left.filepath));
    const rightPrefix = parseNumericPrefix(filestem(right.filepath));

    if (leftPrefix.order !== rightPrefix.order) return leftPrefix.order - rightPrefix.order;

    return getTdResourceTitle(left.header, left.filepath).localeCompare(getTdResourceTitle(right.header, right.filepath));
  });

  for (const child of node.children) {
    sortTree(child);
  }
}
