// Generic worker pool using native worker threads
// Workers listen on parentPort for tasks and post back { result } or { error }

import os from 'node:os';
import {
  Worker,
} from 'node:worker_threads';

export interface WorkerPoolOptions {
  /** Path or URL of the worker module */
  filename: string | URL;
  /** Data passed to each worker via workerData */
  workerData?: unknown;
  /** Number of workers (defaults to half the CPU count) */
  concurrency?: number;
}

export class WorkerPool<TTask = unknown, TResult = unknown> {
  private workers: WorkerState<TResult>[] = [];
  private queue: PendingTask<TResult>[] = [];
  private destroyed = false;
  private startupError?: Error;

  constructor (options: WorkerPoolOptions) {
    const concurrency = options.concurrency
      ?? Math.max(1, Math.floor(os.cpus().length / 2));

    for (let index = 0; index < concurrency; index++) {
      const worker = new Worker(options.filename, {
        workerData: options.workerData,
      });

      const state: WorkerState<TResult> = {
        worker,
      };

      this.workers.push(state);

      worker.on('message', (message: {
        result?: TResult;
        error?: string;
      }) => {
        const pending = state.pending;

        state.pending = undefined;

        if (message.error !== undefined) {
          pending?.reject(new Error(message.error));
        } else {
          pending?.resolve(message.result as TResult);
        }

        this.dispatch(state);
      });

      worker.on('error', (error) => {
        const pending = state.pending;

        state.pending = undefined;
        pending?.reject(error);
        this.destroyAll(error);
      });
    }
  }

  // Submit a task and wait for the result
  run (task: TTask): Promise<TResult> {
    if (this.destroyed) {
      return Promise.reject(this.startupError ?? new Error('Worker pool is destroyed'));
    }

    return new Promise<TResult>((resolve, reject) => {
      const pending: PendingTask<TResult> = {
        task,
        resolve,
        reject,
      };

      const idle = this.workers.find((worker) => worker.pending === undefined);

      if (idle) {
        idle.pending = pending;
        idle.worker.postMessage(pending.task);
      } else {
        this.queue.push(pending);
      }
    });
  }

  // Terminate all workers and reject any remaining tasks
  async destroy (): Promise<void> {
    this.destroyed = true;
    this.rejectAll(new Error('Worker pool is destroyed'));
    await Promise.all(
      this.workers.map((state) => state.worker.terminate()),
    );
  }

  private dispatch (state: WorkerState<TResult>) {
    const next = this.queue.shift();

    if (next === undefined) return;

    state.pending = next;
    state.worker.postMessage(next.task);
  }

  // Reject all queued and in-flight tasks
  private rejectAll (error: unknown) {
    for (const pending of this.queue) {
      pending.reject(error);
    }

    this.queue.length = 0;

    for (const state of this.workers) {
      if (state.pending) {
        state.pending.reject(error);
        state.pending = undefined;
      }
    }
  }

  // On worker crash, reject all pending tasks and terminate remaining workers
  private destroyAll (error: unknown) {
    if (this.destroyed) return;

    this.startupError = error instanceof Error ? error : new Error(String(error));
    this.destroyed = true;
    this.rejectAll(error);
    Promise.all(this.workers.map((state) => state.worker.terminate())).catch(() => {});
  }
}

interface PendingTask<TResult> {
  task: unknown;
  resolve: (result: TResult) => void;
  reject: (error: unknown) => void;
}

interface WorkerState<TResult> {
  worker: Worker;
  pending?: PendingTask<TResult>;
}
