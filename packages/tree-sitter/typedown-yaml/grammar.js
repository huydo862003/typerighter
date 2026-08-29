/**
 * @file Typedown YAML grammar for tree-sitter
 * @author Huy-DNA <huydo862003@gmail.com>
 * @license AGPL
 */

/* eslint-disable id-length */

/// <reference types="tree-sitter-cli/dsl" />
// @ts-check

import {
  expr_rules,
} from '../expressions.js';

export default grammar({
  name: 'typedown_yaml',

  externals: ($) => [
    $._newline,
    $._indent_mapping,
    $._indent_sequence,
    $._block_end,
    $._seq_item_start,
    $.block_scalar_content,
  ],

  extras: ($) => [
    / /,
    /\t/,
    $.comment,
  ],

  word: ($) => $.identifier,

  rules: {
    // top-level has no _block_end, closed by EOF
    source_file: ($) =>
      seq(
        optional($._newline),
        optional(
          seq(
            $.block_mapping_entry,
            repeat(seq($._newline, $.block_mapping_entry)),
          ),
        ),
        optional($._newline),
      ),

    block_mapping_entry: ($) =>
      seq(
        field('key', $.property_key),
        ':',
        field('value', $.value),
      ),

    property_key: ($) =>
      choice(
        $.identifier,
        $.reserved_key,
      ),

    reserved_key: () =>
      choice(
        '_type',
        '_label',
      ),

    value: ($) =>
      choice(
        $.type_value,
        $.block_scalar,
        $._block_value,
        $.expression,
      ),

    type_value: ($) =>
      seq(
        '!type',
        $.type_expression,
      ),

    _block_value: ($) =>
      choice(
        $.block_mapping_value,
        $.block_sequence_value,
      ),

    // nested mapping, closed by _block_end
    block_mapping_value: ($) =>
      seq(
        $._indent_mapping,
        $.block_mapping_entry,
        repeat(seq($._newline, $.block_mapping_entry)),
        $._block_end,
      ),

    // nested sequence, closed by _block_end
    block_sequence_value: ($) =>
      seq(
        $._indent_sequence,
        $.block_sequence_entry,
        repeat(seq($._newline, $.block_sequence_entry)),
        $._block_end,
      ),

    block_sequence_entry: ($) =>
      seq(
        $._seq_item_start,
        choice(
          $.value,
          prec.right(seq(
            $.block_mapping_entry,
            repeat(seq($._newline, $.block_mapping_entry)),
          )),
        ),
        $._block_end,
      ),

    // | and > are internal tokens so they don't conflict with || and > operators
    // The operators are infix (require left operand in binary_expression)
    // Block scalar indicators only appear at the start of a value
    block_scalar: ($) =>
      seq(
        field('indicator', choice(
          token(seq('|', optional(choice('-', '+')))),
          token(seq('>', optional(choice('-', '+')))),
        )),
        optional($.block_scalar_content),
      ),

    // Type expressions
    type_expression: ($) =>
      choice(
        $.optional_type,
        $.primitive_type,
        $.list_type,
        $.dict_type,
        $.fixed_key_dict_type,
        $.union_type,
      ),

    optional_type: ($) =>
      prec(
        1,
        seq(
          choice(
            $.primitive_type,
            $.list_type,
            $.dict_type,
            $.fixed_key_dict_type,
            $.union_type,
          ),
          '?',
        ),
      ),

    primitive_type: () =>
      choice(
        'string',
        'number',
        'boolean',
        'date',
        'time',
        'datetime',
      ),

    list_type: ($) =>
      seq(
        'list',
        '[',
        $.type_expression,
        ']',
      ),

    dict_type: ($) =>
      seq(
        'dict',
        '[',
        $.type_expression,
        ',',
        $.type_expression,
        ']',
      ),

    fixed_key_dict_type: ($) =>
      seq(
        '{',
        $.fixed_key_entry,
        repeat(
          seq(
            ',',
            $.fixed_key_entry,
          ),
        ),
        '}',
      ),

    fixed_key_entry: ($) =>
      seq(
        field('key', $.identifier),
        ':',
        field('type', $.type_expression),
      ),

    union_type: ($) =>
      seq(
        '[',
        $.union_member,
        repeat(
          seq(
            ',',
            $.union_member,
          ),
        ),
        ']',
      ),

    union_member: ($) =>
      choice(
        $.type_expression,
        $.string,
        $.number,
      ),

    comment: () =>
      token(
        seq(
          '#',
          /.*/,
        ),
      ),

    ...expr_rules,
  },
});
