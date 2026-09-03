<script setup lang="ts">
import {
  type Component, computed,
} from 'vue';
import {
  ArrowDown,
  ArrowLeft,
  ArrowRight,
  ArrowUp,
  Command,
  CornerDownLeft,
  Delete,
  Option,
  Space,
} from '@lucide/vue';
import {
  TdKeyName,
} from '../utils/keys';

const {
  keys,
} = defineProps<{
  /** Key combination to render */
  keys: TdKeyName[];
}>();

interface KeyMeta {
  label?: string;
  icon?: Component;
}

const ICON_KEYS: Partial<Record<TdKeyName, Component>> = {
  [TdKeyName.ArrowDown]: ArrowDown,
  [TdKeyName.ArrowUp]: ArrowUp,
  [TdKeyName.ArrowLeft]: ArrowLeft,
  [TdKeyName.ArrowRight]: ArrowRight,
  [TdKeyName.Meta]: Command,
  [TdKeyName.Alt]: Option,
  [TdKeyName.Enter]: CornerDownLeft,
  [TdKeyName.Delete]: Delete,
  [TdKeyName.Space]: Space,
};

const LABEL_KEYS: Partial<Record<TdKeyName, string>> = {
  [TdKeyName.Control]: '⌃',
  [TdKeyName.Shift]: '⇧',
  [TdKeyName.Escape]: 'Esc',
  [TdKeyName.Tab]: 'Tab',
  [TdKeyName.Backspace]: '⌫',
};

const keyMetas = computed((): KeyMeta[] =>
  keys.map((key) => {
    const icon = ICON_KEYS[key];

    if (icon)
      return {
        icon,
      };

    const label = LABEL_KEYS[key];

    if (label)
      return {
        label,
      };

    return {
      label: key.toUpperCase(),
    };
  }));
</script>

<template>
  <span class="td-kbd-shortcut">
    <kbd
      v-for="(meta, index) in keyMetas"
      :key="index"
    >
      <component
        :is="meta.icon"
        v-if="meta.icon"
        :size="12"
      />
      <template v-else>{{ meta.label }}</template>
    </kbd>
  </span>
</template>

<style scoped>
.td-kbd-shortcut {
  display: inline-flex;
  align-items: center;
  gap: 2px;
}

.td-kbd-shortcut kbd {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  min-width: 20px;
  height: 20px;
  padding: 0 4px;
  border: 1px solid var(--color-td-neutral-border-subtle);
  border-radius: 4px;
  background: var(--color-td-neutral-bg-subtle);
  font-family: var(--font-mono);
  font-size: var(--font-size-td-2xs);
  line-height: 1;
  color: var(--color-td-neutral-fg-muted);
}
</style>
