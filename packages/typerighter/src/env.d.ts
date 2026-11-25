declare const __VERSION__: string;
declare const __BUILD_TIMESTAMP__: string;

declare module '@typedown/resources?type=flashcard' {
  import type {
    ShallowRef,
  } from 'vue';

  export const resources: ShallowRef<Array<{
    type: string;
    filepath: string;
    fields: Record<string, string>;
  }>>;
}

declare module '@typedown/link-graph' {
  import type {
    ShallowRef,
  } from 'vue';

  export const linkGraph: ShallowRef<{
    nodes: Array<{
      id: string;
      label: string;
      schema?: string;
    }>;
    edges: Array<{
      source: string;
      target: string;
      kind: 'hierarchy' | 'reference';
    }>;
  }>;
}

declare module '*.vue' {
  import type {
    DefineComponent,
  } from 'vue';

  const component: DefineComponent;

  export default component;
}
