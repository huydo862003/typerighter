import {
  describe, expect, it,
} from 'vitest';
import {
  toPascalCase,
} from './format';

describe('toPascalCase', () => {
  it('converts simple lowercase', () => {
    expect(toPascalCase('chart')).toBe('Chart');
  });

  it('converts kebab-case', () => {
    expect(toPascalCase('video-embed')).toBe('VideoEmbed');
  });

  it('converts multi-segment kebab', () => {
    expect(toPascalCase('my-custom-chart')).toBe('MyCustomChart');
  });

  it('preserves already PascalCase', () => {
    expect(toPascalCase('MyChart')).toBe('MyChart');
  });

  it('converts snake_case', () => {
    expect(toPascalCase('my_chart')).toBe('MyChart');
  });
});
