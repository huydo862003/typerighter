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

export function buildCascadeFilteredItems (items: ContentSummary[]): ContentSummary[] {
  return items.filter((item) => item.ssg?.showInSidebar !== false);
}

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

  function toListingEntries (nodeEntries: ContentTreeEntry[], urlPrefix: string, isDirectoryHidden: boolean): DirectoryListingEntry[] {
    const result: DirectoryListingEntry[] = [];

    for (const entry of nodeEntries) {
      if (entry.kind === 'dir') {
        // Visible directories exclude hidden sub-directories
        if (!isDirectoryHidden && isDirectoryNodeHidden(entry.node)) continue;

        result.push({
          kind: 'dir',
          sub: {
            name: unslugify(entry.node.name),
            url: getChildUrl(entry.node, urlPrefix),
            count: countDescendants(entry.node),
          },
        });
      } else if (!isIndexFile(entry.item.filepath)) {
        // Visible directories exclude hidden files
        if (!isDirectoryHidden && entry.item.ssg?.showInSidebar === false) continue;

        result.push({
          kind: 'file',
          item: toDirectoryEntry(entry.item),
        });
      }
    }

    return result;
  }

  function isDirectoryNodeHidden (node: ContentTreeNode): boolean {
    const indexItem = getNodeIndexItem(node);

    return indexItem?.ssg?.showInSidebar === false;
  }

  function walk (nodeEntries: ContentTreeEntry[], urlPrefix: string) {
    for (const entry of nodeEntries) {
      if (entry.kind !== 'dir') continue;

      const node = entry.node;
      const directoryUrl = urlPrefix ? `${urlPrefix}/${node.name}` : `/${node.name}`;
      const hidden = isDirectoryNodeHidden(node);

      map[directoryUrl] = {
        title: unslugify(node.name),
        url: directoryUrl,
        entries: toListingEntries(node.entries, directoryUrl, hidden),
      };

      walk(node.entries, directoryUrl);
    }
  }

  // Root directory (always treated as visible)
  map['/'] = {
    title: rootTitle,
    url: '/',
    entries: toListingEntries(entries, '', false),
  };

  walk(entries, '');

  return map;
}

// Look up a display label for a URL path segment from the content tree
// For directories, uses the index item's label or unslugified dir name
// For files, uses the item's label or unslugified filename
export function findCrumbLabel (tree: ContentTree, urlSegments: string[], segmentIndex: number): string {
  const segment = urlSegments[segmentIndex];
  let currentEntries = tree.entries;

  // Walk to the right level in the tree
  for (let index = 0; index < segmentIndex; index++) {
    const directoryEntry = currentEntries.find(
      (entry) => entry.kind === 'dir' && entry.node.name === urlSegments[index],
    );

    if (!directoryEntry || directoryEntry.kind !== 'dir') return unslugify(segment);

    currentEntries = directoryEntry.node.entries;
  }

  // Check if segment matches a directory
  const directoryEntry = currentEntries.find(
    (entry) => entry.kind === 'dir' && entry.node.name === segment,
  );

  if (directoryEntry && directoryEntry.kind === 'dir') {
    const indexItem = getNodeIndexItem(directoryEntry.node);

    return indexItem?.label ?? unslugify(segment);
  }

  // Check if segment matches a file
  const fileEntry = currentEntries.find(
    (entry) => entry.kind === 'file' && filestem(entry.item.filepath) === segment,
  );

  if (fileEntry && fileEntry.kind === 'file') {
    return fileEntry.item.label ?? unslugify(segment);
  }

  return unslugify(segment);
}

export function flattenContentTree (entries: ContentTreeEntry[]): ContentSummary[] {
  const result: ContentSummary[] = [];

  for (const entry of entries) {
    if (entry.kind === 'file') {
      result.push(entry.item);
    } else {
      result.push(...flattenContentTree(entry.node.entries));
    }
  }

  return result;
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

    // When neither has a numeric prefix, folders sort above files
    if (leftPrefix.order === Infinity && left.kind !== right.kind) {
      return left.kind === 'dir' ? -1 : 1;
    }

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
    mtimeSecs: item.fileMetadata?.mtimeSecs,
    schema: item.schema,
    showInSidebar: item.ssg?.showInSidebar ?? true,
  };
}
