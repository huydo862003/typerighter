import {
  describe, expect, it,
} from 'vitest';
import {
  getSchemaColor,
} from './useGraphColors';

describe('getSchemaColor', () => {
  it('returns default color for undefined schema', () => {
    expect(getSchemaColor(undefined)).toBe('#6B655C');
  });

  it('returns consistent color for the same schema', () => {
    const first = getSchemaColor('Task');
    const second = getSchemaColor('Task');

    expect(first).toBe(second);
  });

  it('returns different colors for different schemas', () => {
    const task = getSchemaColor('ColorTestA');
    const person = getSchemaColor('ColorTestB');

    expect(task).not.toBe(person);
  });

  it('returns a hex color string', () => {
    const color = getSchemaColor('ColorTestC');

    expect(color).toMatch(/^#[0-9A-Fa-f]{6}$/);
  });
});
