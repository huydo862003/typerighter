const providers: Array<() => string> = [];

export function collectHeadScripts (): string {
  return providers.map((fn) => fn()).join('');
}

// called by each preference module at load time
export function registerHeadScript (provider: () => string): void {
  providers.push(provider);
}
