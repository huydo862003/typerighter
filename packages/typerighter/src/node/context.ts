import fs from 'node:fs';
import path from 'node:path';
import {
  createLogger, type Logger,
} from 'vite';
import {
  RpcServer,
} from '@typerighter/rpc-server';
import {
  RpcClient,
} from '@typerighter/rpc-client';
import {
  TypedownContext,
} from './lib/typedown-context';
import {
  TdLogger,
} from './lib/logger';

export interface AppContext {
  root: string;
  logger: TdLogger;
  getTdContext: () => Promise<TypedownContext>;
  createViteLogger: () => Logger;
  dispose: () => void;
}

export const CONFIG_FILE_NAMES = [
  'typedown.yaml',
  'typedown.yml',
];

export function createAppContext (root: string): AppContext {
  // 1. Set up file logging to .typedown/.local/logs/
  const logger = new TdLogger(initializeLogFile(root));

  // 2. Create a Vite logger that suppresses console but writes to the log file
  function createViteLogger (): Logger {
    const viteLogger = createLogger('warn');

    return {
      ...viteLogger,
      info (message: string) {
        logger.writeToFile('VITE', message);
      },
      warn (message: string) {
        viteLogger.warn(message);
        logger.writeToFile('VITE:WARN', message);
      },
      error (message: string) {
        viteLogger.error(message);
        logger.writeToFile('VITE:ERROR', message);
      },
    };
  }

  let tdContext: TypedownContext | undefined;
  let server: RpcServer | undefined;
  let client: RpcClient | undefined;
  let disposed = false;

  function onSignal () {
    logger.log('');
    dispose();
    process.exit(0);
  }

  function onExit () {
    dispose();
  }

  // 3. Lazy-initialize the RPC server, client, and markdown renderer
  async function getTdContext (): Promise<TypedownContext> {
    if (tdContext) return tdContext;

    process.on('SIGINT', onSignal);
    process.on('SIGTERM', onSignal);

    server = new RpcServer({
      root,
    });

    await new Promise<void>((resolve, reject) => {
      server?.once('error', reject);
      server?.listen(() => {
        server?.removeListener('error', reject);
        resolve();
      });
    });

    const address = server.address;
    const port = server.port;

    if (address === undefined || port === undefined) {
      throw new Error('RPC server started but address/port not available');
    }

    client = await RpcClient.connect(new URL(address).hostname, port);

    tdContext = new TypedownContext(client);
    // Unref the child so it does not prevent Node from exiting after vite build finishes
    server.unref();

    process.on('exit', onExit);

    return tdContext;
  }

  // 4. Tear down all resources
  function dispose () {
    if (disposed) return;
    disposed = true;

    process.removeListener('exit', onExit);
    process.removeListener('SIGINT', onSignal);
    process.removeListener('SIGTERM', onSignal);

    client?.close();
    server?.close();

    tdContext = undefined;
    server = undefined;
    client = undefined;
  }

  return {
    root,
    logger,
    getTdContext,
    createViteLogger,
    dispose,
  };
}

// Walk up from `from` until a directory containing typedown.yaml or typedown.yml is found
export function resolveProjectRoot (from: string): string {
  let current = path.resolve(from);

  while (true) {
    if (hasConfigFile(current)) return current;

    const parent = path.dirname(current);

    if (parent === current) break;
    current = parent;
  }

  return path.resolve(from);
}

function hasConfigFile (directory: string): boolean {
  return CONFIG_FILE_NAMES.some((name) => fs.existsSync(path.join(directory, name)));
}

function initializeLogFile (root: string): string | undefined {
  try {
    const localDirectory = path.join(root, '.typedown', '.local');
    const logDirectory = path.join(localDirectory, 'logs');

    fs.mkdirSync(logDirectory, {
      recursive: true,
    });

    const gitignorePath = path.join(localDirectory, '.gitignore');

    if (!fs.existsSync(gitignorePath)) {
      fs.writeFileSync(gitignorePath, '*\n');
    }

    const timestamp = new Date().toISOString()
      .replace(/[:.]/g, '-');

    return path.join(logDirectory, `${timestamp}.log`);
  } catch {
    return undefined;
  }
}
