import {
  ref, readonly, computed, watch,
} from 'vue';
import {
  useMediaQuery,
} from './useMediaQuery';
import {
  useGlobalHotkey,
} from './useHotkey';
import {
  TdKeyName,
} from '../utils/keys';

const isOpen = ref(false);

export function useMenu () {
  const isLargeScreen = useMediaQuery('(width >= 64rem)');

  // drawer is not mounted on large screens where the sidebar is always visible
  const isDrawerMounted = computed(() => !isLargeScreen.value);

  function toggle (): void {
    isOpen.value = !isOpen.value;
  }

  function open (): void {
    isOpen.value = true;
  }

  function close (): void {
    isOpen.value = false;
  }

  // close drawer when viewport expands to sidebar-visible breakpoint
  watch(isLargeScreen, (large) => {
    if (large) close();
  });

  useGlobalHotkey([TdKeyName.Escape], () => {
    if (isOpen.value) close();
  });

  return {
    isOpen: readonly(isOpen),
    isDrawerMounted,
    toggle,
    open,
    close,
  };
}
