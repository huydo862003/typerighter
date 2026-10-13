import {
  describe, expect, it,
} from 'vitest';
import {
  kebabToPascal, snakeToKebab,
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
