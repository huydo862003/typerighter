import {
  INDEX_FILENAME,
} from '../constants';
import type {
  ContentSummary, ContentTree, ContentTreeEntry, ContentTreeNode, DirectoryEntry, DirectoryListing, DirectoryListingEntry,
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
    entries: [],
  };

  for (const item of items) {
    const parts = item.filepath.split('/');
    const directoryParts = parts.slice(0, -1);

    let current = root;

    for (const part of directoryParts) {
      let directoryEntry = current.entries.find(
        (entry): entry is ContentTreeEntry & {
          kind: 'dir';
        } => entry.kind === 'dir' && entry.node.name === part,
      );

      if (!directoryEntry) {
        const node: ContentTreeNode = {
          name: part,
          entries: [],
        };

        directoryEntry = {
          kind: 'dir',
          node,
        };
        current.entries.push(directoryEntry);
      }

      current = directoryEntry.node;
    }

    current.entries.push({
      kind: 'file',
      item,
    });
  }

  sortTree(root);

  return {
    entries: root.entries,
  };
}

// Build a map of directory paths to their directory listing data
export function buildDirectoryListingMap (entries: ContentTreeEntry[], rootTitle: string): Record<string, DirectoryListing> {
  const map: Record<string, DirectoryListing> = {};

  function countDescendants (node: ContentTreeNode): number {
    let count = 0;

    for (const entry of node.entries) {
      if (entry.kind === 'file') {
        if (!isIndexFile(entry.item.filepath)) count++;
      } else {
        count += countDescendants(entry.node);
      }
    }

    return count;
  }

  function getChildUrl (child: ContentTreeNode, urlPrefix: string): string {
    const indexItem = getNodeIndexItem(child);

    if (indexItem) return getTdContentUrl(indexItem.filepath);

    return getIndexUrl(`${urlPrefix}/${child.name}`);
  }

  function toListingEntries (nodeEntries: ContentTreeEntry[], urlPrefix: string): DirectoryListingEntry[] {
    const result: DirectoryListingEntry[] = [];

    for (const entry of nodeEntries) {
      if (entry.kind === 'dir') {
        result.push({
          kind: 'dir',
          sub: {
            name: unslugify(entry.node.name),
            url: getChildUrl(entry.node, urlPrefix),
            count: countDescendants(entry.node),
          },
        });
      } else if (!isIndexFile(entry.item.filepath)) {
        result.push({
          kind: 'file',
          item: toDirectoryEntry(entry.item),
        });
      }
    }

    return result;
  }

  function walk (nodeEntries: ContentTreeEntry[], urlPrefix: string) {
    for (const entry of nodeEntries) {
      if (entry.kind !== 'dir') continue;

      const node = entry.node;
      const directoryUrl = urlPrefix ? `${urlPrefix}/${node.name}` : `/${node.name}`;

      map[directoryUrl] = {
        title: unslugify(node.name),
        url: directoryUrl,
        entries: toListingEntries(node.entries, directoryUrl),
      };

      walk(node.entries, directoryUrl);
    }
  }

  // Root directory
  map['/'] = {
    title: rootTitle,
    url: '/',
    entries: toListingEntries(entries, ''),
  };

  walk(entries, '');

  return map;
}

export function getNodeIndexItem (node: ContentTreeNode): ContentSummary | undefined {
  for (const entry of node.entries) {
    if (entry.kind === 'file' && isIndexFile(entry.item.filepath)) return entry.item;
  }

  return undefined;
}

// Derive the page title for an index file from its parent directory name
export function getTdIndexTitle (filepath: string, siteTitle: string): string {
  const parent = basename(dirname(filepath));

  return parent ? unslugify(parent) : siteTitle;
}

// Resolve a display title from label or the file path
export function getTdResourceTitle (filepath: string, label?: string): string {
  if (label !== undefined) return label;

  const stem = filestem(filepath);

  // For index files, use the parent directory name
  if (isIndexFile(filepath)) {
    const parent = basename(dirname(filepath));

    return parent ? unslugify(parent) : stem;
  }

  return unslugify(stem);
}

export function isIndexFile (filepath: string): boolean {
  return filestem(filepath) === INDEX_FILENAME;
}

function entryName (entry: ContentTreeEntry): string {
  return entry.kind === 'dir' ? entry.node.name : filestem(entry.item.filepath);
}

// Return the first non-empty string value found among the given header keys
function getFirstString (header: Record<string, unknown>, ...keys: string[]): string | undefined {
  for (const key of keys) {
    const value = header[key];

    if (typeof value === 'string' && 0 < value.length) return value;
  }

  return undefined;
}

// Sort entries by numeric prefix, interleaving files and directories
function sortTree (node: ContentTreeNode) {
  node.entries.sort((left, right) => {
    const leftName = entryName(left);
    const rightName = entryName(right);
    const leftPrefix = parseNumericPrefix(leftName);
    const rightPrefix = parseNumericPrefix(rightName);

    if (leftPrefix.order !== rightPrefix.order) return leftPrefix.order - rightPrefix.order;

    return leftPrefix.rest.localeCompare(rightPrefix.rest);
  });

  for (const entry of node.entries) {
    if (entry.kind === 'dir') sortTree(entry.node);
  }
}

// Project a ContentSummary into a DirectoryEntry for the listing map
function toDirectoryEntry (item: ContentSummary): DirectoryEntry {
  const rawTags = item.header.tags;
  const tags = Array.isArray(rawTags)
    ? rawTags.filter((tag): tag is string => typeof tag === 'string')
    : [];

  return {
    name: getTdResourceTitle(item.filepath, item.label),
    url: getTdContentUrl(item.filepath),
    description: getFirstString(item.header, 'description', 'summary') ?? item.excerpt,
    tags: 0 < tags.length ? tags : undefined,
    mtime: item.metadata.mtime,
    schema: item.schema,
  };
}
