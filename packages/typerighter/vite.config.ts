import {
  createRequire,
} from 'node:module';
import path from 'node:path';
import tailwindcss from '@tailwindcss/vite';
import vue from '@vitejs/plugin-vue';
import {
  defineConfig,
} from 'vitest/config';

const package_ = createRequire(import.meta.url)('./package.json');

export default defineConfig({
  define: {
    __VERSION__: JSON.stringify(package_.version),
  },
  plugins: [
    vue(),
    tailwindcss(),
  ],
  build: {
    lib: {
      entry: {
        index: path.resolve(__dirname, 'src/index.ts'),
        vite: path.resolve(__dirname, 'src/node/plugin/index.ts'),
        client: path.resolve(__dirname, 'src/client/index.ts'),
        cli: path.resolve(__dirname, 'src/node/cli/index.ts'),
        shared: path.resolve(__dirname, 'src/shared/index.ts'),
        'client/theme-default': path.resolve(__dirname, 'src/client/theme-default/index.ts'),
      },
      formats: ['es'],
    },
    rollupOptions: {
      external: [
        'vite',
        'vue',
        '@vitejs/plugin-vue',
        '@typerighter/rpc-client',
        '@typerighter/rpc-server',
        'markdown-it',
        'markdown-it-anchor',
        'markdown-it-container',
        'markdown-it-emoji',
        'markdown-it-task-lists',
        'shiki',
        '@shikijs/markdown-it',
        'tailwindcss',
        '@tailwindcss/vite',
        '@vueuse/core',
        '@vueuse/shared',
        'picocolors',
        /^node:/,
      ],
    },
  },
  resolve: {
    alias: {
      '@': path.resolve(__dirname, './src/'),
    },
  },
  test: {},
});
