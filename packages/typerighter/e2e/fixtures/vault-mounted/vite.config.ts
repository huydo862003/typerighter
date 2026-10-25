import {
  typedown,
} from 'typerighter/vite';
import {
  defineConfig,
} from 'vite';

// Simulates embedding typedown inside a host Vite app:
// the typedown vault lives under docs/ instead of at the project root
export default defineConfig({
  plugins: [
    ...typedown({
      root: 'docs',
    }),
  ],
});
