import fs from 'node:fs/promises';
import path from 'node:path';
import {
  cancel, intro, isCancel, log, outro, select, text,
} from '@clack/prompts';
import {
  CONFIG_FILE_NAMES,
} from '../context';
import {
  BRAND_ARROW, BRAND_BORDER, BRAND_COLOR, BRAND_LETTER, BRAND_VIEWBOX,
} from '@/shared/brand';

export interface InitializeFlags {
  name?: string;
  title?: string;
  description?: string;
  yes?: boolean;
}

export async function initialize (root: string, flags: InitializeFlags = {}): Promise<void> {
  const interactive = !hasAllFlags(flags);
  const options = interactive
    ? await collectUserInput(root, flags)
    : toOptions(flags);

  await scaffold(root, options);

  if (interactive) {
    printFurtherSteps(root);
  }
}

interface ExistingProject {
  hasPackageJson: boolean;
  hasTypedownYaml: boolean;
  packageName: string | undefined;
}

interface InitializeOptions {
  projectName: string;
  siteTitle: string;
  siteDescription: string;
}

async function collectUserInput (
  root: string,
  flags: InitializeFlags,
): Promise<InitializeOptions> {
  intro('typedown');

  const existing = await detectExistingProject(root);

  if (existing.hasTypedownYaml && !flags.yes) {
    await confirmExistingProject();
  }

  const projectName = flags.name ?? await prompt('Project name', existing.packageName ?? path.basename(root));
  const siteTitle = flags.title ?? await prompt('Site title', projectName);
  const siteDescription = flags.description ?? await prompt('Site description', 'A typedown site');

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
  };

  try {
    const raw = await fs.readFile(path.join(root, 'package.json'), 'utf-8');
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
      await fs.access(path.join(root, name));
      result.hasTypedownYaml = true;
      break;
    } catch {
      // No config file
    }
  }

  return result;
}

function generateFaviconSvg (): string {
  return `<svg fill="none" viewBox="${BRAND_VIEWBOX}" xmlns="http://www.w3.org/2000/svg">
  <path clip-rule="evenodd" d="${BRAND_BORDER}" fill-rule="evenodd" fill="${BRAND_COLOR}"/>
  <path d="${BRAND_LETTER}" fill="${BRAND_COLOR}"/>
  <path d="${BRAND_ARROW}" fill="${BRAND_COLOR}"/>
</svg>
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

function generateRobotsText (): string {
  return `User-agent: *
Allow: /

Sitemap: /sitemap.xml
`;
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
  root_dir: "vault"
site:
  title: "${options.siteTitle}"
  description: "${options.siteDescription}"
`;
}

function hasAllFlags (flags: InitializeFlags): flags is Required<InitializeFlags> {
  return flags.name !== undefined && flags.title !== undefined && flags.description !== undefined;
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
  const vaultDirectory = path.join(root, 'vault');
  const typeDirectory = path.join(root, 'vault', '_types');
  const publicDirectory = path.join(root, 'public');
  const localDirectory = path.join(root, '.typedown', '.local');

  await Promise.all([
    fs.mkdir(vaultDirectory, {
      recursive: true,
    }),
    fs.mkdir(typeDirectory, {
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
    writeIfMissing(publicDirectory, 'favicon.svg', generateFaviconSvg(), {
      silent: true,
    }),
    writeIfMissing(publicDirectory, 'robots.txt', generateRobotsText(), {
      silent: true,
    }),
  ]);

  const samples = await Promise.all([
    writeIfMissing(typeDirectory, 'Article.td', generateSampleTdSchema(), {
      silent: true,
    }),
    writeIfMissing(vaultDirectory, 'hello.td', generateSampleTdContent(), {
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

function toOptions (flags: Required<InitializeFlags>): InitializeOptions {
  return {
    projectName: flags.name,
    siteTitle: flags.title,
    siteDescription: flags.description,
  };
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
