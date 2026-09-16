import {
  onMounted, watch,
} from 'vue';
import {
  useRoute,
} from '../../app';

const CDN_URL = 'https://cdn.jsdelivr.net/npm/mermaid@11/dist/mermaid.esm.min.mjs';

// eslint-disable-next-line @typescript-eslint/consistent-type-imports
let mermaidModule: typeof import('mermaid') | undefined;
let idCounter = 0;

// Lazy-load mermaid from CDN and render diagram placeholders
export function useMermaid (): void {
  const route = useRoute();

  async function renderMermaidBlocks () {
    const container = document.getElementById('td-content');

    if (!container) return;

    const placeholders = container.querySelectorAll<HTMLPreElement>('.td-mermaid-placeholder');

    if (placeholders.length === 0) return;

    const module_ = await loadMermaid();

    if (module_ === undefined) return;

    for (const placeholder of placeholders) {
      const code = placeholder.querySelector('code');

      if (!code) continue;

      const id = `td-mermaid-${idCounter++}`;

      try {
        const {
          svg,
        } = await module_.default.render(id, code.textContent ?? '');
        const wrapper = document.createElement('div');

        wrapper.className = 'td-mermaid';
        wrapper.innerHTML = svg;
        placeholder.replaceWith(wrapper);
      } catch {
        placeholder.classList.add('td-mermaid-error');
      }
    }
  }

  onMounted(renderMermaidBlocks);
  watch(() => route.path, renderMermaidBlocks);
}

async function loadMermaid () {
  if (mermaidModule !== undefined) return mermaidModule;

  try {
    mermaidModule = await import(/* @vite-ignore */ CDN_URL);
  } catch {
    return undefined;
  }

  mermaidModule.default.initialize({
    startOnLoad: false,
    theme: document.documentElement.classList.contains('dark') ? 'dark' : 'default',
  });

  return mermaidModule;
}
