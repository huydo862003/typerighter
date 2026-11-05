<script setup lang="ts">
import {
  computed, nextTick, ref, useTemplateRef, watch,
} from 'vue';
import {
  Search,
} from '@lucide/vue';
import {
  usePreviousNext,
} from '../composables/usePreviousNext';
import {
  TdKeyName,
} from '../utils/keys';
import TdKbdShortcut from './TdKbdShortcut.vue';
import {
  highlight,
} from './TdSiteSearch/highlight';
import {
  useSiteSearch,
} from './TdSiteSearch/useSiteSearch';
import {
  getParentUrl,
} from '@/shared';
import {
  useRoute, useRouter, useSiteConfig,
} from '@/client/app';

const emit = defineEmits<{
  'toggle-sidebar': [];
  'show-shortcuts': [];
}>();
const isOpen = ref(false);
const query = ref('');
const selectedIndex = ref(0);
const inputRef = useTemplateRef<HTMLInputElement>('input');
const resultsRef = useTemplateRef<HTMLElement>('results');

const route = useRoute();
const router = useRouter();
const {
  withBase,
} = useSiteConfig();
const {
  previous, next,
} = usePreviousNext();
const {
  results: pageResults, search: searchPages, cancel: cancelSearch,
} = useSiteSearch();

const filterText = computed(() => query.value.trim());

function close () {
  isOpen.value = false;
  cancelSearch();
}

function open () {
  query.value = '';
  isOpen.value = true;
  selectedIndex.value = 0;
  nextTick(() => inputRef.value?.focus());
}

defineExpose({
  open,
  close,
});

watch(filterText, (value, _old, onCleanup) => {
  onCleanup(() => cancelSearch());
  searchPages(value);
  selectedIndex.value = 0;
});

interface Command {
  id: string;
  group: string;
  label: string;
  keys?: TdKeyName[];
  href?: string;
  keepsOpen?: boolean;
  action?: () => void;
}

const parentUrl = computed(() => {
  const parent = getParentUrl(route.path);

  return parent === '/' ? undefined : parent;
});

const allCommands = computed((): Command[] => {
  const cmds: Command[] = [];

  if (next.value) {
    cmds.push({
      id: 'nav-next',
      group: 'Navigate',
      label: `Next page: ${next.value.title}`,
      keys: [
        TdKeyName.Control,
        TdKeyName.ArrowRight,
      ],
      href: next.value.url,
    });
  }
  if (previous.value) {
    cmds.push({
      id: 'nav-prev',
      group: 'Navigate',
      label: `Previous page: ${previous.value.title}`,
      keys: [
        TdKeyName.Control,
        TdKeyName.ArrowLeft,
      ],
      href: previous.value.url,
    });
  }
  if (parentUrl.value) {
    cmds.push({
      id: 'nav-up',
      group: 'Navigate',
      label: 'Up to section overview',
      href: withBase(parentUrl.value),
    });
  }
  cmds.push({
    id: 'view-sidebar',
    group: 'View',
    label: 'Collapse sidebar',
    keys: [
      TdKeyName.Meta,
      TdKeyName.Backslash,
    ],
    keepsOpen: true,
    action: () => emit('toggle-sidebar'),
  });
  cmds.push({
    id: 'view-shortcuts',
    group: 'View',
    label: 'Keyboard shortcuts',
    action: () => {
      close(); emit('show-shortcuts');
    },
  });
  cmds.push({
    id: 'page-copy',
    group: 'This page',
    label: 'Copy link',
    action: () => {
      navigator.clipboard.writeText(window.location.href).catch(() => {}); close();
    },
  });

  return cmds;
});

const visibleCommands = computed(() => {
  if (!filterText.value) return allCommands.value;
  const queryLower = filterText.value.toLowerCase();

  return allCommands.value.filter((command) =>
    command.label.toLowerCase().includes(queryLower) || command.group.toLowerCase().includes(queryLower));
});

interface CommandGroup {
  name: string;
  commands: Command[];
}

const groupedCommands = computed((): CommandGroup[] => {
  const map = new Map<string, Command[]>();

  for (const command of visibleCommands.value) {
    if (!map.has(command.group)) map.set(command.group, []);
    map.get(command.group)!.push(command);
  }

  return [...map.entries()].map(([
    name,
    commands,
  ]) => ({
    name,
    commands,
  }));
});

const totalCount = computed(() => pageResults.value.length + visibleCommands.value.length);

function findCommandIndex (command: Command): number {
  return pageResults.value.length + visibleCommands.value.indexOf(command);
}

function onKeydown (event: KeyboardEvent) {
  const total = totalCount.value;

  if (event.key === 'Escape') {
    event.preventDefault(); close();

    return;
  }
  if (event.key === 'ArrowDown') {
    event.preventDefault();
    selectedIndex.value = 0 < total ? (selectedIndex.value + 1) % total : 0;
  }
  if (event.key === 'ArrowUp') {
    event.preventDefault();
    selectedIndex.value = 0 < total ? (selectedIndex.value - 1 + total) % total : 0;
  }
  if (event.key === 'Enter') {
    event.preventDefault();
    selectAtIndex(selectedIndex.value);
  }
}

function runCommand (command: Command) {
  if (!command.keepsOpen) close();
  if (command.href) router.go(command.href);
  else command.action?.();
}

function selectAtIndex (index: number) {
  if (index < pageResults.value.length) {
    const result = pageResults.value[index];

    if (result) {
      close();
      router.go(withBase(result.id));
    }
  } else {
    const command = visibleCommands.value[index - pageResults.value.length];

    if (command) runCommand(command);
  }
}

function selectPage (index: number) {
  const result = pageResults.value[index];

  if (result) {
    close();
    router.go(withBase(result.id));
  }
}

watch(totalCount, () => {
  selectedIndex.value = 0;
});

watch(selectedIndex, () => {
  requestAnimationFrame(() => {
    resultsRef.value?.querySelector<HTMLElement>('.is-selected')?.scrollIntoView({
      block: 'nearest',
    });
  });
});
</script>

<template>
  <Teleport to="body">
    <template v-if="isOpen">
      <div
        class="td-palette-scrim"
        @click="close"
      />
      <div
        class="td-palette"
        role="dialog"
        aria-modal="true"
        aria-label="Command palette"
      >
        <!-- Input -->
        <div class="td-palette-input-row">
          <Search
            :size="14"
            class="td-palette-input-icon"
          />
          <input
            ref="input"
            v-model="query"
            class="td-palette-input"
            type="text"
            placeholder="Search pages and commands..."
            autocomplete="off"
            spellcheck="false"
            @keydown="onKeydown"
          >
          <button
            class="td-palette-esc"
            type="button"
            aria-label="Close"
            @click="close"
          >
            esc
          </button>
        </div>

        <!-- Results -->
        <div
          ref="results"
          class="td-palette-results"
        >
          <!-- Page results -->
          <template v-if="pageResults.length > 0">
            <div class="td-palette-group-label">
              Pages
            </div>
            <a
              v-for="(result, index) in pageResults"
              :key="result.id"
              :href="withBase(result.id)"
              class="td-palette-item"
              :class="{
                'is-selected': index === selectedIndex,
              }"
              @click.prevent="() => selectPage(index)"
              @mouseenter="() => selectedIndex = index"
            >
              <span
                class="td-palette-item-label"
                v-html="highlight(result.title, filterText)"
              />
              <span class="td-palette-item-path">{{ result.id }}</span>
            </a>
          </template>

          <!-- Commands -->
          <template
            v-for="group in groupedCommands"
            :key="group.name"
          >
            <div class="td-palette-group-label">
              {{ group.name }}
            </div>
            <component
              :is="cmd.href ? 'a' : 'button'"
              v-for="cmd in group.commands"
              :key="cmd.id"
              v-bind="cmd.href
                ? {
                  href: cmd.href,
                }
                : {
                  type: 'button',
                }"
              class="td-palette-item"
              :class="{
                'is-selected': findCommandIndex(cmd) === selectedIndex,
              }"
              @click="() => runCommand(cmd)"
              @mouseenter="() => selectedIndex = findCommandIndex(cmd)"
            >
              <span class="td-palette-item-label">{{ cmd.label }}</span>
              <TdKbdShortcut
                v-if="cmd.keys"
                :keys="cmd.keys"
              />
            </component>
          </template>

          <div
            v-if="totalCount === 0 && filterText"
            class="td-palette-empty"
          >
            No results for "{{ filterText }}"
          </div>
        </div>

        <!-- Footer -->
        <div class="td-palette-footer">
          <span><TdKbdShortcut :keys="[TdKeyName.ArrowUp]" /> <TdKbdShortcut :keys="[TdKeyName.ArrowDown]" /> navigate</span>
          <span><TdKbdShortcut :keys="[TdKeyName.Enter]" /> select</span>
          <span><TdKbdShortcut :keys="[TdKeyName.Escape]" /> close</span>
        </div>
      </div>
    </template>
  </Teleport>
</template>

<style scoped>
.td-palette-scrim {
  position: fixed;
  inset: 0;
  z-index: var(--layer-td-modal);
  background: var(--color-td-scrim);
}

.td-palette {
  position: fixed;
  top: 12vh;
  left: 50%;
  transform: translateX(-50%);
  width: min(720px, calc(100vw - 32px));
  z-index: var(--layer-td-modal);
  background: var(--color-td-bg);
  border: 1px solid var(--color-td-line);
  border-radius: 12px;
  box-shadow: 0 8px 32px rgba(0, 0, 0, 0.16);
  display: flex;
  flex-direction: column;
  overflow: hidden;
}

.td-palette-input-row {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 12px 14px;
  border-bottom: 1px solid var(--color-td-line);
  flex-shrink: 0;
}

.td-palette-input-icon {
  flex-shrink: 0;
  color: var(--color-td-ink-4);
}

.td-palette-input {
  flex: 1;
  min-width: 0;
  border: none;
  outline: none;
  background: none;
  font-size: var(--font-size-td-sm);
  color: var(--color-td-ink);
  font-family: inherit;
}

.td-palette-input::placeholder {
  color: var(--color-td-ink-4);
}

.td-palette-esc {
  display: inline-flex;
  align-items: center;
  padding: 3px 8px;
  border: 1px solid var(--color-td-line);
  border-radius: 4px;
  background: var(--color-td-bg);
  font-family: var(--font-sans);
  font-size: var(--font-size-td-2xs);
  color: var(--color-td-ink-4);
  cursor: pointer;
  flex-shrink: 0;
}

.td-palette-results {
  overflow-y: auto;
  max-height: calc(100vh - 24vh - 120px);
  padding: 6px 0;
}

.td-palette-group-label {
  padding: 12px 14px 4px;
  font-size: var(--font-size-td-2xs);
  font-weight: 600;
  letter-spacing: var(--tracking-td-wide);
  text-transform: uppercase;
  color: var(--color-td-ink-4);
}

.td-palette-item {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 8px;
  padding: 6px 8px;
  margin: 0 6px;
  border-radius: 8px;
  text-decoration: none;
  cursor: pointer;
  border: none;
  background: none;
  width: calc(100% - 12px);
  text-align: left;
  font-family: inherit;
  transition: background var(--duration-td-fast) var(--ease-td-out-quart);
}

.td-palette-item:hover,
.td-palette-item.is-selected {
  background: var(--color-td-hover);
}

.td-palette-item-label {
  flex: 1;
  min-width: 0;
  font-size: var(--font-size-td-sm);
  color: var(--color-td-ink);
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.td-palette-item-label :deep(mark) {
  background: none;
  color: var(--color-td-accent);
}

.td-palette-item-path {
  flex-shrink: 0;
  font-size: var(--font-size-td-2xs);
  color: var(--color-td-ink-4);
  font-family: var(--font-mono);
}

.td-palette-empty {
  padding: 16px 14px;
  font-size: var(--font-size-td-sm);
  color: var(--color-td-ink-4);
}

.td-palette-footer {
  display: flex;
  align-items: center;
  gap: 20px;
  padding: 10px 14px;
  border-top: 1px solid var(--color-td-line-soft);
  background: var(--color-td-surface);
  font-size: var(--font-size-td-xs);
  color: var(--color-td-ink-4);
  flex-shrink: 0;
}
</style>
