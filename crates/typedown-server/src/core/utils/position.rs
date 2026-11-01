use lsp_types::{Position, Range};
use ropey::Rope;
use typedown_lang::syntax::red::RedNode;
use typedown_types::text_range::TextRange;

pub fn text_offset_to_lsp_position(rope: &Rope, char_offset: usize) -> Position {
  let line = rope.char_to_line(char_offset);
  let column = char_offset - rope.line_to_char(line);
  Position {
    line: line as u32,
    character: column as u32,
  }
}

pub fn lsp_position_to_text_offset(rope: &Rope, position: Position) -> Option<usize> {
  let line_start = rope.try_line_to_char(position.line as usize).ok()?;
  Some(line_start + position.character as usize)
}

pub fn text_range_to_lsp_range(rope: &Rope, range: TextRange) -> Range {
  Range {
    start: text_offset_to_lsp_position(rope, range.start_offset),
    end: text_offset_to_lsp_position(rope, range.end_offset),
  }
}

pub fn lsp_range_to_text_range(rope: &Rope, range: Range) -> Option<TextRange> {
  let start = lsp_position_to_text_offset(rope, range.start)?;
  let end = lsp_position_to_text_offset(rope, range.end)?;
  Some(TextRange::new(start, end))
}

/// Convert a RedNode to an LSP range using its trimmed span (excluding leading/trailing trivia)
pub fn node_trimmed_range(rope: &Rope, node: &RedNode) -> Range {
  let (offset, len) = node.trimmed_range();
  Range {
    start: text_offset_to_lsp_position(rope, offset),
    end: text_offset_to_lsp_position(rope, offset + len),
  }
}
