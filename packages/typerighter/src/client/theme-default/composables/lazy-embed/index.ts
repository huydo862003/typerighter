import {
  onMounted, onUnmounted,
} from 'vue';
import type {
  LazyEmbedContext, LazyEmbedOptions,
} from './types';
import {
  useD2,
} from './d2';
import {
  useMermaid,
} from './mermaid';
import {
  useGithubEmbed,
} from './github';

export function useLazyEmbeds (): void {
  const registrations: LazyEmbedOptions[] = [];
  const observed = new WeakSet<Element>();
  const elementToOptions = new WeakMap<Element, LazyEmbedOptions>();
  let intersectionObserver: IntersectionObserver | undefined;
  let mutationObserver: MutationObserver | undefined;

  function scanAll () {
    const container = document.getElementById('td-content');

    if (!container || !intersectionObserver) return;

    for (const registration of registrations) {
      const placeholders = container.querySelectorAll<Element>(
        registration.selector,
      );

      for (const element of placeholders) {
        if (!observed.has(element)) {
          observed.add(element);
          elementToOptions.set(element, registration);
          intersectionObserver.observe(element);
        }
      }
    }
  }

  onMounted(() => {
    intersectionObserver = new IntersectionObserver(
      (entries) => {
        for (const entry of entries) {
          if (entry.isIntersecting) {
            intersectionObserver?.unobserve(entry.target);
            const registration = elementToOptions.get(entry.target);

            if (registration) {
              registration.onRender(entry.target);
            }
          }
        }
      },
      {
        rootMargin: '1000px',
      },
    );

    scanAll();

    const setupMediaLoading = (root: HTMLElement) => {
      for (const iframe of root.querySelectorAll<HTMLIFrameElement>('.td-embed-wrapper iframe')) {
        if (iframe.dataset.tdLoadingBound) continue;
        iframe.dataset.tdLoadingBound = '1';
        const wrapper = iframe.closest('.td-embed-wrapper');

        if (!wrapper) continue;
        const markLoaded = () => wrapper.classList.add('td-loaded');

        iframe.addEventListener('load', markLoaded, {
          once: true,
        });
        iframe.addEventListener('error', markLoaded, {
          once: true,
        });
      }
      for (const image of root.querySelectorAll<HTMLImageElement>('img')) {
        if (image.dataset.tdLoadingBound || image.complete) continue;
        image.dataset.tdLoadingBound = '1';
        image.classList.add('td-loading');
        const markLoaded = () => image.classList.remove('td-loading');

        image.addEventListener('load', markLoaded, {
          once: true,
        });
        image.addEventListener('error', markLoaded, {
          once: true,
        });
      }
    };

    const container = document.getElementById('td-content');

    if (container) {
      setupMediaLoading(container);
      mutationObserver = new MutationObserver(() => {
        scanAll();
        setupMediaLoading(container);
      });
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

  const context: LazyEmbedContext = {
    register (options: LazyEmbedOptions) {
      registrations.push(options);
      scanAll();

      if (options.onVisible && document.querySelector(options.selector)) {
        options.onVisible();
      }
    },
    unregister (options: LazyEmbedOptions) {
      const index = registrations.indexOf(options);

      if (index !== -1) {
        registrations.splice(index, 1);
      }
    },
  };

  useMermaid(context);
  useD2(context);
  useGithubEmbed(context);
}
