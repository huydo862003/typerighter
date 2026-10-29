import {
  onMounted, onUnmounted,
} from 'vue';

const CDN_URL = 'https://esm.sh/@terrastruct/d2';

let d2Instance: any;
let idCounter = 0;

// Lazy-load D2 from CDN and render diagram placeholders
export function useD2 (): void {
  let observer: MutationObserver | undefined;

  async function renderD2Blocks () {
    const container = document.getElementById('td-content');

    if (!container) return;

    const placeholders = container.querySelectorAll<HTMLPreElement>('.td-d2-placeholder');

    if (placeholders.length === 0) return;

    const d2 = await loadD2();

    if (!d2) return;

    for (const placeholder of placeholders) {
      const code = placeholder.querySelector('code');

      if (!code) continue;

      const id = `td-d2-${idCounter++}`;
      const title = placeholder.getAttribute('data-title');

      try {
        const compiled = await d2.compile(code.textContent ?? '', {
          layout: 'dagre',
        });
        const rendered = await d2.render(compiled.diagram, {
          ...compiled.renderOptions,
          themeID: document.documentElement.classList.contains('dark') ? 200 : 0,
        });
        const svgString = typeof rendered === 'string' ? rendered : (rendered?.svg ?? '');

        const figure = document.createElement('figure');

        figure.className = 'td-d2';
        figure.id = id;

        const svgWrapper = document.createElement('div');

        svgWrapper.innerHTML = svgString;
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
  }

  onMounted(() => {
    renderD2Blocks();

    // Re-render when content changes (HMR, navigation)
    const container = document.getElementById('td-content');

    if (container) {
      observer = new MutationObserver(renderD2Blocks);
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
