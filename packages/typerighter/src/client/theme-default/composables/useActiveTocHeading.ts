import {
  nextTick, onMounted, onUnmounted, ref, watch,
  type WatchSource,
} from 'vue';
import type {
  MarkdownHeading,
} from '@/shared';

// Pixels from the top of the viewport within which a heading is considered active
const ACTIVATION_OFFSET = 100;

// Track which TOC heading is active based on scroll position
export function useActiveTocHeading (headings: WatchSource<MarkdownHeading[]>) {
  const activeId = ref<string | undefined>(undefined);
  let elements: HTMLElement[] = [];
  let rafId: number | undefined;

  function setActiveLink () {
    if (elements.length === 0) return;

    const scrollY = window.scrollY;
    const isBottom = document.body.offsetHeight <= scrollY + window.innerHeight;

    // At page bottom, highlight the last heading
    if (isBottom) {
      activeId.value = elements.at(-1)!.id;

      return;
    }

    // Find the last heading scrolled to or past the activation threshold
    let found: string | undefined;

    for (const element of elements) {
      if (getAbsoluteTop(element) <= scrollY + ACTIVATION_OFFSET) {
        found = element.id;
      } else {
        break;
      }
    }

    activeId.value = found;
  }

  function onScroll () {
    if (rafId !== undefined) cancelAnimationFrame(rafId);
    rafId = requestAnimationFrame(setActiveLink);
  }

  // Re-query headings from the DOM and recompute the active link
  function updateElements (tocHeadings: MarkdownHeading[]) {
    if (typeof document === 'undefined') return;
    activeId.value = undefined;

    // Only track headings that appear in the TOC, not all headings in the content
    const tocSlugs = new Set(tocHeadings.map((heading) => heading.slug));

    elements = Array.from(
      document.querySelectorAll<HTMLElement>('.td-content h2[id], .td-content h3[id], .td-content h4[id], .td-content h5[id]'),
    ).filter((element) => tocSlugs.has(element.id));

    setActiveLink();
  }

  onMounted(() => {
    window.addEventListener('scroll', onScroll);
  });

  onUnmounted(() => {
    window.removeEventListener('scroll', onScroll);
    if (rafId !== undefined) cancelAnimationFrame(rafId);
  });

  // Re-query when the headings change (page navigation)
  watch(
    headings,
    (tocHeadings) => {
      if (typeof window === 'undefined') return;
      nextTick(() => updateElements(tocHeadings));
    },
    {
      immediate: true,
    },
  );

  return activeId;
}

// Walk the offsetParent chain to get an element's absolute top position in the document
function getAbsoluteTop (element: HTMLElement): number {
  let top = 0;
  let current: HTMLElement | undefined = element;

  while (current) {
    top += current.offsetTop;
    current = current.offsetParent as HTMLElement | undefined;
  }

  return top;
}
