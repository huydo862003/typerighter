import {
  onMounted, onUnmounted,
} from 'vue';

const CDN_URL = 'https://esm.sh/@terrastruct/d2@0.1.32';

let d2Instance: any;
let idCounter = 0;

// Lazy-load D2 from CDN and render diagram placeholders when they scroll into view
export function useD2 (): void {
  let mutationObserver: MutationObserver | undefined;
  let intersectionObserver: IntersectionObserver | undefined;
  const observed = new WeakSet<Element>();

  function observePlaceholders () {
    const container = document.getElementById('td-content');

    if (!container || !intersectionObserver) return;

    const placeholders = container.querySelectorAll<HTMLPreElement>('.td-d2-placeholder');

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

    const d2 = await loadD2();

    if (!d2) return;

    const id = `td-d2-${idCounter++}`;
    const title = placeholder.getAttribute('data-title');

    try {
      const compiled = await d2.compile(code.textContent ?? '', {
        layout: 'dagre',
      });
      const svg = await d2.render(compiled.diagram, {
        ...compiled.renderOptions,
        themeID: document.documentElement.classList.contains('dark') ? 200 : 0,
      });

      const figure = document.createElement('figure');

      figure.className = 'td-d2';
      figure.id = id;

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
      placeholder.classList.add('td-d2-error');
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

    // Preload the D2 WASM module if any placeholders exist
    if (document.querySelector('.td-d2-placeholder')) {
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

async function loadD2 () {
  if (d2Instance) return d2Instance;

  try {
    const module = await import(/* @vite-ignore */ CDN_URL);
    const D2 = module.D2 || module.default?.D2 || module.default;

    d2Instance = new D2();
  } catch {
    return undefined;
  }

  return d2Instance;
}
