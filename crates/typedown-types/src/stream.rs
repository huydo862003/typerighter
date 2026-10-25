/// A result of reading from a UTF-8 stream
#[derive(Debug, Clone, Copy)]
pub enum Utf8Result {
  /// A valid UTF-8 character
  Char(char),
  /// Some invalid bytes were skipped while recovering
  /// The recovery scheme is up to the implementor
  Invalid { len: usize, bytes: [u8; 4] },
  /// The end of the stream has been reached
  Eof,
}

/// Character-level input source
/// Abstracts over files, strings, stdin, etc
pub trait Utf8Stream {
  /// Look at the next result without consuming it
  fn peek(&self) -> Utf8Result;

  /// Consume and return the next result
  fn advance(&mut self) -> Utf8Result;

  /// Current byte offset in the source
  fn offset(&self) -> usize;

  /// Whether the stream is exhausted
  fn exhausted(&self) -> bool;

  /// Peek at the nth character ahead (0-indexed) without consuming
  /// Advances n chars internally, peeks, then restores the stream
  fn peek_nth(&mut self, n: usize) -> Utf8Result;
}

impl Utf8Stream for Box<dyn Utf8Stream> {
  fn peek(&self) -> Utf8Result {
    (**self).peek()
  }

  fn advance(&mut self) -> Utf8Result {
    (**self).advance()
  }

  fn offset(&self) -> usize {
    (**self).offset()
  }

  fn exhausted(&self) -> bool {
    (**self).exhausted()
  }

  fn peek_nth(&mut self, n: usize) -> Utf8Result {
    (**self).peek_nth(n)
  }
}
