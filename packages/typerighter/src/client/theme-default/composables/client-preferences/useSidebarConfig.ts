import {
  computed, onMounted, ref,
} from 'vue';
import {
  useClientPreferences,
} from './utils/useClientPreferences';
import {
  wrapHeadScript,
  readLocalStorage,
  setDocumentCssVariable,
} from './utils/buildHeadScript';

const WIDTH_KEY = 'td-sidebar-width';
const COLLAPSED_KEY = 'td-sidebar-collapsed';
const CSS_VAR = '--td-sidebar-width';

export const SIDEBAR_DEFAULT_WIDTH = 300;
export const SIDEBAR_COLLAPSED_WIDTH = 48;
export const SIDEBAR_MIN_WIDTH = 200;
export const SIDEBAR_MAX_WIDTH = 500;

// Head script - inlined into the HTML <head> to prevent flash of wrong sidebar width
export function buildSidebarHeadScript (): string {
  const collapsedExpr = `${readLocalStorage(COLLAPSED_KEY)}==='true'`;
  const widthExpr = `collapsed?${SIDEBAR_COLLAPSED_WIDTH}:(Number(${readLocalStorage(WIDTH_KEY)})||${SIDEBAR_DEFAULT_WIDTH})`;

  return wrapHeadScript(
    `var collapsed=${collapsedExpr};`
    + `var width=${widthExpr};`
    + setDocumentCssVariable(CSS_VAR, '`${width}px`'),
  );
}

export function useSidebarConfig () {
  const sidebarWidth = useClientPreferences(WIDTH_KEY, SIDEBAR_DEFAULT_WIDTH);
  const sidebarCollapsed = useClientPreferences(COLLAPSED_KEY, false);

  // empty until mounted so the pre-rendered width is not overwritten before hydration
  const isMounted = ref(false);

  onMounted(() => {
    isMounted.value = true;
  });

  const effectiveWidth = computed(() =>
    sidebarCollapsed.value ? SIDEBAR_COLLAPSED_WIDTH : sidebarWidth.value);

  const sidebarStyle = computed(() =>
    isMounted.value
      ? {
        width: `${effectiveWidth.value}px`,
      }
      : {
        width: `var(${CSS_VAR})`,
      });

  return {
    sidebarWidth,
    sidebarCollapsed,
    sidebarStyle,
  };
}
