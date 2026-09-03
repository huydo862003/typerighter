import TdLayout from './TdLayout.vue';
import TdNotFound from './TdNotFound.vue';

export {
  TdLayout, TdNotFound,
};
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
};
