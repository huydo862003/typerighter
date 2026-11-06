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

export function isTypingTarget (target: EventTarget | null): boolean {
  if (!(target instanceof Element)) return false;
  const tag = target.tagName.toLowerCase();

  return tag === 'input' || tag === 'textarea' || tag === 'select' || (target as HTMLElement).isContentEditable;
}

export function useGlobalHotkey (
  keys: TdKeyName[],
  handler: HotkeyHandler,
  options?: {
    ignoreWhenTyping?: boolean;
    // Use capture phase to override browser shortcuts (e.g. Ctrl+P for print)
    capture?: boolean;
  },
): void {
  function onKeydown (event: KeyboardEvent) {
    if (event.defaultPrevented) return;
    if (options?.ignoreWhenTyping && isTypingTarget(event.target)) return;

    if (matchesKeys(event, keys)) {
      event.preventDefault();
      handler(event);
    }
  }

  const listenerOptions = {
    capture: options?.capture ?? false,
  };

  onMounted(() => window.addEventListener('keydown', onKeydown, listenerOptions));
  onUnmounted(() => window.removeEventListener('keydown', onKeydown, listenerOptions));
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

function getCodeFromKey (key: string): string | undefined {
  if (key.length === 1 && /[a-z]/i.test(key)) return `Key${key.toUpperCase()}`;

  return undefined;
}

function matchesKeys (event: KeyboardEvent, keys: TdKeyName[]): boolean {
  const modifiers = keys.filter((key) => MODIFIERS.has(key));
  const nonModifiers = keys.filter((key) => !MODIFIERS.has(key));

  if (nonModifiers.length !== 1) return false;

  // Use event.code for letter keys (stable across modifiers/layouts)
  const expectedCode = getCodeFromKey(nonModifiers[0]);
  const keyMatches = expectedCode
    ? event.code === expectedCode
    : event.key.toLowerCase() === nonModifiers[0].toLowerCase();

  if (!keyMatches) return false;

  for (const modifier of modifiers) {
    if (!event[MODIFIERS.get(modifier)!]) return false;
  }

  if (event.ctrlKey && !modifiers.includes(TdKeyName.Control)) return false;
  if (event.altKey && !modifiers.includes(TdKeyName.Alt)) return false;
  if (event.shiftKey && !modifiers.includes(TdKeyName.Shift)) return false;
  if (event.metaKey && !modifiers.includes(TdKeyName.Meta)) return false;

  return true;
}
