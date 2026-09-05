import fs from 'node:fs';
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

async function printVersionInfo (context: ReturnType<typeof createAppContext>) {
  const tdContext = await context.getTdContext();
  const rpcVersion = await tdContext.rpc.getVersion();

  console.log(`${TAG} cli v${__VERSION__} (built ${__BUILD_TIMESTAMP__})`);
  console.log(`${TAG} rpc v${rpcVersion}`);
  console.log('');
}

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
    .option('--verbose', 'Print version and build info')
    .action(async (root: string | undefined, options: {
      port?: number;
      host?: boolean;
      verbose?: boolean;
    }) => {
      if (options.verbose) {
        console.log(`${TAG} cli v${__VERSION__} (built ${__BUILD_TIMESTAMP__})\n`);
      }

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
    .option('--base <path>', 'Base public path')
    .option('--verbose', 'Print version and build info')
    .action(async (root: string | undefined, options: {
      outDir: string;
      base?: string;
      verbose?: boolean;
    }) => {
      const context = createAppContext(resolveRoot(root));

      if (options.verbose) await printVersionInfo(context);

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
    .option('--name <name>', 'Project name')
    .option('--title <title>', 'Site title')
    .option('--description <description>', 'Site description')
    .option('--yes', 'Skip confirmation for existing projects')
    .action(async (root: string | undefined, options: {
      name?: string;
      title?: string;
      description?: string;
      yes?: boolean;
    }) => {
      const {
        initialize,
      } = await import('./init');

      await initialize(resolveRoot(root), options);
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
    .option('--fix', 'Auto-fix formatting issues')
    .option('--verbose', 'Print version and build info')
    .action(async (root: string | undefined, options: {
      fix?: boolean;
      verbose?: boolean;
    }) => {
      const context = createAppContext(resolveRoot(root));

      if (options.verbose) await printVersionInfo(context);

      const {
        logger,
      } = context;

      try {
        logger.start('Checking vault...');
        const tdContext = await context.getTdContext();
        const report = await tdContext.checkVault();

        for (const diag of report.diagnostics) {
          if (diag.severity === 'error') {
            logger.error(`${diag.filepath}:${diag.line}:${diag.column} ${diag.message} (${diag.code})`);
          } else {
            logger.warn(`${diag.filepath}:${diag.line}:${diag.column} ${diag.message} (${diag.code})`);
          }
        }

        if (options.fix && 0 < report.warningCount) {
          const config = await tdContext.getConfig();
          const files = await tdContext.listFiles();
          let fixedCount = 0;

          for (const filepath of files) {
            const result = await tdContext.formatFile(filepath);

            if (result.changed) {
              const fullPath = path.resolve(config.rootDir, filepath);

              fs.writeFileSync(fullPath, result.content);
              fixedCount++;
            }
          }

          if (0 < fixedCount) {
            logger.success(`Fixed ${fixedCount} file(s)`);
          }
        }

        const summary = `${report.fileCount} files checked, ${report.errorCount} error(s), ${report.warningCount} warning(s)`;

        if (0 < report.errorCount) {
          logger.error(summary);
          process.exit(1);
        } else {
          logger.success(summary);
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
