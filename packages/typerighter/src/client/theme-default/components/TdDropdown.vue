<script setup lang="ts">
import {
  onUnmounted, useTemplateRef, watch,
} from 'vue';

defineOptions({
  inheritAttrs: false,
});

const open = defineModel<boolean>('open', {
  default: false,
});

const wrapper = useTemplateRef<HTMLElement>('wrapper');

function handleClickOutside (event: MouseEvent) {
  if (wrapper.value?.contains(event.target as Node)) return;

  open.value = false;
}

function toggle () {
  open.value = !open.value;
}

watch(open, (isOpen) => {
  if (isOpen) {
    document.addEventListener('click', handleClickOutside);
  } else {
    document.removeEventListener('click', handleClickOutside);
  }
});

onUnmounted(() => {
  document.removeEventListener('click', handleClickOutside);
});
</script>

<template>
  <span
    ref="wrapper"
    v-bind="$attrs"
    class="td-dropdown"
  >
    <span @click="toggle">
      <slot name="trigger" />
    </span>
    <div
      v-if="open"
      class="td-dropdown-panel"
    >
      <slot />
    </div>
  </span>
</template>

<style scoped>
.td-dropdown {
  position: relative;
}

.td-dropdown-panel {
  position: absolute;
  top: 100%;
  left: 0;
  z-index: 50;
  min-width: 160px;
  margin-top: 4px;
  padding: 4px;
  background: var(--color-td-neutral-bg);
  border: 1px solid var(--color-td-neutral-border-subtle);
  border-radius: 8px;
  box-shadow: 0 4px 12px rgba(0, 0, 0, 0.1);
}
</style>
