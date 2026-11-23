import {
  describe, it, expect,
} from 'vitest';
import type {
  ContentSummary,
} from '../types/content';
import {
  buildCascadeFilteredItems, buildContentTree, buildDirectoryListingMap, findCrumbLabel,
} from './content';

function item (filepath: string, ssg?: Partial<ContentSummary['ssg']>): ContentSummary {
  return {
    filepath,
    header: {},
    ssg: {
      showInSidebar: ssg?.showInSidebar ?? true,
      showInSearch: ssg?.showInSearch ?? true,
      crawlable: ssg?.crawlable ?? true,
    },
  };
}

describe('buildCascadeFilteredItems', () => {
  it('keeps all items when none are hidden', () => {
    const items = [
      item('people/index.td'),
      item('people/alice.td'),
      item('notes/note.td'),
    ];
    const result = buildCascadeFilteredItems(items);

    expect(result.map((item) => item.filepath)).toEqual([
      'people/index.td',
      'people/alice.td',
      'notes/note.td',
    ]);
  });

  it('excludes individually hidden items', () => {
    const items = [
      item('people/index.td'),
      item('people/alice.td', {
        showInSidebar: false,
      }),
    ];
    const result = buildCascadeFilteredItems(items);

    expect(result.map((item) => item.filepath)).toEqual(['people/index.td']);
  });

  it('filters items with showInSidebar false (cascade is applied by Rust RPC layer)', () => {
    // After Rust cascade, children of hidden index already have showInSidebar: false
    const items = [
      item('root.td'),
      item('people/index.td', {
        showInSidebar: false,
      }),
      item('people/alice.td', {
        showInSidebar: false,
      }),
      item('people/bob.td', {
        showInSidebar: false,
      }),
    ];
    const result = buildCascadeFilteredItems(items);

    expect(result.map((item) => item.filepath)).toEqual(['root.td']);
  });
});

describe('buildDirectoryListingMap - visibility filtering', () => {
  it('visible directory hides entries with showInSidebar false', () => {
    const items = [
      item('people/index.td'),
      item('people/alice.td'),
      item('people/secret.td', {
        showInSidebar: false,
      }),
    ];
    const tree = buildContentTree(items);
    const map = buildDirectoryListingMap(tree.entries, 'Site');
    const entries = map['/people']?.entries ?? [];
    const names = entries.map((entry) => entry.kind === 'file' ? entry.item.name : entry.sub.name);

    expect(names.some((name) => name.toLowerCase().includes('alice'))).toBe(true);
    expect(names.some((name) => name.toLowerCase().includes('secret'))).toBe(false);
  });

  it('hidden directory shows all entries including hidden ones', () => {
    const items = [
      item('people/index.td', {
        showInSidebar: false,
      }),
      item('people/alice.td'),
      item('people/secret.td', {
        showInSidebar: false,
      }),
    ];
    const tree = buildContentTree(items);
    const map = buildDirectoryListingMap(tree.entries, 'Site');
    const entries = map['/people']?.entries ?? [];
    const names = entries.map((entry) => entry.kind === 'file' ? entry.item.name : entry.sub.name);

    expect(names.some((name) => name.toLowerCase().includes('alice'))).toBe(true);
    expect(names.some((name) => name.toLowerCase().includes('secret'))).toBe(true);
  });

  it('directory with no index treats itself as visible', () => {
    const items = [
      item('people/alice.td'),
      item('people/secret.td', {
        showInSidebar: false,
      }),
    ];
    const tree = buildContentTree(items);
    const map = buildDirectoryListingMap(tree.entries, 'Site');
    const entries = map['/people']?.entries ?? [];
    const names = entries.map((entry) => entry.kind === 'file' ? entry.item.name : entry.sub.name);

    expect(names.some((name) => name.toLowerCase().includes('alice'))).toBe(true);
    expect(names.some((name) => name.toLowerCase().includes('secret'))).toBe(false);
  });

  it('visible root excludes hidden sub-directories', () => {
    const items = [
      item('public/index.td'),
      item('public/page.td'),
      item('archive/index.td', {
        showInSidebar: false,
      }),
      item('archive/old.td'),
    ];
    const tree = buildContentTree(items);
    const map = buildDirectoryListingMap(tree.entries, 'Site');
    const rootEntries = map['/']?.entries ?? [];
    const rootNames = rootEntries.map((entry) => entry.kind === 'file' ? entry.item.name : entry.sub.name);

    expect(rootNames.some((name) => name.toLowerCase().includes('public'))).toBe(true);
    expect(rootNames.some((name) => name.toLowerCase().includes('archive'))).toBe(false);
  });

  it('hidden directory listing includes all its children', () => {
    const items = [
      item('archive/index.td', {
        showInSidebar: false,
      }),
      item('archive/old-a.td'),
      item('archive/old-b.td', {
        showInSidebar: false,
      }),
    ];
    const tree = buildContentTree(items);
    const map = buildDirectoryListingMap(tree.entries, 'Site');
    const archiveEntries = map['/archive']?.entries ?? [];
    const names = archiveEntries.map((entry) => entry.kind === 'file' ? entry.item.name : entry.sub.name);

    expect(names.some((name) => name.toLowerCase().includes('old a'))).toBe(true);
    expect(names.some((name) => name.toLowerCase().includes('old b'))).toBe(true);
  });
});

function labeledItem (filepath: string, label: string): ContentSummary {
  return {
    filepath,
    label,
    header: {},
    ssg: {
      showInSidebar: true,
      showInSearch: true,
      crawlable: true,
    },
  };
}

describe('findCrumbLabel', () => {
  it('returns file label for leaf page', () => {
    const tree = buildContentTree([labeledItem('people/alice.td', 'Alice (Dev)')]);

    expect(findCrumbLabel(tree, [
      'people',
      'alice',
    ], 1)).toBe('Alice (Dev)');
  });

  it('returns directory index label for folder crumb', () => {
    const tree = buildContentTree([
      labeledItem('people/index.td', 'Team Members'),
      labeledItem('people/alice.td', 'Alice'),
    ]);

    expect(findCrumbLabel(tree, [
      'people',
      'alice',
    ], 0)).toBe('Team Members');
  });

  it('falls back to unslugify when no label', () => {
    const tree = buildContentTree([item('people/alice.td')]);

    expect(findCrumbLabel(tree, [
      'people',
      'alice',
    ], 1)).toBe('Alice');
  });

  it('falls back to unslugify for directory without index', () => {
    const tree = buildContentTree([item('people/alice.td')]);

    expect(findCrumbLabel(tree, [
      'people',
      'alice',
    ], 0)).toBe('People');
  });

  it('handles deeply nested paths', () => {
    const tree = buildContentTree([labeledItem('a/b/c/page.td', 'Deep Page')]);

    expect(findCrumbLabel(tree, [
      'a',
      'b',
      'c',
      'page',
    ], 3)).toBe('Deep Page');
    expect(findCrumbLabel(tree, [
      'a',
      'b',
      'c',
      'page',
    ], 0)).toBe('A');
  });

  it('returns unslugify for nonexistent segment', () => {
    const tree = buildContentTree([item('people/alice.td')]);

    expect(findCrumbLabel(tree, [
      'missing',
      'page',
    ], 0)).toBe('Missing');
  });
});
