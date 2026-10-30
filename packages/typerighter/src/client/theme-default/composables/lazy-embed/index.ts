import {
  onMounted, onUnmounted, provide,
} from 'vue';
import {
  LAZY_EMBED_KEY,
} from './types';
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
        rootMargin: '200px',
      },
    );

    scanAll();

    const container = document.getElementById('td-content');

    if (container) {
      mutationObserver = new MutationObserver(scanAll);
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

  provide(LAZY_EMBED_KEY, context);

  useMermaid();
  useD2();
  useGithubEmbed();
}
