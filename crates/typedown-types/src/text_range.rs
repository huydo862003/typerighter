/// A byte range in source text: [start, end)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct TextRange {
  pub start_offset: usize,
  pub end_offset: usize,
}

impl TextRange {
  pub fn new(start: usize, end: usize) -> Self {
    debug_assert!(start <= end, "start ({}) must be <= end ({})", start, end);
    Self {
      start_offset: start,
      end_offset: end,
    }
  }

  pub fn len(&self) -> usize {
    self.end_offset - self.start_offset
  }

  pub fn is_empty(&self) -> bool {
    self.start_offset == self.end_offset
  }
}
