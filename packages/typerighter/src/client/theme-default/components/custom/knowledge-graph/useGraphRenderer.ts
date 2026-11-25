import {
  computed, onMounted, onUnmounted, ref, watch,
  type Ref, type ShallowRef,
} from 'vue';
import type {
  TdLinkGraph,
} from '@typerighter/rpc-client';
import {
  getSchemaColor,
} from './useGraphColors';
import {
  useRoute, useRouter, useSiteConfig,
} from '@/client/app';
import {
  getTdContentUrl,
} from '@/shared';

const CDN_URL = 'https://cdn.jsdelivr.net/npm/force-graph@1/+esm';
const ZOOM_FACTOR = 1.25;
const LABEL_ZOOM_THRESHOLD = 1.5;
const TWO_PI = Math.PI * 2;

type ForceGraphFactory = (container: HTMLDivElement) => ForceGraphInstance;

interface ForceGraphInstance {
  graphData: (data?: {
    nodes: GraphNode[];
    links: GraphLink[];
  }) => {
    nodes: GraphNode[];
    links: GraphLink[];
  };
  nodeLabel: (callback: (node: GraphNode) => string) => ForceGraphInstance;
  nodeCanvasObject: (callback: (node: GraphNode, context: CanvasRenderingContext2D, globalScale: number) => void) => ForceGraphInstance;
  nodePointerAreaPaint: (callback: (node: GraphNode, color: string, context: CanvasRenderingContext2D, globalScale: number) => void) => ForceGraphInstance;
  linkColor: (callback: ((link: GraphLink) => string) | string) => ForceGraphInstance;
  linkWidth: (callback: ((link: GraphLink) => number) | number) => ForceGraphInstance;
  linkLineDash: (callback: (link: GraphLink) => number[] | null) => ForceGraphInstance;
  onNodeHover: (callback: (node: GraphNode | null) => void) => ForceGraphInstance;
  onNodeClick: (callback: (node: GraphNode) => void) => ForceGraphInstance;
  onEngineStop: (callback: () => void) => ForceGraphInstance;
  onZoom: (callback: (transform: {
    k: number;
  }) => void) => ForceGraphInstance;
  backgroundColor: (color: string) => ForceGraphInstance;
  cooldownTicks: (ticks: number) => ForceGraphInstance;
  cooldownTime: (ms: number) => ForceGraphInstance;
  warmupTicks: (ticks: number) => ForceGraphInstance;
  d3AlphaDecay: (decay: number) => ForceGraphInstance;
  d3VelocityDecay: (decay: number) => ForceGraphInstance;
  d3Force: (name: string, force?: unknown) => ForceGraphInstance;
  autoPauseRedraw: (enable: boolean) => ForceGraphInstance;
  minZoom: (zoom: number) => ForceGraphInstance;
  maxZoom: (zoom: number) => ForceGraphInstance;
  width: (width: number) => ForceGraphInstance;
  height: (height: number) => ForceGraphInstance;
  zoom: (level: number, duration?: number) => ForceGraphInstance;
  zoomToFit: (duration?: number, padding?: number) => ForceGraphInstance;
  centerAt: (x: number, y: number, duration?: number) => ForceGraphInstance;
  _destructor?: () => void;
}

// Force-graph link (source/target can be string before simulation, object after)
interface GraphLink {
  source: string | GraphNode;
  target: string | GraphNode;
  kind: string;
}

// Force-graph node after simulation (has x/y coordinates)
interface GraphNode {
  id: string;
  label: string;
  schema?: string;
  nodeType: 'page' | 'index';
  url: string;
  // Pre-computed at data load time to avoid per-frame work
  __color?: string;
  __radius?: number;
  // Set by force simulation
  x?: number;
  y?: number;
  // Neighbor lookup for hover highlighting
  __neighbors?: Set<string>;
}

let forceGraphPromise: Promise<ForceGraphFactory> | undefined;

export function useGraphRenderer (
  canvasRef: Readonly<ShallowRef<HTMLDivElement | null>>,
  filteredData: Ref<TdLinkGraph>,
  searchQuery: Ref<string>,
) {
  const route = useRoute();
  const router = useRouter();
  const {
    withBase,
  } = useSiteConfig();

  let graph: ForceGraphInstance | undefined;
  let isLargeGraph = false;
  const currentZoom = ref(1);
  const hoveredNode = ref<string | undefined>();
  const zoomPercent = computed(() => `${Math.round(currentZoom.value * 100)}%`);

  async function initialize () {
    if (!canvasRef.value) return;

    const createGraph = await loadForceGraph();

    if (!canvasRef.value) return;

    const nodeCount = filteredData.value.nodes.length;

    isLargeGraph = 300 < nodeCount;

    graph = createGraph(canvasRef.value)
      .backgroundColor('transparent')
      .warmupTicks(isLargeGraph ? 30 : 15)
      .cooldownTime(isLargeGraph ? 8000 : 15000)
      .d3AlphaDecay(isLargeGraph ? 0.05 : 0.03)
      .d3VelocityDecay(isLargeGraph ? 0.6 : 0.5)
      .autoPauseRedraw(true)
      .minZoom(0.1)
      .maxZoom(10)
      .nodeLabel((node: GraphNode) => node.label)
      .nodeCanvasObject(paintNode)
      .nodePointerAreaPaint(paintNodeHitArea)
      .linkColor(colorLink)
      .linkWidth(1)
      .linkLineDash(dashLink)
      .onNodeHover(handleNodeHover)
      .onNodeClick((node: GraphNode) => router.go(withBase(getTdContentUrl(node.id))))
      .onEngineStop(() => graph?.zoomToFit(400, 40))
      .onZoom((transform) => {
        currentZoom.value = transform.k;
      });

    handleResize();
    updateData();
  }

  // Pre-compute colors, radii, and neighbor sets at data load time
  function updateData () {
    if (!canvasRef.value || !graph) return;

    const data = filteredData.value;

    isLargeGraph = 300 < data.nodes.length;

    const nodes: GraphNode[] = data.nodes.map((node) => {
      const graphNode: GraphNode = {
        ...node,
        url: getTdContentUrl(node.id),
        __color: getSchemaColor(node.schema),
        __radius: node.nodeType === 'index' ? 3 : 4,
        __neighbors: new Set<string>(),
      };

      return graphNode;
    });

    const nodeIndex = new Map(nodes.map((node) => [
      node.id,
      node,
    ]));

    const links: GraphLink[] = data.edges.map((edge) => {
      // Build neighbor sets for hover highlighting
      nodeIndex.get(edge.source)?.__neighbors?.add(edge.target);
      nodeIndex.get(edge.target)?.__neighbors?.add(edge.source);

      return {
        source: edge.source,
        target: edge.target,
        kind: edge.kind,
      };
    });

    graph.graphData({
      nodes,
      links,
    });
  }

  function paintNode (node: GraphNode, context: CanvasRenderingContext2D, globalScale: number) {
    const currentPath = route.path;
    const query = searchQuery.value.toLowerCase();
    const isCurrent = getTdContentUrl(node.id) === currentPath;
    const isMatch = query && node.label.toLowerCase().includes(query);
    const isHoverNeighbor = hoveredNode.value
      && (hoveredNode.value === node.id || node.__neighbors?.has(hoveredNode.value));

    let radius = node.__radius ?? 4;
    let color = node.__color ?? '#6B655C';

    if (isCurrent) {
      radius = 6;
      color = '#B5461F';
    } else if (isMatch) {
      radius = 5;
    } else if (query) {
      color = 'rgba(180, 175, 165, 0.3)';
    } else if (isHoverNeighbor) {
      radius = 5;
    }

    context.beginPath();
    context.arc(node.x!, node.y!, radius, 0, TWO_PI);
    context.fillStyle = color;
    context.fill();

    // Show label when zoomed in enough or for special nodes
    if (LABEL_ZOOM_THRESHOLD < globalScale || isCurrent || isHoverNeighbor) {
      const fontSize = Math.max(10 / globalScale, 1.5);

      context.font = `${fontSize}px sans-serif`;
      context.textAlign = 'center';
      context.textBaseline = 'top';
      context.fillStyle = isCurrent ? '#B5461F' : 'rgba(60, 55, 48, 0.8)';
      context.fillText(node.label, node.x!, node.y! + radius + 2);
    }
  }

  function paintNodeHitArea (node: GraphNode, color: string, context: CanvasRenderingContext2D) {
    const radius = (node.__radius ?? 4) + 2;

    context.beginPath();
    context.arc(node.x!, node.y!, radius, 0, TWO_PI);
    context.fillStyle = color;
    context.fill();
  }

  function colorLink (link: GraphLink): string {
    const {
      source, target,
    } = extractLinkEndpoints(link);
    const hovered = hoveredNode.value;
    const highlighted = hovered === source || hovered === target;

    if (highlighted) return link.kind === 'hierarchy' ? '#B5461F' : '#7B8794';

    // Hide non-highlighted reference links on large graphs when zoomed out
    if (isLargeGraph && currentZoom.value < 0.5) return 'transparent';

    return link.kind === 'hierarchy' ? '#D4CFC6' : 'rgba(180, 175, 165, 0.3)';
  }

  function dashLink (link: GraphLink): number[] | null {
    return link.kind === 'reference'
      ? [
        4,
        2,
      ]
      : null;
  }

  function handleNodeHover (node: GraphNode | null) {
    hoveredNode.value = node?.id;

    if (canvasRef.value) {
      canvasRef.value.style.cursor = node ? 'pointer' : '';
    }
  }

  function focusCurrentPage () {
    if (!graph) return;

    const currentPath = route.path;
    const {
      nodes,
    } = graph.graphData();
    const node = nodes.find((item) => getTdContentUrl(item.id) === currentPath);

    if (node?.x !== undefined && node?.y !== undefined) {
      graph.centerAt(node.x, node.y, 400);
      graph.zoom(2, 400);
    }
  }

  function handleResize () {
    if (!canvasRef.value || !graph) return;

    const width = canvasRef.value.clientWidth || window.innerWidth;
    const height = canvasRef.value.clientHeight || window.innerHeight;

    graph.width(width).height(height);
  }

  function zoomFit () {
    graph?.zoomToFit(400, 40);
  }

  function zoomIn () {
    graph?.zoom(currentZoom.value * ZOOM_FACTOR, 300);
  }

  function zoomOut () {
    graph?.zoom(currentZoom.value / ZOOM_FACTOR, 300);
  }

  watch(filteredData, updateData);

  onMounted(() => {
    initialize();
    window.addEventListener('resize', handleResize);
  });

  onUnmounted(() => {
    window.removeEventListener('resize', handleResize);
    graph?._destructor?.();
    graph = undefined;
  });

  return {
    currentZoom,
    zoomPercent,
    focusCurrentPage,
    zoomFit,
    zoomIn,
    zoomOut,
  };
}

function extractLinkEndpoints (link: GraphLink): {
  source: string;
  target: string;
} {
  return {
    source: typeof link.source === 'string' ? link.source : link.source.id,
    target: typeof link.target === 'string' ? link.target : link.target.id,
  };
}

function loadForceGraph (): Promise<ForceGraphFactory> {
  if (!forceGraphPromise) {
    // force-graph uses kapsule: module.default is a component definition,
    // calling it with no args returns the actual factory (container => instance)
    forceGraphPromise = import(/* @vite-ignore */ CDN_URL)
      .then((module) => module.default() as ForceGraphFactory);
  }

  return forceGraphPromise;
}
