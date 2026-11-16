import type {
  ViteDevServer,
} from 'vite';
import {
  PAGE_DATA_PREFIX,
  RESOLVED_PAGE_DATA_PREFIX,
  PAGE_DATA_EVENT,
} from '../constants';
import type {
  TypedownContext,
} from '@/node/lib/typedown-context';
import {
  buildPageData,
} from '@/node/lib/render/vue';

export class VirtualPageData {
  resolve (id: string): string | undefined {
    if (id.startsWith(PAGE_DATA_PREFIX)) {
      return '\0' + id;
    }
  }

  isResolved (resolvedId: string): boolean {
    return resolvedId.startsWith(RESOLVED_PAGE_DATA_PREFIX);
  }

  async load (resolvedId: string, context: TypedownContext): Promise<string> {
    const filepath = decodeURIComponent(
      resolvedId.slice(RESOLVED_PAGE_DATA_PREFIX.length),
    );
    const resource = await context.getFile(filepath);
    const pageData = await buildPageData(context, resource, filepath);

    return `
import { shallowRef } from 'vue'
export const pageData = shallowRef(${JSON.stringify(pageData)})
if (import.meta.hot) {
  import.meta.hot.on(${JSON.stringify(PAGE_DATA_EVENT)}, (event) => {
    if (event.filepath === ${JSON.stringify(filepath)}) pageData.value = event.pageData
  })
  import.meta.hot.accept()
}`;
  }

  // Send fresh page data via HMR so route.data updates without Vue SFC setup re-running
  notifyHmrClient (
    server: ViteDevServer,
    context: TypedownContext,
    filepath: string,
  ): void {
    context
      .getFile(filepath)
      .then(async (resource) => {
        const pageData = await buildPageData(context, resource, filepath);

        server.hot.send({
          type: 'custom',
          event: PAGE_DATA_EVENT,
          data: {
            filepath,
            pageData,
          },
        });
      })
      .catch((error) => {
        console.error(
          '[typedown] Failed to send page data:',
          error instanceof Error ? error.message : error,
        );
      });
  }
}
