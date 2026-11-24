// Component resource declaration
//
// Usage in a Vue SFC:
//   import { defineResource } from 'typerighter/client';
//   defineResource({
//     fields: {
//       question: { from: 'slot', name: 'default' },
//       answer: { from: 'slot', name: 'key' },
//     },
//   });

export interface FieldSource {
  from: 'slot' | 'prop';
  name: string;
}

/** A single extracted resource instance produced by a component on a page */
export interface Resource {
  /** Component name this resource was extracted from */
  type: string;
  /** Source page filepath relative to vault root */
  filepath: string;
  /** Extracted field values */
  fields: Record<string, string>;
}

export interface ResourceDeclaration {
  fields: Record<string, FieldSource>;
}

/**
 * Declares that this component produces a structured resource.
 * No-op at runtime. The Vite plugin extracts the declaration at build time
 */
export function defineResource (_declaration: ResourceDeclaration): void {
  // No-op at runtime
}
