import {
  ref, watch, type Ref,
} from 'vue';

// Returns defaultValue when localStorage is unavailable (SSR), persists changes on write
export function useClientPreferences (
  key: string,
  defaultValue: string,
): Ref<string>;
export function useClientPreferences (
  key: string,
  defaultValue: boolean,
): Ref<boolean>;
export function useClientPreferences (
  key: string,
  defaultValue: number,
): Ref<number>;
export function useClientPreferences<T extends string | boolean | number> (
  key: string,
  defaultValue: T,
): Ref<T> {
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

function coerce<T extends string | boolean | number> (
  stored: string,
  defaultValue: T,
): T {
  if (typeof defaultValue === 'boolean') return (stored === 'true') as T;
  // numeric 0 stored as '0' is treated as unset and falls back to defaultValue
  if (typeof defaultValue === 'number')
    return (Number(stored) || defaultValue) as T;

  return stored as T;
}
