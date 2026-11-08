import {
  ref, computed, readonly, onMounted,
} from 'vue';
import {
  wrapHeadScript,
  readLocalStorage,
  addDocumentClass,
  setDocumentCssVariable,
} from './utils/buildHeadScript';

export type Theme = 'light' | 'dark' | 'system';

const STORAGE_KEY = 'td-theme';
const DARK_MEDIA_QUERY = '(prefers-color-scheme: dark)';

const INITIAL_DARK_THEME_TOGGLE_CSS_VAR = '--td-dark-theme-toggle';
const INITIAL_LIGHT_THEME_TOGGLE_CSS_VAR = '--td-light-theme-toggle';

const theme = ref<Theme>(getStoredTheme());
const systemTheme = ref<'light' | 'dark'>(getSystemTheme());

// Head script - inlined into the HTML <head> to prevent flash of wrong theme
export function buildThemeHeadScript (): string {
  return wrapHeadScript(
    `var theme=${readLocalStorage(STORAGE_KEY)};`
    + `var isDarkTheme=theme==='dark'||(theme!=='light'&&matchMedia(${JSON.stringify(DARK_MEDIA_QUERY)}).matches);`
    + addDocumentClass('dark', 'isDarkTheme')
    + setDocumentCssVariable(
      INITIAL_DARK_THEME_TOGGLE_CSS_VAR,
      '"none"',
      '!isDarkTheme',
    )
    + setDocumentCssVariable(
      INITIAL_DARK_THEME_TOGGLE_CSS_VAR,
      '"block"',
      'isDarkTheme',
    )
    + setDocumentCssVariable(
      INITIAL_LIGHT_THEME_TOGGLE_CSS_VAR,
      '"none"',
      'isDarkTheme',
    )
    + setDocumentCssVariable(
      INITIAL_LIGHT_THEME_TOGGLE_CSS_VAR,
      '"block"',
      '!isDarkTheme',
    ),
  );
}

let initialized = false;

export function useTheme () {
  /* Initialize on first call */
  initialized = true;

  // Listen to user's preference on system theme, apply theme if current theme is system
  if (typeof window !== 'undefined') {
    const mediaQuery = window.matchMedia(DARK_MEDIA_QUERY);

    mediaQuery.addEventListener('change', () => {
      systemTheme.value = getSystemTheme();

      if (theme.value === 'system') applyTheme('system');
    });
  }

  // empty until mounted so the pre-rendered toggle theme is not overwritten before hydration
  const isMounted = ref(false);

  onMounted(() => {
    isMounted.value = true;
  });

  /* Expose themes */
  const effectiveTheme = computed<'light' | 'dark'>(() =>
    theme.value === 'system' ? systemTheme.value : theme.value);

  const isDark = computed(() => effectiveTheme.value === 'dark');

  const darkToggleStyle = computed(() =>
    isMounted.value
      ? {
        display: isDark.value ? 'block' : 'none',
      }
      : {
        display: `var(${INITIAL_DARK_THEME_TOGGLE_CSS_VAR})`,
      });

  const lightToggleStyle = computed(() =>
    isMounted.value
      ? {
        display: isDark.value ? 'none' : 'block',
      }
      : {
        display: `var(${INITIAL_LIGHT_THEME_TOGGLE_CSS_VAR})`,
      });

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
    darkToggleStyle,
    lightToggleStyle,
  };
}

let transitionTimer: ReturnType<typeof setTimeout> | undefined; // Apply transition on user initiated theme toggle only

function applyTheme (theme: Theme): void {
  if (typeof document === 'undefined') return;

  const effectiveTheme = theme === 'system' ? getSystemTheme() : theme; // Effective theme to transition into

  const rootDomNode = document.documentElement;

  if (transitionTimer !== undefined) {
    // Cancel any in-flight transition cleanup from a previous toggle
    clearTimeout(transitionTimer);
    transitionTimer = undefined;
    // Cancel the transition
    rootDomNode.classList.remove('td-theme-transition');
  }

  // Enable transition only during user-initiated toggles (not on page load)
  if (initialized) {
    rootDomNode.classList.add('td-theme-transition');
  }

  rootDomNode.classList.toggle('dark', effectiveTheme === 'dark'); // Toggle to trigger transition

  // Schedule to end the transition
  if (initialized) {
    // Event is undefined when setTimeout fires
    const onTransitionEnd = (event?: TransitionEvent): void => {
      if (event && event.propertyName !== 'background-color') {
        return;
      }

      clearTimeout(transitionTimer);
      transitionTimer = undefined;
      // Remove the transition
      rootDomNode.classList.remove('td-theme-transition');
      // Cleanup the handler
      rootDomNode.removeEventListener('transitionend', onTransitionEnd);
    };

    rootDomNode.addEventListener('transitionend', onTransitionEnd);
    transitionTimer = setTimeout(onTransitionEnd, 500);
  }
}

function getStoredTheme (): 'light' | 'dark' | 'system' {
  if (typeof window === 'undefined') return 'system';

  const theme = localStorage.getItem(STORAGE_KEY);

  if (theme === 'light' || theme === 'dark' || theme === 'system') {
    return theme;
  }

  return 'system';
}

function getSystemTheme (): 'light' | 'dark' {
  if (typeof window === 'undefined') return 'light';

  return window.matchMedia(DARK_MEDIA_QUERY).matches ? 'dark' : 'light';
}
