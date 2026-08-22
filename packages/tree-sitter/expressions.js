/**
 * Shared expression rules for typedown sub-grammars
 */

/* eslint-disable id-length */

/// <reference types="tree-sitter-cli/dsl" />
// @ts-check

const PREC = {
  OR: 1,
  AND: 2,
  EQUALITY: 3,
  COMPARISON: 4,
  ADD: 5,
  MULTIPLY: 6,
  EXPONENT: 7,
  UNARY: 8,
  POSTFIX: 9,
  ACCESS: 10,
};

/** @type {Record<string, ($: GrammarSymbols<string>) => RuleOrLiteral>} */
export const expr_rules = {
  expression: ($) =>
    choice(
      $.binary_expression,
      $.unary_expression,
      $.postfix_expression,
      $.access_expression,
      $.index_expression,
      $.call_expression,
      $.string,
      $.number,
      $.boolean,
      $.identifier,
      $.self_expression,
      $.fref,
      $.tag_expression,
      $.list_expression,
      $.dict_expression,
      $.parenthesized_expression,
    ),

  // Binary operators

  binary_expression: ($) =>
    choice(
      prec.left(
        PREC.OR,
        seq(
          $.expression,
          '||',
          $.expression,
        ),
      ),
      prec.left(
        PREC.AND,
        seq(
          $.expression,
          '&&',
          $.expression,
        ),
      ),
      prec.left(
        PREC.EQUALITY,
        seq(
          $.expression,
          choice('==', '!='),
          $.expression,
        ),
      ),
      prec.left(
        PREC.COMPARISON,
        seq(
          $.expression,
          choice('<', '>', '<=', '>='),
          $.expression,
        ),
      ),
      prec.left(
        PREC.ADD,
        seq(
          $.expression,
          choice('+', '-'),
          $.expression,
        ),
      ),
      prec.left(
        PREC.MULTIPLY,
        seq(
          $.expression,
          choice('*', '/', '%'),
          $.expression,
        ),
      ),
      prec.right(
        PREC.EXPONENT,
        seq(
          $.expression,
          '**',
          $.expression,
        ),
      ),
    ),

  // Unary operators

  unary_expression: ($) =>
    prec(
      PREC.UNARY,
      seq(
        '~',
        $.expression,
      ),
    ),

  // Postfix operators: ?

  postfix_expression: ($) =>
    prec(
      PREC.POSTFIX,
      seq(
        $.expression,
        '?',
      ),
    ),

  // Property access: self.foo, self.author.name

  access_expression: ($) =>
    prec.left(
      PREC.ACCESS,
      seq(
        $.expression,
        '.',
        $.identifier,
      ),
    ),

  // Index access: foo[0], foo["key"]

  index_expression: ($) =>
    prec.left(
      PREC.ACCESS,
      seq(
        $.expression,
        '[',
        $.expression,
        ']',
      ),
    ),

  // Function call: self.items.length()

  call_expression: ($) =>
    prec.left(
      PREC.ACCESS,
      seq(
        $.expression,
        '(',
        optional($.argument_list),
        ')',
      ),
    ),

  argument_list: ($) =>
    seq(
      $.expression,
      repeat(
        seq(
          ',',
          $.expression,
        ),
      ),
    ),

  // self keyword

  self_expression: () => 'self',

  // File reference: fref("filename.td")

  fref: ($) =>
    seq(
      'fref',
      '(',
      $.string,
      ')',
    ),

  // Tag expression: !string "hello", !type list[string]

  tag_expression: ($) =>
    seq(
      $.tag_operator,
      $.expression,
    ),

  tag_operator: () =>
    token(
      seq(
        '!',
        token.immediate(/[a-zA-Z_]\w*/),
      ),
    ),

  // Parenthesized expression

  parenthesized_expression: ($) =>
    seq(
      '(',
      $.expression,
      ')',
    ),

  // Inline list: [1, 2, 3]

  list_expression: ($) =>
    seq(
      '[',
      optional(
        seq(
          $.expression,
          repeat(
            seq(
              ',',
              $.expression,
            ),
          ),
          optional(','),
        ),
      ),
      ']',
    ),

  // Inline dict: { key: value, key2: value2 }

  dict_expression: ($) =>
    seq(
      '{',
      optional(
        seq(
          $.dict_entry,
          repeat(
            seq(
              ',',
              $.dict_entry,
            ),
          ),
          optional(','),
        ),
      ),
      '}',
    ),

  dict_entry: ($) =>
    seq(
      field('key', $.identifier),
      ':',
      field('value', $.expression),
    ),

  // Literals

  string: ($) =>
    choice(
      seq(
        '"',
        repeat(
          choice(
            $.escape_sequence,
            $.interpolation,
            /[^"\\$]+/,
          ),
        ),
        '"',
      ),
      seq(
        '\'',
        repeat(
          choice(
            $.escape_sequence,
            /[^'\\$]+/,
          ),
        ),
        '\'',
      ),
    ),

  interpolation: ($) =>
    seq(
      '$',
      token.immediate('{'),
      $.expression,
      '}',
    ),

  escape_sequence: () =>
    token(
      choice(
        '\\\\',
        '\\"',
        '\\\'',
        '\\n',
        '\\t',
        '\\r',
        '\\b',
        '\\f',
        '\\v',
        '\\/',
        '\\$',
        /\\u[0-9a-fA-F]{4}/,
        /\\x[0-9a-fA-F]{2}/,
        /\\[0-7]{1,3}/,
      ),
    ),

  number: () =>
    /\d+(\.\d+)?/,

  boolean: () =>
    choice(
      'true',
      'false',
    ),

  identifier: () =>
    /[a-zA-Z_]\w*/,
};
