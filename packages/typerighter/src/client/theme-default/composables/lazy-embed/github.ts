import {
  onMounted, onUnmounted,
} from 'vue';
import type {
  LazyEmbedContext,
} from './types';
import {
  escapeHtml,
} from '@/shared/utils/html';
import type {
  ShikiHighlightFn,
  transformerLineNumbers,
} from '@/shared/utils/highlight';

// Register/unregister the GitHub embed renderer with the lazy-embed system
export function useGithubEmbed (context: LazyEmbedContext): void {
  const options = {
    selector: '.td-github-placeholder',
    onRender: renderPlaceholder,
  };

  onMounted(() => context.register(options));
  onUnmounted(() => context.unregister(options));
}

interface FigureOptions {
  highlighted: string;
  badge: string;
  label: string;
  lineCount: number;
  source: ParsedGithubUrl;
}

interface ParsedGithubUrl {
  owner: string;
  repo: string;
  ref: string;
  filepath: string;
  startLine: number | undefined;
  endLine: number | undefined;
  rawUrl: string;
  pageUrl: string;
  language: string;
}

// Icons

const GITHUB_ICON =
  '<svg width="14" height="14" viewBox="0 0 24 24" fill="currentColor" aria-hidden="true"><path d="M12 2C6.477 2 2 6.477 2 12c0 4.418 2.865 8.166 6.839 9.489.5.09.682-.217.682-.482 0-.237-.009-.868-.013-1.703-2.782.603-3.369-1.342-3.369-1.342-.454-1.154-1.11-1.462-1.11-1.462-.908-.62.069-.608.069-.608 1.003.07 1.531 1.03 1.531 1.03.892 1.529 2.341 1.087 2.91.831.092-.646.35-1.086.636-1.336-2.22-.253-4.555-1.11-4.555-4.943 0-1.091.39-1.984 1.029-2.683-.103-.253-.446-1.27.098-2.647 0 0 .84-.269 2.75 1.025A9.578 9.578 0 0 1 12 6.836c.85.004 1.705.115 2.504.337 1.909-1.294 2.747-1.025 2.747-1.025.546 1.377.202 2.394.1 2.647.64.699 1.028 1.592 1.028 2.683 0 3.842-2.339 4.687-4.566 4.935.359.309.678.919.678 1.852 0 1.336-.012 2.415-.012 2.741 0 .267.18.577.688.48C19.138 20.163 22 16.418 22 12c0-5.523-4.477-10-10-10z"/></svg>';

const COPY_ICON =
  '<svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true"><rect x="9" y="9" width="13" height="13" rx="2"></rect><path d="M5 15H4a2 2 0 0 1-2-2V4a2 2 0 0 1 2-2h9a2 2 0 0 1 2 2v1"></path></svg>';

/* Syntax highlighting */

// Lazily loaded highlight module + Shiki instance - only created when first embed appears
let shikiPromise:
  | Promise<{
    highlight: ShikiHighlightFn;
    transformerLineNumbers: typeof transformerLineNumbers;
    LANGUAGE_LABEL: Record<string, string>;
  }>
  | undefined;

// Build a placeholder div showing an error message when the embed fails
function buildErrorFigure (message: string): HTMLElement {
  const div = document.createElement('div');

  div.className = 'td-github-placeholder td-github-error';
  div.textContent = message;

  return div;
}

/* URL parsing */

// Build the full embed figure: header bar, highlighted code block, and footer
function buildFigure (options: FigureOptions): HTMLElement {
  const {
    highlighted, badge, label, lineCount, source,
  } = options;
  const lineFragment =
    source.startLine !== undefined
      ? `#L${source.startLine}${source.endLine !== undefined ? `-L${source.endLine}` : ''}`
      : '';
  const pageUrl = source.pageUrl + lineFragment;
  const figure = document.createElement('figure');

  figure.className =
    'td-code-block td-github-embed has-title line-numbers-mode';

  const figcaption = document.createElement('figcaption');

  figcaption.className = 'td-code-header';

  const left = document.createElement('span');

  left.className = 'td-code-left';
  left.innerHTML =
    `<span class="td-code-lang">${badge}</span>`
    + `<span class="td-code-label">${escapeHtml(label)}</span>`
    + `<span class="td-code-linecount">${lineCount} line${lineCount === 1 ? '' : 's'}</span>`;

  const actions = document.createElement('span');

  actions.className = 'td-code-actions';
  actions.innerHTML = `<button type="button" aria-label="Copy code" data-copied="Copied" class="td-code-copy">${COPY_ICON}<span class="td-code-copy-text">Copy</span></button>`;

  figcaption.appendChild(left);
  figcaption.appendChild(actions);
  figure.appendChild(figcaption);

  const wrapper = document.createElement('div');

  wrapper.innerHTML = highlighted;
  const pre = wrapper.firstElementChild;

  if (pre) figure.appendChild(pre);

  const isCommitSha = /^[0-9a-f]{40}$/i.test(source.ref);
  const repoUrl = `https://github.com/${source.owner}/${source.repo}`;
  const refUrl = isCommitSha
    ? `${repoUrl}/commit/${source.ref}`
    : `${repoUrl}/tree/${source.ref}`;

  const footer = document.createElement('div');

  footer.className = 'td-github-footer';
  footer.innerHTML =
    `<a class="td-github-footer-link td-github-footer-path" href="${escapeHtml(pageUrl)}" target="_blank" rel="noopener noreferrer">`
    + buildFilepathHtml(source.filepath)
    + '</a>'
    + `<a class="td-github-footer-link td-github-footer-repo" href="${escapeHtml(refUrl)}" target="_blank" rel="noopener noreferrer">`
    + `${GITHUB_ICON}${escapeHtml(source.owner)}/${escapeHtml(source.repo)} (${escapeHtml(formatRef(source.ref))})`
    + '</a>';

  figure.appendChild(footer);

  return figure;
}

/* DOM builders */

// Render a filepath as a breadcrumb: truncatable dir prefix + non-truncatable filename
function buildFilepathHtml (filepath: string): string {
  const slash = filepath.lastIndexOf('/');

  if (slash === -1) return escapeHtml(filepath);

  const directory = filepath.slice(0, slash + 1);
  const file = filepath.slice(slash + 1);

  return (
    `<span class="td-github-filepath-dir">${escapeHtml(directory)}</span>`
    + `<span class="td-github-filepath-file">${escapeHtml(file)}</span>`
  );
}

// Shorten a 40-character commit SHA to 7 chars; leave branch names unchanged
function formatRef (ref: string): string {
  return /^[0-9a-f]{40}$/i.test(ref) ? ref.slice(0, 7) : ref;
}

// Load the shared Shiki module on first call and cache the promise
function getShiki () {
  shikiPromise ??= import('@/shared/utils/highlight').then(async (module_) => ({
    highlight: await module_.createHighlighter(),
    transformerLineNumbers: module_.transformerLineNumbers,
    LANGUAGE_LABEL: module_.LANGUAGE_LABEL,
  }));

  return shikiPromise;
}

// Parse a GitHub blob URL into its components, returning undefined if not a valid GitHub URL
// Supports optional line range anchors: #L10 or #L10-L20
function parseGithubUrl (url: string): ParsedGithubUrl | undefined {
  // https://github.com/owner/repo/blob/ref/path/to/file[#L10[-L20]]
  const match =
    /^(https:\/\/github\.com\/([^/]+)\/([^/]+)\/blob\/([^/]+)\/(.+?))(?:#L(\d+)(?:-L(\d+))?)?$/.exec(
      url,
    );

  if (!match) return undefined;

  const [
    , pageUrl,
    owner,
    repo,
    ref,
    filepath,
    startString,
    endString,
  ] = match;
  const startLine =
    startString !== undefined ? parseInt(startString, 10) : undefined;
  const endLine = endString !== undefined ? parseInt(endString, 10) : undefined;
  const extension = filepath.split('.').pop() ?? '';

  return {
    owner,
    repo,
    ref,
    filepath,
    startLine,
    endLine,
    rawUrl: `https://raw.githubusercontent.com/${owner}/${repo}/${ref}/${filepath}`,
    pageUrl,
    language: extension,
  };
}

/* Render */

// In-memory cache for raw file content keyed by raw URL
const fetchCache = new Map<string, string>();

// Fetch, highlight, and replace a placeholder element with the rendered embed figure
async function renderPlaceholder (element: Element): Promise<void> {
  const placeholder = element as HTMLDivElement;
  const url = placeholder.getAttribute('data-url') ?? '';
  const label = placeholder.getAttribute('data-label') ?? '';
  const title = placeholder.getAttribute('data-title') ?? '';

  const parsed = parseGithubUrl(url);

  if (!parsed) {
    placeholder.replaceWith(buildErrorFigure('Invalid GitHub URL'));

    return;
  }

  let rawContent: string;

  if (fetchCache.has(parsed.rawUrl)) {
    rawContent = fetchCache.get(parsed.rawUrl)!;
  } else {
    try {
      const response = await fetch(parsed.rawUrl);

      if (!response.ok) throw new Error(`${response.status}`);
      rawContent = await response.text();
      fetchCache.set(parsed.rawUrl, rawContent);
    } catch {
      placeholder.replaceWith(
        buildErrorFigure('Failed to fetch file from GitHub'),
      );

      return;
    }
  }

  const allLines = rawContent.split('\n');
  const startLine = parsed.startLine ?? 1;
  const endLine = parsed.endLine ?? allLines.length;
  const slicedLines = allLines.slice(startLine - 1, endLine);
  const slicedContent = slicedLines.join('\n');

  let highlighted: string;

  const {
    highlight, transformerLineNumbers, LANGUAGE_LABEL,
  } =
    await getShiki();

  try {
    highlighted = await highlight(slicedContent, parsed.language, [transformerLineNumbers(startLine)]);
  } catch {
    placeholder.replaceWith(buildErrorFigure('Failed to highlight code'));

    return;
  }

  const filename = parsed.filepath.split('/').pop() ?? parsed.filepath;
  const badge =
    LANGUAGE_LABEL[parsed.language.toLowerCase()]
    ?? (parsed.language.toUpperCase() || 'Code');

  const figure = buildFigure({
    highlighted,
    badge,
    label: title || label || filename,
    lineCount: slicedLines.length,
    source: parsed,
  });

  placeholder.replaceWith(figure);
}
