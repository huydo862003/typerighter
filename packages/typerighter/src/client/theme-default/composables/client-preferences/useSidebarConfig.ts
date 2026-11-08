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

const INITIAL_SIDEBAR_WIDTH_CSS_VAR = '--td-sidebar-width';
const INITIAL_EXPANDED_SIDEBAR_CSS_VAR = '--td-expanded-sidebar-display';
const INITIAL_COLLAPSED_SIDEBAR_CSS_VAR = '--td-collapsed-sidebar-display';

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
    + setDocumentCssVariable(
      INITIAL_SIDEBAR_WIDTH_CSS_VAR,
      '`${width}px`',
    )
    + setDocumentCssVariable(
      INITIAL_EXPANDED_SIDEBAR_CSS_VAR,
      '"none"',
      'collapsed',
    )
    + setDocumentCssVariable(
      INITIAL_EXPANDED_SIDEBAR_CSS_VAR,
      undefined, // intentionally invalid css
      '!collapsed',
    )
    + setDocumentCssVariable(
      INITIAL_COLLAPSED_SIDEBAR_CSS_VAR,
      undefined, // intentionally invalid css
      'collapsed',
    )
    + setDocumentCssVariable(
      INITIAL_COLLAPSED_SIDEBAR_CSS_VAR,
      '"none"',
      '!collapsed',
    ),
  );
}

export function useSidebarConfig () {
  const sidebarWidth = useClientPreferences(WIDTH_KEY, SIDEBAR_DEFAULT_WIDTH);
  const isSidebarCollapsed = useClientPreferences(COLLAPSED_KEY, false);

  // empty until mounted so the pre-rendered width is not overwritten before hydration
  const isMounted = ref(false);

  onMounted(() => {
    isMounted.value = true;
  });

  const effectiveWidth = computed(() =>
    isSidebarCollapsed.value ? SIDEBAR_COLLAPSED_WIDTH : sidebarWidth.value);

  const sidebarWidthStyle = computed(() =>
    isMounted.value
      ? {
        width: `${effectiveWidth.value}px`,
      }
      : {
        width: `var(${INITIAL_SIDEBAR_WIDTH_CSS_VAR})`,
      });

  const expandedSidebarVisibilityStyle = computed(() =>
    isMounted.value
      ? {
        display: isSidebarCollapsed.value ? 'none' : undefined,
      }
      : {
        display: `var(${INITIAL_EXPANDED_SIDEBAR_CSS_VAR})`,
      });

  const collapsedSidebarVisibilityStyle = computed(() =>
    isMounted.value
      ? {
        display: !isSidebarCollapsed.value ? 'none' : undefined,
      }
      : {
        display: `var(${INITIAL_COLLAPSED_SIDEBAR_CSS_VAR})`,
      });

  return {
    sidebarWidth,
    isSidebarCollapsed,
    sidebarWidthStyle,
    collapsedSidebarVisibilityStyle,
    expandedSidebarVisibilityStyle,
  };
}
