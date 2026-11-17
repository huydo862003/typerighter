// Extracts specific builtin fields from raw .td file content without a full YAML parse

export interface FrontmatterBuiltinFields {
  label: string | undefined;
  showInSidebar: boolean;
  showInSearch: boolean;
  crawlable: boolean;
}

const LABEL_RE = /^_label:\s*["'](.+)["']\s*$/;
const SSG_BLOCK_RE = /^_ssg:\s*$/;
const SSG_BOOL_RE = /^\s+(show_in_sidebar|show_in_search|crawlable):\s*(true|false)\s*$/;

// Returns undefined if the file has no frontmatter block
export function extractFrontmatterBuiltinFields (
  tdSource: string,
): FrontmatterBuiltinFields | undefined {
  if (!tdSource.startsWith('---')) return undefined;

  const end = tdSource.indexOf('\n---', 3);

  if (end === -1) return undefined;

  const lines = tdSource.slice(4, end).split('\n');

  let label: string | undefined;
  let showInSidebar = true;
  let showInSearch = true;
  let crawlable = true;
  let inSsgBlock = false;

  for (const line of lines) {
    if (line.startsWith(' ') || line.startsWith('\t')) {
      if (inSsgBlock) {
        const match = SSG_BOOL_RE.exec(line);

        if (match) {
          const value = match[2] === 'true';

          switch (match[1]) {
            case 'show_in_sidebar':
              showInSidebar = value;
              break;
            case 'show_in_search':
              showInSearch = value;
              break;
            case 'crawlable':
              crawlable = value;
              break;
          }
        }
      }
      continue;
    }

    inSsgBlock = false;

    if (label === undefined) {
      const match = LABEL_RE.exec(line);

      if (match) {
        label = match[1].trim();
        continue;
      }
    }

    if (SSG_BLOCK_RE.test(line)) {
      inSsgBlock = true;
    }
  }

  return {
    label,
    showInSidebar,
    showInSearch,
    crawlable,
  };
}
