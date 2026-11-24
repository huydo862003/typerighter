import {
  describe, expect, it,
} from 'vitest';
import {
  kebabToPascal, parseNumericPrefix, snakeToKebab, unslugify,
} from './format';

describe('kebabToPascal', () => {
  it('converts simple lowercase', () => {
    expect(kebabToPascal('chart')).toBe('Chart');
  });

  it('converts kebab-case', () => {
    expect(kebabToPascal('video-embed')).toBe('VideoEmbed');
  });

  it('converts multi-segment kebab', () => {
    expect(kebabToPascal('my-custom-chart')).toBe('MyCustomChart');
  });

  it('preserves already PascalCase', () => {
    expect(kebabToPascal('MyChart')).toBe('MyChart');
  });

  it('converts snake_case', () => {
    expect(kebabToPascal('my_chart')).toBe('MyChart');
  });
});

describe('snakeToKebab', () => {
  it('converts underscores to hyphens', () => {
    expect(snakeToKebab('layout_top')).toBe('layout-top');
  });

  it('converts multi-segment', () => {
    expect(snakeToKebab('doc_content_after')).toBe('doc-content-after');
  });

  it('preserves already kebab-case', () => {
    expect(snakeToKebab('layout-top')).toBe('layout-top');
  });

  it('handles no underscores', () => {
    expect(snakeToKebab('sidebar')).toBe('sidebar');
  });
});

describe('parseNumericPrefix', () => {
  it('extracts numeric prefix and rest', () => {
    expect(parseNumericPrefix('01-foundations')).toEqual({
      order: 1,
      rest: 'foundations',
    });
  });

  it('handles multi-digit prefix', () => {
    expect(parseNumericPrefix('10-advanced')).toEqual({
      order: 10,
      rest: 'advanced',
    });
  });

  it('returns Infinity for no prefix', () => {
    expect(parseNumericPrefix('review')).toEqual({
      order: Infinity,
      rest: 'review',
    });
  });

  it('handles underscore separator', () => {
    expect(parseNumericPrefix('03_proofs')).toEqual({
      order: 3,
      rest: 'proofs',
    });
  });

  it('preserves rest with hyphens', () => {
    expect(parseNumericPrefix('02-type-theory')).toEqual({
      order: 2,
      rest: 'type-theory',
    });
  });
});

describe('unslugify', () => {
  it('strips numeric prefix and capitalizes', () => {
    expect(unslugify('01-foundations')).toBe('Foundations');
  });

  it('replaces hyphens with spaces', () => {
    expect(unslugify('02-type-theory')).toBe('Type theory');
  });

  it('handles no prefix', () => {
    expect(unslugify('review')).toBe('Review');
  });

  it('handles underscores', () => {
    expect(unslugify('my_deck')).toBe('My deck');
  });
});
