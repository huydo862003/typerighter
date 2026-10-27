import path from 'node:path';
import {
  snakeToKebab,
} from '@/shared';

const DEFAULT_THEME = 'typerighter/client/theme-default';

// Generate the theme module source code
export function generateThemeModule (
  layout:
    | {
      component: string | undefined;
      slots: Record<string, string>;
      notFound: string | undefined;
    }
    | undefined,
  projectRoot?: string,
): string {
  const notFoundImport = layout?.notFound
    ? `import CustomNotFound from ${JSON.stringify(projectRoot ? path.resolve(projectRoot, layout.notFound) : layout.notFound)};`
    : '';
  const notFoundExport = layout?.notFound ? ', NotFound: CustomNotFound' : '';

  if (layout?.component) {
    const layoutPath = projectRoot
      ? path.resolve(projectRoot, layout.component)
      : layout.component;

    return [
      `import CustomLayout from ${JSON.stringify(layoutPath)};`,
      notFoundImport,
      `export default { Layout: CustomLayout${notFoundExport} };`,
    ]
      .filter(Boolean)
      .join('\n');
  }

  const slotEntries = Object.entries(layout?.slots ?? {});

  if (0 < slotEntries.length) {
    const rendered = slotEntries
      .map(
        ([
          name,
          component,
        ]) =>
          `${JSON.stringify(snakeToKebab(name))}: () => h(resolveComponent(${JSON.stringify(component)}))`,
      )
      .join(', ');

    return [
      `import DefaultTheme from ${JSON.stringify(DEFAULT_THEME)};`,
      `import { h, resolveComponent } from 'vue';`,
      notFoundImport,
      `export default { Layout: { render() { return h(DefaultTheme.Layout, null, { ${rendered} }); } }${notFoundExport} };`,
    ]
      .filter(Boolean)
      .join('\n');
  }

  if (notFoundImport) {
    return [
      `import DefaultTheme from ${JSON.stringify(DEFAULT_THEME)};`,
      notFoundImport,
      `export default { ...DefaultTheme${notFoundExport} };`,
    ].join('\n');
  }

  return `export { default } from ${JSON.stringify(DEFAULT_THEME)};`;
}
