import {
  ref, computed, readonly,
} from 'vue';
import {
  wrapHeadScript, readLocalStorage, addDocumentClass,
} from './utils/buildHeadScript';
import {
  registerHeadScript,
} from './utils/registry';

export type Theme = 'light' | 'dark' | 'system';

const STORAGE_KEY = 'td-theme';
const DARK_MEDIA_QUERY = '(prefers-color-scheme: dark)';

function applyTheme (theme: Theme): void {
  if (typeof document === 'undefined') return;

  const effective = theme === 'system' ? getSystemTheme() : theme;

  document.documentElement.classList.toggle('dark', effective === 'dark');
}

function getSystemTheme (): 'light' | 'dark' {
  if (typeof window === 'undefined') return 'light';

  return window.matchMedia(DARK_MEDIA_QUERY).matches ? 'dark' : 'light';
}

const theme = ref<Theme>('system');
const systemTheme = ref<'light' | 'dark'>(getSystemTheme());
let initialized = false;

export function useTheme () {
  initialize();

  const effectiveTheme = computed<'light' | 'dark'>(() =>
    theme.value === 'system' ? systemTheme.value : theme.value);

  const isDark = computed(() => effectiveTheme.value === 'dark');

  function setTheme (newTheme: Theme): void {
    theme.value = newTheme;
    applyTheme(newTheme);

    if (typeof localStorage !== 'undefined') {
      localStorage.setItem(STORAGE_KEY, newTheme);
    }
  }

  function toggle (): void {
    if (theme.value === 'system') {
      setTheme(systemTheme.value === 'light' ? 'dark' : 'light');
    } else {
      setTheme('system');
    }
  }

  return {
    theme: readonly(theme),
    effectiveTheme,
    isDark,
    setTheme,
    toggle,
  };
}

// Head script
function buildHeadScript (): string {
  return wrapHeadScript(
    `var t=${readLocalStorage(STORAGE_KEY)};`
    + addDocumentClass(
      `t==='dark'||(t!=='light'&&matchMedia(${JSON.stringify(DARK_MEDIA_QUERY)}).matches)`,
      'dark',
    ),
  );
}

registerHeadScript(buildHeadScript);

function initialize (): void {
  if (initialized) return;
  initialized = true;

  if (typeof localStorage !== 'undefined') {
    const stored = localStorage.getItem(STORAGE_KEY) as string | null;

    if (stored === 'light' || stored === 'dark' || stored === 'system') {
      theme.value = stored;
    }
  }

  applyTheme(theme.value);

  if (typeof window !== 'undefined') {
    const mediaQuery = window.matchMedia(DARK_MEDIA_QUERY);

    mediaQuery.addEventListener('change', () => {
      systemTheme.value = getSystemTheme();

      if (theme.value === 'system') applyTheme('system');
    });
  }
}
