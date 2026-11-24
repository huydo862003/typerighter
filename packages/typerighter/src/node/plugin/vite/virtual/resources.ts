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

  // Full fetch from RPC, used on initial load
  async fetch (context: TypedownContext, componentType: string): Promise<void> {
    const resources = await context.listComponentResources(componentType);

    for (const resource of resources) {
      for (const key of Object.keys(resource.fields)) {
        resource.fields[key] = renderMathBlocks(resource.fields[key]);
      }
    }

    this.cache.set(componentType, resources);
  }

  // Patch cache for a single file: remove old entries, add new ones
  // Returns the set of component types that changed (need HMR push)
  patchFile (filepath: string, newResources: Resource[]): Set<string> {
    const changed = new Set<string>();

    // Render math in new resources
    for (const resource of newResources) {
      for (const key of Object.keys(resource.fields)) {
        resource.fields[key] = renderMathBlocks(resource.fields[key]);
      }
    }

    for (const [
      componentType,
      cached,
    ] of this.cache) {
      const hadEntries = cached.some((resource) => resource.filepath === filepath);
      const newEntries = newResources.filter((resource) => resource.type === componentType);

      if (!hadEntries && newEntries.length === 0) continue;

      // Replace entries for this filepath
      const filtered = cached.filter((resource) => resource.filepath !== filepath);

      this.cache.set(componentType, [
        ...filtered,
        ...newEntries,
      ]);
      changed.add(componentType);
    }

    // Handle new component types not yet in cache
    for (const resource of newResources) {
      if (!changed.has(resource.type) && !this.cache.has(resource.type)) {
        // New type not in cache yet, skip (will be fetched on first load)
        continue;
      }
    }

    return changed;
  }

  // Remove all entries for a deleted file
  removeFile (filepath: string, server: ViteDevServer): void {
    for (const [
      componentType,
      cached,
    ] of this.cache) {
      const hadEntries = cached.some((resource) => resource.filepath === filepath);

      if (!hadEntries) continue;

      this.cache.set(
        componentType,
        cached.filter((resource) => resource.filepath !== filepath),
      );
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
