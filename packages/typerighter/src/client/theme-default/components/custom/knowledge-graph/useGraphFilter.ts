import {
  computed, ref,
} from 'vue';
import {
  linkGraph,
} from '@typedown/link-graph';
import type {
  TdLinkGraph,
} from '@typerighter/rpc-client';

export function useGraphFilter () {
  const searchQuery = ref('');
  const hiddenSchemas = ref<Set<string>>(new Set());

  const schemaNames = computed(() => {
    const names = new Set<string>();

    for (const node of linkGraph.value.nodes) {
      if (node.schema) names.add(node.schema);
    }

    return [...names].sort();
  });

  const filteredData = computed((): TdLinkGraph => {
    const data = linkGraph.value;
    const hidden = hiddenSchemas.value;

    const visibleNodes = data.nodes.filter((node) => {
      if (node.schema && hidden.has(node.schema)) return false;

      return true;
    });

    const visibleIds = new Set(visibleNodes.map((node) => node.id));
    const visibleEdges = data.edges.filter(
      (edge) => visibleIds.has(edge.source) && visibleIds.has(edge.target),
    );

    return {
      nodes: visibleNodes,
      edges: visibleEdges,
    };
  });

  function toggleSchema (schema: string) {
    const next = new Set(hiddenSchemas.value);

    if (next.has(schema)) {
      next.delete(schema);
    } else {
      next.add(schema);
    }

    hiddenSchemas.value = next;
  }

  return {
    searchQuery,
    hiddenSchemas,
    schemaNames,
    filteredData,
    toggleSchema,
  };
}
