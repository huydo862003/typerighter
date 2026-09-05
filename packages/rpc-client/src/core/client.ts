/**
 * JSON-RPC 2.0 client with Content-Length framing
 * Simplified from vscode-jsonrpc (MIT, Microsoft Corporation)
 * Ref: https://github.com/microsoft/vscode-languageserver-node/blob/5010cdf9822e1038a30ee7eb6ee5d7aaa79acc4a/jsonrpc/src/common/connection.ts#L506-L591
 */

import { createConnection } from 'node:net';
import type { Readable, Writable } from 'node:stream';
import type { Message, RequestMessage, NotificationMessage, ResponseMessage } from './types.js';
import { Message as MessageGuards, ErrorCodes, ResponseError } from './types.js';

type PendingRequest = {
  resolve: (value: unknown) => void;
  reject: (error: ResponseError) => void;
};

// A Disposable interface to undo a previous side-effectful step
interface Disposable {
  dispose (): void;
}

const HEADER_SEPARATOR = Buffer.from('\r\n\r\n', 'ascii');
const HEADER_SEPARATOR_LENGTH = 4; // \r\n\r\n

/**
 * Simplified MessageConnection interface
 * https://github.com/microsoft/vscode-languageserver-node/blob/5010cdf9822e1038a30ee7eb6ee5d7aaa79acc4a/jsonrpc/src/common/connection.ts#L506-L591
 */
export interface MessageConnection extends Disposable {
  /** Send a JSON-RPC request and wait for the response */
  sendRequest<R> (method: string, params?: any[] | object): Promise<R>;
  /** Send a one-way JSON-RPC notification (no response expected) */
  sendNotification (method: string, params?: any[] | object): void;
  /** Register a handler for server-sent notifications, returns an unsubscribe handle */
  onNotification (method: string, handler: (params: any) => void): Disposable;
  /** Register a listener for connection close, returns an unsubscribe handle */
  onClose (listener: () => void): Disposable;
  /** Reject all pending requests and stop listening */
  dispose (): void;
}

export class JsonRpcClient implements MessageConnection {
  // Transport
  private reader: Readable;
  private writer: Writable;

  // Request/response matching
  private sequenceNumber = 1;
  private responsePromises = new Map<number, PendingRequest>();
  private notificationHandlers = new Map<string, (params: unknown) => void>();

  // Inbound buffer, raw chunks to avoid corrupting multi-byte UTF-8
  private chunks: Buffer[] = [];
  private chunksLength = 0;
  private contentLength = -1; // -1 = waiting for header

  // Lifecycle
  private disposed = false;
  private closeHandlers: Array<() => void> = [];

  constructor (reader: Readable, writer: Writable) {
    this.reader = reader;
    this.writer = writer;

    this.reader.on('data', (chunk: Buffer) => this.onData(chunk));
    this.reader.on('end', () => this.handleStreamClose());
    this.reader.on('close', () => this.handleStreamClose());
  }

  static connectTcp (host: string, port: number): Promise<JsonRpcClient> {
    return new Promise((resolve, reject) => {
      const socket = createConnection({ host, port }, () => {
        resolve(new JsonRpcClient(socket, socket));
      });
      socket.on('error', reject);
    });
  }

  static connectStdio (stdin: Readable, stdout: Writable): JsonRpcClient {
    return new JsonRpcClient(stdin, stdout);
  }

  // https://github.com/microsoft/vscode-languageserver-node/blob/5010cdf9822e1038a30ee7eb6ee5d7aaa79acc4a/jsonrpc/src/common/connection.ts#L1563-L1590
  dispose (): void {
    if (this.disposed) return;
    this.disposed = true;

    this.rejectAllPending('Connection disposed');
  }

  // https://github.com/microsoft/vscode-languageserver-node/blob/5010cdf9822e1038a30ee7eb6ee5d7aaa79acc4a/jsonrpc/src/common/connection.ts#L565
  onClose (listener: () => void): Disposable {
    this.closeHandlers.push(listener);
    return {
      dispose: () => {
        const index = this.closeHandlers.indexOf(listener);
        if (index >= 0) this.closeHandlers.splice(index, 1);
      },
    };
  }

  // https://github.com/microsoft/vscode-languageserver-node/blob/5010cdf9822e1038a30ee7eb6ee5d7aaa79acc4a/jsonrpc/src/common/connection.ts#L537
  sendRequest<R> (method: string, params?: any[] | object): Promise<R> {
    const id = this.sequenceNumber++;
    const message: RequestMessage = { jsonrpc: '2.0', id, method, params };

    return new Promise<R>((resolve, reject) => {
      this.responsePromises.set(id, { resolve: resolve as (v: unknown) => void, reject });
      this.writeMessage(message);
    });
  }

  // https://github.com/microsoft/vscode-languageserver-node/blob/5010cdf9822e1038a30ee7eb6ee5d7aaa79acc4a/jsonrpc/src/common/connection.ts#L549
  sendNotification (method: string, params?: any[] | object): void {
    const message: NotificationMessage = { jsonrpc: '2.0', method, params };
    this.writeMessage(message);
  }

  // https://github.com/microsoft/vscode-languageserver-node/blob/5010cdf9822e1038a30ee7eb6ee5d7aaa79acc4a/jsonrpc/src/common/connection.ts#L557
  onNotification (method: string, handler: (params: any) => void): Disposable {
    this.notificationHandlers.set(method, handler);
    return {
      dispose: () => this.notificationHandlers.delete(method),
    };
  }

  // Accumulate chunk and parse as many complete messages as possible
  // A single chunk can contain multiple messages
  // Ref: vscode-jsonrpc ReadableStreamMessageReader.nextMessage
  private onData (chunk: Buffer): void {
    this.chunks.push(chunk);
    this.chunksLength += chunk.length;

    while (true) {
      if (this.contentLength < 0) {
        const buffer = this.compact();
        const separatorIndex = buffer.indexOf(HEADER_SEPARATOR);

        if (separatorIndex < 0) return;

        const header = buffer.subarray(0, separatorIndex).toString('ascii');
        const match = header.match(/Content-Length:\s*(\d+)/i);

        if (!match) {
          this.consume(separatorIndex + HEADER_SEPARATOR_LENGTH);
          continue;
        }

        this.contentLength = Number(match[1]);
        this.consume(separatorIndex + HEADER_SEPARATOR_LENGTH);
      }

      if (this.chunksLength < this.contentLength) return;

      const buffer = this.compact();
      const body = buffer.subarray(0, this.contentLength).toString('utf-8');

      this.consume(this.contentLength);
      this.contentLength = -1;

      try {
        const message = JSON.parse(body);

        if (MessageGuards.isResponse(message)) {
          this.handleResponse(message);
        } else if (MessageGuards.isNotification(message)) {
          this.handleNotification(message);
        }
      } catch (error) {
        console.error('[jsonrpc] Failed to parse message:', error);
      }
    }
  }

  // Ref: vscode-jsonrpc connection.ts

  private handleResponse (response: ResponseMessage): void {
    if (typeof response.id !== 'number') return;

    const pending = this.responsePromises.get(response.id);

    if (!pending) return;
    this.responsePromises.delete(response.id);

    if (response.error) {
      pending.reject(new ResponseError(response.error.code, response.error.message, response.error.data));
    } else if (response.result !== undefined) {
      pending.resolve(response.result);
    }
  }

  private handleNotification (notification: NotificationMessage): void {
    const handler = this.notificationHandlers.get(notification.method);

    if (!handler) return;

    try {
      handler(notification.params);
    } catch (error) {
      console.error(`[jsonrpc] Notification handler for '${notification.method}' threw:`, error);
    }
  }

  // Cleanup

  private rejectAllPending (reason: string): void {
    for (const [, pending] of this.responsePromises) {
      pending.reject(new ResponseError(ErrorCodes.PendingResponseRejected, reason));
    }
    this.responsePromises.clear();
  }

  // Fires once on stream end/close
  private handleStreamClose (): void {
    if (this.disposed) return;
    this.disposed = true;

    this.rejectAllPending('Server disconnected');

    for (const handler of this.closeHandlers) {
      handler();
    }
  }

  /* Helpers */

  // Single buffer write to prevent interleaving under concurrent requests
  private writeMessage (message: Message): void {
    const body = Buffer.from(JSON.stringify(message), 'utf-8');
    const header = Buffer.from(`Content-Length: ${body.length}\r\n\r\n`, 'ascii');

    this.writer.write(Buffer.concat([header, body]));
  }

  // Concat all chunks into 1 buffer
  private compact (): Buffer {
    if (this.chunks.length === 1) return this.chunks[0];

    const buffer = Buffer.concat(this.chunks);

    this.chunks = [buffer];
    this.chunksLength = buffer.length;

    return buffer;
  }

  // Consume n bytes from buffer
  private consume (bytes: number): void {
    const buffer = this.compact();
    const remaining = buffer.subarray(bytes);

    if (remaining.length > 0) {
      this.chunks = [remaining];
      this.chunksLength = remaining.length;
    } else {
      this.chunks = [];
      this.chunksLength = 0;
    }
  }
}
