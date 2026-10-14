import {
  describe, it, expect,
} from 'vitest';
import {
  generateThemeModule,
} from './theme';

describe('generateThemeModule', () => {
  it('re-exports default theme when no layout', () => {
    const result = generateThemeModule(undefined);

    expect(result).toContain('export { default }');
    expect(result).toContain('typerighter/client/theme-default');
  });

  it('imports custom layout', () => {
    const result = generateThemeModule({
      component: './Layout.vue',
      slots: {},
      notFound: undefined,
    });

    expect(result).toContain('import CustomLayout from');
    expect(result).toContain('Layout: CustomLayout');
    expect(result).not.toContain('NotFound');
  });

  it('includes NotFound when configured', () => {
    const result = generateThemeModule({
      component: './Layout.vue',
      slots: {},
      notFound: './NotFound.vue',
    });

    expect(result).toContain('import CustomLayout from');
    expect(result).toContain('import CustomNotFound from');
    expect(result).toContain('Layout: CustomLayout');
    expect(result).toContain('NotFound: CustomNotFound');
  });

  it('includes NotFound with slots', () => {
    const result = generateThemeModule({
      component: undefined,
      slots: {
        doc_after: 'MyWidget',
      },
      notFound: './NotFound.vue',
    });

    expect(result).toContain('import CustomNotFound from');
    expect(result).toContain('NotFound: CustomNotFound');
    expect(result).toContain('resolveComponent');
  });

  it('includes NotFound with default theme', () => {
    const result = generateThemeModule({
      component: undefined,
      slots: {},
      notFound: './NotFound.vue',
    });

    expect(result).toContain('import CustomNotFound from');
    expect(result).toContain('...DefaultTheme');
    expect(result).toContain('NotFound: CustomNotFound');
  });

  it('resolves paths relative to projectRoot', () => {
    const result = generateThemeModule({
      component: './theme/Layout.vue',
      slots: {},
      notFound: './theme/NotFound.vue',
    }, '/my/project');

    expect(result).toContain('/my/project/theme/Layout.vue');
    expect(result).toContain('/my/project/theme/NotFound.vue');
  });
});
