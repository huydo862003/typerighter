// Posix-style path utilities for content filepaths
// These work with forward-slash paths and run in both Node and browser

import {
  CONTENT_EXTENSIONS,
} from '../constants';

export function basename (filepath: string, extension?: string): string {
  const name = normalize(filepath).split('/')
    .pop() ?? '';

  if (extension !== undefined && name.endsWith(extension)) {
    return name.slice(0, -extension.length);
  }

  return name;
}

export function dirname (filepath: string): string {
  const parts = normalize(filepath).split('/');

  parts.pop();

  return parts.join('/');
}

export function extname (filepath: string): string {
  const name = basename(filepath);
  const dot = name.lastIndexOf('.');

  return dot <= 0 ? '' : name.slice(dot);
}

export function filestem (filepath: string): string {
  const name = basename(filepath);
  const dot = name.lastIndexOf('.');

  return dot <= 0 ? name : name.slice(0, dot);
}

export function isContentFile (filepath: string): boolean {
  return CONTENT_EXTENSIONS.includes(extname(filepath));
}

// Type schema files live under the _types/ directory convention
export function isTypeFile (filepath: string): boolean {
  return isContentFile(filepath) && filepath.split('/').includes('_types');
}

export function join (...segments: string[]): string {
  return segments
    .filter(Boolean)
    .join('/')
    .replace(/\\+/g, '/')
    .replace(/\/+/g, '/')
    .replace(/\/$/, '') || '/';
}

export function stripExtension (filepath: string): string {
  const extension = extname(filepath);

  return extension ? filepath.slice(0, -extension.length) : filepath;
}

// Extract the portion of a path after a named directory segment
// Uses path segment boundaries to avoid matching partial directory names
export function stripPrefix (filepath: string, prefix: string): string {
  const normalized = normalize(filepath);
  const segment = `/${prefix}/`;
  const index = normalized.lastIndexOf(segment);

  return index !== -1
    ? normalized.slice(index + segment.length)
    : normalized;
}

function normalize (filepath: string): string {
  return filepath.replace(/\\/g, '/');
}
