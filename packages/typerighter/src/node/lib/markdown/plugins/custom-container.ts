/* This file lets you create block-level custom containers in Markdown
 * component-container.ts
 * ::: custom_vue {prop1="" prop2}
 * default slot
 * === slot-name2
 * slot 2
 * :::
 */
import {
  isHTMLTag,
  isMathMLTag,
  isSVGTag,
} from '@vue/shared';
import {
  container,
} from '@mdit/plugin-container';
import {
  attrs,
} from '@mdit/plugin-attrs';
import type {
  MarkdownItAsync,
} from 'markdown-it-async';
import type Token from 'markdown-it/lib/token.mjs';
import type StateBlock from 'markdown-it/lib/rules_block/state_block.mjs';
import type StateCore from 'markdown-it/lib/rules_core/state_core.mjs';
import {
  DEFAULT_TITLES,
} from './callout-container';

const NAME = 'vue_component';
const OPEN = `container_${NAME}_open`;
const CLOSE = `container_${NAME}_close`;

// The whole header with unparsed props
const VALIDATE_RE = /^([A-Za-z_][A-Za-z0-9_-]*)\s*(?:\{.*\})?\s*$/;
// The header with stripped props
const HEADER_RE = /^([A-Za-z_][A-Za-z0-9_-]*)\s*$/;
// The container slot sep pattern
const SLOT_RE = /^===\s*([A-Za-z_][A-Za-z0-9_-]*)?\s*$/;
// Attribute name is simple identifier
const ATTR_NAME_RE = /^[A-Za-z_][A-Za-z0-9_-]*$/;

// Names owned by the built-in container plugin
const RESERVED = new Set(Object.keys(DEFAULT_TITLES));

const isNativeTag = (name: string) => {
  const lower = name.toLowerCase();

  return isHTMLTag(lower) || isSVGTag(lower) || isMathMLTag(lower);
};

export function componentContainerPlugin (md: MarkdownItAsync): void {
  // Parses `{prop1="" prop2}` on container info lines into `token.attrs`, scoped to `blockInfo`
  md.use(attrs, {
    rule: ['blockInfo'],
  });

  md.use(container, {
    name: NAME,

    validate: (params) => {
      const match = VALIDATE_RE.exec(params.trim());

      return (
        !!match
         && !RESERVED.has(match[1])
         && !isNativeTag(match[1])
      );
    },

    openRender: (tokens, index) => {
      const token = tokens[index];
      // Normally attrs has stripped `{...}`
      // However, if it could not parse the block (empty braces, unterminated quote), the braces survive, so fall back
      const match =
        HEADER_RE.exec(token.info.trim()) ?? VALIDATE_RE.exec(token.info.trim());

      if (!match) return '<div>\n';

      const name = match[1];
      const attributeList = (token.attrs ?? [])
        .filter(([key]) => ATTR_NAME_RE.test(key))
        .map(([
          key,
          value,
        ]) =>
          value === ''
            ? key
            : `${key}="${md.utils.escapeHtml(value)}"`)
        .join(' ');

      token.meta = {
        ...token.meta,
        name,
      };

      return `<${name}${attributeList ? ` ${attributeList}` : ''}>\n`;
    },

    closeRender: (tokens, index) => `</${findOpenName(tokens, index)}>\n`,
  });

  // `=== slot-name` becomes its own block token
  md.block.ruler.before(
    'paragraph',
    'component_slot_marker',
    (state: StateBlock, startLine, _endLine, silent) => {
      if (4 <= state.sCount[startLine] - state.blkIndent) return false;

      const text = state.src
        .slice(
          state.bMarks[startLine] + state.tShift[startLine],
          state.eMarks[startLine],
        )
        .trim();

      const match = SLOT_RE.exec(text);

      if (!match) return false;
      if (silent) return true;

      const token = state.push('component_slot_marker', '', 0);

      token.meta = {
        name: match[1] ?? null,
        raw: text,
      };
      token.map = [
        startLine,
        startLine + 1,
      ];
      state.line = startLine + 1;

      return true;
    },
    {
      alt: [
        'paragraph',
        'blockquote',
        'list',
      ],
    },
  );

  // markers -> <template> pairs
  md.core.ruler.after('block', 'component_slots', (state: StateCore) => {
    const out: Token[] = [];
    // One frame per open container, holding the level of its currently open named slot (or null)
    const stack: {
      slotLevel: number | null;
      contentLevel: number;
    }[] = [];

    let offset = 0; // `offset` is how many <template> wrappers we have inserted above the current position

    const slotOpen = (name: string, level: number) => {
      const token = new state.Token('component_slot_open', 'template', 1);

      token.block = true;
      token.level = level;
      token.meta = {
        name,
      };

      return token;
    };
    const slotClose = (level: number) => {
      const token = new state.Token('component_slot_close', 'template', -1);

      token.block = true;
      token.level = level;

      return token;
    };

    for (const token of state.tokens) {
      switch (token.type) {
        case OPEN:
          stack.push({
            slotLevel: null,
            // Level a direct child of this container will have, recorded
            // before `offset` is applied
            contentLevel: token.level + 1,
          });
          token.level += offset;
          out.push(token);
          break;

        case CLOSE: {
          const frame = stack.pop();

          if (frame && frame.slotLevel !== null) {
            out.push(slotClose(frame.slotLevel));
            offset--;
          }
          token.level += offset;
          out.push(token);
          break;
        }

        case 'component_slot_marker': {
          const name = token.meta?.name;
          const frame = stack[stack.length - 1];

          // A separator only counts as one when it is a DIRECT child of the component
          if (!frame || !name || token.level !== frame.contentLevel) {
            token.level += offset;
            out.push(token);
            break;
          }

          // Close the previous slot before opening the next
          if (frame.slotLevel !== null) {
            out.push(slotClose(frame.slotLevel));
            offset--;
          }

          const level = token.level + offset;

          out.push(slotOpen(name, level));
          frame.slotLevel = level;
          offset++;
          break;
        }

        default:
          token.level += offset;
          out.push(token);
      }
    }

    state.tokens = out;
  });

  // renderers
  md.renderer.rules.component_slot_open = (tokens, index) =>
    `<template #${tokens[index].meta?.name ?? 'default'}>\n`;
  md.renderer.rules.component_slot_close = () => '</template>\n';
  // Only reached for markers that were not consumed as separators
  md.renderer.rules.component_slot_marker = (tokens, index) =>
    `<p>${md.utils.escapeHtml(tokens[index].meta?.raw ?? '')}</p>\n`;
}

/** The close token carries no `info`; recover the tag from its matching open */
function findOpenName (tokens: Token[], index: number): string {
  let depth = 0;

  for (let tokenIndex = index - 1; 0 <= tokenIndex; tokenIndex--) {
    if (tokens[tokenIndex].type === CLOSE) depth++;
    else if (tokens[tokenIndex].type === OPEN) {
      if (depth === 0) return tokens[tokenIndex].meta?.name ?? 'div';
      depth--;
    }
  }

  return 'div';
}
