// Virtual module: @typedown/resources
// Exposes component resources extracted from page bodies
// Fetched via RPC filtered by component type, cached per type

import type {
  ViteDevServer,
} from 'vite';
import {
  RESOURCES_EVENT,
} from '../constants';
import type {
  Resource,
} from '@/shared';
import type {
  TypedownContext,
} from '@/node/lib/typedown-context';
import {
  renderMathBlocks,
} from '@/node/lib/render/postprocess';

export class VirtualResources {
  private cache = new Map<string, Resource[]>();

  load (componentType: string): string {
    const payload = JSON.stringify(this.cache.get(componentType) ?? []);

    return `
import { shallowRef } from 'vue'
export const resources = shallowRef(${payload})
export default resources
if (import.meta.hot) {
  import.meta.hot.on(\`${RESOURCES_EVENT}:${componentType}\`, (data) => {
    resources.value = data
  })
  import.meta.hot.accept()
}`;
  }

  async fetch (context: TypedownContext, componentType: string): Promise<void> {
    const resources = await context.listComponentResources(componentType);

    // Render math placeholders in resource fields (slot HTML contains raw LaTeX placeholders)
    for (const resource of resources) {
      for (const key of Object.keys(resource.fields)) {
        resource.fields[key] = renderMathBlocks(resource.fields[key]);
      }
    }

    this.cache.set(componentType, resources);
  }

  async invalidate (context: TypedownContext, server: ViteDevServer): Promise<void> {
    for (const componentType of this.cache.keys()) {
      await this.fetch(context, componentType);
      this.notifyHmrClient(server, componentType);
    }
  }

  notifyHmrClient (server: ViteDevServer, componentType: string): void {
    server.hot.send({
      type: 'custom',
      event: `${RESOURCES_EVENT}:${componentType}`,
      data: this.cache.get(componentType) ?? [],
    });
  }
}
