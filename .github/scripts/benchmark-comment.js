// Generates a benchmark comparison comment for PRs
// Called by the benchmark workflow via actions/github-script
//
// eslint-disable-next-line @typescript-eslint/no-require-imports
const fs = require('node:fs');

const MARKER = '<!-- benchmark-comment -->';

module.exports = async ({
  github, context,
}) => {
  const current = JSON.parse(
    fs.readFileSync('results/benchmark-current/benchmark-current.json', 'utf8'),
  );
  const base = JSON.parse(
    fs.readFileSync('results/benchmark-main/benchmark-main.json', 'utf8'),
  );

  const body = formatComment(current, base, context);

  await createOrUpdateComment(github, context, body);
};

async function createOrUpdateComment (github, context, body) {
  const params = {
    owner: context.repo.owner,
    repo: context.repo.repo,
    issue_number: context.issue.number,
  };

  const {
    data: comments,
  } = await github.rest.issues.listComments(params);
  const existing = comments.find((comment) => comment.body.includes(MARKER));

  if (existing) {
    await github.rest.issues.updateComment({
      ...params,
      comment_id: existing.id,
      body,
    });
  } else {
    await github.rest.issues.createComment({
      ...params,
      body,
    });
  }
}

function formatChange (currentValue, baseValue, lowerIsBetter) {
  const current = parseFloat(currentValue);
  const base = parseFloat(baseValue);

  if (isNaN(current) || isNaN(base) || base === 0) return '';

  const percent = ((current - base) / base) * 100;

  if (Math.abs(percent) < 0.5) return '~';

  const improved = lowerIsBetter ? percent < 0 : 0 < percent;
  const sign = 0 < percent ? '+' : '';
  const icon = improved ? '\u2705' : '\u26a0\ufe0f';

  return `${sign}${percent.toFixed(1)}% ${icon}`;
}

// Builds the full markdown comment body with two sections:
// artifact sizes (what users download) and performance (build speed)
function formatComment (current, base, context) {
  const headSha = context.payload.pull_request?.head?.sha?.slice(0, 7) ?? '';
  const baseSha = context.payload.pull_request?.base?.sha?.slice(0, 7) ?? '';

  // DX: compile time and binary size (what developers wait for)
  const dxRows = [
    renderRow({
      label: 'Compile time',
      current: current.lib_build_s,
      base: base.lib_build_s,
      unit: 's',
    }),
    renderRow({
      label: 'Binary size (stripped)',
      current: current.binary_stripped_kb,
      base: base.binary_stripped_kb,
      unit: 'KB',
    }),
    renderRow({
      label: 'Install size (npm)',
      current: current.install_kb,
      base: base.install_kb,
      unit: 'KB',
    }),
  ];

  // UX: SSG runtime and output sizes (what end users experience)
  const uxRows = [
    renderRow({
      label: 'Build speed',
      current: current.ssg_ms_per_page,
      base: base.ssg_ms_per_page,
      unit: 'ms/page',
    }),
    renderRow({
      label: `Build time (${current.pages} pages)`,
      current: current.ssg_build_s,
      base: base.ssg_build_s,
      unit: 's',
    }),
    renderRow({
      label: 'JS bundle (gzip)',
      current: current.js_gz_kb,
      base: base.js_gz_kb,
      unit: 'KB',
    }),
    renderRow({
      label: 'CSS (gzip)',
      current: current.css_gz_kb,
      base: base.css_gz_kb,
      unit: 'KB',
    }),
  ];

  // Assemble the markdown comment
  return [
    MARKER,
    `## Benchmark \`${baseSha}\` -> \`${headSha}\``,
    '',
    '**DX** (library compile)',
    '',
    '| Metric | base | PR | Change |',
    '|--------|-----:|---:|--------|',
    ...dxRows,
    '',
    '**UX** (SSG runtime, 1.3k page vault)',
    '',
    '| Metric | base | PR | Change |',
    '|--------|-----:|---:|--------|',
    ...uxRows,
    '',
    // Collapsible raw sizes for debugging
    '<details><summary>Raw sizes</summary>',
    '',
    '| Metric | base | PR |',
    '|--------|-----:|---:|',
    `| LSP binary (debug) | ${base.binary_kb} KB | ${current.binary_kb} KB |`,
    `| JS bundle (raw) | ${base.js_kb} KB | ${current.js_kb} KB |`,
    `| CSS (raw) | ${base.css_kb} KB | ${current.css_kb} KB |`,
    `| HTML | ${base.html_kb} KB | ${current.html_kb} KB |`,
    `| Total output | ${base.total_size_kb} KB | ${current.total_size_kb} KB |`,
    '',
    '</details>',
  ].join('\n');
}

function renderRow ({
  label, current: currentValue, base: baseValue, unit, lowerIsBetter = true,
}) {
  const change = formatChange(currentValue, baseValue, lowerIsBetter);
  const formatValue = (value) => unit ? `${value} ${unit}` : `${value}`;

  return `| ${label} | ${formatValue(baseValue)} | ${formatValue(currentValue)} | ${change} |`;
}
