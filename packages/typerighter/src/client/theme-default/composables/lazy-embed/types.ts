import type {
  InjectionKey,
} from 'vue';

export interface LazyEmbedContext {
  register: (options: LazyEmbedOptions) => void;
  unregister: (options: LazyEmbedOptions) => void;
}

export interface LazyEmbedOptions {
  selector: string;
  onRender: (element: Element) => void | Promise<void>;
  onVisible?: () => void;
}

export const LAZY_EMBED_KEY: InjectionKey<LazyEmbedContext> =
  Symbol('lazyEmbed');
