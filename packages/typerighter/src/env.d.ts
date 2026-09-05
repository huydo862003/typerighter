declare const __VERSION__: string;
declare const __BUILD_TIMESTAMP__: string;

declare module '*.vue' {
  import type {
    DefineComponent,
  } from 'vue';

  const component: DefineComponent;

  export default component;
}
