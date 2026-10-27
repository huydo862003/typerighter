/**
 * @file Typedown Markdown inline grammar for tree-sitter
 * @author Huy-DNA <huydo862003@gmail.com>
 * @license AGPL
 *
 * Inline-level grammar for Typedown Markdown
 * Re-parses opaque `inline` nodes from the block grammar
 */

/* eslint-disable id-length */
/// <reference types="tree-sitter-cli/dsl" />
// @ts-check

import {
  expr_rules,
} from '../expressions.js';

export default grammar({
  name: 'typedown_md_inline',

  externals: ($) => [
    $._emphasis_open_star,
    $._emphasis_close_star,
    $._emphasis_open_underscore,
    $._emphasis_close_underscore,

    $._strong_emphasis_open_star,
    $._strong_emphasis_close_star,
    $._strong_emphasis_open_underscore,
    $._strong_emphasis_close_underscore,

    $._strikethrough_open,
    $._strikethrough_close,

    $.code_span_delimiter,
    $.code_span_content,

    $.math_span_delimiter,
    $.math_span_content,

    $._text_content,
  ],

  extras: () => [],

  conflicts: () => [],

  rules: {
    inline: ($) =>
      repeat1($._inline_element),

    _inline_element: ($) =>
      choice(
        $.strong_emphasis,
        $.emphasis,
        $.strikethrough,
        $.code_span,
        $.math_span,
        $.inline_link,
        $.image,
        $.footnote_reference,
        $.citation,
        $.interpolation,
        $.hard_line_break,
        $.backslash_escape,
        $._text_content,
      ),

    // *text* or _text_
    emphasis: ($) =>
      choice(
        seq(
          $._emphasis_open_star,
          repeat1($._inline_element),
          $._emphasis_close_star,
        ),
        seq(
          $._emphasis_open_underscore,
          repeat1($._inline_element),
          $._emphasis_close_underscore,
        ),
      ),

    // **text** or __text__
    strong_emphasis: ($) =>
      choice(
        seq(
          $._strong_emphasis_open_star,
          repeat1($._inline_element),
          $._strong_emphasis_close_star,
        ),
        seq(
          $._strong_emphasis_open_underscore,
          repeat1($._inline_element),
          $._strong_emphasis_close_underscore,
        ),
      ),

    // ~~text~~
    strikethrough: ($) =>
      seq(
        $._strikethrough_open,
        repeat1($._inline_element),
        $._strikethrough_close,
      ),

    code_span: ($) =>
      seq(
        $.code_span_delimiter,
        optional($.code_span_content),
        $.code_span_delimiter,
      ),

    math_span: ($) =>
      seq(
        $.math_span_delimiter,
        optional($.math_span_content),
        $.math_span_delimiter,
      ),

    inline_link: ($) =>
      seq(
        '[',
        field('text', $.link_text),
        ']',
        '(',
        field('destination', $.link_destination),
        ')',
      ),

    link_text: ($) =>
      repeat1(
        choice(
          $._text_content,
          $.strong_emphasis,
          $.emphasis,
          $.strikethrough,
          $.code_span,
          $.math_span,
        ),
      ),

    // Balanced parens in URLs like https://example.com/wiki/Foo_(bar)
    link_destination: ($) =>
      repeat1(
        choice(
          /[^\r\n()]+/,
          seq('(', optional($.link_destination), ')'),
        ),
      ),

    image: ($) =>
      seq(
        '!',
        '[',
        field('alt', $.image_alt),
        ']',
        '(',
        field('source', $.link_destination),
        ')',
      ),

    image_alt: ($) =>
      repeat1($._text_content),

    footnote_reference: ($) =>
      seq(
        '[',
        '^',
        field('label', $.footnote_label),
        ']',
      ),

    footnote_label: () =>
      /[a-zA-Z_]\w*/,

    citation: ($) =>
      seq(
        '[',
        '@',
        field('key', $.citation_key),
        ']',
      ),

    citation_key: () =>
      /[a-zA-Z_]\w*/,

    interpolation: ($) =>
      seq(
        '$',
        token.immediate('{'),
        $.expression,
        '}',
      ),

    hard_line_break: () =>
      token(
        choice(
          /\\\r?\n/,
          /  +\r?\n/,
        ),
      ),

    backslash_escape: () =>
      token(/\\[\\`*_\[\](){}#+\-.!|$@^~>:]/),

    // Full Typedown expression grammar for ${...} interpolation
    ...expr_rules,
  },
});
