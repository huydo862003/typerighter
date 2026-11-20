import {
  describe, it, expect,
} from 'vitest';
import type {
  ContentSummary,
} from '../types/content';
import {
  buildCascadeFilteredItems, buildContentTree, buildDirectoryListingMap,
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
});
