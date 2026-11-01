export interface LazyEmbedContext {
  register: (options: LazyEmbedOptions) => void;
  unregister: (options: LazyEmbedOptions) => void;
}

export interface LazyEmbedOptions {
  selector: string;
  onRender: (element: Element) => void | Promise<void>;
  onVisible?: () => void;
}
