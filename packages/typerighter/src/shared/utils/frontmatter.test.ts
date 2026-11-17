import {
  describe, it, expect,
} from 'vitest';
import {
  extractFrontmatterBuiltinFields,
} from './frontmatter';

describe('extractFrontmatterBuiltinFields', () => {
  it('returns undefined for files without frontmatter', () => {
    expect(extractFrontmatterBuiltinFields('Hello world')).toBeUndefined();
  });

  it('returns undefined for unclosed frontmatter', () => {
    expect(extractFrontmatterBuiltinFields('---\nfoo: bar\n')).toBeUndefined();
  });

  it('defaults all flags to true', () => {
    const result = extractFrontmatterBuiltinFields('---\n_type: Person\n---\n');

    expect(result).toEqual({
      label: undefined,
      showInSidebar: true,
      showInSearch: true,
      crawlable: true,
    });
  });

  it('extracts label', () => {
    const result = extractFrontmatterBuiltinFields('---\n_label: "Hello"\n---\n');

    expect(result?.label).toBe('Hello');
  });

  it('parses show_in_sidebar: false', () => {
    const source = `---
_ssg:
  show_in_sidebar: false
---
`;
    const result = extractFrontmatterBuiltinFields(source);

    expect(result?.showInSidebar).toBe(false);
    expect(result?.showInSearch).toBe(true);
    expect(result?.crawlable).toBe(true);
  });

  it('parses show_in_search: false', () => {
    const source = `---
_ssg:
  show_in_search: false
---
`;
    const result = extractFrontmatterBuiltinFields(source);

    expect(result?.showInSidebar).toBe(true);
    expect(result?.showInSearch).toBe(false);
    expect(result?.crawlable).toBe(true);
  });

  it('parses crawlable: false', () => {
    const source = `---
_ssg:
  crawlable: false
---
`;
    const result = extractFrontmatterBuiltinFields(source);

    expect(result?.showInSidebar).toBe(true);
    expect(result?.showInSearch).toBe(true);
    expect(result?.crawlable).toBe(false);
  });

  it('parses multiple _ssg fields together', () => {
    const source = `---
_type: Person
_ssg:
  show_in_sidebar: false
  show_in_search: false
  crawlable: false
name: "Alice"
---
`;
    const result = extractFrontmatterBuiltinFields(source);

    expect(result?.showInSidebar).toBe(false);
    expect(result?.showInSearch).toBe(false);
    expect(result?.crawlable).toBe(false);
  });

  it('handles _ssg fields mixed with other frontmatter', () => {
    const source = `---
_type: Person
_label: "Test Page"
_ssg:
  show_in_search: false
name: "Alice"
---
`;
    const result = extractFrontmatterBuiltinFields(source);

    expect(result?.label).toBe('Test Page');
    expect(result?.showInSearch).toBe(false);
    expect(result?.showInSidebar).toBe(true);
    expect(result?.crawlable).toBe(true);
  });

  it('ignores non-_ssg indented lines', () => {
    const source = `---
_meta:
  title: "My Title"
_ssg:
  crawlable: false
---
`;
    const result = extractFrontmatterBuiltinFields(source);

    expect(result?.crawlable).toBe(false);
    expect(result?.showInSidebar).toBe(true);
  });
});
