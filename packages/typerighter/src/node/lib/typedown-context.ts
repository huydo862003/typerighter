import {
  RPC_CANCELLED_CODE,
  type RpcClient,
  type TdBuiltResource, type TdDiagnosticReport, type TdFormatResult, type TdSidebarItem, type TdSiteConfig,
  type TdSchemaInfo,
} from '@typerighter/rpc-client';
import {
  createMarkdownRenderer, type MarkdownRenderer,
} from './markdown';
import type {
  SchemaGroups,
} from '@/shared';

// The context is always rooted at the current directory
// This is fine: `typedown.yaml` should be at project root + user can run within any dir nested in the project root
export class TypedownContext {
  private client: RpcClient;
  private _md: MarkdownRenderer;

  constructor (client: RpcClient, md: MarkdownRenderer) {
    this.client = client;
    this._md = md;
    this.registerNotificationHandlers(client);
  }

  private cachedConfig: TdSiteConfig | undefined;
  private cachedFiles: string[] | undefined;
  private cachedFilesGroupedBySchema: SchemaGroups | undefined;
  private cachedSchemas: string[] | undefined;
  private cachedSchemaMap = new Map<string, TdSchemaInfo>();
  private cachedFileMap = new Map<string, TdBuiltResource>();

  private configVersion = 0;

  private registerNotificationHandlers (client: RpcClient) {
    client.onConfigChanged((config: TdSiteConfig) => {
      this.cachedConfig = config;
      // Recreate the markdown renderer with the new config (e.g. basePath may have changed)
      const version = ++this.configVersion;

      createMarkdownRenderer(config).then((newMd) => {
        if (this.configVersion === version) {
          this._md = newMd;
        }
      })
        .catch((error) => {
          console.error(`Failed to recreate markdown renderer: ${error}`);
        });
    });

    client.onContentChanged(({
      content,
    }: {
      content: string;
    }) => {
      this.cachedFileMap.delete(content);
      this.cachedFilesGroupedBySchema = undefined;
    });

    client.onContentCreated(() => {
      this.cachedFiles = undefined;
      this.cachedFilesGroupedBySchema = undefined;
    });

    client.onContentDeleted(({
      content,
    }: {
      content: string;
    }) => {
      this.cachedFiles = undefined;
      this.cachedFilesGroupedBySchema = undefined;
      this.cachedFileMap.delete(content);
    });

    client.onSchemaChanged(({
      schema,
    }: {
      schema: string;
    }) => {
      this.cachedSchemaMap.delete(schema);
      this.cachedFilesGroupedBySchema = undefined;
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

  async listFilesGroupedBySchema (): Promise<SchemaGroups> {
    if (this.cachedFilesGroupedBySchema) return this.cachedFilesGroupedBySchema;

    const raw = await withRetry(() => this.rpc.listFilesGroupedBySchema());
    // serde_wasm_bindgen converts HashMap to a JS Map, convert to plain object
    const result: SchemaGroups = raw instanceof Map
      ? Object.fromEntries(raw)
      : raw ?? {};

    // serde_json::Value::Object fields (like header) also arrive as JS Maps
    for (const items of Object.values(result)) {
      for (const item of items) {
        if (item.header instanceof Map) {
          item.header = Object.fromEntries(item.header);
        }
      }
    }

    this.cachedFilesGroupedBySchema = result;

    return result;
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

  get md (): MarkdownRenderer {
    return this._md;
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
