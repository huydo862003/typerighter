// Deterministic pill colors based on string value
const PALETTE = [
  {
    fg: '#a0522d',
    bg: 'rgba(160, 82, 45, 0.1)',
  },   // sienna
  {
    fg: '#4F6BCA',
    bg: 'rgba(79, 107, 202, 0.1)',
  },   // blue
  {
    fg: '#10b981',
    bg: 'rgba(16, 185, 129, 0.1)',
  },    // emerald
  {
    fg: '#8b5cf6',
    bg: 'rgba(139, 92, 246, 0.1)',
  },    // violet
  {
    fg: '#ec4899',
    bg: 'rgba(236, 72, 153, 0.1)',
  },    // pink
  {
    fg: '#f59e0b',
    bg: 'rgba(245, 158, 11, 0.1)',
  },    // amber
  {
    fg: '#06b6d4',
    bg: 'rgba(6, 182, 212, 0.1)',
  },     // cyan
  {
    fg: '#ef4444',
    bg: 'rgba(239, 68, 68, 0.1)',
  },     // red
];

export function getPillColor (value: unknown): {
  color: string;
  background: string;
} {
  const string_ = String(value ?? '');
  const entry = PALETTE[hash(string_) % PALETTE.length];

  return {
    color: entry.fg,
    background: entry.bg,
  };
}

function hash (value: string): number {
  let result = 0;

  for (let index = 0; index < value.length; index++) {
    result = ((result << 5) - result + value.charCodeAt(index)) | 0;
  }

  return Math.abs(result);
}
