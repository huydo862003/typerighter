<script setup lang="ts">
import {
  onBeforeUnmount, ref, useTemplateRef,
} from 'vue';
import {
  useFloating, offset, flip, shift, arrow,
} from '@floating-ui/vue';

defineOptions({
  inheritAttrs: false,
});

const {
  text,
} = defineProps<{
  /** Tooltip text, shown only when the slot content is truncated */
  text: string;
}>();

const trigger = useTemplateRef<HTMLElement>('trigger');
const tooltip = useTemplateRef<HTMLElement>('tooltip');
const arrowRef = useTemplateRef<HTMLElement>('arrowEl');
const visible = ref(false);
let showTimeout: ReturnType<typeof setTimeout> | undefined;
let hideTimeout: ReturnType<typeof setTimeout> | undefined;

const {
  floatingStyles, middlewareData, placement,
} = useFloating(trigger, tooltip, {
  placement: 'right',
  middleware: [
    offset(8),
    flip(),
    shift({
      padding: 8,
    }),
    arrow({
      element: arrowRef,
    }),
  ],
});

function clearTimeouts () {
  if (showTimeout !== undefined) {
    clearTimeout(showTimeout);
    showTimeout = undefined;
  }
  if (hideTimeout !== undefined) {
    clearTimeout(hideTimeout);
    hideTimeout = undefined;
  }
}

function show () {
  const element = trigger.value;

  if (!element || element.scrollWidth <= element.clientWidth) return;

  clearTimeouts();
  showTimeout = setTimeout(() => {
    visible.value = true;
  }, 400);
}

onBeforeUnmount(clearTimeouts);

function hide () {
  clearTimeouts();
  hideTimeout = setTimeout(() => {
    visible.value = false;
  }, 100);
}
</script>

<template>
  <span
    ref="trigger"
    v-bind="$attrs"
    class="td-tooltip-trigger"
    @mouseenter="show"
    @mouseleave="hide"
  >
    <slot />
  </span>
  <Teleport to="body">
    <div
      v-if="visible"
      ref="tooltip"
      class="td-tooltip"
      :style="floatingStyles"
      role="tooltip"
      @mouseenter="clearTimeouts"
      @mouseleave="hide"
    >
      {{ text }}
      <div
        ref="arrowEl"
        class="td-tooltip-arrow"
        :style="{
          left: middlewareData.arrow?.x === undefined ? '' : `${middlewareData.arrow.x}px`,
          top: middlewareData.arrow?.y === undefined ? '' : `${middlewareData.arrow.y}px`,
          ...(placement.startsWith('bottom')
            ? {
              top: '-4px',
            }
            : {}),
          ...(placement.startsWith('top')
            ? {
              bottom: '-4px',
            }
            : {}),
          ...(placement.startsWith('right')
            ? {
              left: '-4px',
            }
            : {}),
          ...(placement.startsWith('left')
            ? {
              right: '-4px',
            }
            : {}),
        }"
      />
    </div>
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
  position: absolute;
  z-index: 1000;
  max-width: 300px;
  padding: 7px 12px 6px;
  font-size: var(--font-size-td-xs);
  line-height: 1.4;
  color: var(--color-td-tooltip-fg);
  background: var(--color-td-tooltip-bg);
  border: 1px solid var(--color-td-border);
  border-radius: 6px;
  pointer-events: auto;
  word-break: break-word;
  white-space: normal;
  box-shadow: 0 2px 8px rgba(0, 0, 0, 0.12);
}

.td-tooltip-arrow {
  position: absolute;
  width: 8px;
  height: 8px;
  background: var(--color-td-tooltip-bg);
  transform: rotate(45deg);
}
</style>
<!-- eslint-enable vue/enforce-style-attribute -->
