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

// Strip YAML frontmatter, resolve title from _label or filepath
function parseFrontmatter (relative: string, raw: string): {
  title: string;
  body: string;
} {
  const fallbackTitle = getTdResourceTitle(relative);

  if (!raw.startsWith('---')) return {
    title: fallbackTitle,
    body: raw,
  };

  const end = raw.indexOf('\n---', 3);

  if (end === -1) return {
    title: fallbackTitle,
    body: raw,
  };

  const frontmatter = raw.slice(4, end);
  const body = raw.slice(end + 4).trim();

  const match = /^_label:\s*"(.+)"$/m.exec(frontmatter)
    ?? /^_label:\s*'(.+)'$/m.exec(frontmatter);
  const title = match?.[1]?.trim() || fallbackTitle;

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
  } = parseFrontmatter(relative, raw);

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
