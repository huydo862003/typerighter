import {
  describe, it, expect,
} from 'vitest';
import type {
  ContentSummary,
} from '../types/content';
import {
  buildCascadeFilteredItems, buildContentTree, buildDirectoryListingMap,
} from './content';

function item (filepath: string, showInSidebar?: boolean): ContentSummary {
  return {
    filepath,
    header: {},
    ssg: {
      showInSidebar: showInSidebar ?? true,
      showInSearch: true,
      crawlable: true,
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
      item('people/alice.td', false),
    ];
    const result = buildCascadeFilteredItems(items);

    expect(result.map((item) => item.filepath)).toEqual(['people/index.td']);
  });

  it('cascade-hides children when parent index is hidden', () => {
    const items = [
      item('root.td'),
      item('people/index.td', false),
      item('people/alice.td'),
      item('people/bob.td'),
    ];
    const result = buildCascadeFilteredItems(items);

    expect(result.map((item) => item.filepath)).toEqual(['root.td']);
  });

  it('cascade does not affect other directories', () => {
    const items = [
      item('people/index.td', false),
      item('people/alice.td'),
      item('notes/note.td'),
    ];
    const result = buildCascadeFilteredItems(items);

    expect(result.map((item) => item.filepath)).toEqual(['notes/note.td']);
  });

  it('cascade hides nested subdirectories too', () => {
    const items = [
      item('people/index.td', false),
      item('people/alice.td'),
      item('people/authors/hemingway.td'),
    ];
    const result = buildCascadeFilteredItems(items);

    expect(result).toHaveLength(0);
  });
});

describe('buildDirectoryListingMap - visibility filtering', () => {
  it('visible directory hides entries with showInSidebar false', () => {
    const items = [
      item('people/index.td', true),
      item('people/alice.td', true),
      item('people/secret.td', false),
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
      item('people/index.td', false),
      item('people/alice.td', true),
      item('people/secret.td', false),
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
      item('people/alice.td', true),
      item('people/secret.td', false),
    ];
    const tree = buildContentTree(items);
    const map = buildDirectoryListingMap(tree.entries, 'Site');
    const entries = map['/people']?.entries ?? [];
    const names = entries.map((entry) => entry.kind === 'file' ? entry.item.name : entry.sub.name);

    expect(names.some((name) => name.toLowerCase().includes('alice'))).toBe(true);
    expect(names.some((name) => name.toLowerCase().includes('secret'))).toBe(false);
  });
});
