import {
  inject, onMounted, onUnmounted,
} from 'vue';
import {
  LAZY_EMBED_KEY,
} from './types';

const EMGITHUB_SCRIPT = 'https://emgithub.com/embed-v2.js';

export function useGithubEmbed (): void {
  const context = inject(LAZY_EMBED_KEY);

  if (!context) return;

  let themeObserver: MutationObserver | undefined;
  const rendered: Array<{
    container: HTMLElement;
    url: string;
  }> = [];

  const options = {
    selector: '.td-github-placeholder',
    onRender (element: Element) {
      const placeholder = element as HTMLDivElement;
      const url = placeholder.getAttribute('data-url');

      if (!url) return;

      const isDark = document.documentElement.classList.contains('dark');
      const script = document.createElement('script');

      script.src = buildScriptUrl(url, isDark);

      const wrapper = document.createElement('div');

      wrapper.className = 'td-github-wrapper';
      wrapper.dataset.url = url;
      wrapper.appendChild(script);

      placeholder.replaceWith(wrapper);
      rendered.push({
        container: wrapper,
        url,
      });
    },
  };

  function onThemeChange () {
    const isDark = document.documentElement.classList.contains('dark');

    for (let index = rendered.length - 1; 0 <= index; index--) {
      if (!rendered[index].container.isConnected) {
        rendered.splice(index, 1);
      }
    }

    for (const entry of rendered) {
      const script = document.createElement('script');

      script.src = buildScriptUrl(entry.url, isDark);
      entry.container.innerHTML = '';
      entry.container.appendChild(script);
    }
  }

  onMounted(() => {
    context.register(options);

    let wasDark = document.documentElement.classList.contains('dark');

    themeObserver = new MutationObserver(() => {
      const isDark = document.documentElement.classList.contains('dark');

      if (isDark !== wasDark) {
        wasDark = isDark;
        onThemeChange();
      }
    });
    themeObserver.observe(document.documentElement, {
      attributes: true,
      attributeFilter: ['class'],
    });
  });

  onUnmounted(() => {
    context.unregister(options);
    themeObserver?.disconnect();
    rendered.length = 0;
  });
}

function buildScriptUrl (url: string, isDark: boolean): string {
  const encoded = encodeURIComponent(url);

  return `${EMGITHUB_SCRIPT}?target=${encoded}&style=${isDark ? 'base16/gruvbox-dark-medium' : 'grayscale'}&type=code&showBorder=on&showFileMeta=on&showFullPath=on&showCopy=on&showLineNumbers=on`;
}
