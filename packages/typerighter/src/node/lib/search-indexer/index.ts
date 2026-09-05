import MiniSearch from 'minisearch';
import {
  SEARCH_FIELDS, SEARCH_STORE_FIELDS,
} from '@/shared';

export interface PageIndexInput {
  id: string;
  title: string;
  text: string;
}

export class SearchIndexer {
  private index: MiniSearch<SearchIndexEntry>;

  constructor () {
    this.index = new MiniSearch<SearchIndexEntry>({
      fields: SEARCH_FIELDS,
      storeFields: SEARCH_STORE_FIELDS,
    });
  }

  addAll (pages: PageIndexInput[]): void {
    this.index.removeAll();
    for (const page of pages) {
      this.index.add(page);
    }
  }

  addPage (page: PageIndexInput): void {
    if (this.index.has(page.id)) this.index.discard(page.id);
    this.index.add(page);
  }

  discardPage (pageId: string): void {
    if (this.index.has(pageId)) this.index.discard(pageId);
  }

  serialize (): string {
    return JSON.stringify(this.index);
  }
}

interface SearchIndexEntry {
  id: string;
  title: string;
  text: string;
}
