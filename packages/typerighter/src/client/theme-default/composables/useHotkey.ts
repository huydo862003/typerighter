// Local hotkeys override global ones when both match the same combo

import type {
  ComponentInstance, TemplateRef,
} from 'vue';
import {
  onMounted,
  onUnmounted,
  watch,
} from 'vue';
import {
  TdKeyName,
} from '../utils/keys';

type HotkeyHandler = (event: KeyboardEvent) => void;

const MODIFIERS: Map<string, keyof KeyboardEvent> = new Map([
  [
    TdKeyName.Control,
    'ctrlKey',
  ],
  [
    TdKeyName.Alt,
    'altKey',
  ],
  [
    TdKeyName.Shift,
    'shiftKey',
  ],
  [
    TdKeyName.Meta,
    'metaKey',
  ],
]);

export function useGlobalHotkey (
  keys: TdKeyName[],
  handler: HotkeyHandler,
): void {
  function onKeydown (event: KeyboardEvent) {
    if (event.defaultPrevented) return;

    if (matchesKeys(event, keys)) {
      event.preventDefault();
      handler(event);
    }
  }

  onMounted(() => window.addEventListener('keydown', onKeydown));
  onUnmounted(() => window.removeEventListener('keydown', onKeydown));
}

// Local keyboard shortcuts bound to an element
// Only fires when the element or its descendants have focus
export function useLocalHotkeys (element: TemplateRef<HTMLElement | ComponentInstance<unknown> | null | undefined>) {
  const entries: LocalShortcutEntry[] = [];

  function register (keys: TdKeyName[], handler: HotkeyHandler) {
    entries.push({
      keys,
      handler,
    });
  }

  function onKeydown (event: KeyboardEvent) {
    for (const entry of entries) {
      if (matchesKeys(event, entry.keys)) {
        event.preventDefault();
        event.stopPropagation();
        entry.handler(event);

        return;
      }
    }
  }

  watch(element, (newElement, oldElement) => {
    (oldElement as HTMLElement | null)?.removeEventListener('keydown', onKeydown);
    (newElement as HTMLElement | null)?.addEventListener('keydown', onKeydown);
  }, {
    immediate: true,
  });

  onUnmounted(() => {
    (element.value as HTMLElement | null)?.removeEventListener('keydown', onKeydown);
  });

  return {
    register,
  };
}

interface LocalShortcutEntry {
  keys: TdKeyName[];
  handler: HotkeyHandler;
}

function matchesKeys (event: KeyboardEvent, keys: TdKeyName[]): boolean {
  const modifiers = keys.filter((key) => MODIFIERS.has(key));
  const nonModifiers = keys.filter((key) => !MODIFIERS.has(key));

  if (nonModifiers.length !== 1) return false;
  if (event.key.toLowerCase() !== nonModifiers[0].toLowerCase()) return false;

  for (const modifier of modifiers) {
    if (!event[MODIFIERS.get(modifier)!]) return false;
  }

  if (event.ctrlKey && !modifiers.includes(TdKeyName.Control)) return false;
  if (event.altKey && !modifiers.includes(TdKeyName.Alt)) return false;
  if (event.shiftKey && !modifiers.includes(TdKeyName.Shift)) return false;
  if (event.metaKey && !modifiers.includes(TdKeyName.Meta)) return false;

  return true;
}
