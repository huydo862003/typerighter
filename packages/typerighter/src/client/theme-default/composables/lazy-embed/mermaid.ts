import {
  inject, onMounted, onUnmounted,
} from 'vue';
import {
  LAZY_EMBED_KEY,
} from './types';

const CDN_URL =
  'https://cdn.jsdelivr.net/npm/mermaid@11/dist/mermaid.esm.min.mjs';

let mermaidModule: any;
let idCounter = 0;

export function useMermaid (): void {
  const context = inject(LAZY_EMBED_KEY);

  if (!context) return;

  const options = {
    selector: '.td-mermaid-placeholder',
    onRender: renderPlaceholder,
    onVisible () {
      const link = document.createElement('link');

      link.rel = 'modulepreload';
      link.href = CDN_URL;
      document.head.appendChild(link);
    },
  };

  onMounted(() => context.register(options));
  onUnmounted(() => context.unregister(options));
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
    theme: document.documentElement.classList.contains('dark')
      ? 'dark'
      : 'default',
  });

  return mermaidModule;
}

async function renderPlaceholder (element: Element) {
  const placeholder = element as HTMLPreElement;
  const code = placeholder.querySelector('code');

  if (!code) return;

  const module_ = await loadMermaid();

  if (module_ === undefined) return;

  const id = `td-mermaid-${idCounter++}`;
  const title = placeholder.getAttribute('data-title');

  try {
    const {
      svg,
    } = await module_.default.render(id, code.textContent ?? '');

    const figure = document.createElement('figure');

    figure.className = 'td-mermaid';

    const svgWrapper = document.createElement('div');

    svgWrapper.innerHTML = svg;
    figure.appendChild(svgWrapper);

    if (title) {
      const caption = document.createElement('figcaption');

      caption.textContent = title;
      figure.appendChild(caption);
    }

    placeholder.replaceWith(figure);
  } catch {
    placeholder.classList.add('td-mermaid-error');
  }
}
