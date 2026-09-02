/*
* Copied from https://github.com/vuejs/vitepress/blob/main/src/node/markdown/plugins/restoreEntities.ts
* Commit: 2fa0ded
* */

// markdown-it's default text rendering uses utils.escapeHtml which double-encodes HTML entities (e.g. `&amp;` becomes `&amp;amp;`)
// This plugin replaces the built-in text_join rule and text renderer to preserve entities as-is
// See: https://github.com/vuejs/vitepress/pull/3882

import type {
  MarkdownItAsync,
} from 'markdown-it-async';
import type StateCore from 'markdown-it/lib/rules_core/state_core.mjs';
import type Token from 'markdown-it/lib/token.mjs';

export function restoreEntities (md: MarkdownItAsync): void {
  const {
    escapeHtml,
  } = md.utils;

  md.core.ruler.at('text_join', textJoin);
  md.renderer.rules.text = function (tokens, index): string {
    return escapeHtml(tokens[index].content);
  };
}

// Get the raw content of a token, preserving entity markup and escaped ampersands
function getContent (token: Token): string {
  if (token.info === 'entity') return token.markup;
  if (token.info === 'escape' && token.content === '&') return '&amp;';

  return token.content;
}

// Merge adjacent text/text_special tokens into one, preserving entity markup
function textJoin (state: StateCore): void {
  const blockTokens = state.tokens;
  const length = blockTokens.length;

  for (let jj = 0; jj < length; ++jj) {
    if (blockTokens[jj].type !== 'inline') continue;

    const tokens = blockTokens[jj].children || [];
    const max = tokens.length;
    let curr: number;
    let last: number;

    for (curr = 0; curr < max; ++curr) {
      // text_special tokens are created by markdown-it for HTML entities (&amp;) and backslash escapes (\&)
      if (tokens[curr].type === 'text_special') tokens[curr].type = 'text'; // Convert them to regular text so they can be merged below
    }

    // Merge consecutive text tokens into one, using getContent() to preserve the raw entity markup
    for (curr = 0, last = 0; curr < max; ++curr) {
      if (
        tokens[curr].type === 'text'
        && curr + 1 < max
        && tokens[curr + 1].type === 'text'
      ) {
        // Fold current token's content into the next one
        tokens[curr + 1].content =
          getContent(tokens[curr]) + getContent(tokens[curr + 1]);
        tokens[curr + 1].info = '';
        tokens[curr + 1].markup = '';
      } else {
        // Keep this token, shift it to the compacted position
        if (curr !== last) tokens[last] = tokens[curr];
        ++last;
      }
    }

    // Trim the array to remove the merged-away tokens
    if (curr !== last) tokens.length = last;
  }
}
