import {
  ref, watch, type Ref,
} from 'vue';

// Reactive ref backed by localStorage, SSR-safe
export function useLocalStorage (key: string, defaultValue: string): Ref<string>;
export function useLocalStorage (key: string, defaultValue: boolean): Ref<boolean>;
export function useLocalStorage (key: string, defaultValue: number): Ref<number>;
export function useLocalStorage<T extends string | boolean | number> (key: string, defaultValue: T): Ref<T> {
  const stored =
    typeof localStorage !== 'undefined' ? localStorage.getItem(key) : null;

  const initial = stored === null ? defaultValue : coerce(stored, defaultValue);
  const value = ref(initial) as Ref<T>;

  watch(value, (newValue) => {
    if (typeof localStorage !== 'undefined') {
      localStorage.setItem(key, String(newValue));
    }
  });

  return value;
}

function coerce<T extends string | boolean | number> (stored: string, defaultValue: T): T {
  if (typeof defaultValue === 'boolean') return (stored === 'true') as T;
  if (typeof defaultValue === 'number') return (Number(stored) || defaultValue) as T;

  return stored as T;
}
