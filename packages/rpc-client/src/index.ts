/**
 * Typedown RPC client
 *
 * Thin wrapper over JsonRpcClient that provides typed methods
 * matching the Rust RPC server's contract (rpc/contract.rs)
 *
 * The server (typedown-rpc binary) sends notifications for FS changes;
 * register handlers via onContentChanged/onSchemaChanged/onConfigChanged
 */

import type { Readable, Writable } from 'node:stream';
import { JsonRpcClient } from './core/index.js';

export { JsonRpcClient, Message, ResponseError, ErrorCodes } from './core/index.js';
export type { RequestMessage, ResponseMessage, NotificationMessage, ResponseErrorLiteral } from './core/index.js';

/* Types contract
 * WARNING: must match the Rust contract in rpc/contract.rs */

export interface TdDiagnosticItem {
  filepath: string;
  line: number;
  column: number;
  severity: string;
  code: string;
  message: string;
}

export interface TdContentNotification {
  content: string;
}

export interface TdFileMetadata {
  mtime: number;
  ctime: number;
}

export interface TdHeading {
  level: number;
  title: string;
  slug: string;
}

export interface TdSidebarItem {
  filepath: string;
  schema?: string;
  schemaLabel?: string;
  label?: string;
  icon?: TdIcon;
  metadata: TdFileMetadata;
}

export interface TdContentSummary {
  filepath: string;
  schema?: string;
  schemaLabel?: string;
  label?: string;
  icon?: TdIcon;
  header: Record<string, any>;
  excerpt?: string;
  metadata: TdFileMetadata;
}

export interface TdNavItem {
  title: string;
  link: string;
  icon?: string;
}

export interface TdIcon {
  name: string;
}

export interface TdDiagnosticReport {
  diagnostics: TdDiagnosticItem[];
  fileCount: number;
  errorCount: number;
  warningCount: number;
}

export interface TdFormatResult {
  content: string;
  changed: boolean;
}

export interface TdSchemaNotification {
  schema: string;
}

export interface TdSchemaInfo {
  schema: string;
  label: string;
  properties: Record<string, any>;
}

export interface TdSiteConfig {
  version: string;
  basePath: string;
  rootDir: string;
  siteTitle: string;
  siteDescription: string;
  repo: string | undefined;
  author: string | undefined;
  license: string | undefined;
  publicDir: string;
  nav: TdNavItem[];
}

export interface TdBuiltResource {
  schema?: string;
  schemaLabel?: string;
  label?: string;
  icon?: TdIcon;
  header: Record<string, any>;
  content: string;
  headings: TdHeading[];
  title?: string;
  metadata: TdFileMetadata;
}

// Server-reserved JSON-RPC error code for cancelled queries (-32000 to -32099)
export const RPC_CANCELLED_CODE = -32002;

/* Method names
 * WARNING: must match Rust contract constants in rpc/contract.rs */

const METHOD_REQUEST_FILE = 'typedown_build.request_file';
const METHOD_REQUEST_FILES = 'typedown_build.request_files';
const METHOD_LIST_VAULT = 'typedown_build.list_vault';
const METHOD_LIST_FILES_GROUPED_BY_SCHEMA = 'typedown_build.list_files_grouped_by_schema';
const METHOD_LIST_SIDEBAR = 'typedown_build.list_sidebar';
const METHOD_LIST_SCHEMAS = 'typedown_build.list_schemas';
const METHOD_GET_SCHEMA = 'typedown_build.get_schema';
const METHOD_GET_VERSION = 'typedown_build.get_version';
const METHOD_GET_CONFIG = 'typedown_build.get_config';
const METHOD_CHECK_VAULT = 'typedown_build.check_vault';
const METHOD_FORMAT_FILE = 'typedown_build.format_file';

const NOTIF_CONTENT_CHANGED = 'typedown_build.content_changed';
const NOTIF_CONTENT_CREATED = 'typedown_build.content_created';
const NOTIF_CONTENT_DELETED = 'typedown_build.content_deleted';
const NOTIF_SCHEMA_CHANGED = 'typedown_build.schema_changed';
const NOTIF_SCHEMA_CREATED = 'typedown_build.schema_created';
const NOTIF_SCHEMA_DELETED = 'typedown_build.schema_deleted';
const NOTIF_CONFIG_CHANGED = 'typedown_build.config_changed';

/**
 * Typed RPC client for the Typedown build server
 *
 * Wraps a generic JsonRpcClient with typed request methods
 * and notification handlers matching the Rust RPC contract
 */
export class RpcClient {
  private rpc: JsonRpcClient;

  private constructor (rpc: JsonRpcClient) {
    this.rpc = rpc;
  }

  static async connectTcp (host: string, port: number): Promise<RpcClient> {
    const rpc = await JsonRpcClient.connectTcp(host, port);
    return new RpcClient(rpc);
  }

  static connectStdio (stdin: Readable, stdout: Writable): RpcClient {
    return new RpcClient(JsonRpcClient.connectStdio(stdin, stdout));
  }

  /** Send exit notification and close the connection
   * The detached server will detect the disconnect and save its cache */
  dispose (): void {
    this.rpc.sendNotification('exit');
    this.rpc.dispose();
  }

  onClose (callback: () => void): { dispose: () => void } {
    return this.rpc.onClose(callback);
  }

  /* Request methods */

  requestFile (path: string): Promise<TdBuiltResource> {
    return this.rpc.sendRequest(METHOD_REQUEST_FILE, { filePath: path });
  }

  requestFiles (paths: string[]): Promise<TdBuiltResource[]> {
    return this.rpc.sendRequest(METHOD_REQUEST_FILES, { filePaths: paths });
  }

  listVault (): Promise<string[]> {
    return this.rpc.sendRequest(METHOD_LIST_VAULT, {});
  }

  listFilesGroupedBySchema (): Promise<Record<string, TdContentSummary[]>> {
    return this.rpc.sendRequest(METHOD_LIST_FILES_GROUPED_BY_SCHEMA, {});
  }

  listSidebar (): Promise<TdSidebarItem[]> {
    return this.rpc.sendRequest(METHOD_LIST_SIDEBAR, {});
  }

  listSchemas (): Promise<string[]> {
    return this.rpc.sendRequest(METHOD_LIST_SCHEMAS, {});
  }

  getSchema (schema: string): Promise<TdSchemaInfo> {
    return this.rpc.sendRequest(METHOD_GET_SCHEMA, { schema });
  }

  getVersion (): Promise<string> {
    return this.rpc.sendRequest(METHOD_GET_VERSION, {});
  }

  getConfig (): Promise<TdSiteConfig> {
    return this.rpc.sendRequest(METHOD_GET_CONFIG, {});
  }

  checkVault (): Promise<TdDiagnosticReport> {
    return this.rpc.sendRequest(METHOD_CHECK_VAULT, {});
  }

  formatFile (path: string): Promise<TdFormatResult> {
    return this.rpc.sendRequest(METHOD_FORMAT_FILE, { filePath: path });
  }

  /* Notification handlers, server pushes these when FS changes are detected */

  onContentChanged (callback: (notification: TdContentNotification) => void): void {
    this.rpc.onNotification(NOTIF_CONTENT_CHANGED, callback);
  }

  onContentCreated (callback: (notification: TdContentNotification) => void): void {
    this.rpc.onNotification(NOTIF_CONTENT_CREATED, callback);
  }

  onContentDeleted (callback: (notification: TdContentNotification) => void): void {
    this.rpc.onNotification(NOTIF_CONTENT_DELETED, callback);
  }

  onSchemaChanged (callback: (notification: TdSchemaNotification) => void): void {
    this.rpc.onNotification(NOTIF_SCHEMA_CHANGED, callback);
  }

  onSchemaCreated (callback: (notification: TdSchemaNotification) => void): void {
    this.rpc.onNotification(NOTIF_SCHEMA_CREATED, callback);
  }

  onSchemaDeleted (callback: (notification: TdSchemaNotification) => void): void {
    this.rpc.onNotification(NOTIF_SCHEMA_DELETED, callback);
  }

  onConfigChanged (callback: (config: TdSiteConfig) => void): void {
    this.rpc.onNotification(NOTIF_CONFIG_CHANGED, callback);
  }
}
