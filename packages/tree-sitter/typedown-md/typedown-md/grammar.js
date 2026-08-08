/**
 * @file Typedown Markdown block grammar for tree-sitter
 * @author Huy-DNA <huydo862003@gmail.com>
 * @license AGPL
 *
 * Block-level grammar for Typedown Markdown
 * Inline content is opaque nodes re-parsed by tdr_md_inline
 * Follows tree-sitter-markdown's scanner design
 * All top-level blocks are wrapped in an implicit section
 */

/* eslint-disable id-length */
/// <reference types="tree-sitter-cli/dsl" />
// @ts-check

export default grammar({
  name: 'tdr_md',

  externals: ($) => [
    $._line_ending,
    $._soft_line_ending,
    $._block_close,
    $._block_continuation,

    $.atx_h1_marker,
    $.atx_h2_marker,
    $.atx_h3_marker,
    $.atx_h4_marker,
    $.atx_h5_marker,
    $.atx_h6_marker,

    $.fenced_code_block_delimiter,
    $.code_fence_content,
    $.language,

    $.math_block_delimiter,
    $.math_block_content,

    $.block_quote_marker,
    $.toggle_list_marker,

    $.list_marker_minus,
    $.list_marker_star,
    $.list_marker_dot,

    $._container_open,
    $._container_close,

    $._pipe_table_start,
    $._pipe_table_line_ending,
    $.pipe_table_delimiter_row,

    $._blank_line,
  ],

  extras: () => [],

  precedences: ($) => [
    [
      $._section1,
      $._block,
    ],
    [
      $._section2,
      $._block,
    ],
    [
      $._section3,
      $._block,
    ],
    [
      $._section4,
      $._block,
    ],
    [
      $._section5,
      $._block,
    ],
    [
      $._section6,
      $._block,
    ],
  ],

  supertypes: ($) => [$._block],

  rules: {
    document: ($) =>
      seq(
        repeat($._blank_line),
        optional(
          alias(
            $._implicit_section,
            $.section,
          ),
        ),
        repeat(
          seq(
            $.section,
            repeat($._blank_line),
          ),
        ),
      ),

    _implicit_section: ($) =>
      prec.right(
        seq(
          $._block,
          repeat(
            choice(
              $._block,
              $._blank_line,
            ),
          ),
        ),
      ),

    section: ($) =>
      choice(
        $._section1,
        $._section2,
        $._section3,
        $._section4,
        $._section5,
        $._section6,
      ),

    _section1: ($) => sectionRule($, 1),
    _section2: ($) => sectionRule($, 2),
    _section3: ($) => sectionRule($, 3),
    _section4: ($) => sectionRule($, 4),
    _section5: ($) => sectionRule($, 5),
    _section6: ($) => sectionRule($, 6),

    _atx_heading1: ($) => headingRule($, 1),
    _atx_heading2: ($) => headingRule($, 2),
    _atx_heading3: ($) => headingRule($, 3),
    _atx_heading4: ($) => headingRule($, 4),
    _atx_heading5: ($) => headingRule($, 5),
    _atx_heading6: ($) => headingRule($, 6),

    _block: ($) =>
      choice(
        $.paragraph,
        $.fenced_code_block,
        $.math_block,
        $.block_quote,
        $.toggle_list,
        $.list,
        $.container_block,
        $.pipe_table,
      ),

    // Paragraph

    paragraph: ($) =>
      seq(
        $.inline,
        $._line_ending,
      ),

    inline: ($) =>
      $._line,

    _line: ($) =>
      prec.left(
        repeat1(
          choice(
            $._word,
            $._whitespace,
            $._punctuation,
            $._soft_line_ending,
          ),
        ),
      ),

    _word: () => /[^\r\n \t!-\/:-@\[-`{-~]+/,

    _whitespace: () => /[ \t]+/,

    _punctuation: () => /[!-\/:-@\[-`{-~]/,

    // Fenced code block

    fenced_code_block: ($) =>
      seq(
        $.fenced_code_block_delimiter,
        optional(field('language', $.language)),
        optional($.code_fence_content),
        $.fenced_code_block_delimiter,
      ),

    // Math block

    math_block: ($) =>
      seq(
        $.math_block_delimiter,
        optional($.math_block_content),
        $.math_block_delimiter,
      ),

    // Block quote

    block_quote: ($) =>
      prec.right(
        seq(
          $.block_quote_marker,
          repeat(
            choice(
              $._block,
              $._blank_line,
              $._block_continuation,
            ),
          ),
          $._block_close,
        ),
      ),

    // Toggle list

    toggle_list: ($) =>
      prec.right(
        repeat1($.toggle_list_item),
      ),

    toggle_list_item: ($) =>
      prec.right(
        seq(
          $.toggle_list_marker,
          repeat(
            choice(
              $._block,
              $._blank_line,
              $._block_continuation,
            ),
          ),
          $._block_close,
        ),
      ),

    // Lists

    list: ($) =>
      choice(
        $._list_minus,
        $._list_star,
        $._list_dot,
      ),

    _list_minus: ($) =>
      prec.right(repeat1(alias($._list_item_minus, $.list_item))),

    _list_star: ($) =>
      prec.right(repeat1(alias($._list_item_star, $.list_item))),

    _list_dot: ($) =>
      prec.right(repeat1(alias($._list_item_dot, $.list_item))),

    _list_item_minus: ($) => listItemBody($, $.list_marker_minus),
    _list_item_star: ($) => listItemBody($, $.list_marker_star),
    _list_item_dot: ($) => listItemBody($, $.list_marker_dot),

    // Pipe table

    pipe_table: ($) =>
      prec.right(
        seq(
          $._pipe_table_start,
          alias($.pipe_table_row, $.pipe_table_header),
          $._line_ending,
          $.pipe_table_delimiter_row,
          repeat(
            seq(
              $._pipe_table_line_ending,
              optional($.pipe_table_row),
            ),
          ),
          $._line_ending,
        ),
      ),

    pipe_table_row: ($) =>
      seq(
        optional('|'),
        repeat1(
          prec.right(
            seq(
              $.pipe_table_cell,
              '|',
            ),
          ),
        ),
        optional($.pipe_table_cell),
      ),

    pipe_table_cell: () =>
      /[^\r\n|]+/,

    // Callout block

    container_block: ($) =>
      seq(
        alias($._container_open, $.container_block_delimiter),
        optional(field('type', $.container_type)),
        $._line_ending,
        repeat(
          choice(
            $._block,
            $._blank_line,
          ),
        ),
        alias($._container_close, $.container_block_delimiter),
        optional($._line_ending),
      ),

    container_type: () =>
      /[a-zA-Z_]\w*/,
  },
});

/**
 * @param {GrammarSymbols<string>} $
 * @param {number} level - Heading level 1-6
 * @returns {SeqRule}
 */
function headingRule ($, level) {
  return seq(
    $[`atx_h${level}_marker`],
    optional(field('heading_content', $.inline)),
    $._line_ending,
  );
}

/**
 * @param {GrammarSymbols<string>} $
 * @param {RuleOrLiteral} marker - The list marker rule
 * @returns {SeqRule}
 */
function listItemBody ($, marker) {
  return seq(
    marker,
    repeat(
      choice(
        $._block,
        $._blank_line,
        $._block_continuation,
      ),
    ),
    $._block_close,
  );
}

/**
 * @param {GrammarSymbols<string>} $
 * @param {number} level - Section level 1-6
 * @returns {PrecRightRule}
 */
function sectionRule ($, level) {
  const subsections = [];

  for (let sub = level + 1; sub <= 6; sub++) {
    subsections.push(alias($[`_section${sub}`], $.section));
  }

  return prec.right(
    seq(
      alias($[`_atx_heading${level}`], $.atx_heading),
      repeat(
        choice(
          ...subsections,
          $._block,
          $._blank_line,
        ),
      ),
    ),
  );
}
