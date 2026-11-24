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

declare module '*.vue' {
  import type {
    DefineComponent,
  } from 'vue';

  const component: DefineComponent;

  export default component;
}
