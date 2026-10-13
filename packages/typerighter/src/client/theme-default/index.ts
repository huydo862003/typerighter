import type {
  TypedownTheme,
} from '../app';
import TdLayout from './TdLayout.vue';
import TdNotFound from './TdNotFound.vue';

export type {
  TypedownTheme,
} from '../app';
export {
  default as TdLayout,
} from './TdLayout.vue';
export {
  default as TdNotFound,
} from './TdNotFound.vue';
// Components for theme composition
export {
  default as TdBrandIcon,
} from './components/TdBrandIcon.vue';
export {
  default as TdContentNav,
} from './components/TdContentNav.vue';
export {
  default as TdFooter,
} from './components/TdFooter.vue';
export {
  default as TdPreviousNext,
} from './components/TdPreviousNext.vue';
export {
  default as TdRail,
} from './components/TdRail.vue';
export {
  default as TdThemeToggle,
} from './components/TdThemeToggle.vue';
export {
  default as TdToc,
} from './components/TdToc.vue';
export {
  useCopyCode,
} from './composables/useCopyCode';
export {
  TdKeyName,
} from './utils/keys';
export {
  useGlobalHotkey, useLocalHotkeys,
} from './composables/useHotkey';
export {
  default as TdKbdShortcut,
} from './components/TdKbdShortcut.vue';
export {
  default as TdDirectoryIndex,
} from './components/custom/TdDirectoryIndex.vue';
export {
  default as TdGlossaryIndex,
} from './components/custom/glossary/TdGlossaryIndex.vue';

export default {
  Layout: TdLayout,
  NotFound: TdNotFound,
} satisfies TypedownTheme;
