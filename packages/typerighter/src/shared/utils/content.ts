import {
  INDEX_FILENAME,
} from '../constants';
import type {
  ContentSummary, ContentTree, ContentTreeNode, DirectoryEntry, DirectoryListing,
} from '../types/content';
import {
  parseNumericPrefix, unslugify,
} from './format';
import {
  getIndexUrl, getTdContentUrl,
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
    const indexItem = child.items.find((item) => filestem(item.filepath) === INDEX_FILENAME);

    if (indexItem) return getTdContentUrl(indexItem.filepath);

    return getIndexUrl(`${urlPrefix}/${child.name}`);
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
          .map((item) => toDirectoryEntry(item)),
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

// Derive the page title for an index file from its parent directory name
export function getTdIndexTitle (filepath: string, siteTitle: string): string {
  const parent = basename(dirname(filepath));

  return parent ? unslugify(parent) : siteTitle;
}

// Resolve a display title from frontmatter _label or the file path
export function getTdResourceTitle (header: Record<string, unknown>, filepath: string): string {
  if (header._label !== undefined) return String(header._label);

  const stem = filestem(filepath);

  // For index files, use the parent directory name
  if (stem === INDEX_FILENAME) {
    const parent = basename(dirname(filepath));

    return parent ? unslugify(parent) : stem;
  }

  return unslugify(stem);
}

// Return the first non-empty string value found among the given header keys
function getFirstString (header: Record<string, unknown>, ...keys: string[]): string | undefined {
  for (const key of keys) {
    const value = header[key];

    if (typeof value === 'string' && 0 < value.length) return value;
  }

  return undefined;
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

// Project a ContentSummary into a DirectoryEntry for the listing map
function toDirectoryEntry (item: ContentSummary): DirectoryEntry {
  const rawTags = item.header.tags;
  const tags = Array.isArray(rawTags)
    ? rawTags.filter((tag): tag is string => typeof tag === 'string')
    : [];

  return {
    name: getTdResourceTitle(item.header, item.filepath),
    url: getTdContentUrl(item.filepath),
    description: getFirstString(item.header, 'description', 'summary', 'excerpt'),
    tags: 0 < tags.length ? tags : undefined,
    mtime: item.metadata.mtime,
    schema: item.schema,
  };
}
