import type {
  ViteDevServer,
} from 'vite';
import {
  invalidateVirtualModule,
  type VirtualModule,
} from './utils';
import type {
  TypedownContext,
} from '@/node/lib/typedown-context';
import {
  SearchIndexer,
} from '@/node/lib/search-indexer';
import {
  indexAllFiles, reindexFile,
} from '@/node/lib/search-indexer/scan';

const RESOLVED_ID = '\0@typedown/search-index';

export class VirtualSearchIndex implements VirtualModule {
  private indexer = new SearchIndexer();

  load (_context: TypedownContext): string {
    return `export default ${JSON.stringify(this.indexer.serialize())}`;
  }

  index (rootDirectory: string): void {
    indexAllFiles(rootDirectory, this.indexer);
  }

  reindex (rootDirectory: string, filepath: string): void {
    reindexFile(rootDirectory, filepath, this.indexer);
  }

  invalidate (server: ViteDevServer): void {
    invalidateVirtualModule(server, RESOLVED_ID);
  }
}
