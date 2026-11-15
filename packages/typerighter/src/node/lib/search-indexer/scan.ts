import {
  globSync, readFileSync,
} from 'node:fs';
import {
  resolve,
} from 'node:path';
import type {
  SearchIndexer, PageIndexInput,
} from './index';
import {
  extractFrontmatterBuiltinFields,
  getTdContentUrl,
  getTdResourceTitle,
} from '@/shared';

// Scan .td files from disk and populate the search index
// Reads raw file content instead of going through the RPC pipeline
export function indexAllFiles (rootDirectory: string, indexer: SearchIndexer): void {
  const files = scanContentFiles(rootDirectory);
  const pages: PageIndexInput[] = [];

  for (const filepath of files) {
    const page = readPage(rootDirectory, filepath);

    if (page) pages.push(page);
  }

  indexer.addAll(pages);
}

// Re-index a single file after content change
export function reindexFile (rootDirectory: string, filepath: string, indexer: SearchIndexer): void {
  const absolute = resolve(rootDirectory, filepath);
  const page = readPage(rootDirectory, absolute);

  if (page) {
    indexer.addPage(page);
  } else {
    indexer.discardPage(getTdContentUrl(filepath));
  }
}

function parsePage (relative: string, raw: string): {
  title: string;
  body: string;
} {
  const fields = extractFrontmatterBuiltinFields(raw);
  const title = fields?.label ?? getTdResourceTitle(relative);
  const end = raw.indexOf('\n---', 3);
  const body = end === -1 ? raw : raw.slice(end + 4).trim();

  return {
    title,
    body,
  };
}

function readPage (rootDirectory: string, filepath: string): PageIndexInput | undefined {
  let raw: string;

  try {
    raw = readFileSync(filepath, 'utf-8');
  } catch {
    return undefined;
  }

  if (!raw.trim()) return undefined;

  const relative = filepath.slice(rootDirectory.length + 1);
  const {
    title, body,
  } = parsePage(relative, raw);

  return {
    id: getTdContentUrl(relative),
    title,
    text: `${title}\n${body}`,
  };
}

function scanContentFiles (rootDirectory: string): string[] {
  return globSync('**/*.td', {
    cwd: rootDirectory,
    exclude: (name) => name.startsWith('_'),
  }).map((relative) => resolve(rootDirectory, relative));
}
