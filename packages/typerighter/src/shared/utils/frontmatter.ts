// Extracts specific builtin fields from raw .td file content without a full YAML parse

export interface FrontmatterBuiltinFields {
  label: string | undefined;
  showInSidebar: boolean;
}

const LABEL_RE = /^_label:\s*["'](.+)["']\s*$/;
const SSG_BLOCK_RE = /^_ssg:\s*$/;
const SHOW_IN_SIDEBAR_RE = /^\s+show_in_sidebar:\s*(true|false)\s*$/;

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
  let inSsgBlock = false;

  for (const line of lines) {
    if (line.startsWith(' ') || line.startsWith('\t')) {
      if (inSsgBlock) {
        const match = SHOW_IN_SIDEBAR_RE.exec(line);

        if (match) {
          showInSidebar = match[1] === 'true';
          inSsgBlock = false;
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
  };
}
