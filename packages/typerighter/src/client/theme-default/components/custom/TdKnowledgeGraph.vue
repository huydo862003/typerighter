<script setup lang="ts">
import {
  computed,
  onMounted, onUnmounted, ref, useTemplateRef, watch,
} from 'vue';
import {
  Minimize2,
  Minus,
  Plus,
  X,
} from '@lucide/vue';
import {
  linkGraph,
} from '@typedown/link-graph';
import {
  useRoute, useRouter, useSiteConfig,
} from '@/client/app';
import {
  getTdContentUrl,
} from '@/shared';

const {
  /** Render as full-screen overlay instead of inline */
  overlay = false,
} = defineProps<{
  /** Render as full-screen overlay instead of inline */
  overlay?: boolean;
}>();

const emit = defineEmits<{
  close: [];
}>();

const canvasRef = useTemplateRef<HTMLDivElement>('canvas');
const route = useRoute();
const router = useRouter();
const {
  withBase,
} = useSiteConfig();

const CDN_URL = 'https://cdn.jsdelivr.net/npm/force-graph@1/+esm';

const ZOOM_IN_FACTOR = 1.25;
const ZOOM_OUT_FACTOR = 1 / ZOOM_IN_FACTOR;
const GRID_BASE_SIZE = 20;

let graphInstance: any = null;

const currentZoom = ref(1);

const zoomPercent = computed(
  () => `${Math.round(currentZoom.value * 100)}%`,
);

// Schema -> color mapping
const SCHEMA_COLORS: Record<string, string> = {
  default: '#6B655C',
};

let colorIndex = 0;
const PALETTE = [
  '#B5461F',
  '#2457A6',
  '#2E7D32',
  '#8A5A00',
  '#7B1FA2',
  '#C62828',
  '#00695C',
  '#AD1457',
  '#1565C0',
  '#E65100',
];

function getSchemaColor (schema: string | undefined): string {
  const key = schema ?? 'default';

  if (!SCHEMA_COLORS[key]) {
    SCHEMA_COLORS[key] = PALETTE[colorIndex % PALETTE.length];
    colorIndex++;
  }

  return SCHEMA_COLORS[key];
}

function handleResize () {
  if (canvasRef.value && graphInstance) {
    graphInstance
      .width(canvasRef.value.clientWidth)
      .height(canvasRef.value.clientHeight);
  }
}

async function initializeGraph () {
  if (!canvasRef.value) return;

  const ForceGraph = await loadForceGraph();

  graphInstance = ForceGraph()(canvasRef.value);
  graphInstance
    .backgroundColor('transparent')
    .cooldownTicks(100)
    .warmupTicks(50);

  // Track zoom changes
  graphInstance.onZoom((transform: {
    k: number;
  }) => {
    currentZoom.value = transform.k;
  });

  renderGraph();
}

let forceGraphModule: any;

function closeOverlay () {
  emit('close');
}

function handleKeydown (event: KeyboardEvent) {
  if (event.key === 'Escape') {
    event.stopPropagation();
    closeOverlay();
  }
}

async function loadForceGraph (): Promise<any> {
  if (forceGraphModule) return forceGraphModule;

  const module = await import(/* @vite-ignore */ CDN_URL);

  forceGraphModule = module.default;

  return forceGraphModule;
}

function renderGraph () {
  if (!canvasRef.value || !graphInstance) return;

  const data = linkGraph.value;
  const currentPath = route.path;

  // Build node map for quick lookup
  const nodeMap = new Map(data.nodes.map((node) => [
    node.id,
    node,
  ]));

  graphInstance
    .graphData({
      nodes: data.nodes.map((node) => ({
        ...node,
        url: getTdContentUrl(node.id),
      })),
      links: data.edges.map((edge) => ({
        source: edge.source,
        target: edge.target,
      })),
    })
    .nodeLabel((node: any) => node.label)
    .nodeColor((node: any) => {
      const url = getTdContentUrl(node.id);

      if (url === currentPath) return '#B5461F';

      return getSchemaColor(nodeMap.get(node.id)?.schema);
    })
    .nodeVal((node: any) => {
      const url = getTdContentUrl(node.id);

      return url === currentPath ? 6 : 3;
    })
    .linkColor(() => '#E6E1D8')
    .linkWidth(1)
    .onNodeClick((node: any) => {
      const url = getTdContentUrl(node.id);

      router.go(withBase(url));
    })
    .width(canvasRef.value.clientWidth)
    .height(canvasRef.value.clientHeight);
}

function zoomFit () {
  if (!graphInstance) return;

  graphInstance.zoomToFit(400, 40);
}

function zoomIn () {
  if (!graphInstance) return;

  const current = currentZoom.value;

  graphInstance.zoom(current * ZOOM_IN_FACTOR, 300);
}

function zoomOut () {
  if (!graphInstance) return;

  const current = currentZoom.value;

  graphInstance.zoom(current * ZOOM_OUT_FACTOR, 300);
}

const overlayGridStyle = computed(() => {
  const scale = currentZoom.value;
  const size = GRID_BASE_SIZE * scale;

  return {
    backgroundSize: `${size}px ${size}px`,
  };
});

watch(linkGraph, renderGraph);
watch(() => route.path, renderGraph);

onMounted(() => {
  initializeGraph();
  window.addEventListener('resize', handleResize);
});

onUnmounted(() => {
  window.removeEventListener('resize', handleResize);

  if (graphInstance) {
    graphInstance._destructor?.();
    graphInstance = null;
  }
});
</script>

<template>
  <Teleport
    v-if="overlay"
    to="body"
  >
    <div
      class="td-kg-overlay"
      :style="overlayGridStyle"
      tabindex="-1"
      @keydown="handleKeydown"
    >
      <div
        ref="canvas"
        class="td-kg-overlay-canvas"
      />

      <div class="td-kg-overlay-controls">
        <button
          type="button"
          class="td-diagram-button"
          aria-label="Zoom out"
          @click="zoomOut"
        >
          <Minus :size="16" />
        </button>
        <button
          type="button"
          class="td-diagram-button td-kg-zoom-level"
          aria-label="Zoom level"
          @click="zoomFit"
        >
          {{ zoomPercent }}
        </button>
        <button
          type="button"
          class="td-diagram-button"
          aria-label="Zoom in"
          @click="zoomIn"
        >
          <Plus :size="16" />
        </button>
        <button
          type="button"
          class="td-diagram-button"
          aria-label="Fit to view"
          @click="zoomFit"
        >
          <Minimize2 :size="16" />
        </button>
        <button
          type="button"
          class="td-diagram-button"
          aria-label="Close"
          @click="closeOverlay"
        >
          <X :size="16" />
        </button>
      </div>
    </div>
  </Teleport>

  <div
    v-else
    class="td-knowledge-graph"
  >
    <div
      ref="canvas"
      class="td-knowledge-graph-canvas"
    />

    <div class="td-kg-zoom">
      <button
        type="button"
        class="td-diagram-button"
        aria-label="Zoom out"
        @click="zoomOut"
      >
        <Minus :size="16" />
      </button>
      <button
        type="button"
        class="td-diagram-button td-kg-zoom-level"
        aria-label="Zoom level"
        @click="zoomFit"
      >
        {{ zoomPercent }}
      </button>
      <button
        type="button"
        class="td-diagram-button"
        aria-label="Zoom in"
        @click="zoomIn"
      >
        <Plus :size="16" />
      </button>
    </div>
  </div>
</template>

<style scoped>
.td-diagram-button {
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

.td-diagram-button:hover {
  background: var(--color-td-hover);
  color: var(--color-td-ink);
}

.td-diagram-button:focus-visible {
  outline: 2px solid #2563eb;
  outline-offset: 1px;
}

.td-kg-zoom-level {
  min-width: 52px;
  font-variant-numeric: tabular-nums;
}

/* Inline mode */

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
  transition: opacity 140ms ease;
}

.td-knowledge-graph:hover .td-kg-zoom,
.td-knowledge-graph:focus-within .td-kg-zoom {
  opacity: 1;
}

/* Overlay mode */

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

.td-kg-overlay-controls {
  position: absolute;
  top: 12px;
  right: 12px;
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
  transition: opacity 140ms ease;
}

.td-kg-overlay:hover .td-kg-overlay-controls,
.td-kg-overlay:focus-within .td-kg-overlay-controls {
  opacity: 1;
}
</style>
