<script setup lang="ts">
import {
  ref, onBeforeUnmount,
  useTemplateRef,
} from 'vue';

const {
  text,
} = defineProps<{
  /** Tooltip text, shown only when the slot content is truncated */
  text: string;
}>();

const wrapper = useTemplateRef<HTMLElement>('wrapper');
const visible = ref(false);
const tipStyle = ref({
  top: '0px',
  left: '0px',
});
let hideTimeout: ReturnType<typeof setTimeout> | undefined;

function clearHideTimeout () {
  if (hideTimeout !== undefined) {
    clearTimeout(hideTimeout);
    hideTimeout = undefined;
  }
}

function hide () {
  hideTimeout = setTimeout(() => {
    visible.value = false;
  }, 100);
}

function show () {
  const element = wrapper.value;

  if (!element || element.scrollWidth <= element.clientWidth) return;

  clearHideTimeout();

  const rect = element.getBoundingClientRect();

  tipStyle.value = {
    top: `${rect.bottom + 6}px`,
    left: `${rect.left}px`,
  };
  visible.value = true;
}

onBeforeUnmount(() => {
  clearHideTimeout();
});
</script>

<template>
  <span
    ref="wrapper"
    class="td-tooltip-trigger"
    @mouseenter="show"
    @mouseleave="hide"
  >
    <slot />
  </span>
  <Teleport to="body">
    <span
      v-if="visible"
      class="td-tooltip"
      :style="tipStyle"
      @mouseenter="clearHideTimeout"
      @mouseleave="hide"
    >{{ text }}</span>
  </Teleport>
</template>

<style scoped>
.td-tooltip-trigger {
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  min-width: 0;
}
</style>

<!-- eslint-disable vue/enforce-style-attribute "We use teleport" -->
<style>
.td-tooltip {
  position: fixed;
  z-index: 1000;
  max-width: 300px;
  padding: 4px 8px;
  padding: 7px 12px 6px;
  font-size: 0.8125rem;
  line-height: 1.4;
  color: #fff;
  background: rgba(0, 0, 0, 0.8);
  border-radius: 6px;
  pointer-events: auto;
  word-break: break-word;
  white-space: normal;
}
</style>
<!-- eslint-enable vue/enforce-style-attribute -->
