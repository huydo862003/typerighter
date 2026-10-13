import path from 'node:path';
import {
  snakeToKebab,
} from '@/shared';

const DEFAULT_THEME = 'typerighter/client/theme-default';

// Generate the theme module source code
// Used by VirtualTheme (dev) and the SSG build (temp file)
export function generateThemeModule (
  layout: {
    component: string | undefined;
    slots: Record<string, string>;
  } | undefined,
  projectRoot?: string,
): string {
  if (layout?.component) {
    const layoutPath = projectRoot ? path.resolve(projectRoot, layout.component) : layout.component;

    return [
      `import CustomLayout from ${JSON.stringify(layoutPath)};`,
      'export default { Layout: CustomLayout };',
    ].join('\n');
  }

  const slotEntries = Object.entries(layout?.slots ?? {});

  if (0 < slotEntries.length) {
    const rendered = slotEntries
      .map(([
        name,
        component,
      ]) => `${JSON.stringify(snakeToKebab(name))}: () => h(resolveComponent(${JSON.stringify(component)}))`)
      .join(', ');

    return [
      `import DefaultTheme from ${JSON.stringify(DEFAULT_THEME)};`,
      'import { h, resolveComponent } from \'vue\';',
      `export default { Layout: { render() { return h(DefaultTheme.Layout, null, { ${rendered} }); } } };`,
    ].join('\n');
  }

  return `export { default } from ${JSON.stringify(DEFAULT_THEME)};`;
}
