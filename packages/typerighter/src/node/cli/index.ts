import path from 'node:path';
import cac from 'cac';
import pc from 'picocolors';
import {
  createServer,
} from 'vite';
import {
  buildSite,
} from '../build';
import {
  typedown,
} from '../plugin/vite';
import {
  createAppContext,
} from '../context';

const TAG = pc.bold(pc.cyan('[typerighter]'));

export function cli () {
  const program = cac('typerighter');

  process.on('uncaughtException', (error) => {
    handleError('runtime', error);
  });

  process.on('unhandledRejection', (error) => {
    handleError('runtime', error instanceof Error ? error : new Error(String(error)));
  });

  program
    .command('[root]', 'Start dev server')
    .alias('dev')
    .option('--port <port>', 'Port to listen on')
    .option('--host', 'Expose to network')
    .action(async (root: string | undefined, options: {
      port?: number;
      host?: boolean;
    }) => {
      const server = await createServer({
        root: resolveRoot(root),
        // Skip configFile so user's vite.config.ts does not duplicate the typedown plugin
        configFile: false,
        plugins: [typedown()],
        server: {
          port: options.port,
          host: options.host,
        },
      });

      await server.listen();
      server.printUrls();
    });

  program
    .command('build [root]', 'Build for production')
    .option('--outDir <dir>', 'Output directory', {
      default: 'dist',
    })
    .option('--base <path>', 'Base public path', {
      default: '/',
    })
    .action(async (root: string | undefined, options: {
      outDir: string;
      base: string;
    }) => {
      const context = createAppContext(resolveRoot(root));

      try {
        await buildSite(context, {
          outDir: options.outDir,
          base: options.base,
        });
      } finally {
        context.dispose();
      }
    });

  program
    .command('init [root]', 'Scaffold a new project')
    .action(async (root: string | undefined) => {
      const {
        initialize,
      } = await import('./init');

      await initialize(resolveRoot(root));
    });

  program
    .command('preview [root]', 'Preview production build')
    .option('--port <port>', 'Port to listen on')
    .action(async (root: string | undefined, options: {
      port?: number;
    }) => {
      const {
        preview,
      } = await import('vite');
      const server = await preview({
        root: resolveRoot(root),
        preview: {
          port: options.port,
        },
      });

      server.printUrls();
    });

  program
    .command('check [root]', 'Check vault for errors')
    .action(async (root: string | undefined) => {
      const context = createAppContext(resolveRoot(root));

      try {
        process.stdout.write(`${TAG} Checking vault...\r`);
        const tdContext = await context.getTdContext();
        const report = await tdContext.checkVault();
        process.stdout.write('\x1B[2K'); // clear the line

        for (const d of report.diagnostics) {
          const prefix = d.severity === 'error' ? pc.red('error') : pc.yellow('warn');

          console.log(`  ${prefix} ${d.filepath}:${d.line}:${d.column} ${d.message} ${pc.dim(`(${d.code})`)}`);
        }

        const summary = `${report.fileCount} files checked, ${report.errorCount} error(s), ${report.warningCount} warning(s)`;

        if (report.errorCount > 0) {
          console.log(`\n${TAG} ${pc.red(summary)}`);
          process.exit(1);
        } else {
          console.log(`\n${TAG} ${pc.green(summary)}`);
        }
      } finally {
        context.dispose();
      }
    });

  program.help();
  program.version(__VERSION__);

  program.parse();
}

function handleError (command: string, error: unknown): never {
  const message = error instanceof Error ? error.message : String(error);

  console.error(`\n${TAG} ${pc.red(`${command} failed`)}\n`);
  console.error(pc.red(message));
  console.error('');
  process.exit(1);
}

function resolveRoot (root: string | undefined): string {
  return path.resolve(root ?? process.cwd());
}
