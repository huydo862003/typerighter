import {
  nextTick, onUnmounted, ref, watch,
  type WatchSource,
} from 'vue';
import type {
  MarkdownHeading,
} from '@/shared';

// Track which heading is currently visible and expose its id
export function useActiveTocHeading (headings: WatchSource<MarkdownHeading[]>) {
  const activeId = ref('');
  let observer: IntersectionObserver | undefined;

  function observe () {
    observer?.disconnect();
    activeId.value = '';

    const elements = Array.from(
      document.querySelectorAll<HTMLElement>('.td-content h2[id], .td-content h3[id], .td-content h4[id], .td-content h5[id]'),
    );

    if (elements.length === 0) return;

    // Pick the topmost visible heading, or the last heading scrolled past
    observer = new IntersectionObserver(
      (entries) => {
        for (const entry of entries) {
          if (entry.isIntersecting) {
            activeId.value = entry.target.id;

            return;
          }
        }

        // No heading visible - find the last one above the viewport
        const scrollY = window.scrollY;

        for (let index = elements.length - 1; 0 <= index; index--) {
          if (elements[index].offsetTop <= scrollY + 100) {
            activeId.value = elements[index].id;

            return;
          }
        }
      },
      {
        rootMargin: '-64px 0px -70% 0px',
        threshold: 0,
      },
    );

    for (const element of elements) {
      observer.observe(element);
    }
  }

  // Re-observe when the headings prop changes (page navigation)
  watch(headings, () => nextTick(observe), {
    immediate: true,
  });

  onUnmounted(() => {
    observer?.disconnect();
  });

  return activeId;
}
