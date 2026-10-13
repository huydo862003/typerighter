// Generate .typedown/components.json from Vue component defineProps
// Extracts prop types via vue-component-meta and maps TS types to Typedown types

import fs, {
  globSync,
} from 'node:fs';
import path from 'node:path';
import type { TdComponentEntry } from '@typerighter/rpc-client';
import { toPascalCase } from '../../shared/utils/format';

type ComponentContract = Record<string, CustomComponentTyp>;

interface CustomComponentTyp {
  source: string;
  props: Record<string, CustomComponentProp>;
}

interface CustomComponentProp {
  type: string;
  required: boolean;
}

interface ResolvedComponent {
  name: string;
  source: string;
}

const CONTRACT_DIR = '.typedown';
const CONTRACT_FILE = 'components.json';

// Derive a PascalCase component name from a Vue filename
// e.g. "MyChart.vue" -> "MyChart", "video-embed.vue" -> "VideoEmbed"
function deriveComponentName (filename: string): string {
  return toPascalCase(path.basename(filename, '.vue'));
}

// Extract props from all declared components and write the contract
export async function generateComponentContract (
  entries: TdComponentEntry[],
  projectRoot: string,
  tsconfigPath: string,
): Promise<ComponentContract> {
  const resolved = resolveEntries(entries, projectRoot);

  if (resolved.length === 0) return {};

  const contract: ComponentContract = {};

  try {
    const {
      createChecker,
    } = await import('vue-component-meta');
    const checker = createChecker(tsconfigPath);

    for (const decl of resolved) {
      const absolutePath = path.resolve(projectRoot, decl.source);
      const props: Record<string, CustomComponentProp> = {};

      try {
        const meta = checker.getComponentMeta(absolutePath);

        for (const property of meta.props) {
          if (property.global) continue;

          props[property.name] = {
            type: mapTsType(property.type),
            required: property.required,
          };
        }
      } catch (error) {
        console.warn(`[typedown] Failed to extract props from ${decl.source}: ${error}`);
      }

      contract[decl.name] = {
        source: decl.source,
        props,
      };
    }

    checker.clearCache();
  } catch {
    console.warn('[typedown] vue-component-meta not available, skipping prop extraction');

    for (const decl of resolved) {
      contract[decl.name] = {
        source: decl.source,
        props: {},
      };
    }
  }

  return contract;
}

// Map a TS type string from vue-component-meta to a Typedown type expression
function mapTsType (tsType: string): string {
  const trimmed = tsType.trim();

  if (trimmed === 'string') return 'string';
  if (trimmed === 'number') return 'number';
  if (trimmed === 'boolean') return 'boolean';
  if (trimmed === 'Date') return 'date';

  const arrayBracket = trimmed.match(/^(.+)\[\]$/);

  if (arrayBracket) return `list[${mapTsType(arrayBracket[1])}]`;

  const arrayGeneric = trimmed.match(/^Array<(.+)>$/);

  if (arrayGeneric) return `list[${mapTsType(arrayGeneric[1])}]`;

  const recordMatch = trimmed.match(/^Record<(.+),\s*(.+)>$/);

  if (recordMatch) return `dict[${mapTsType(recordMatch[1])}, ${mapTsType(recordMatch[2])}]`;

  // Simple split on ' | ', does not handle unions inside generic arguments
  // e.g. Record<string, number | string> would be split incorrectly
  if (trimmed.includes(' | ')) {
    return trimmed.split(' | ').map(mapTsType)
      .join(' | ');
  }

  return 'any';
}

// Resolve component entries into concrete name/source pairs
// Later entries with the same name override earlier ones
function resolveEntries (
  entries: TdComponentEntry[],
  projectRoot: string,
): ResolvedComponent[] {
  const result = new Map<string, string>();

  for (const entry of entries) {
    if (entry.kind === 'dir') {
      const directory = path.resolve(projectRoot, entry.path);

      if (!fs.existsSync(directory)) {
        console.warn(`[typedown] Component directory not found: ${entry.path}`);
        continue;
      }

      for (const file of globSync('**/*.vue', {
        cwd: directory,
      })) {
        const name = deriveComponentName(file);
        const source = path.relative(projectRoot, path.join(directory, file));

        result.set(name, source);
      }
    } else {
      result.set(entry.name, entry.source);
    }
  }

  return [...result.entries()].map(([
    name,
    source,
  ]) => ({
    name,
    source,
  }));
}

// Write the contract to .typedown/components.json
export function writeComponentContract (
  contract: ComponentContract,
  projectRoot: string,
): void {
  const directory = path.join(projectRoot, CONTRACT_DIR);

  if (!fs.existsSync(directory)) {
    fs.mkdirSync(directory, {
      recursive: true,
    });
  }

  fs.writeFileSync(
    path.join(directory, CONTRACT_FILE),
    JSON.stringify(contract, undefined, 2) + '\n',
  );
}
