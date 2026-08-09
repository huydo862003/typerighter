/**
 * External scanner for typedown_md block grammar
 */

#include "tree_sitter/parser.h"

#include <string.h>
#include <stdlib.h>
#include <stdbool.h>

enum TokenType {
  LINE_ENDING,
  SOFT_LINE_ENDING,
  BLOCK_CLOSE,
  BLOCK_CONTINUATION,

  ATX_H1_MARKER,
  ATX_H2_MARKER,
  ATX_H3_MARKER,
  ATX_H4_MARKER,
  ATX_H5_MARKER,
  ATX_H6_MARKER,

  FENCED_CODE_BLOCK_DELIMITER,
  CODE_FENCE_CONTENT,
  LANGUAGE,
  LINE_RANGE_INDICATOR,

  MATH_BLOCK_DELIMITER,
  MATH_BLOCK_CONTENT,

  BLOCK_QUOTE_MARKER,
  TOGGLE_LIST_MARKER,

  LIST_MARKER_MINUS,
  LIST_MARKER_STAR,
  LIST_MARKER_DOT,

  CONTAINER_OPEN,
  CONTAINER_CLOSE,
  CONTAINER_SLOT_SEPARATOR,

  PIPE_TABLE_START,
  PIPE_TABLE_LINE_ENDING,
  PIPE_TABLE_DELIMITER_ROW,

  BLANK_LINE,
};

enum BlockType {
  BLOCK_QUOTE_BLOCK = 1,
  LIST_ITEM = 2,
  LIST_ITEM_MAX_INDENTATION = 18,
  TOGGLE_LIST_BLOCK = 19,
};

#define MAX_BLOCK_DEPTH 32
#define STATE_MATCHING 0x1

typedef struct {
  uint8_t open_blocks[MAX_BLOCK_DEPTH];
  uint8_t num_open_blocks;

  uint8_t code_fence_count;
  char code_fence_char;
  bool code_fence_info_pending;
  uint8_t math_fence_count;

  uint8_t matched;
  uint8_t state;
  uint16_t indentation;
} Scanner;

void *tree_sitter_typedown_md_external_scanner_create(void) {
  return calloc(1, sizeof(Scanner));
}

void tree_sitter_typedown_md_external_scanner_destroy(void *payload) {
  free(payload);
}

unsigned tree_sitter_typedown_md_external_scanner_serialize(void *payload,
                                                            char *buffer) {
  Scanner *scanner = (Scanner *)payload;
  unsigned pos = 0;
  buffer[pos++] = (char)scanner->num_open_blocks;
  for (int idx = 0; idx < scanner->num_open_blocks; idx++) {
    buffer[pos++] = (char)scanner->open_blocks[idx];
  }
  buffer[pos++] = (char)scanner->code_fence_count;
  buffer[pos++] = scanner->code_fence_char;
  buffer[pos++] = (char)scanner->code_fence_info_pending;
  buffer[pos++] = (char)scanner->math_fence_count;
  buffer[pos++] = (char)scanner->matched;
  buffer[pos++] = (char)scanner->state;
  memcpy(buffer + pos, &scanner->indentation, sizeof(uint16_t));
  pos += sizeof(uint16_t);
  return pos;
}

void tree_sitter_typedown_md_external_scanner_deserialize(void *payload,
                                                          const char *buffer,
                                                          unsigned length) {
  Scanner *scanner = (Scanner *)payload;
  memset(scanner, 0, sizeof(Scanner));
  if (length == 0) return;

  unsigned pos = 0;
  scanner->num_open_blocks = (uint8_t)buffer[pos++];
  if (scanner->num_open_blocks > MAX_BLOCK_DEPTH) {
    scanner->num_open_blocks = 0;
    return;
  }
  for (int idx = 0; idx < scanner->num_open_blocks && pos < length; idx++) {
    scanner->open_blocks[idx] = (uint8_t)buffer[pos++];
  }
  if (pos < length) scanner->code_fence_count = (uint8_t)buffer[pos++];
  if (pos < length) scanner->code_fence_char = buffer[pos++];
  if (pos < length) scanner->code_fence_info_pending = (bool)buffer[pos++];
  if (pos < length) scanner->math_fence_count = (uint8_t)buffer[pos++];
  if (pos < length) scanner->matched = (uint8_t)buffer[pos++];
  if (pos < length) scanner->state = (uint8_t)buffer[pos++];
  if (pos + sizeof(uint16_t) <= length) {
    memcpy(&scanner->indentation, buffer + pos, sizeof(uint16_t));
    pos += sizeof(uint16_t);
  }
}

static bool is_newline(int32_t character) {
  return character == '\n' || character == '\r';
}

static void eat_newline(TSLexer *lexer) {
  if (lexer->lookahead == '\r') lexer->advance(lexer, false);
  if (lexer->lookahead == '\n') lexer->advance(lexer, false);
}

static void skip_spaces(TSLexer *lexer) {
  while (lexer->lookahead == ' ' || lexer->lookahead == '\t') {
    lexer->advance(lexer, true);
  }
}

static void advance_past_spaces(TSLexer *lexer) {
  while (lexer->lookahead == ' ' || lexer->lookahead == '\t') {
    lexer->advance(lexer, false);
  }
}

static void push_block(Scanner *scanner, uint8_t block_type) {
  if (scanner->num_open_blocks < MAX_BLOCK_DEPTH) {
    scanner->open_blocks[scanner->num_open_blocks++] = block_type;
  }
}

static void pop_block(Scanner *scanner) {
  if (scanner->num_open_blocks > 0) {
    scanner->num_open_blocks--;
  }
}

// Try to continue an open block on the current line
// Returns true if the block matches, false to close it
static bool match_block(Scanner *scanner, TSLexer *lexer,
                        uint8_t block_type) {
  if (block_type == BLOCK_QUOTE_BLOCK) {
    skip_spaces(lexer);
    if (lexer->lookahead == '>') {
      lexer->advance(lexer, false);
      if (lexer->lookahead == ' ') lexer->advance(lexer, false);
      scanner->indentation = 0;
      return true;
    }
    return false;
  }
  if (block_type == TOGGLE_LIST_BLOCK) {
    skip_spaces(lexer);
    if (lexer->lookahead == '>') {
      lexer->mark_end(lexer);
      lexer->advance(lexer, false);
      // >- starts a new toggle item; close this block
      if (lexer->lookahead == '-') return false;
      if (lexer->lookahead == ' ') lexer->advance(lexer, false);
      scanner->indentation = 0;
      return true;
    }
    return false;
  }
  if (block_type >= LIST_ITEM && block_type <= LIST_ITEM_MAX_INDENTATION) {
    uint16_t required = (uint16_t)(block_type - LIST_ITEM);
    if (scanner->indentation >= required) {
      scanner->indentation -= required;
      return true;
    }
    if (is_newline(lexer->lookahead) || lexer->eof(lexer)) {
      scanner->indentation = 0;
      return true;
    }
    return false;
  }
  return true;
}

// Consume content lines until closing fence or EOF
static bool scan_fenced_content(TSLexer *lexer, char fence_char,
                                uint8_t fence_count, int result_symbol) {
  bool has_content = false;
  while (!lexer->eof(lexer)) {
    if (is_newline(lexer->lookahead)) {
      eat_newline(lexer);
      lexer->mark_end(lexer);
      has_content = true;
      advance_past_spaces(lexer);
      if (lexer->lookahead == fence_char) {
        uint8_t count = 0;
        while (lexer->lookahead == fence_char) {
          count++;
          lexer->advance(lexer, false);
        }
        if (count >= fence_count &&
            (is_newline(lexer->lookahead) || lexer->eof(lexer))) {
          lexer->result_symbol = result_symbol;
          return true;
        }
      }
      continue;
    }
    lexer->advance(lexer, false);
    has_content = true;
  }
  if (has_content) {
    lexer->mark_end(lexer);
    lexer->result_symbol = result_symbol;
    return true;
  }
  return false;
}

// Check if current position has a closing fence
static bool try_closing_fence(TSLexer *lexer, char fence_char,
                              uint8_t fence_count) {
  advance_past_spaces(lexer);
  if (lexer->lookahead == fence_char) {
    uint8_t count = 0;
    while (lexer->lookahead == fence_char) {
      count++;
      lexer->advance(lexer, false);
    }
    if (count >= fence_count &&
        (is_newline(lexer->lookahead) || lexer->eof(lexer))) {
      return true;
    }
  }
  return false;
}

// Check if next line would interrupt a paragraph
// Sets has_pipe if a pipe is found on the line
static bool next_line_starts_block(TSLexer *lexer, bool exclude_pipe,
                                     bool *has_pipe) {
  skip_spaces(lexer);

  if (lexer->eof(lexer) || is_newline(lexer->lookahead)) return true;

  int32_t next = lexer->lookahead;

  if (next == '#' || next == '>' || next == '`' || next == '~' ||
      next == '$') {
    return true;
  }

  // Single or double colon is not a callout
  if (next == ':') {
    lexer->advance(lexer, false);
    if (lexer->lookahead == ':') {
      lexer->advance(lexer, false);
      if (lexer->lookahead == ':') return true;
    }
    return false;
  }

  // Triple equals opens a container slot, but only where one is valid --
  // elsewhere `===` is ordinary text, since there are no setext headings.
  if (next == '=') {
    lexer->advance(lexer, false);
    if (lexer->lookahead == '=') {
      lexer->advance(lexer, false);
      if (lexer->lookahead == '=') return true;
    }
    return false;
  }

  if (next == '|') {
    if (has_pipe) *has_pipe = true;
    if (!exclude_pipe) return true;
  }

  if (next == '-' || next == '*') {
    lexer->advance(lexer, false);
    return lexer->lookahead == ' ' || lexer->lookahead == '\t';
  }

  if (next >= '0' && next <= '9') {
    while (lexer->lookahead >= '0' && lexer->lookahead <= '9') {
      lexer->advance(lexer, false);
    }
    if (lexer->lookahead == '.') {
      lexer->advance(lexer, false);
      return lexer->lookahead == ' ' || lexer->lookahead == '\t';
    }
    return false;
  }

  return false;
}

// Count cells in a delimiter row, return 0 if invalid
static uint8_t validate_delimiter_row(TSLexer *lexer) {
  while (lexer->lookahead == ' ' || lexer->lookahead == '\t') {
    lexer->advance(lexer, false);
  }
  if (lexer->lookahead == '|') {
    lexer->advance(lexer, false);
  }

  uint8_t cell_count = 0;
  while (!is_newline(lexer->lookahead) && !lexer->eof(lexer)) {
    while (lexer->lookahead == ' ' || lexer->lookahead == '\t') {
      lexer->advance(lexer, false);
    }
    if (is_newline(lexer->lookahead) || lexer->eof(lexer)) break;

    if (lexer->lookahead == ':') {
      lexer->advance(lexer, false);
    }
    bool had_dash = false;
    while (lexer->lookahead == '-') {
      had_dash = true;
      lexer->advance(lexer, false);
    }
    if (!had_dash) return 0;
    if (lexer->lookahead == ':') {
      lexer->advance(lexer, false);
    }
    cell_count++;

    while (lexer->lookahead == ' ' || lexer->lookahead == '\t') {
      lexer->advance(lexer, false);
    }

    if (lexer->lookahead == '|') {
      lexer->advance(lexer, false);
    } else if (!is_newline(lexer->lookahead) && !lexer->eof(lexer)) {
      return 0;
    }
  }
  return cell_count;
}

// Zero-width token, validates header and delimiter rows match
static bool scan_pipe_table_start(Scanner *scanner, TSLexer *lexer) {
  lexer->mark_end(lexer);

  uint8_t cell_count = 0;
  bool starting_pipe = false;
  bool ending_pipe = false;

  if (lexer->lookahead == '|') {
    starting_pipe = true;
    lexer->advance(lexer, false);
  }

  while (!is_newline(lexer->lookahead) && !lexer->eof(lexer)) {
    if (lexer->lookahead == '|') {
      cell_count++;
      ending_pipe = true;
      lexer->advance(lexer, false);
    } else {
      ending_pipe = false;
      if (lexer->lookahead == '\\') {
        lexer->advance(lexer, false);
        if (!lexer->eof(lexer) && !is_newline(lexer->lookahead)) {
          lexer->advance(lexer, false);
        }
      } else {
        lexer->advance(lexer, false);
      }
    }
  }

  if (cell_count == 0 && !(starting_pipe && ending_pipe)) return false;
  if (!ending_pipe) cell_count++;

  if (!is_newline(lexer->lookahead)) return false;
  eat_newline(lexer);

  uint8_t delim_count = validate_delimiter_row(lexer);
  if (delim_count == 0 || delim_count != cell_count) return false;

  lexer->result_symbol = PIPE_TABLE_START;
  return true;
}

// Choose between line ending, soft line ending, or table line ending
// Peeks at next line to decide
static bool scan_line_ending(Scanner *scanner, TSLexer *lexer,
                             const bool *valid_symbols) {
  if (!is_newline(lexer->lookahead) && !lexer->eof(lexer)) return false;
  if (!valid_symbols[LINE_ENDING] && !valid_symbols[SOFT_LINE_ENDING] &&
      !valid_symbols[PIPE_TABLE_LINE_ENDING]) {
    return false;
  }

  if (is_newline(lexer->lookahead)) {
    eat_newline(lexer);
  }
  lexer->mark_end(lexer);

  scanner->state |= STATE_MATCHING;
  scanner->matched = 0;
  scanner->indentation = 0;

  if (valid_symbols[PIPE_TABLE_LINE_ENDING] ||
      valid_symbols[SOFT_LINE_ENDING]) {
    bool has_pipe = false;
    bool starts_block = next_line_starts_block(
        lexer, valid_symbols[PIPE_TABLE_LINE_ENDING], &has_pipe);


    if (valid_symbols[PIPE_TABLE_LINE_ENDING] && !starts_block) {
      if (!has_pipe) {
        while (!lexer->eof(lexer) && !is_newline(lexer->lookahead)) {
          if (lexer->lookahead == '|') {
            has_pipe = true;
            break;
          }
          lexer->advance(lexer, false);
        }
      }
      if (has_pipe) {
        lexer->result_symbol = PIPE_TABLE_LINE_ENDING;
        return true;
      }
    }

    if (valid_symbols[SOFT_LINE_ENDING] && !starts_block) {
      lexer->result_symbol = SOFT_LINE_ENDING;
      return true;
    }
  }

  if (valid_symbols[LINE_ENDING]) {
    lexer->result_symbol = LINE_ENDING;
    return true;
  }

  return false;
}

// Try bullet list marker, mark_end before consuming for safe fallback
static bool try_list_marker_bullet(Scanner *scanner, TSLexer *lexer,
                                   uint16_t indent, int result_symbol) {
  lexer->mark_end(lexer);
  lexer->advance(lexer, false);
  if (lexer->lookahead == ' ' || lexer->lookahead == '\t') {
    lexer->advance(lexer, false);
    uint16_t content_indent = indent + 2;
    uint8_t block_val = LIST_ITEM +
        (content_indent > (LIST_ITEM_MAX_INDENTATION - LIST_ITEM)
         ? (LIST_ITEM_MAX_INDENTATION - LIST_ITEM)
         : (uint8_t)content_indent);
    push_block(scanner, block_val);
    lexer->mark_end(lexer);
    lexer->result_symbol = result_symbol;
    return true;
  }
  return false;
}

bool tree_sitter_typedown_md_external_scanner_scan(void *payload,
                                                   TSLexer *lexer,
                                                   const bool *valid_symbols) {
  Scanner *scanner = (Scanner *)payload;

  if (scanner->code_fence_count > 0) {
    if (scanner->code_fence_info_pending) {
      // Scan language tag, stopping at '{' for range indicator
      if (valid_symbols[LANGUAGE] && !is_newline(lexer->lookahead) &&
          lexer->lookahead != '{' && !lexer->eof(lexer)) {
        while (!is_newline(lexer->lookahead) && lexer->lookahead != '{' &&
               !lexer->eof(lexer)) {
          lexer->advance(lexer, false);
        }
        lexer->mark_end(lexer);
        if (lexer->lookahead != '{') {
          if (is_newline(lexer->lookahead)) eat_newline(lexer);
          scanner->code_fence_info_pending = false;
        }
        lexer->result_symbol = LANGUAGE;
        return true;
      }
      // Scan line range indicator like {1,3,5-8}
      if (valid_symbols[LINE_RANGE_INDICATOR] && lexer->lookahead == '{') {
        lexer->advance(lexer, false);
        while (lexer->lookahead != '}' && !is_newline(lexer->lookahead) &&
               !lexer->eof(lexer)) {
          lexer->advance(lexer, false);
        }
        if (lexer->lookahead == '}') {
          lexer->advance(lexer, false);
        }
        lexer->mark_end(lexer);
        if (is_newline(lexer->lookahead)) eat_newline(lexer);
        scanner->code_fence_info_pending = false;
        lexer->result_symbol = LINE_RANGE_INDICATOR;
        return true;
      }
      if (is_newline(lexer->lookahead)) eat_newline(lexer);
      scanner->code_fence_info_pending = false;
    }

    if (valid_symbols[FENCED_CODE_BLOCK_DELIMITER]) {
      if (try_closing_fence(lexer, scanner->code_fence_char,
                            scanner->code_fence_count)) {
        scanner->code_fence_count = 0;
        scanner->code_fence_char = 0;
        lexer->mark_end(lexer);
        lexer->result_symbol = FENCED_CODE_BLOCK_DELIMITER;
        return true;
      }
    }

    if (valid_symbols[CODE_FENCE_CONTENT]) {
      return scan_fenced_content(lexer, scanner->code_fence_char,
                                 scanner->code_fence_count,
                                 CODE_FENCE_CONTENT);
    }
    return false;
  }

  if (scanner->math_fence_count > 0) {
    if (valid_symbols[MATH_BLOCK_DELIMITER]) {
      if (try_closing_fence(lexer, '$', scanner->math_fence_count)) {
        scanner->math_fence_count = 0;
        lexer->mark_end(lexer);
        lexer->result_symbol = MATH_BLOCK_DELIMITER;
        return true;
      }
    }

    if (valid_symbols[MATH_BLOCK_CONTENT]) {
      return scan_fenced_content(lexer, '$', scanner->math_fence_count,
                                 MATH_BLOCK_CONTENT);
    }
    return false;
  }

  // Matching phase runs before block-start checks
  if (scanner->state & STATE_MATCHING) {
    while (lexer->lookahead == ' ' || lexer->lookahead == '\t') {
      scanner->indentation++;
      lexer->advance(lexer, false);
    }

    if (scanner->matched < scanner->num_open_blocks) {
      uint8_t block_type = scanner->open_blocks[scanner->matched];
      if (match_block(scanner, lexer, block_type)) {
        scanner->matched++;
        if (valid_symbols[BLOCK_CONTINUATION]) {
          lexer->mark_end(lexer);
          lexer->result_symbol = BLOCK_CONTINUATION;
          return true;
        }
      } else {
        if (valid_symbols[BLOCK_CLOSE]) {
          pop_block(scanner);
          if (scanner->matched == scanner->num_open_blocks) {
            scanner->state &= ~STATE_MATCHING;
          }
          lexer->result_symbol = BLOCK_CLOSE;
          return true;
        }
      }
    }

    if (scanner->matched == scanner->num_open_blocks) {
      scanner->state &= ~STATE_MATCHING;
    }
  }

  if (!(scanner->state & STATE_MATCHING)) {
    skip_spaces(lexer);
  }
  uint16_t indent = scanner->indentation;
  scanner->indentation = 0;

  if (lexer->eof(lexer)) {
    if (valid_symbols[BLOCK_CLOSE] && scanner->num_open_blocks > 0) {
      pop_block(scanner);
      lexer->result_symbol = BLOCK_CLOSE;
      return true;
    }
    if (valid_symbols[LINE_ENDING]) {
      lexer->result_symbol = LINE_ENDING;
      return true;
    }
    return false;
  }

  if (valid_symbols[BLANK_LINE] && is_newline(lexer->lookahead)) {
    eat_newline(lexer);
    lexer->mark_end(lexer);
    lexer->result_symbol = BLANK_LINE;
    scanner->state |= STATE_MATCHING;
    scanner->matched = 0;
    scanner->indentation = 0;
    return true;
  }

  if ((valid_symbols[LINE_ENDING] || valid_symbols[SOFT_LINE_ENDING] ||
       valid_symbols[PIPE_TABLE_LINE_ENDING]) &&
      is_newline(lexer->lookahead)) {
    return scan_line_ending(scanner, lexer, valid_symbols);
  }

  // #
  if (lexer->lookahead == '#') {
    bool any_valid =
        valid_symbols[ATX_H1_MARKER] || valid_symbols[ATX_H2_MARKER] ||
        valid_symbols[ATX_H3_MARKER] || valid_symbols[ATX_H4_MARKER] ||
        valid_symbols[ATX_H5_MARKER] || valid_symbols[ATX_H6_MARKER];
    if (any_valid) {
      uint8_t level = 0;
      while (lexer->lookahead == '#' && level < 7) {
        level++;
        lexer->advance(lexer, false);
      }
      if (level >= 1 && level <= 6 &&
          (lexer->eof(lexer) || lexer->lookahead == ' ' ||
           lexer->lookahead == '\t' || is_newline(lexer->lookahead))) {
        if (lexer->lookahead == ' ' || lexer->lookahead == '\t') {
          lexer->advance(lexer, false);
        }
        lexer->mark_end(lexer);
        lexer->result_symbol = ATX_H1_MARKER + (level - 1);
        return true;
      }
    }
  }

  // ``` or ~~~
  if ((lexer->lookahead == '`' || lexer->lookahead == '~') &&
      valid_symbols[FENCED_CODE_BLOCK_DELIMITER]) {
    char fence_char = lexer->lookahead;
    uint8_t count = 0;
    while (lexer->lookahead == fence_char) {
      count++;
      lexer->advance(lexer, false);
    }
    if (count >= 3) {
      scanner->code_fence_count = count;
      scanner->code_fence_char = fence_char;
      scanner->code_fence_info_pending = true;
      lexer->mark_end(lexer);
      lexer->result_symbol = FENCED_CODE_BLOCK_DELIMITER;
      return true;
    }
  }

  // $$
  if (lexer->lookahead == '$' && valid_symbols[MATH_BLOCK_DELIMITER]) {
    uint8_t count = 0;
    while (lexer->lookahead == '$') {
      count++;
      lexer->advance(lexer, false);
    }
    if (count >= 2 && (is_newline(lexer->lookahead) || lexer->eof(lexer))) {
      scanner->math_fence_count = count;
      if (is_newline(lexer->lookahead)) eat_newline(lexer);
      lexer->mark_end(lexer);
      lexer->result_symbol = MATH_BLOCK_DELIMITER;
      return true;
    }
  }

  // >- (toggle list) or > (block quote)
  if (lexer->lookahead == '>' &&
      (valid_symbols[TOGGLE_LIST_MARKER] || valid_symbols[BLOCK_QUOTE_MARKER])) {
    lexer->advance(lexer, false);
    if (lexer->lookahead == '-' && valid_symbols[TOGGLE_LIST_MARKER]) {
      lexer->advance(lexer, false);
      if (lexer->lookahead == ' ' || lexer->lookahead == '\t') {
        lexer->advance(lexer, false);
      }
      push_block(scanner, TOGGLE_LIST_BLOCK);
      lexer->mark_end(lexer);
      lexer->result_symbol = TOGGLE_LIST_MARKER;
      return true;
    }
    if (valid_symbols[BLOCK_QUOTE_MARKER]) {
      if (lexer->lookahead == ' ') lexer->advance(lexer, false);
      push_block(scanner, BLOCK_QUOTE_BLOCK);
      lexer->mark_end(lexer);
      lexer->result_symbol = BLOCK_QUOTE_MARKER;
      return true;
    }
  }

  // -, *, N.
  if (lexer->lookahead == '-' && valid_symbols[LIST_MARKER_MINUS]) {
    if (try_list_marker_bullet(scanner, lexer, indent, LIST_MARKER_MINUS)) {
      return true;
    }
  }

  if (lexer->lookahead == '*' && valid_symbols[LIST_MARKER_STAR]) {
    if (try_list_marker_bullet(scanner, lexer, indent, LIST_MARKER_STAR)) {
      return true;
    }
  }

  if (lexer->lookahead >= '0' && lexer->lookahead <= '9' &&
      valid_symbols[LIST_MARKER_DOT]) {
    lexer->mark_end(lexer);
    uint8_t marker_width = 0;
    while (lexer->lookahead >= '0' && lexer->lookahead <= '9') {
      marker_width++;
      lexer->advance(lexer, false);
    }
    if (lexer->lookahead == '.') {
      marker_width++;
      lexer->advance(lexer, false);
      if (lexer->lookahead == ' ' || lexer->lookahead == '\t') {
        lexer->advance(lexer, false);
        uint16_t content_indent = indent + marker_width + 1;
        uint8_t block_val = LIST_ITEM +
            (content_indent > (LIST_ITEM_MAX_INDENTATION - LIST_ITEM)
             ? (LIST_ITEM_MAX_INDENTATION - LIST_ITEM)
             : (uint8_t)content_indent);
        push_block(scanner, block_val);
        lexer->mark_end(lexer);
        lexer->result_symbol = LIST_MARKER_DOT;
        return true;
      }
    }
  }

  // :::
  if (lexer->lookahead == ':' && valid_symbols[CONTAINER_CLOSE]) {
    lexer->mark_end(lexer);
    lexer->advance(lexer, false);
    if (lexer->lookahead == ':') {
      lexer->advance(lexer, false);
      if (lexer->lookahead == ':') {
        lexer->advance(lexer, false);
        lexer->mark_end(lexer);
        lexer->result_symbol = CONTAINER_CLOSE;
        return true;
      }
    }
  }

  // === name
  if (lexer->lookahead == '=' && valid_symbols[CONTAINER_SLOT_SEPARATOR]) {
    lexer->mark_end(lexer);
    lexer->advance(lexer, false);
    if (lexer->lookahead == '=') {
      lexer->advance(lexer, false);
      if (lexer->lookahead == '=') {
        lexer->advance(lexer, false);
        lexer->mark_end(lexer);
        lexer->result_symbol = CONTAINER_SLOT_SEPARATOR;
        return true;
      }
    }
  }


  if (lexer->lookahead == ':' && valid_symbols[CONTAINER_OPEN]) {
    lexer->mark_end(lexer);
    lexer->advance(lexer, false);
    if (lexer->lookahead == ':') {
      lexer->advance(lexer, false);
      if (lexer->lookahead == ':') {
        lexer->advance(lexer, false);
        while (lexer->lookahead == ' ' || lexer->lookahead == '\t') {
          lexer->advance(lexer, false);
        }
        lexer->mark_end(lexer);
        lexer->result_symbol = CONTAINER_OPEN;
        return true;
      }
    }
  }

  // | --- | --- |
  if (valid_symbols[PIPE_TABLE_DELIMITER_ROW] && lexer->lookahead == '|') {
    lexer->mark_end(lexer);
    bool valid_delim = true;
    bool has_dashes = false;
    while (!is_newline(lexer->lookahead) && !lexer->eof(lexer)) {
      if (lexer->lookahead == '-') {
        has_dashes = true;
      } else if (lexer->lookahead != '|' && lexer->lookahead != ':' &&
                 lexer->lookahead != ' ' && lexer->lookahead != '\t') {
        valid_delim = false;
        break;
      }
      lexer->advance(lexer, false);
    }
    if (valid_delim && has_dashes) {
      lexer->mark_end(lexer);
      lexer->result_symbol = PIPE_TABLE_DELIMITER_ROW;
      return true;
    }
  }

  // | ... |
  if (valid_symbols[PIPE_TABLE_START] && lexer->lookahead == '|') {
    return scan_pipe_table_start(scanner, lexer);
  }

  return false;
}
