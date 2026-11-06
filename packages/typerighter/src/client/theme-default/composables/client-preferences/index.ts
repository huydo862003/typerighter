// Centralized localStorage-backed preferences

import './useTheme';
import './useSidebarConfig';
import {
  collectHeadScripts,
} from './utils/registry';

export {
  useClientPreferences,
} from './utils/useClientPreferences';

export function provideHeadScript (): string {
  return collectHeadScripts();
}
