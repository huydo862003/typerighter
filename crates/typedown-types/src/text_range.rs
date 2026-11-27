/// A byte range in source text: [start, end)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct TextRange {
  pub start_byte_offset: usize,
  pub end_byte_offset: usize,
}

impl TextRange {
  pub fn new(start: usize, end: usize) -> Self {
    debug_assert!(start <= end, "start ({}) must be <= end ({})", start, end);
    Self {
      start_byte_offset: start,
      end_byte_offset: end,
    }
  }

  pub fn len(&self) -> usize {
    self.end_byte_offset - self.start_byte_offset
  }

  pub fn is_empty(&self) -> bool {
    self.start_byte_offset == self.end_byte_offset
  }
}
