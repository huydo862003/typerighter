import {
  ref, onMounted, onUnmounted,
} from 'vue';

// returns false on SSR - only reflects actual match after mount
export function useMediaQuery (query: string) {
  const matches = ref(false);
  let mediaQuery: MediaQueryList | undefined = undefined;

  function handleChange (event: MediaQueryListEvent): void {
    matches.value = event.matches;
  }

  onMounted(() => {
    mediaQuery = window.matchMedia(query);
    matches.value = mediaQuery.matches;
    mediaQuery?.addEventListener('change', handleChange);
  });

  onUnmounted(() => {
    mediaQuery?.removeEventListener('change', handleChange);
  });

  return matches;
}
