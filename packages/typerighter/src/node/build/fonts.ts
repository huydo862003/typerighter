import path from 'node:path';

// @fontsource variable fonts follow this naming convention:
// <family>-<subset>-wght-normal-<hash>.woff2
// The Latin subset (not Latin Extended) is critical for most content; others load on demand
// This works for any @fontsource font added in the future without changes here
export function isCriticalFontFile (filePath: string): boolean {
  const basename = path.basename(filePath);
  return (
    basename.endsWith('.woff2') &&
    basename.includes('-latin-') &&
    !basename.includes('-latin-ext-')
  );
}
