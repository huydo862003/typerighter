<script setup lang="ts">
import {
  Check,
  ClipboardCopy,
  Code,
  Download,
  Maximize,
  Minimize2,
  Minus,
  Plus,
  X,
} from '@lucide/vue';
import {
  computed,
  nextTick,
  onBeforeUnmount,
  onMounted,
  ref,
  useTemplateRef,
  watch,
} from 'vue';

const {
  svg,
  source,
  title = undefined,
  label,
  diagramId,
} = defineProps<{
  /** Rendered SVG markup */
  svg: string;
  /** Diagram source code */
  source: string;
  /** Optional caption */
  title?: string;
  /** Display label (Mermaid, D2) */
  label: string;
  /** Unique element id */
  diagramId: string;
}>();

const ZOOM_MIN = 0.15;
const ZOOM_MAX = 5;
const CLICK_THRESHOLD = 4;
const INLINE_FIT_PADDING = 0.9;
const MODAL_FIT_PADDING = 0.82;
const ZOOM_IN_FACTOR = 1.25;
const ZOOM_OUT_FACTOR = 0.8;
const DOUBLE_CLICK_FACTOR = 1.6;
const COPY_FEEDBACK_MS = 1400;
const GRID_BASE_SIZE = 20;

const IS_MAC =
  typeof navigator !== 'undefined' && /Mac/i.test(navigator.userAgent);
const MOD_KEY = IS_MAC ? '\u2318' : 'Ctrl';

const stageRef = useTemplateRef<HTMLElement>('stage');
const canvasRef = useTemplateRef<HTMLElement>('canvas');
const modalStageRef = useTemplateRef<HTMLElement>('modalStage');
const modalCanvasRef = useTemplateRef<HTMLElement>('modalCanvas');
const modalDialogRef = useTemplateRef<HTMLElement>('modalDialog');
const expandButtonRef = useTemplateRef<HTMLElement>('expandButton');

const isCodeVisible = ref(false);
const isModalOpen = ref(false);
const isModalCodeVisible = ref(false);
const inlineCopyFeedback = ref(false);
const modalCopyFeedback = ref(false);
const inlineTransform = ref({
  x: 0,
  y: 0,
  scale: 1,
});
const modalTransform = ref({
  x: 0,
  y: 0,
  scale: 1,
});

// Parse SVG content dimensions from the markup string (before DOM insertion)
const svgContentSize = computed(() => {
  const viewBoxMatch = svg.match(/viewBox=["']([^"']+)["']/);

  if (viewBoxMatch) {
    const parts = viewBoxMatch[1].split(/[\s,]+/);

    return {
      width: parseFloat(parts[2]),
      height: parseFloat(parts[3]),
    };
  }

  // Fall back to absolute width/height attributes
  const widthMatch = svg.match(/\bwidth=["'](\d+(?:\.\d+)?)(?:px)?["']/);
  const heightMatch = svg.match(/\bheight=["'](\d+(?:\.\d+)?)(?:px)?["']/);

  if (widthMatch && heightMatch) {
    return {
      width: parseFloat(widthMatch[1]),
      height: parseFloat(heightMatch[1]),
    };
  }

  return {
    width: 400,
    height: 300,
  };
});

// Derived state

const codeLines = computed(() => source.split('\n'));
const lineCount = computed(() => codeLines.value.length);
const inlineZoomPercent = computed(
  () => `${Math.round(inlineTransform.value.scale * 100)}%`,
);
const modalZoomPercent = computed(
  () => `${Math.round(modalTransform.value.scale * 100)}%`,
);

const fileExtension = computed(() => {
  if (label === 'Mermaid') return 'mmd';

  if (label === 'D2') return 'd2';

  return 'txt';
});

const canvasSizeStyle = computed(() => ({
  width: `${svgContentSize.value.width}px`,
  height: `${svgContentSize.value.height}px`,
}));

const inlineCanvasStyle = computed(() => {
  const {
    x, y, scale,
  } = inlineTransform.value;

  return {
    ...canvasSizeStyle.value,
    transform: `translate(${x}px, ${y}px) scale(${scale})`,
  };
});

const modalCanvasStyle = computed(() => {
  const {
    x, y, scale,
  } = modalTransform.value;

  return {
    ...canvasSizeStyle.value,
    transform: `translate(${x}px, ${y}px) scale(${scale})`,
  };
});

const modalGridStyle = computed(() => {
  const {
    x, y, scale,
  } = modalTransform.value;
  const size = GRID_BASE_SIZE * scale;

  return {
    backgroundSize: `${size}px ${size}px`,
    backgroundPosition: `${x % size}px ${y % size}px`,
  };
});

// Transform math

function clampScale (value: number): number {
  return Math.min(ZOOM_MAX, Math.max(ZOOM_MIN, value));
}

function closeModal (): void {
  isModalOpen.value = false;
  cleanupModalWheel?.();
  cleanupModalWheel = undefined;
  expandButtonRef.value?.focus();
}

function computeFitTransform (
  stageWidth: number,
  stageHeight: number,
  contentWidth: number,
  contentHeight: number,
  padding: number,
): {
  x: number;
  y: number;
  scale: number;
} {
  if (contentWidth <= 0 || contentHeight <= 0) {
    return {
      x: 0,
      y: 0,
      scale: 1,
    };
  }

  const scale =
    Math.min(stageWidth / contentWidth, stageHeight / contentHeight) * padding;

  return {
    x: (stageWidth - contentWidth * scale) / 2,
    y: (stageHeight - contentHeight * scale) / 2,
    scale,
  };
}

function copyInline (): void {
  copyToClipboard(inlineCopyFeedback);
}

function copyModal (): void {
  copyToClipboard(modalCopyFeedback);
}

// Shared actions parameterized by surface

function copyToClipboard (feedbackRef: typeof inlineCopyFeedback): void {
  navigator.clipboard.writeText(source).catch(() => {});

  feedbackRef.value = true;

  clearTimeout(copyTimer);
  copyTimer = setTimeout(() => {
    feedbackRef.value = false;
  }, COPY_FEEDBACK_MS);
}

function downloadSvgFile (): void {
  const svgElement = (modalCanvasRef.value ?? canvasRef.value)?.querySelector(
    'svg',
  );

  if (!svgElement) return;

  const clone = svgElement.cloneNode(true) as SVGElement;

  clone.setAttribute('xmlns', 'http://www.w3.org/2000/svg');
  clone.style.transform = '';
  clone.style.transformOrigin = '';

  const serialized = new XMLSerializer().serializeToString(clone);
  const blob = new Blob([serialized], {
    type: 'image/svg+xml',
  });
  const url = URL.createObjectURL(blob);
  const anchor = document.createElement('a');

  anchor.href = url;
  anchor.download = `${diagramId}.svg`;
  anchor.click();
  URL.revokeObjectURL(url);
}

function fitInline (): void {
  fitSurface(stageRef.value ?? undefined, inlineTransform, INLINE_FIT_PADDING);
}

// Inline actions

function fitModal (): void {
  fitSurface(
    modalStageRef.value ?? undefined,
    modalTransform,
    MODAL_FIT_PADDING,
  );
}

function fitSurface (
  stageElement: HTMLElement | undefined,
  transformRef: typeof inlineTransform,
  padding: number,
): void {
  if (!stageElement || stageElement.offsetWidth <= 0) return;

  transformRef.value = computeFitTransform(
    stageElement.offsetWidth,
    stageElement.offsetHeight,
    svgContentSize.value.width,
    svgContentSize.value.height,
    padding,
  );
}

function getPointerInStage (
  event: MouseEvent,
  stage: HTMLElement,
): {
  x: number;
  y: number;
} {
  const rect = stage.getBoundingClientRect();
  const scale = (stage.offsetWidth ? rect.width / stage.offsetWidth : 1) || 1;

  return {
    x: (event.clientX - rect.left) / scale,
    y: (event.clientY - rect.top) / scale,
  };
}

function handleBackdropClick (event: MouseEvent): void {
  if (
    (event.target as HTMLElement).classList.contains(
      'td-diagram-modal-backdrop',
    )
  ) {
    closeModal();
  }
}

function handleModalKeydown (event: KeyboardEvent): void {
  if (event.key === 'Escape') {
    event.stopPropagation();
    closeModal();
  } else if (event.key === '+' || event.key === '=') {
    modalZoomIn();
  } else if (event.key === '-') {
    modalZoomOut();
  } else if (event.key === '0') {
    fitModal();
  }
}

// Modal actions

function inlineZoomIn (): void {
  zoomCenter(stageRef.value ?? undefined, inlineTransform, ZOOM_IN_FACTOR);
}

function inlineZoomOut (): void {
  zoomCenter(stageRef.value ?? undefined, inlineTransform, ZOOM_OUT_FACTOR);
}

function modalZoomIn (): void {
  zoomCenter(modalStageRef.value ?? undefined, modalTransform, ZOOM_IN_FACTOR);
}

function modalZoomOut (): void {
  zoomCenter(modalStageRef.value ?? undefined, modalTransform, ZOOM_OUT_FACTOR);
}

function modalZoomReset (): void {
  const stage = modalStageRef.value;

  if (!stage) return;

  const resetScale = clampScale(1);

  modalTransform.value = zoomAtPoint(
    modalTransform.value,
    stage.offsetWidth / 2,
    stage.offsetHeight / 2,
    resetScale / modalTransform.value.scale,
  );
}

function openModal (): void {
  isModalOpen.value = true;
  isModalCodeVisible.value = isCodeVisible.value;
  modalTransform.value = {
    x: 0,
    y: 0,
    scale: 1,
  };

  // Wait for DOM update (nextTick), then layout (rAF) before fitting
  nextTick(() => {
    requestAnimationFrame(() => {
      fitModal();
      modalDialogRef.value?.focus({
        preventScroll: true,
      });
      attachModalWheel();
    });
  });
}

function showCode (): void {
  isCodeVisible.value = true;
}

function showPreview (): void {
  isCodeVisible.value = false;
}

function toggleModalCode (): void {
  isModalCodeVisible.value = !isModalCodeVisible.value;
}

function zoomAtPoint (
  transform: {
    x: number;
    y: number;
    scale: number;
  },
  cursorX: number,
  cursorY: number,
  factor: number,
): {
  x: number;
  y: number;
  scale: number;
} {
  const newScale = clampScale(transform.scale * factor);
  const ratio = newScale / transform.scale;

  return {
    x: cursorX - (cursorX - transform.x) * ratio,
    y: cursorY - (cursorY - transform.y) * ratio,
    scale: newScale,
  };
}

function zoomCenter (
  stageElement: HTMLElement | undefined,
  transformRef: typeof inlineTransform,
  factor: number,
): void {
  if (!stageElement) return;

  transformRef.value = zoomAtPoint(
    transformRef.value,
    stageElement.offsetWidth / 2,
    stageElement.offsetHeight / 2,
    factor,
  );
}

// Pan/zoom via pointer events (template-bound for inline, imperative for modal stage)

// Shared drag state per surface
const inlineDrag = {
  active: false,
  startClientX: 0,
  startClientY: 0,
  startTransformX: 0,
  startTransformY: 0,
  moved: false,
  clientScale: 1,
};

const modalDrag = {
  active: false,
  startClientX: 0,
  startClientY: 0,
  startTransformX: 0,
  startTransformY: 0,
  moved: false,
  clientScale: 1,
};

function attachInlineWheel (stage: HTMLElement): () => void {
  function handleWheel (event: WheelEvent) {
    if (!event.ctrlKey && !event.metaKey) return; // let page scroll

    event.preventDefault();

    const position = getPointerInStage(event, stage);
    const factor = Math.exp(-event.deltaY * 0.01);

    inlineTransform.value = zoomAtPoint(
      inlineTransform.value,
      position.x,
      position.y,
      factor,
    );
  }

  stage.addEventListener('wheel', handleWheel, {
    passive: false,
  });

  return () => stage.removeEventListener('wheel', handleWheel);
}

function attachModalWheel (): void {
  const stage = modalStageRef.value;

  if (!stage) return;

  function handleWheel (event: WheelEvent) {
    if (!stage) return;

    event.preventDefault();

    if (event.ctrlKey || event.metaKey) {
      const position = getPointerInStage(event, stage);
      const factor = Math.exp(-event.deltaY * 0.01);

      modalTransform.value = zoomAtPoint(
        modalTransform.value,
        position.x,
        position.y,
        factor,
      );
    } else {
      const rectScale =
        (stage.offsetWidth
          ? stage.getBoundingClientRect().width / stage.offsetWidth
          : 1) || 1;

      modalTransform.value = {
        x: modalTransform.value.x - event.deltaX / rectScale,
        y: modalTransform.value.y - event.deltaY / rectScale,
        scale: modalTransform.value.scale,
      };
    }
  }

  stage.addEventListener('wheel', handleWheel, {
    passive: false,
  });
  cleanupModalWheel = () => stage.removeEventListener('wheel', handleWheel);
}

function handlePointerDown (
  event: PointerEvent,
  drag: typeof inlineDrag,
  stageElement: HTMLElement,
  transformRef: typeof inlineTransform,
): void {
  if (event.button !== 0) return;

  drag.active = true;
  drag.moved = false;

  const rect = stageElement.getBoundingClientRect();

  drag.clientScale =
    (stageElement.offsetWidth ? rect.width / stageElement.offsetWidth : 1) || 1;
  drag.startClientX = event.clientX;
  drag.startClientY = event.clientY;
  drag.startTransformX = transformRef.value.x;
  drag.startTransformY = transformRef.value.y;
  stageElement.setPointerCapture(event.pointerId);
  stageElement.style.cursor = 'grabbing';
}

// Inline pointer events (bound in template)

function handlePointerEnd (
  event: PointerEvent,
  drag: typeof inlineDrag,
  stageElement: HTMLElement,
  onClickSurface?: () => void,
): void {
  if (!drag.active) return;

  drag.active = false;
  stageElement.releasePointerCapture(event.pointerId);
  stageElement.style.cursor = '';

  if (!drag.moved && onClickSurface) {
    onClickSurface();
  }
}

function handlePointerMove (
  event: PointerEvent,
  drag: typeof inlineDrag,
  transformRef: typeof inlineTransform,
): void {
  if (!drag.active) return;

  const deltaX = (event.clientX - drag.startClientX) / drag.clientScale;
  const deltaY = (event.clientY - drag.startClientY) / drag.clientScale;

  if (!drag.moved && CLICK_THRESHOLD <= Math.hypot(deltaX, deltaY)) {
    drag.moved = true;
  }

  if (drag.moved) {
    transformRef.value = {
      x: drag.startTransformX + deltaX,
      y: drag.startTransformY + deltaY,
      scale: transformRef.value.scale,
    };
  }
}

function onInlinePointerDown (event: PointerEvent): void {
  const stage = stageRef.value;

  if (stage) handlePointerDown(event, inlineDrag, stage, inlineTransform);
}

// Modal pointer events (bound in template)

function onInlinePointerEnd (event: PointerEvent): void {
  const stage = stageRef.value;

  if (stage) handlePointerEnd(event, inlineDrag, stage, openModal);
}

function onInlinePointerMove (event: PointerEvent): void {
  handlePointerMove(event, inlineDrag, inlineTransform);
}

function onModalDoubleClick (event: MouseEvent): void {
  const stage = modalStageRef.value;

  if (!stage) return;

  const position = getPointerInStage(event, stage);

  modalTransform.value = zoomAtPoint(
    modalTransform.value,
    position.x,
    position.y,
    DOUBLE_CLICK_FACTOR,
  );
}

function onModalPointerDown (event: PointerEvent): void {
  const stage = modalStageRef.value;

  if (stage) handlePointerDown(event, modalDrag, stage, modalTransform);
}

// Wheel zoom needs { passive: false }, so it must be imperative

function onModalPointerEnd (event: PointerEvent): void {
  const stage = modalStageRef.value;

  if (stage) handlePointerEnd(event, modalDrag, stage);
}

function onModalPointerMove (event: PointerEvent): void {
  handlePointerMove(event, modalDrag, modalTransform);
}

// Re-fit modal when toggling code panel
watch(isModalCodeVisible, () => {
  nextTick(() => fitModal());
});

// Lifecycle

let cleanupInlineWheel: (() => void) | undefined;
let cleanupModalWheel: (() => void) | undefined;
let copyTimer: ReturnType<typeof setTimeout> | undefined;

onMounted(() => {
  const stage = stageRef.value;

  if (!stage) return;

  // Initial fit once the element has layout
  const observer = new ResizeObserver(() => {
    if (0 < stage.offsetWidth && 0 < stage.offsetHeight) {
      fitInline();
      observer.disconnect();
    }
  });

  observer.observe(stage);
  cleanupInlineWheel = attachInlineWheel(stage);
});

onBeforeUnmount(() => {
  cleanupInlineWheel?.();
  cleanupModalWheel?.();
  clearTimeout(copyTimer);
});
</script>

<template>
  <figure
    :id="diagramId"
    class="td-diagram"
  >
    <div class="td-diagram-frame">
      <div
        v-show="!isCodeVisible"
        ref="stage"
        class="td-diagram-stage"
        @pointerdown="onInlinePointerDown"
        @pointermove="onInlinePointerMove"
        @pointerup="onInlinePointerEnd"
        @pointercancel="onInlinePointerEnd"
      >
        <div
          ref="canvas"
          class="td-diagram-canvas"
          :style="inlineCanvasStyle"
          v-html="svg"
        />
      </div>

      <pre
        v-show="isCodeVisible"
        class="td-diagram-code"
      >
        <div
v-for="(line, index) in codeLines"
:key="index"
class="td-diagram-code-row"
>
          <span class="td-diagram-code-number">{{ index + 1 }}</span>
          <span class="td-diagram-code-column">{{ line || ' ' }}</span>
        </div>
      </pre>

      <div class="td-diagram-toolbar">
        <span class="td-diagram-toolbar-label">{{ label }}</span>
        <div class="td-diagram-segment">
          <button
            type="button"
            class="td-diagram-segment-button"
            :class="{
              'is-active': !isCodeVisible,
            }"
            @click="showPreview"
          >
            Preview
          </button>
          <button
            type="button"
            class="td-diagram-segment-button"
            :class="{
              'is-active': isCodeVisible,
            }"
            @click="showCode"
          >
            Code
          </button>
        </div>
        <button
          type="button"
          class="td-diagram-button"
          :class="{
            'is-copied': inlineCopyFeedback,
          }"
          aria-label="Copy source"
          @click="copyInline"
        >
          <Check
            v-if="inlineCopyFeedback"
            :size="16"
          />
          <ClipboardCopy
            v-else
            :size="16"
          />
        </button>
        <button
          ref="expandButton"
          type="button"
          class="td-diagram-button"
          aria-label="Open full-screen preview"
          @click="openModal"
        >
          <Maximize :size="16" />
        </button>
      </div>

      <div
        v-show="!isCodeVisible"
        class="td-diagram-zoom"
      >
        <button
          type="button"
          class="td-diagram-button"
          aria-label="Zoom out"
          @click="inlineZoomOut"
        >
          <Minus :size="16" />
        </button>
        <button
          type="button"
          class="td-diagram-button td-diagram-zoom-level"
          aria-label="Reset zoom"
          @click="fitInline"
        >
          {{ inlineZoomPercent }}
        </button>
        <button
          type="button"
          class="td-diagram-button"
          aria-label="Zoom in"
          @click="inlineZoomIn"
        >
          <Plus :size="16" />
        </button>
      </div>

      <div
        v-show="!isCodeVisible"
        class="td-diagram-hint"
      >
        Click to expand &middot; Drag to pan &middot; {{ MOD_KEY }} + scroll to
        zoom
      </div>
    </div>

    <figcaption
      v-if="title"
      class="td-diagram-caption"
    >
      {{ title }}
    </figcaption>

    <Teleport to="body">
      <div
        v-if="isModalOpen"
        class="td-diagram-modal-backdrop"
        @click="handleBackdropClick"
      >
        <div
          ref="modalDialog"
          class="td-diagram-modal"
          role="dialog"
          :aria-label="title || `${label} diagram`"
          tabindex="-1"
          @keydown="handleModalKeydown"
        >
          <div class="td-diagram-modal-header">
            <div class="td-diagram-modal-header-left">
              <span class="td-diagram-modal-title">{{
                title || "Diagram"
              }}</span>
              <span class="td-diagram-modal-chip">{{ label }}</span>
            </div>
            <div class="td-diagram-modal-header-right">
              <button
                type="button"
                class="td-diagram-modal-button"
                :class="{
                  'is-active': isModalCodeVisible,
                }"
                :aria-pressed="isModalCodeVisible"
                aria-label="Toggle code"
                @click="toggleModalCode"
              >
                <Code :size="16" />
                <span class="td-diagram-button-label">Code</span>
              </button>
              <span class="td-diagram-modal-separator" />
              <button
                type="button"
                class="td-diagram-modal-button"
                aria-label="Zoom out"
                @click="modalZoomOut"
              >
                <Minus :size="16" />
              </button>
              <button
                type="button"
                class="td-diagram-modal-button td-diagram-modal-zoom-level"
                aria-label="Zoom to 100%"
                @click="modalZoomReset"
              >
                {{ modalZoomPercent }}
              </button>
              <button
                type="button"
                class="td-diagram-modal-button"
                aria-label="Zoom in"
                @click="modalZoomIn"
              >
                <Plus :size="16" />
              </button>
              <button
                type="button"
                class="td-diagram-modal-button"
                aria-label="Fit to view"
                @click="fitModal"
              >
                <Minimize2 :size="16" />
                <span class="td-diagram-button-label">Fit</span>
              </button>
              <span class="td-diagram-modal-separator" />
              <button
                type="button"
                class="td-diagram-modal-button"
                :class="{
                  'is-copied': modalCopyFeedback,
                }"
                aria-label="Copy source"
                @click="copyModal"
              >
                <Check
                  v-if="modalCopyFeedback"
                  :size="16"
                />
                <ClipboardCopy
                  v-else
                  :size="16"
                />
                <span class="td-diagram-button-label">{{
                  modalCopyFeedback ? "Copied" : "Copy"
                }}</span>
              </button>
              <button
                type="button"
                class="td-diagram-modal-button"
                aria-label="Download SVG"
                @click="downloadSvgFile"
              >
                <Download :size="16" />
                <span class="td-diagram-button-label">SVG</span>
              </button>
              <span class="td-diagram-modal-separator" />
              <button
                type="button"
                class="td-diagram-modal-button"
                aria-label="Close preview"
                @click="closeModal"
              >
                <X :size="16" />
              </button>
            </div>
          </div>

          <div class="td-diagram-modal-body">
            <div
              v-if="isModalCodeVisible"
              class="td-diagram-modal-code-panel"
            >
              <div class="td-diagram-modal-code-panel-header">
                diagram.{{ fileExtension }} &middot; {{ lineCount }} lines
              </div>
              <div class="td-diagram-modal-code-panel-body">
                <pre class="td-diagram-code td-diagram-code--modal">
                  <div
v-for="(line, index) in codeLines"
:key="index"
class="td-diagram-code-row"
>
                    <span class="td-diagram-code-number">{{ index + 1 }}</span>
                    <span class="td-diagram-code-column">{{ line || ' ' }}</span>
                  </div>
                </pre>
              </div>
            </div>

            <div
              ref="modalStage"
              class="td-diagram-modal-stage"
              :style="modalGridStyle"
              @pointerdown="onModalPointerDown"
              @pointermove="onModalPointerMove"
              @pointerup="onModalPointerEnd"
              @pointercancel="onModalPointerEnd"
              @dblclick="onModalDoubleClick"
            >
              <div
                ref="modalCanvas"
                class="td-diagram-modal-canvas"
                :style="modalCanvasStyle"
                v-html="svg"
              />
              <div class="td-diagram-modal-hint">
                Drag or scroll to pan &middot; {{ MOD_KEY }} + scroll or pinch
                to zoom &middot; Double-click to zoom in &middot; Esc to close
              </div>
            </div>
          </div>
        </div>
      </div>
    </Teleport>
  </figure>
</template>

<style scoped>
:deep(svg) {
  flex: none;
}

.td-diagram-button,
.td-diagram-modal-button {
  appearance: none;
  border: 0;
  background: transparent;
  color: var(--color-td-ink-3);
  height: 32px;
  min-width: 32px;
  padding: 0 8px;
  border-radius: var(--radius-td-md);
  display: inline-flex;
  align-items: center;
  justify-content: center;
  gap: 6px;
  font-family: var(--font-sans);
  font-size: 13px;
  font-weight: 500;
  cursor: pointer;
  white-space: nowrap;
}

.td-diagram-button:hover,
.td-diagram-modal-button:hover {
  background: var(--color-td-hover);
  color: var(--color-td-ink);
}

.td-diagram-button:focus-visible,
.td-diagram-modal-button:focus-visible {
  outline: 2px solid #2563eb;
  outline-offset: 1px;
}

.td-diagram-button.is-copied,
.td-diagram-modal-button.is-copied {
  color: var(--color-td-tip);
}

.td-diagram-button-label {
  pointer-events: none;
}

.td-diagram {
  margin: 16px 0;
  display: flex;
  flex-direction: column;
  gap: 8px;
}

.td-diagram-frame {
  position: relative;
  height: 480px;
  border: 1px solid var(--color-td-line);
  border-radius: 10px;
  background: var(--color-td-surface);
  overflow: hidden;
}

.td-diagram-toolbar,
.td-diagram-zoom,
.td-diagram-hint {
  opacity: 0;
  transition: opacity 140ms ease;
}

.td-diagram-frame:hover .td-diagram-toolbar,
.td-diagram-frame:hover .td-diagram-zoom,
.td-diagram-frame:hover .td-diagram-hint,
.td-diagram-frame:focus-within .td-diagram-toolbar,
.td-diagram-frame:focus-within .td-diagram-zoom,
.td-diagram-frame:focus-within .td-diagram-hint {
  opacity: 1;
}

.td-diagram-stage {
  position: absolute;
  inset: 0;
  user-select: none;
  cursor: zoom-in;
  touch-action: none;
}

.td-diagram-canvas {
  position: absolute;
  left: 0;
  top: 0;
  transform-origin: 0 0;
}

.td-diagram-canvas :deep(svg) {
  display: block;
  width: 100%;
  height: 100%;
}

.td-diagram-code {
  position: absolute;
  inset: 0;
  overflow: auto;
  margin: 0;
  padding: 16px 0;
  font-family: var(--font-mono);
  font-size: 13px;
  line-height: 1.75;
  color: var(--color-td-ink);
}

.td-diagram-code--modal {
  position: static;
  inset: auto;
  overflow: visible;
  padding: 0;
}

.td-diagram-code-row {
  display: flex;
}

.td-diagram-code-number {
  width: 52px;
  flex: none;
  text-align: right;
  padding-right: 18px;
  color: var(--color-td-ink-4);
  user-select: none;
}

.td-diagram-code--modal .td-diagram-code-number {
  width: 44px;
  padding-right: 16px;
}

.td-diagram-code-column {
  white-space: pre;
}

.td-diagram-toolbar {
  position: absolute;
  top: 10px;
  right: 10px;
  display: flex;
  align-items: center;
  gap: 4px;
  padding: 3px;
  border-radius: 9px;
  background: var(--color-td-bg);
  border: 1px solid var(--color-td-line);
  box-shadow: 0 2px 8px rgba(15, 15, 15, 0.06);
  z-index: 2;
}

.td-diagram-toolbar-label {
  font-family: var(--font-sans);
  font-size: 12px;
  color: var(--color-td-ink-3);
  padding: 0 8px;
  user-select: none;
}

.td-diagram-segment {
  display: flex;
  gap: 2px;
  padding: 2px;
  border-radius: 7px;
  background: var(--color-td-hover);
}

.td-diagram-segment-button {
  appearance: none;
  border: 0;
  background: transparent;
  height: 28px;
  padding: 0 10px;
  border-radius: 6px;
  font-family: var(--font-sans);
  font-size: 13px;
  font-weight: 500;
  color: var(--color-td-ink-3);
  cursor: pointer;
  white-space: nowrap;
}

.td-diagram-segment-button.is-active {
  background: var(--color-td-bg);
  color: var(--color-td-ink);
  box-shadow: 0 1px 2px rgba(15, 15, 15, 0.12);
}

.td-diagram-zoom-level {
  min-width: 52px;
  font-variant-numeric: tabular-nums;
}

.td-diagram-zoom {
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
}

.td-diagram-hint {
  position: absolute;
  left: 12px;
  bottom: 12px;
  display: flex;
  align-items: center;
  gap: 6px;
  padding: 6px 10px;
  border-radius: 8px;
  background: color-mix(in srgb, var(--color-td-bg) 92%, transparent);
  border: 1px solid var(--color-td-line);
  font-family: var(--font-sans);
  font-size: 12px;
  color: var(--color-td-ink-3);
  pointer-events: none;
  z-index: 2;
}

.td-diagram-caption {
  font-family: var(--font-sans);
  font-size: 13px;
  color: var(--color-td-ink-3);
  padding-left: 2px;
}

.td-diagram-modal-backdrop {
  position: fixed;
  inset: 0;
  z-index: var(--layer-td-modal);
  background: var(--color-td-scrim);
}

.td-diagram-modal {
  position: absolute;
  inset: 20px;
  background: var(--color-td-bg);
  border-radius: 14px;
  box-shadow: 0 24px 64px rgba(15, 15, 15, 0.28);
  display: flex;
  flex-direction: column;
  overflow: hidden;
  outline: none;
}

.td-diagram-modal-header {
  height: 56px;
  flex: none;
  box-sizing: border-box;
  border-bottom: 1px solid var(--color-td-line);
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 16px;
  padding: 0 12px 0 20px;
}

.td-diagram-modal-header-left {
  display: flex;
  align-items: center;
  gap: 10px;
  min-width: 0;
}

.td-diagram-modal-title {
  font-family: var(--font-sans);
  font-size: 15px;
  font-weight: 600;
  color: var(--color-td-ink);
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

.td-diagram-modal-chip {
  font-family: var(--font-sans);
  font-size: 12px;
  color: var(--color-td-ink-3);
  padding: 3px 8px;
  border-radius: var(--radius-td-md);
  background: var(--color-td-hover);
  white-space: nowrap;
}

.td-diagram-modal-header-right {
  display: flex;
  align-items: center;
  gap: 4px;
  flex-shrink: 0;
}

.td-diagram-modal-separator {
  width: 1px;
  height: 20px;
  background: var(--color-td-line);
  margin: 0 6px;
}

.td-diagram-modal-zoom-level {
  min-width: 56px;
  font-variant-numeric: tabular-nums;
}

.td-diagram-modal-button.is-active {
  background: color-mix(in srgb, var(--color-td-accent) 12%, transparent);
  color: var(--color-td-accent);
}

.td-diagram-modal-body {
  flex: 1;
  min-height: 0;
  display: flex;
}

.td-diagram-modal-code-panel {
  width: 400px;
  flex: none;
  border-right: 1px solid var(--color-td-line);
  background: var(--color-td-surface);
  display: flex;
  flex-direction: column;
  min-height: 0;
}

.td-diagram-modal-code-panel-header {
  height: 40px;
  flex: none;
  display: flex;
  align-items: center;
  padding: 0 16px;
  font-family: var(--font-mono);
  font-size: 12px;
  color: var(--color-td-ink-3);
  border-bottom: 1px solid var(--color-td-line);
}

.td-diagram-modal-code-panel-body {
  flex: 1;
  overflow: auto;
  padding: 14px 0;
}

.td-diagram-modal-stage {
  position: relative;
  flex: 1;
  min-width: 0;
  overflow: hidden;
  touch-action: none;
  user-select: none;
  background-color: var(--color-td-surface);
  background-image: radial-gradient(
    var(--color-td-line) 1px,
    transparent 1.2px
  );
  background-size: 20px 20px;
  cursor: grab;
}

.td-diagram-modal-stage:active {
  cursor: grabbing;
}

.td-diagram-modal-canvas {
  position: absolute;
  left: 0;
  top: 0;
  transform-origin: 0 0;
}

.td-diagram-modal-canvas :deep(svg) {
  display: block;
}

.td-diagram-modal-hint {
  position: absolute;
  left: 16px;
  bottom: 16px;
  padding: 7px 12px;
  border-radius: 8px;
  background: color-mix(in srgb, var(--color-td-bg) 94%, transparent);
  border: 1px solid var(--color-td-line);
  font-family: var(--font-sans);
  font-size: 12px;
  color: var(--color-td-ink-3);
  pointer-events: none;
}
</style>
