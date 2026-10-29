import {
  onMounted, onUnmounted,
} from 'vue';

const EMGITHUB_SCRIPT = 'https://emgithub.com/embed-v2.js';

// Render GitHub embed placeholders using emgithub when they scroll into view
export function useGithubEmbed (): void {
  let contentObserver: MutationObserver | undefined;
  let themeObserver: MutationObserver | undefined;
  let intersectionObserver: IntersectionObserver | undefined;
  const observed = new WeakSet<Element>();

  // Track rendered embeds so we can re-render on theme change
  const rendered: Array<{
    container: HTMLElement;
    url: string;
  }> = [];

  function observePlaceholders () {
    const container = document.getElementById('td-content');

    if (!container || !intersectionObserver) return;

    const placeholders = container.querySelectorAll<HTMLDivElement>('.td-github-placeholder');

    for (const placeholder of placeholders) {
      if (!observed.has(placeholder)) {
        observed.add(placeholder);
        intersectionObserver.observe(placeholder);
      }
    }
  }

  function renderPlaceholder (placeholder: HTMLDivElement) {
    const url = placeholder.getAttribute('data-url');

    if (!url) return;

    const isDark = document.documentElement.classList.contains('dark');
    const script = document.createElement('script');

    script.src = buildScriptUrl(url, isDark);

    // emgithub replaces the script with an .emgithub-container div
    // We wrap in a sentinel so we can find and re-render on theme toggle
    const wrapper = document.createElement('div');

    wrapper.className = 'td-github-wrapper';
    wrapper.dataset.url = url;
    wrapper.appendChild(script);

    placeholder.replaceWith(wrapper);
    rendered.push({
      container: wrapper,
      url,
    });
  }

  function onThemeChange () {
    const isDark = document.documentElement.classList.contains('dark');

    for (const entry of rendered) {
      if (!entry.container.isConnected) continue;

      const script = document.createElement('script');

      script.src = buildScriptUrl(entry.url, isDark);

      entry.container.innerHTML = '';
      entry.container.appendChild(script);
    }
  }

  onMounted(() => {
    intersectionObserver = new IntersectionObserver((entries) => {
      for (const entry of entries) {
        if (entry.isIntersecting) {
          intersectionObserver?.unobserve(entry.target);
          renderPlaceholder(entry.target as HTMLDivElement);
        }
      }
    }, {
      rootMargin: '200px',
    });

    observePlaceholders();

    const container = document.getElementById('td-content');

    if (container) {
      contentObserver = new MutationObserver(observePlaceholders);
      contentObserver.observe(container, {
        childList: true,
        subtree: true,
      });
    }

    // Watch for dark/light theme toggle on <html>
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
    contentObserver?.disconnect();
    themeObserver?.disconnect();
    intersectionObserver?.disconnect();
    rendered.length = 0;
  });
}

function buildScriptUrl (url: string, isDark: boolean): string {
  const encoded = encodeURIComponent(url);

  return `${EMGITHUB_SCRIPT}?target=${encoded}&style=${isDark ? 'base16/gruvbox-dark-medium' : 'grayscale'}&type=code&showBorder=on&showFileMeta=on&showFullPath=on&showCopy=on&showLineNumbers=on`;
}
