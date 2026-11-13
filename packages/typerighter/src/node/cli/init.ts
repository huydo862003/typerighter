import fs from 'node:fs/promises';
import {
  existsSync,
} from 'node:fs';
import path from 'node:path';
import {
  execSync,
} from 'node:child_process';
import {
  cancel,
  confirm,
  intro,
  isCancel,
  log,
  outro,
  select,
  text,
} from '@clack/prompts';
import {
  CONFIG_FILE_NAMES,
} from '../context';
import {
  detectPackageManager,
} from '../sys-utils/detect-package-manager';
import {
  BRAND_ARROW,
  BRAND_BORDER,
  BRAND_COLOR,
  BRAND_LETTER,
  BRAND_VIEWBOX,
} from '@/shared/brand';

export interface InitializeFlags {
  name?: string;
  title?: string;
  description?: string;
  yes?: boolean;
}

export async function initialize (
  root: string,
  flags: InitializeFlags = {},
): Promise<void> {
  const interactive = !hasAllFlags(flags);
  const options = interactive
    ? await collectUserInput(root, flags)
    : toOptions(flags);

  // Create root directory if it doesn't exist
  if (!existsSync(root)) {
    await fs.mkdir(root, {
      recursive: true,
    });
  }

  await scaffold(root, options);

  const packageManager = detectPackageManager(root);

  if (interactive) {
    await postScaffoldSteps(root, packageManager);
  } else if (flags.yes) {
    // Non-interactive: auto-install deps, skip editor setup
    installDependencies(root, packageManager);
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
  siteOrigin: string;
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

  const projectName =
    flags.name
    ?? (await prompt('Project name', existing.packageName ?? path.basename(root)));
  const siteTitle = flags.title ?? (await prompt('Site title', projectName));
  const siteDescription =
    flags.description ?? (await prompt('Site description', 'A typedown site'));
  const siteOrigin = await prompt(
    'Site origin (used for sitemap and robots.txt)',
    'https://example.com',
  );

  return {
    projectName,
    siteTitle,
    siteDescription,
    siteOrigin,
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

function generateArticleSchema (): string {
  return `---
_type: schema
properties:
  title:
    type: string
  description:
    type: string?
  tags:
    type: list[string]?
---
`;
}

function generateComponentsDocs (): string {
  return `---
_type: Article
title: "Built-in components"
description: "Components that ship with the default theme"
---

Typedown's default theme ships two built-in Vue components you can use in any \`.td\` file.

## Flashcard

Use a \`flashcard\` container fence to create a flip card for study or review.

\`\`\`
::: flashcard
What does \`fref\` stand for in Typedown?
=== key
File reference: a typed link from one \`.td\` node to another.
:::
\`\`\`

The rendered result is interactive: click or press Space to reveal the answer.

::: flashcard
What does \`fref\` stand for in Typedown?
=== key
File reference: a typed link from one \`.td\` node to another.
:::

The card starts face-down. Pass \`revealed\` to start with the answer visible, or \`disabled\` to prevent flipping.

## Glossary index

\`glossary-index\` renders all sibling pages in the current directory as a searchable, alphabetical index. Place it on a folder's \`index.td\` page.

Informative mode (default) shows the \`description\` and \`tags\` from each page's frontmatter:

\`\`\`md
[[glossary-index]]
\`\`\`

Dense mode renders a compact three-column link list suited for large directories:

\`\`\`html
[[glossary-index {mode="dense"}]]
\`\`\`

For each entry the component reads the \`description\` and \`tags\` fields from the page's schema, so make sure your schema exposes them.
`;
}

function generateCoreConceptsTd (): string {
  return `---
_type: Note
title: "Core concepts"
description: "Schemas, frefs, and vault queries explained"
tags:
  - "reference"
author: "You"
---

## Defining a schema

A schema is a \`.td\` file with \`_type: schema\` in the frontmatter.

\`\`\`yaml
---
_type: schema
properties:
  title:
    type: string
  published:
    type: date?
  tags:
    type: list[string]?
---
\`\`\`

Place it in \`_types/\` and reference it from any file with \`_type: SchemaName\`.

## fref

\`fref("./path/to/file.td")\` creates a typed link to another node.
Access its fields directly: \`fref("./other.td").title\`.

frefs are rename-safe - the LSP tracks all references and updates them when a file moves.

## Vault queries

Query all nodes of a given schema:

\`\`\`
vault.from_nodes(Note).filter((n) -> n.tags).to_list()
\`\`\`

Use \`.first()\` to get a single result, \`.to_list()\` for an array.
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

function generateGettingStartedTd (): string {
  return `---
_type: Note
title: "Getting started"
description: "A quick tour of typedown basics"
tags:
  - "intro"
author: "You"
---

## Schemas

This file uses \`_type: Note\`, defined in \`_types/Note.td\`.
The \`title\`, \`description\`, \`tags\`, and \`author\` fields above are all type-checked.
Try changing \`author\` to a number - the LSP will flag it.

## Body interpolation

Frontmatter values are available in the body as expressions.
This note was written by **\${self.author}** and is titled "\${self.title}".

## fref

A \`fref\` is a typed link to another file.
The companion note is titled "\${fref("./core-concepts.td").title}".

Rename \`core-concepts.td\` and the LSP will update this fref automatically.

## Directory index

The [notes index](/notes) uses \`glossary-index\` to list this page and its siblings.
Add more \`.td\` files to this folder and they appear there without any config.
`;
}

function generateIndexTd (): string {
  return `---
_type: Article
title: "Quick start"
description: "Your new typedown vault"
---

This is your vault\'s home page. Each \`.td\` file is a page - frontmatter at the top, Markdown body below.

## File structure

\`\`\`
vault/
  _types/       schema definitions
  notes/        a section with its own directory index
  index.td      this page
  components.td theme component demos
\`\`\`

## Commands

| Command | Action |
| --- | --- |
| \`dev\` | Start the dev server with hot reload |
| \`build\` | Build the static site |
| \`preview\` | Preview the built site locally |
| \`check\` | Type-check the whole vault |

## Next steps

- Open [notes/getting-started](/notes/getting-started) to see schemas and frefs in action
- Open [components](/components) to explore the built-in theme components
- Edit \`_types/Article.td\` to add fields to this page\'s schema
`;
}

function generateNoteSchema (): string {
  return `---
_type: schema
properties:
  title:
    type: string
  description:
    type: string?
  tags:
    type: list[string]?
  author:
    type: string?
---
`;
}

function generateNotesIndexTd (): string {
  return `---
_type: Article
title: "Notes"
description: "All notes in this vault"
---

[[glossary-index]]
`;
}

function generatePackageJson (options: InitializeOptions): string {
  return (
    JSON.stringify(
      {
        name: options.projectName,
        version: '0.0.0',
        private: true,
        type: 'module',
        scripts: {
          dev: 'typerighter dev',
          build: 'typerighter build',
          preview: 'typerighter preview',
          check: 'typerighter check',
        },
        devDependencies: {
          typerighter: `^${__VERSION__}`,
          // typescript is a peer required by vue-component-meta at build time
          typescript: '^5.0.0',
        },
      },
      undefined,
      2,
    ) + '\n'
  );
}

function generateRobotsText (): string {
  return `User-agent: *
Allow: /

Sitemap: /sitemap.xml
`;
}

function generateTypedownYaml (options: InitializeOptions): string {
  const originLine =
    options.siteOrigin !== '' ? `\n  origin: "${options.siteOrigin}"` : '';

  return `# yaml-language-server: $schema=./node_modules/typerighter/dist/typedown.schema.json
version: "1.0.0"
vault:
  root_dir: "vault"
site:
  title: "${options.siteTitle}"
  description: "${options.siteDescription}"${originLine}
`;
}

function hasAllFlags (
  flags: InitializeFlags,
): flags is Required<InitializeFlags> {
  return (
    flags.name !== undefined
    && flags.title !== undefined
    && flags.description !== undefined
  );
}

function installDependencies (root: string, packageManager: string): void {
  log.step(`Installing dependencies with ${packageManager}...`);
  execSync(`${packageManager} install`, {
    cwd: root,
    stdio: 'inherit',
  });

  log.success('Dependencies installed');
}

async function postScaffoldSteps (root: string, packageManager: string): Promise<void> {
  // 1. Offer to install dependencies
  const shouldInstall = await confirm({
    message: `Install dependencies with ${packageManager}?`,
    initialValue: true,
  });

  if (isCancel(shouldInstall)) {
    cancel('Cancelled.');
    process.exit(0);
  }

  if (shouldInstall) {
    installDependencies(root, packageManager);
  }

  // 2. Offer to set up editor
  const shouldSetupEditor = await confirm({
    message: 'Set up your editor for Typedown?',
    initialValue: true,
  });

  if (isCancel(shouldSetupEditor)) {
    cancel('Cancelled.');
    process.exit(0);
  }

  if (shouldSetupEditor) {
    const {
      setupEditor,
    } = await import('./setup-editor');

    await setupEditor(undefined);
  }

  // 3. Print next steps
  const runDev = packageManager === 'npm' ? 'npm run dev' : `${packageManager} dev`;
  const steps = ['Done.'];

  if (root !== process.cwd()) {
    steps.push(`cd ${path.relative(process.cwd(), root)}`);
  }

  if (!shouldInstall) {
    steps.push(`${packageManager} install`);
  }

  steps.push(runDev);

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

async function scaffold (
  root: string,
  options: InitializeOptions,
): Promise<void> {
  const vaultDirectory = path.join(root, 'vault');
  const typeDirectory = path.join(root, 'vault', '_types');
  const notesDirectory = path.join(root, 'vault', 'notes');
  const publicDirectory = path.join(root, 'public');
  const localDirectory = path.join(root, '.typedown', '.local');

  await Promise.all([
    fs.mkdir(vaultDirectory, {
      recursive: true,
    }),
    fs.mkdir(typeDirectory, {
      recursive: true,
    }),
    fs.mkdir(notesDirectory, {
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
    // Schemas
    writeIfMissing(typeDirectory, 'Article.td', generateArticleSchema(), {
      silent: true,
    }),
    writeIfMissing(typeDirectory, 'Note.td', generateNoteSchema(), {
      silent: true,
    }),
    // Vault root
    writeIfMissing(vaultDirectory, 'index.td', generateIndexTd(), {
      silent: true,
    }),
    writeIfMissing(vaultDirectory, 'components.td', generateComponentsDocs(), {
      silent: true,
    }),
    // Notes section
    writeIfMissing(notesDirectory, 'index.td', generateNotesIndexTd(), {
      silent: true,
    }),
    writeIfMissing(notesDirectory, 'getting-started.td', generateGettingStartedTd(), {
      silent: true,
    }),
    writeIfMissing(notesDirectory, 'core-concepts.td', generateCoreConceptsTd(), {
      silent: true,
    }),
  ]);

  const created = samples.filter(Boolean);
  const skipped = samples.filter((sample) => !Boolean(sample));

  if (0 < created.length) {
    log.success(`created sample vault (${created.length} files)`);
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
    siteOrigin: '',
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
