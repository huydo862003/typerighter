<script setup lang="ts">
import {
  Crosshair,
  Maximize,
  Minus,
  Plus,
  SlidersHorizontal,
  X,
} from '@lucide/vue';
import {
  inject, useTemplateRef,
} from 'vue';
import {
  getSchemaColor,
} from './useGraphColors';
import {
  useGraphFilter,
} from './useGraphFilter';
import {
  useGraphRenderer,
} from './useGraphRenderer';
import {
  TdKeyName,
} from '@/client/theme-default/utils/keys';
import TdTooltip from '@/client/theme-default/components/TdTooltip.vue';

const {
  overlay = false,
} = defineProps<{
  /** Render as full-screen overlay instead of inline */
  overlay?: boolean;
}>();

const emit = defineEmits<{
  close: [];
}>();

const canvasRef = useTemplateRef<HTMLDivElement>('canvas');

const {
  searchQuery, hiddenSchemas, schemaNames, filteredData, toggleSchema,
} =
  useGraphFilter();

const {
  zoomPercent, focusCurrentPage, zoomFit, zoomIn, zoomOut,
} =
  useGraphRenderer(canvasRef, filteredData, searchQuery);

const openGraphOverlay = inject<(() => void) | undefined>(
  'openGraphOverlay',
  undefined,
);

function closeOverlay () {
  emit('close');
}
</script>

<template>
  <!-- Overlay mode -->
  <Teleport
    v-if="overlay"
    to="body"
  >
    <Transition
      appear
      name="td-kg-fade"
    >
      <div
        class="td-kg-overlay"
        tabindex="-1"
        @keydown.escape.stop="closeOverlay"
      >
        <div
          ref="canvas"
          class="td-kg-overlay-canvas"
        />

        <!-- Right rail: circle that expands to panel on hover -->
        <div class="td-kg-rail">
          <!-- Circle trigger -->
          <div class="td-kg-rail-trigger">
            <SlidersHorizontal :size="18" />
          </div>
          <!-- Expanded panel -->
          <div class="td-kg-rail-panel">
            <div class="td-kg-panel-header">
              <span class="td-kg-panel-title">Knowledge graph</span>
              <button
                type="button"
                class="td-kg-close-btn"
                aria-label="Close"
                @click="closeOverlay"
              >
                <X :size="16" />
              </button>
            </div>
            <input
              v-model="searchQuery"
              class="td-kg-search"
              type="text"
              placeholder="Filter nodes..."
              @keydown.escape.stop
            >
            <div
              v-if="schemaNames.length > 0"
              class="td-kg-filters"
            >
              <label
                v-for="schema in schemaNames"
                :key="schema"
                class="td-kg-filter"
              >
                <input
                  type="checkbox"
                  :checked="!hiddenSchemas.has(schema)"
                  @change="() => toggleSchema(schema)"
                >
                <span
                  class="td-kg-filter-dot"
                  :style="{
                    background: getSchemaColor(schema),
                  }"
                />
                <span>{{ schema }}</span>
              </label>
            </div>
          </div>
        </div>

        <!-- Bottom center: zoom controls -->
        <div class="td-kg-bottom-bar">
          <TdTooltip
            text="Focus current page"
            placement="top"
            always
          >
            <button
              type="button"
              class="td-diagram-button"
              aria-label="Focus current page"
              @click="focusCurrentPage"
            >
              <Crosshair :size="16" />
            </button>
          </TdTooltip>
          <TdTooltip
            text="Fit to view"
            placement="top"
            always
          >
            <button
              type="button"
              class="td-diagram-button"
              aria-label="Fit to view"
              @click="zoomFit"
            >
              <Maximize :size="16" />
            </button>
          </TdTooltip>
          <TdTooltip
            text="Zoom out"
            placement="top"
            always
          >
            <button
              type="button"
              class="td-diagram-button"
              @click="zoomOut"
            >
              <Minus :size="16" />
            </button>
          </TdTooltip>
          <TdTooltip
            text="Zoom level"
            placement="top"
            always
          >
            <button
              type="button"
              class="td-diagram-button td-kg-zoom-level"
              @click="zoomFit"
            >
              {{ zoomPercent }}
            </button>
          </TdTooltip>
          <TdTooltip
            text="Zoom in"
            placement="top"
            always
          >
            <button
              type="button"
              class="td-diagram-button"
              @click="zoomIn"
            >
              <Plus :size="16" />
            </button>
          </TdTooltip>
        </div>
      </div>
    </Transition>
  </Teleport>

  <!-- Inline mode -->
  <div
    v-else
    class="td-knowledge-graph"
  >
    <div
      ref="canvas"
      class="td-knowledge-graph-canvas"
    />

    <div class="td-kg-zoom">
      <TdTooltip
        text="Zoom out"
        placement="top"
        always
      >
        <button
          type="button"
          class="td-diagram-button"
          aria-label="Zoom out"
          @click="zoomOut"
        >
          <Minus :size="16" />
        </button>
      </TdTooltip>
      <TdTooltip
        text="Zoom level"
        placement="top"
        always
      >
        <button
          type="button"
          class="td-diagram-button td-kg-zoom-level"
          aria-label="Zoom level"
          @click="zoomFit"
        >
          {{ zoomPercent }}
        </button>
      </TdTooltip>
      <TdTooltip
        text="Zoom in"
        placement="top"
        always
      >
        <button
          type="button"
          class="td-diagram-button"
          aria-label="Zoom in"
          @click="zoomIn"
        >
          <Plus :size="16" />
        </button>
      </TdTooltip>
      <TdTooltip
        v-if="openGraphOverlay"
        text="Open fullscreen"
        placement="top"
        always
      >
        <button
          type="button"
          class="td-diagram-button"
          aria-label="Open fullscreen"
          @click="openGraphOverlay"
        >
          <Maximize :size="16" />
        </button>
      </TdTooltip>
    </div>
  </div>
</template>

<style scoped>
@import "./graph-controls.css";

.td-knowledge-graph {
  position: relative;
  width: 100%;
  height: 500px;
  border: 1px solid var(--color-td-line);
  border-radius: var(--radius-td-lg);
  overflow: hidden;
  background: var(--color-td-surface);
}

.td-knowledge-graph-canvas {
  width: 100%;
  height: 100%;
}

.td-kg-zoom {
  position: absolute;
  right: 12px;
  bottom: 12px;
  display: flex;
  align-items: center;
  gap: 2px;
  padding: 3px;
  border-radius: 9px;
  background: var(--color-td-bg);
  border: 1px solid var(--color-td-line);
  box-shadow: 0 2px 8px rgba(15, 15, 15, 0.06);
  z-index: 2;
  opacity: 0;
  transition: opacity var(--duration-td-fast) var(--ease-td-out-quart);
}

.td-knowledge-graph:hover .td-kg-zoom,
.td-knowledge-graph:focus-within .td-kg-zoom {
  opacity: 1;
}

.td-kg-overlay {
  position: fixed;
  inset: 0;
  z-index: var(--layer-td-modal);
  background-color: var(--color-td-surface);
  background-image: radial-gradient(
    var(--color-td-line) 1px,
    transparent 1.2px
  );
  background-size: 20px 20px;
  outline: none;
}

.td-kg-overlay-canvas {
  width: 100%;
  height: 100%;
}

/* Right rail: circle trigger that expands to sidebar */
.td-kg-rail {
  position: absolute;
  top: 16px;
  right: 16px;
  z-index: 2;
}

/* Circle trigger - visible by default, hides when panel opens */
.td-kg-rail-trigger {
  width: 40px;
  height: 40px;
  border-radius: 50%;
  background: var(--color-td-bg);
  border: 1px solid var(--color-td-line);
  box-shadow: 0 2px 8px rgba(15, 15, 15, 0.08);
  display: flex;
  align-items: center;
  justify-content: center;
  color: var(--color-td-ink-3);
  cursor: pointer;
  transition:
    transform var(--duration-td-move) var(--ease-td-out-quart),
    opacity var(--duration-td-fast) var(--ease-td-out-quart);
}

.td-kg-rail:hover .td-kg-rail-trigger {
  transform: rotate(90deg);
  opacity: 0;
  pointer-events: none;
}

/* Expanded panel - hidden by default, slides in on hover */
.td-kg-rail-panel {
  position: absolute;
  top: 0;
  right: 0;
  width: 220px;
  display: flex;
  flex-direction: column;
  gap: 8px;
  padding: 12px;
  border-radius: 12px;
  background: var(--color-td-bg);
  border: 1px solid var(--color-td-line);
  box-shadow: 0 4px 16px rgba(15, 15, 15, 0.1);
  opacity: 0;
  transform: translateX(8px);
  pointer-events: none;
  transition:
    opacity var(--duration-td-move) var(--ease-td-out-quart),
    transform var(--duration-td-move) var(--ease-td-out-quart);
}

.td-kg-rail:hover .td-kg-rail-panel,
.td-kg-rail:focus-within .td-kg-rail-panel {
  opacity: 1;
  transform: translateX(0);
  pointer-events: auto;
}

.td-kg-search {
  width: 100%;
  padding: 7px 10px;
  border: 1px solid var(--color-td-line);
  border-radius: 6px;
  background: var(--color-td-surface);
  font: inherit;
  font-size: 12px;
  color: var(--color-td-ink);
  outline: none;
}

.td-kg-search:focus {
  border-color: var(--color-td-accent);
}

.td-kg-search::placeholder {
  color: var(--color-td-ink-4);
}

.td-kg-filters {
  display: flex;
  flex-direction: column;
  gap: 2px;
}

.td-kg-filter {
  display: flex;
  align-items: center;
  gap: 6px;
  padding: 2px 0;
  font-size: 12px;
  color: var(--color-td-ink-3);
  cursor: pointer;
}

.td-kg-filter input[type="checkbox"] {
  margin: 0;
  width: 13px;
  height: 13px;
  accent-color: var(--color-td-accent);
}

.td-kg-filter-dot {
  width: 8px;
  height: 8px;
  border-radius: 50%;
  flex-shrink: 0;
}

.td-kg-panel-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
}

.td-kg-panel-title {
  font-size: 13px;
  font-weight: 600;
  color: var(--color-td-ink-2);
}

.td-kg-close-btn {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  flex-shrink: 0;
  width: 28px;
  height: 28px;
  padding: 0;
  border: 1px solid var(--color-td-line);
  border-radius: 6px;
  background: none;
  color: var(--color-td-ink-3);
  cursor: pointer;
}

.td-kg-close-btn:hover {
  background: var(--color-td-hover);
}

@media (prefers-reduced-motion: reduce) {
  .td-kg-rail-trigger,
  .td-kg-rail-panel {
    transition: none;
  }
}

/* Bottom center: zoom + focus controls */
.td-kg-bottom-bar {
  position: absolute;
  bottom: 16px;
  left: 50%;
  transform: translateX(-50%);
  display: flex;
  align-items: center;
  gap: 2px;
  padding: 3px;
  border-radius: 9px;
  background: var(--color-td-bg);
  border: 1px solid var(--color-td-line);
  box-shadow: 0 2px 8px rgba(15, 15, 15, 0.06);
  z-index: 2;
  opacity: 0;
  transition: opacity var(--duration-td-fast) var(--ease-td-out-quart);
}

.td-kg-overlay:hover .td-kg-bottom-bar,
.td-kg-overlay:focus-within .td-kg-bottom-bar {
  opacity: 1;
}

/* Overlay fade transition */
.td-kg-fade-enter-active {
  transition: opacity var(--duration-td-open) var(--ease-td-out-quart);
}

.td-kg-fade-leave-active {
  transition: opacity var(--duration-td-fast) var(--ease-td-out-quart);
}

.td-kg-fade-enter-from,
.td-kg-fade-leave-to {
  opacity: 0;
}
</style>
