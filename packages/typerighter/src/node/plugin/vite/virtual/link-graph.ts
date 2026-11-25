import type {
  ViteDevServer,
} from 'vite';
import {
  LINK_GRAPH_EVENT,
} from '../constants';
import type {
  TypedownContext,
} from '@/node/lib/typedown-context';

export interface LinkGraph {
  nodes: LinkGraphNode[];
  edges: LinkGraphEdge[];
}

export interface LinkGraphEdge {
  source: string;
  target: string;
}

export interface LinkGraphNode {
  id: string;
  label: string;
  schema?: string;
}

const EMPTY: LinkGraph = {
  nodes: [],
  edges: [],
};

export class VirtualLinkGraph {
  private data: LinkGraph = EMPTY;

  load (): string {
    const payload = JSON.stringify(this.data);

    return `
import { shallowRef } from 'vue'
export const linkGraph = shallowRef(${payload})
export default linkGraph
if (import.meta.hot) {
  import.meta.hot.on(${JSON.stringify(LINK_GRAPH_EVENT)}, (data) => {
    linkGraph.value = data
  })
  import.meta.hot.accept()
}`;
  }

  // Sync fetch for production builds
  async fetchSync (context: TypedownContext): Promise<void> {
    this.data = await context.getLinkGraph();
  }

  // Async fetch for dev: pushes HMR update when data arrives
  fetch (context: TypedownContext, server: ViteDevServer): void {
    context
      .getLinkGraph()
      .then((data) => {
        this.data = data;
        this.notifyHmrClient(server);
      })
      .catch(() => { });
  }

  notifyHmrClient (server: ViteDevServer): void {
    server.hot.send({
      type: 'custom',
      event: LINK_GRAPH_EVENT,
      data: this.data,
    });
  }
}
