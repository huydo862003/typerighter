import path from 'node:path';
import type {
  RpcClient,
  TdBuiltResource, TdDiagnosticReport, TdSiteConfig, TdSchemaInfo,
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
    return this.rpc.requestFile(filepath);
  }

  async getFiles (paths: string[]): Promise<TdBuiltResource[]> {
    const results = await this.rpc.requestFiles(paths);

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

    this.cachedFiles = await this.rpc.listVault();

    return this.cachedFiles;
  }

  async listFilesGroupedBySchema (): Promise<SchemaGroups> {
    if (this.cachedFilesGroupedBySchema) return this.cachedFilesGroupedBySchema;

    const raw = await this.rpc.listFilesGroupedBySchema();
    // serde_wasm_bindgen converts HashMap to a JS Map, convert to plain object
    const result: SchemaGroups = raw instanceof Map
      ? Object.fromEntries(raw)
      : raw ?? {};

    this.cachedFilesGroupedBySchema = result;

    return result;
  }

  /* Project operations */

  async getConfig (): Promise<TdSiteConfig> {
    if (this.cachedConfig) return this.cachedConfig;

    this.cachedConfig = await this.rpc.getConfig();

    return this.cachedConfig;
  }

  // Get the asset directory for a given file
  async getAssetDir (filepath: string): Promise<string> {
    const config = await this.getConfig();

    return path.join(path.dirname(filepath), config.assetsDir.path);
  }

  async getSchema (schema: string): Promise<TdSchemaInfo> {
    const cached = this.cachedSchemaMap.get(schema);

    if (cached) return cached;

    const result = await this.rpc.getSchema(schema);

    this.cachedSchemaMap.set(schema, result);

    return result;
  }

  async listSchemas (): Promise<string[]> {
    if (this.cachedSchemas) return this.cachedSchemas;

    this.cachedSchemas = await this.rpc.listSchemas();

    return this.cachedSchemas;
  }

  async checkVault (): Promise<TdDiagnosticReport> {
    return this.rpc.checkVault();
  }

  get md (): MarkdownRenderer {
    return this._md;
  }
}
