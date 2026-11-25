import {
  describe, expect, it, vi,
} from 'vitest';
import {
  ref, nextTick,
} from 'vue';
import type {
  TdLinkGraph, TdLinkGraphNode, TdLinkGraphEdge,
} from '@typerighter/rpc-client';

// Mock the virtual module before importing the composable
const mockGraph = ref<TdLinkGraph>({
  nodes: [],
  edges: [],
});

vi.mock('@typedown/link-graph', () => ({
  linkGraph: mockGraph,
}));

const {
  useGraphFilter,
} = await import('./useGraphFilter');

function makeEdge (source: string, target: string): TdLinkGraphEdge {
  return {
    source,
    target,
    kind: 'reference',
  };
}

function makeNode (id: string, schema?: string, nodeType: 'page' | 'index' = 'page'): TdLinkGraphNode {
  return {
    id,
    label: id,
    schema,
    nodeType,
  };
}

describe('useGraphFilter', () => {
  it('returns all nodes when no filters active', () => {
    mockGraph.value = {
      nodes: [
        makeNode('a.td', 'Task'),
        makeNode('b.td', 'Person'),
      ],
      edges: [makeEdge('a.td', 'b.td')],
    };

    const {
      filteredData,
    } = useGraphFilter();

    expect(filteredData.value.nodes).toHaveLength(2);
    expect(filteredData.value.edges).toHaveLength(1);
  });

  it('hides nodes by schema', async () => {
    mockGraph.value = {
      nodes: [
        makeNode('a.td', 'Task'),
        makeNode('b.td', 'Person'),
      ],
      edges: [makeEdge('a.td', 'b.td')],
    };

    const {
      filteredData, toggleSchema,
    } = useGraphFilter();

    toggleSchema('Task');
    await nextTick();

    expect(filteredData.value.nodes).toHaveLength(1);
    expect(filteredData.value.nodes[0].id).toBe('b.td');
    // Edge should also be removed since source is hidden
    expect(filteredData.value.edges).toHaveLength(0);
  });

  it('toggling schema back shows nodes again', async () => {
    mockGraph.value = {
      nodes: [
        makeNode('a.td', 'Task'),
        makeNode('b.td', 'Person'),
      ],
      edges: [],
    };

    const {
      filteredData, toggleSchema,
    } = useGraphFilter();

    toggleSchema('Task');
    await nextTick();
    expect(filteredData.value.nodes).toHaveLength(1);

    toggleSchema('Task');
    await nextTick();
    expect(filteredData.value.nodes).toHaveLength(2);
  });

  it('collects unique schema names sorted', () => {
    mockGraph.value = {
      nodes: [
        makeNode('a.td', 'Task'),
        makeNode('b.td', 'Person'),
        makeNode('c.td', 'Task'),
        makeNode('d.td'),
      ],
      edges: [],
    };

    const {
      schemaNames,
    } = useGraphFilter();

    expect(schemaNames.value).toEqual([
      'Person',
      'Task',
    ]);
  });

  it('index nodes without schema are not filtered by schema toggles', async () => {
    mockGraph.value = {
      nodes: [
        makeNode('index.td', undefined, 'index'),
        makeNode('a.td', 'Task'),
      ],
      edges: [],
    };

    const {
      filteredData, toggleSchema,
    } = useGraphFilter();

    toggleSchema('Task');
    await nextTick();

    // Index node should still be visible
    expect(filteredData.value.nodes).toHaveLength(1);
    expect(filteredData.value.nodes[0].id).toBe('index.td');
  });
});
