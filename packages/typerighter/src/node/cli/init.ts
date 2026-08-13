import fs from 'node:fs/promises';
import path from 'node:path';
import {
  load,
} from 'js-yaml';
import {
  cancel, intro, isCancel, log, outro, select, text,
} from '@clack/prompts';
import {
  VIRTUAL_APP_ID,
} from '../plugin/vite/constants';
import {
  CONFIG_FILE_NAMES,
} from '../context';
import {
  escapeHtml,
} from '@/shared';
import {
  BRAND_ARROW, BRAND_BORDER, BRAND_COLOR, BRAND_LETTER, BRAND_VIEWBOX,
} from '@/shared/brand';

// Interactive project scaffolding
export async function initialize (root: string): Promise<void> {
  const options = await collectUserInput(root);

  await scaffold(root, options);

  printFurtherSteps(root);
}

interface ExistingProject {
  hasPackageJson: boolean;
  hasTypedownYaml: boolean;
  packageName: string | undefined;
  siteTitle: string | undefined;
  siteDescription: string | undefined;
}

interface InitializeOptions {
  projectName: string;
  siteTitle: string;
  siteDescription: string;
}

async function collectUserInput (
  root: string,
): Promise<InitializeOptions> {
  intro('typedown');

  const existing = await detectExistingProject(root);

  if (existing.hasTypedownYaml) {
    await confirmExistingProject();
  }

  const projectName = await prompt('Project name', existing.packageName ?? path.basename(root));
  const siteTitle = await prompt('Site title', existing.siteTitle ?? projectName);
  const siteDescription = await prompt('Site description', existing.siteDescription ?? 'A typedown site');

  return {
    projectName,
    siteTitle,
    siteDescription,
  };
}

async function confirmExistingProject (): Promise<void> {
  log.warn('typedown.yaml already exists in this directory');

  const proceed = await select({
    message: 'What would you like to do?',
    options: [
      {
        value: 'scaffold',
        label: 'Continue scaffolding (skip existing files)',
      },
      {
        value: 'cancel',
        label: 'Cancel',
      },
    ],
  });

  if (isCancel(proceed) || proceed === 'cancel') {
    cancel('Cancelled.');
    process.exit(0);
  }
}

async function detectExistingProject (root: string): Promise<ExistingProject> {
  const result: ExistingProject = {
    hasPackageJson: false,
    hasTypedownYaml: false,
    packageName: undefined,
    siteTitle: undefined,
    siteDescription: undefined,
  };

  const packagePath = path.join(root, 'package.json');

  try {
    const raw = await fs.readFile(packagePath, 'utf-8');
    const package_ = JSON.parse(raw);

    result.hasPackageJson = true;
    if (typeof package_.name === 'string' && 0 < package_.name.length) {
      result.packageName = package_.name;
    }
  } catch {
    // No package.json
  }

  for (const name of CONFIG_FILE_NAMES) {
    try {
      const raw = await fs.readFile(path.join(root, name), 'utf-8');
      const document = load(raw) as Record<string, unknown> | undefined;

      result.hasTypedownYaml = true;

      const site = document?.site as Record<string, unknown> | undefined;

      if (typeof site?.title === 'string') {
        result.siteTitle = site.title;
      }

      if (typeof site?.description === 'string') {
        result.siteDescription = site.description;
      }
      break;
    } catch {
      // No config file or invalid YAML
    }
  }

  return result;
}

function generateIndexHtml (options: InitializeOptions): string {
  return `<!doctype html>
<html lang="en">
  <head>
    <meta charset="UTF-8" />
    <meta name="viewport" content="width=device-width, initial-scale=1.0" />
    <title>${escapeHtml(options.siteTitle)}</title>
    <link rel="icon" href="/favicon.svg" type="image/svg+xml">
  </head>
  <body>
    <div id="app"></div>
    <script
      type="module"
      src="/${VIRTUAL_APP_ID}"
    ></script>
  </body>
</html>
`;
}

function generatePackageJson (options: InitializeOptions): string {
  return JSON.stringify({
    name: options.projectName,
    version: '0.0.0',
    private: true,
    type: 'module',
    scripts: {
      dev: 'typerighter dev',
      build: 'typerighter build',
      preview: 'typerighter preview',
    },
    devDependencies: {
      typerighter: `^${__VERSION__}`,
    },
  }, undefined, 2) + '\n';
}

function generateSampleTdContent (): string {
  return `---
_type: Article
title: "Hello, world"
---

Welcome to your new typedown site.
`;
}

function generateSampleTdSchema (): string {
  return `---
_type: schema
properties:
  title:
    type: string
  tags:
    type: list[string]
    optional: true
---
`;
}

function generateTypedownYaml (options: InitializeOptions): string {
  return `version: "1.0.0"
vault:
  content_dir: vault/content
  schema_dir: vault/schemas
site:
  title: "${options.siteTitle}"
  description: "${options.siteDescription}"
`;
}

function generateFaviconSvg (): string {
  return `<svg fill="none" viewBox="${BRAND_VIEWBOX}" xmlns="http://www.w3.org/2000/svg">
  <path clip-rule="evenodd" d="${BRAND_BORDER}" fill-rule="evenodd" fill="${BRAND_COLOR}"/>
  <path d="${BRAND_LETTER}" fill="${BRAND_COLOR}"/>
  <path d="${BRAND_ARROW}" fill="${BRAND_COLOR}"/>
</svg>
`;
}

function printFurtherSteps (root: string): void {
  const steps = ['Done scaffolding.'];

  if (root !== '.') {
    steps.push(`cd ${root}`);
  }

  steps.push('pnpm install');

  steps.push('pnpm dev');

  outro(steps.join('\n'));
}

async function prompt (message: string, defaultValue: string): Promise<string> {
  const value = await text({
    message,
    placeholder: defaultValue,
    defaultValue,
  });

  if (isCancel(value)) {
    cancel('Cancelled.');
    process.exit(0);
  }

  return value;
}

async function scaffold (root: string, options: InitializeOptions): Promise<void> {
  const contentDirectory = path.join(root, 'vault', 'content');
  const schemaDirectory = path.join(root, 'vault', 'schemas');
  const publicDirectory = path.join(root, 'public');
  const localDirectory = path.join(root, '.typedown', '.local');

  await Promise.all([
    fs.mkdir(contentDirectory, {
      recursive: true,
    }),
    fs.mkdir(schemaDirectory, {
      recursive: true,
    }),
    fs.mkdir(publicDirectory, {
      recursive: true,
    }),
    fs.mkdir(localDirectory, {
      recursive: true,
    }),
  ]);

  await writeIfMissing(localDirectory, '.gitignore', '*\n', {
    silent: true,
  });

  await Promise.all([
    writeIfMissing(root, '.gitignore', 'node_modules/\ndist/\n', {
      silent: true,
    }),
    writeIfMissing(root, 'package.json', generatePackageJson(options)),
    writeIfMissing(root, 'typedown.yaml', generateTypedownYaml(options)),
    writeIfMissing(root, 'index.html', generateIndexHtml(options)),
    writeIfMissing(publicDirectory, 'favicon.svg', generateFaviconSvg(), {
      silent: true,
    }),
  ]);

  const samples = await Promise.all([
    writeIfMissing(schemaDirectory, 'Article.td', generateSampleTdSchema(), {
      silent: true,
    }),
    writeIfMissing(contentDirectory, 'hello.td', generateSampleTdContent(), {
      silent: true,
    }),
  ]);

  const created = samples.filter(Boolean);
  const skipped = samples.filter((sample) => !Boolean(sample));

  if (0 < created.length) {
    log.success(`created sample schema and content (${created.length} files)`);
  }

  if (0 < skipped.length) {
    log.warn(`skipped ${skipped.length} sample files (already exist)`);
  }
}

async function writeIfMissing (
  directory: string,
  name: string,
  content: string,
  options: {
    silent?: boolean;
  } = {},
): Promise<boolean> {
  const filepath = path.join(directory, name);

  try {
    await fs.access(filepath);
    if (!options.silent) log.warn(`skip ${name} (already exists)`);

    return false;
  } catch {
    await fs.writeFile(filepath, content);
    if (!options.silent) log.success(`created ${name}`);

    return true;
  }
}
