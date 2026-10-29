import {
  onMounted, onUnmounted,
} from 'vue';

const CDN_URL = 'https://cdn.jsdelivr.net/npm/mermaid@11/dist/mermaid.esm.min.mjs';

let mermaidModule: any;
let idCounter = 0;

// Lazy-load mermaid from CDN and render diagram placeholders when they scroll into view
export function useMermaid (): void {
  let mutationObserver: MutationObserver | undefined;
  let intersectionObserver: IntersectionObserver | undefined;
  const observed = new WeakSet<Element>();

  function observePlaceholders () {
    const container = document.getElementById('td-content');

    if (!container || !intersectionObserver) return;

    const placeholders = container.querySelectorAll<HTMLPreElement>('.td-mermaid-placeholder');

    for (const placeholder of placeholders) {
      if (!observed.has(placeholder)) {
        observed.add(placeholder);
        intersectionObserver.observe(placeholder);
      }
    }
  }

  async function renderPlaceholder (placeholder: HTMLPreElement) {
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

  onMounted(() => {
    intersectionObserver = new IntersectionObserver((entries) => {
      for (const entry of entries) {
        if (entry.isIntersecting) {
          intersectionObserver?.unobserve(entry.target);
          renderPlaceholder(entry.target as HTMLPreElement);
        }
      }
    }, {
      rootMargin: '200px',
    });

    observePlaceholders();

    // Preload mermaid module if any placeholders exist
    if (document.querySelector('.td-mermaid-placeholder')) {
      const link = document.createElement('link');

      link.rel = 'modulepreload';
      link.href = CDN_URL;
      document.head.appendChild(link);
    }

    const container = document.getElementById('td-content');

    if (container) {
      mutationObserver = new MutationObserver(observePlaceholders);
      mutationObserver.observe(container, {
        childList: true,
        subtree: true,
      });
    }
  });

  onUnmounted(() => {
    mutationObserver?.disconnect();
    intersectionObserver?.disconnect();
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
