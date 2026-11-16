import type {
  ViteDevServer,
} from 'vite';
import {
  SEARCH_INDEX_EVENT,
} from '../constants';
import {
  SearchIndexer,
} from '@/node/lib/search-indexer';
import {
  indexAllFiles, reindexFile,
} from '@/node/lib/search-indexer/scan';

export class VirtualSearchIndex {
  private indexer = new SearchIndexer();

  load (): string {
    const serialized = JSON.stringify(this.indexer.serialize());

    return `
import { shallowRef } from 'vue'
export const searchIndex = shallowRef(${serialized})
export default searchIndex
if (import.meta.hot) {
  import.meta.hot.on(${JSON.stringify(SEARCH_INDEX_EVENT)}, (data) => {
    searchIndex.value = data
  })
  import.meta.hot.accept()
}`;
  }

  index (rootDirectory: string): void {
    indexAllFiles(rootDirectory, this.indexer);
  }

  reindex (rootDirectory: string, filepath: string): void {
    reindexFile(rootDirectory, filepath, this.indexer);
  }

  notifyHmrClient (server: ViteDevServer): void {
    server.hot.send({
      type: 'custom',
      event: SEARCH_INDEX_EVENT,
      data: this.indexer.serialize(),
    });
  }
}
