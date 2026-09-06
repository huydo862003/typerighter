import { EventEmitter } from 'node:events';

export interface RpcServerOptions {
  /** Project root (default: cwd) */
  root?: string;
  /** Bind address (default: "127.0.0.1") */
  addr?: string;
  /** Bind port (default: 0, OS picks a free port) */
  port?: number;
}

export class RpcServer extends EventEmitter {
  constructor(options?: RpcServerOptions);

  /** The host:port address the server is listening on, or undefined if not started */
  get address(): string | undefined;

  /** The host the server is bound to, or undefined if not started */
  get host(): string | undefined;

  /** The port the server is listening on, or undefined if not started */
  get port(): number | undefined;

  /** Whether the server is currently listening */
  get listening(): boolean;

  /** Start the server. Emits "listening" when ready */
  listen(callback?: () => void): this;

  /** Stop the server. Emits "close" when the process exits */
  close(): this;

  /** Unref the child process so it does not prevent Node from exiting */
  unref(): this;

  on(event: 'listening', listener: () => void): this;
  on(event: 'close', listener: (code: number | null, signal: string | null) => void): this;
  on(event: 'error', listener: (error: Error) => void): this;
}
