import {
  onMounted, onUnmounted,
} from 'vue';
import type {
  LazyEmbedContext,
} from './types';
import {
  mountDiagram,
} from './mountDiagram';

const CDN_URL = 'https://esm.sh/@terrastruct/d2@0.1.32';

let d2Instance: any;
let idCounter = 0;

export function useD2 (context: LazyEmbedContext): void {
  const options = {
    selector: '.td-d2-placeholder',
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

async function renderPlaceholder (element: Element) {
  const placeholder = element as HTMLPreElement;
  const code = placeholder.querySelector('code');

  if (!code) return;

  const source = code.textContent ?? '';
  const d2 = await loadD2();

  if (!d2) return;

  const id = `td-d2-${idCounter++}`;
  const title = placeholder.getAttribute('data-title');

  try {
    const compiled = await d2.compile(source, {
      layout: 'dagre',
    });
    const svg = await d2.render(compiled.diagram, {
      ...compiled.renderOptions,
      themeID: document.documentElement.classList.contains('dark') ? 200 : 0,
    });

    mountDiagram(placeholder, {
      svg,
      source,
      title: title ?? undefined,
      label: 'D2',
      diagramId: id,
    });
  } catch {
    placeholder.classList.add('td-d2-error');
  }
}
