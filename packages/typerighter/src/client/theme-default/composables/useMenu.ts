import {
  ref, readonly,
} from 'vue';

const isOpen = ref(false);

export function useMenu () {
  function toggle (): void {
    isOpen.value = !isOpen.value;
  }

  function open (): void {
    isOpen.value = true;
  }

  function close (): void {
    isOpen.value = false;
  }

  return {
    isOpen: readonly(isOpen),
    toggle,
    open,
    close,
  };
}
