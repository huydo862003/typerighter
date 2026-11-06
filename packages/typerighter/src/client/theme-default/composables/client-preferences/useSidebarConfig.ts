import {
  computed, onMounted, ref,
} from 'vue';
import {
  useClientPreferences,
} from './utils/useClientPreferences';
import {
  wrapHeadScript, readLocalStorage, setDocumentCssVariable,
} from './utils/buildHeadScript';
import {
  registerHeadScript,
} from './utils/registry';

const WIDTH_KEY = 'td-sidebar-width';
const COLLAPSED_KEY = 'td-sidebar-collapsed';
const CSS_VAR = '--td-sidebar-width';

export const SIDEBAR_DEFAULT_WIDTH = 300;
export const SIDEBAR_COLLAPSED_WIDTH = 48;
export const SIDEBAR_MIN_WIDTH = 200;
export const SIDEBAR_MAX_WIDTH = 500;

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
      : {});

  return {
    sidebarWidth,
    sidebarCollapsed,
    sidebarStyle,
  };
}

// Head script
function buildHeadScript (): string {
  const collapsedExpr = `${readLocalStorage(COLLAPSED_KEY)}==='true'`;
  const widthExpr = `collapsed?${SIDEBAR_COLLAPSED_WIDTH}:(Number(${readLocalStorage(WIDTH_KEY)})||${SIDEBAR_DEFAULT_WIDTH})`;

  return wrapHeadScript(
    `var collapsed=${collapsedExpr};`
    + `var width=${widthExpr};`
    + setDocumentCssVariable(CSS_VAR, '`${width}px`'),
  );
}

registerHeadScript(buildHeadScript);
