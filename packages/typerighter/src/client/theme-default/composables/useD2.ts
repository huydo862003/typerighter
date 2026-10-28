import {
  onMounted, onUnmounted,
} from 'vue';

const CDN_URL = 'https://esm.sh/@terrastruct/d2';

let d2Module: any;
let idCounter = 0;

// Lazy-load D2 from CDN and render diagram placeholders
export function useD2 (): void {
  let observer: MutationObserver | undefined;

  async function renderD2Blocks () {
    const container = document.getElementById('td-content');

    if (!container) return;

    const placeholders = container.querySelectorAll<HTMLPreElement>('.td-d2-placeholder');

    if (placeholders.length === 0) return;

    const module_ = await loadD2();

    if (module_ === undefined) return;

    for (const placeholder of placeholders) {
      const code = placeholder.querySelector('code');

      if (!code) continue;

      const id = `td-d2-${idCounter++}`;

      try {
        const result = await module_.compile(code.textContent ?? '', {
          themeId: document.documentElement.classList.contains('dark') ? 200 : 0,
        });
        const wrapper = document.createElement('div');

        wrapper.className = 'td-d2';
        wrapper.id = id;
        wrapper.innerHTML = result.svg;
        placeholder.replaceWith(wrapper);
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
  if (d2Module !== undefined) return d2Module;

  try {
    d2Module = await import(/* @vite-ignore */ CDN_URL);
  } catch {
    return undefined;
  }

  return d2Module;
}
