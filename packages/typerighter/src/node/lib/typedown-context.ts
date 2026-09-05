import {
  RPC_CANCELLED_CODE,
  type RpcClient,
  type TdBuiltResource, type TdDiagnosticReport, type TdFormatResult, type TdSidebarItem, type TdSiteConfig,
  type TdSchemaInfo,
} from '@typerighter/rpc-client';

// The context is always rooted at the current directory
// This is fine: `typedown.yaml` should be at project root + user can run within any dir nested in the project root
export class TypedownContext {
  private client: RpcClient;

  constructor (client: RpcClient) {
    this.client = client;
    this.registerNotificationHandlers(client);
  }

  private cachedConfig: TdSiteConfig | undefined;
  private cachedFiles: string[] | undefined;
  private cachedSchemas: string[] | undefined;
  private cachedSchemaMap = new Map<string, TdSchemaInfo>();
  private cachedFileMap = new Map<string, TdBuiltResource>();

  private registerNotificationHandlers (client: RpcClient) {
    client.onConfigChanged((config: TdSiteConfig) => {
      this.cachedConfig = config;
    });

    client.onContentChanged(({
      content,
    }: {
      content: string;
    }) => {
      this.cachedFileMap.delete(content);
    });

    client.onContentCreated(() => {
      this.cachedFiles = undefined;
    });

    client.onContentDeleted(({
      content,
    }: {
      content: string;
    }) => {
      this.cachedFiles = undefined;
      this.cachedFileMap.delete(content);
    });

    client.onSchemaChanged(({
      schema,
    }: {
      schema: string;
    }) => {
      this.cachedSchemaMap.delete(schema);
    });

    client.onSchemaCreated(() => {
      this.cachedSchemas = undefined;
    });

    client.onSchemaDeleted(({
      schema,
    }: {
      schema: string;
    }) => {
      this.cachedSchemas = undefined;
      this.cachedSchemaMap.delete(schema);
    });
  }

  get rpc (): RpcClient {
    return this.client;
  }

  /* File operations */

  async getFile (filepath: string): Promise<TdBuiltResource> {
    return withRetry(() => this.rpc.requestFile(filepath));
  }

  async getFiles (paths: string[]): Promise<TdBuiltResource[]> {
    const results = await withRetry(() => this.rpc.requestFiles(paths));

    for (const [
      index,
      filepath,
    ] of paths.entries()) {
      this.cachedFileMap.set(filepath, results[index]);
    }

    return results;
  }

  async listFiles (): Promise<string[]> {
    if (this.cachedFiles) return this.cachedFiles;

    this.cachedFiles = await withRetry(() => this.rpc.listVault());

    return this.cachedFiles;
  }

  async listSidebar (): Promise<TdSidebarItem[]> {
    return withRetry(() => this.rpc.listSidebar());
  }

  /* Project operations */

  async getConfig (): Promise<TdSiteConfig> {
    if (this.cachedConfig) return this.cachedConfig;

    this.cachedConfig = await withRetry(() => this.rpc.getConfig());

    return this.cachedConfig;
  }

  async getSchema (schema: string): Promise<TdSchemaInfo> {
    const cached = this.cachedSchemaMap.get(schema);

    if (cached) return cached;

    const result = await withRetry(() => this.rpc.getSchema(schema));

    this.cachedSchemaMap.set(schema, result);

    return result;
  }

  async listSchemas (): Promise<string[]> {
    if (this.cachedSchemas) return this.cachedSchemas;

    this.cachedSchemas = await withRetry(() => this.rpc.listSchemas());

    return this.cachedSchemas;
  }

  async checkVault (): Promise<TdDiagnosticReport> {
    return withRetry(() => this.rpc.checkVault());
  }

  async formatFile (filepath: string): Promise<TdFormatResult> {
    return withRetry(() => this.rpc.formatFile(filepath));
  }
}

export function isRpcCancelled (error: unknown): boolean {
  if (error instanceof Error && 'code' in error) {
    return (error as Error & {
      code: number;
    }).code === RPC_CANCELLED_CODE();
  }

  return false;
}

async function withRetry<T> (fn: () => Promise<T>, retries = 5): Promise<T> {
  for (let attempt = 0; attempt < retries; attempt++) {
    try {
      return await fn();
    } catch (error: unknown) {
      if (!isRpcCancelled(error) || attempt === retries - 1) throw error;

      // Backoff: wait for the server to finish processing file changes
      await new Promise((resolve) => setTimeout(resolve, 50 * (attempt + 1)));
    }
  }

  throw new Error('unreachable');
}
