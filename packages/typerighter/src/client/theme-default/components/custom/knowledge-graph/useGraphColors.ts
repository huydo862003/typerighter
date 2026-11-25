const PALETTE = [
  '#B5461F',
  '#2457A6',
  '#2E7D32',
  '#8A5A00',
  '#7B1FA2',
  '#C62828',
  '#00695C',
  '#AD1457',
  '#1565C0',
  '#E65100',
];

const DEFAULT_COLOR = '#6B655C';

const assigned: Record<string, string> = {};
let nextIndex = 0;

export function getSchemaColor (schema: string | undefined): string {
  const key = schema ?? 'default';

  if (!assigned[key]) {
    if (key === 'default') {
      assigned[key] = DEFAULT_COLOR;
    } else {
      assigned[key] = PALETTE[nextIndex++ % PALETTE.length];
    }
  }

  return assigned[key];
}
