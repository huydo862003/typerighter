// Centralized localStorage-backed preferences

import {
  buildThemeHeadScript,
} from './useTheme';
import {
  buildSidebarHeadScript,
} from './useSidebarConfig';

export {
  useClientPreferences,
} from './utils/useClientPreferences';

export function provideHeadScript (): string {
  return buildThemeHeadScript() + buildSidebarHeadScript();
}
