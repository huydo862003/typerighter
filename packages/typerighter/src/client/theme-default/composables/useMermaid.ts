import {
  onMounted, onUnmounted,
} from 'vue';

const CDN_URL = 'https://cdn.jsdelivr.net/npm/mermaid@11/dist/mermaid.esm.min.mjs';

let mermaidModule: any;
let idCounter = 0;

// Lazy-load mermaid from CDN and render diagram placeholders
export function useMermaid (): void {
  let observer: MutationObserver | undefined;

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
  }

  onMounted(() => {
    renderMermaidBlocks();

    // Re-render when content changes (HMR, navigation)
    const container = document.getElementById('td-content');

    if (container) {
      observer = new MutationObserver(renderMermaidBlocks);
      observer.observe(container, {
        childList: true,
        subtree: true,
      });
    }
  });

  onUnmounted(() => {
    observer?.disconnect();
  });
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
