import {
  describe, expect, it,
} from 'vitest';
import {
  resolveFrefPath,
} from './path';

describe('resolveFrefPath', () => {
  it('resolves absolute path from root', () => {
    expect(resolveFrefPath('/flashcards', 'notes/chapter-1')).toBe('flashcards');
  });

  it('resolves relative ./ path', () => {
    expect(resolveFrefPath('./assets/photo.png', 'notes/chapter-1')).toBe('notes/chapter-1/assets/photo.png');
  });

  it('resolves ../ path', () => {
    expect(resolveFrefPath('../images/logo.png', 'notes/chapter-1')).toBe('notes/images/logo.png');
  });

  it('resolves bare path from root', () => {
    expect(resolveFrefPath('flashcards', 'notes/chapter-1')).toBe('notes/chapter-1/flashcards');
  });

  it('handles multiple ../ segments', () => {
    expect(resolveFrefPath('../../other', 'a/b/c')).toBe('a/other');
  });

  it('handles empty base directory', () => {
    expect(resolveFrefPath('./sub', '')).toBe('sub');
  });

  it('strips leading slash for absolute paths', () => {
    expect(resolveFrefPath('/a/b/c', 'x/y')).toBe('a/b/c');
  });
});
