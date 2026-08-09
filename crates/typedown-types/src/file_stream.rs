use std::{
  cell::{Cell, RefCell},
  collections::VecDeque,
  io::{BufReader, Read},
};

use crate::stream::{Utf8Result, Utf8Stream};

/// A Utf8Stream over any `Read` source (file, stdin, etc.) via BufReader.
pub struct FileStream<T: Read> {
  reader: RefCell<BufReader<T>>,
  /// Buffered result for fast repeated peek access
  buffer: Cell<Option<Utf8Result>>,
  /// Current byte offset in the source
  offset: Cell<usize>,
  /// Number of bytes to skip (from invalid UTF-8 recovery)
  skip: Cell<usize>,
  /// Lookahead buffer for peek_nth, drained before reading the reader
  lookahead: VecDeque<Utf8Result>,
}

impl<T: Read> FileStream<T> {
  pub fn new(source: T) -> Self {
    Self {
      reader: RefCell::new(BufReader::new(source)),
      buffer: Cell::new(None),
      offset: Cell::new(0),
      skip: Cell::new(0),
      lookahead: VecDeque::with_capacity(4),
    }
  }
}

impl<T: Read> FileStream<T> {
  // Read the next char from the underlying reader, bypassing lookahead
  fn read_next(&self) -> Utf8Result {
    if let Some(result) = self.buffer.get() {
      return result;
    }

    let mut bytes = [0u8; 4];
    let mut filled = 0;

    let result = loop {
      match self
        .reader
        .borrow_mut()
        .read(&mut bytes[filled..filled + 1])
      {
        Ok(0) => break Utf8Result::Eof,
        Ok(_) => {
          filled += 1;
          if let Ok(s) = std::str::from_utf8(&bytes[..filled]) {
            let ch = s.chars().next().expect("valid UTF-8 must yield a char");
            break Utf8Result::Char(ch);
          }
          if filled >= 4 {
            self.skip.set(filled);
            break Utf8Result::Invalid { len: filled, bytes };
          }
        }
        Err(_) => break Utf8Result::Eof,
      }
    };

    self.buffer.set(Some(result));
    result
  }
}

impl<T: Read> Utf8Stream for FileStream<T> {
  fn peek(&self) -> Utf8Result {
    if !self.lookahead.is_empty() {
      return self.lookahead[0];
    }
    self.read_next()
  }

  fn advance(&mut self) -> Utf8Result {
    let result = if !self.lookahead.is_empty() {
      self.lookahead.pop_front().unwrap()
    } else {
      match self.buffer.take() {
        Some(r) => r,
        None => {
          let r = self.peek();
          self.buffer.take();
          r
        }
      }
    };

    match &result {
      Utf8Result::Char(char) => {
        self.offset.update(|v| v + char.len_utf8());
      }
      Utf8Result::Invalid { .. } => {
        self.offset.update(|v| v + self.skip.get());
        self.skip.set(0);
      }
      Utf8Result::Eof => {}
    }

    result
  }

  fn offset(&self) -> usize {
    self.offset.get()
  }

  fn exhausted(&self) -> bool {
    matches!(self.peek(), Utf8Result::Eof)
  }

  fn peek_nth(&mut self, n: usize) -> Utf8Result {
    // Fill lookahead buffer up to n + 1 entries from the reader
    if self.lookahead.capacity() < n + 1 {
      self.lookahead.reserve(n + 1 - self.lookahead.capacity());
    }
    while self.lookahead.len() <= n {
      let result = match self.buffer.take() {
        Some(r) => r,
        None => {
          // Read from buffer/reader, bypassing lookahead
          let r = self.read_next();
          self.buffer.take();
          r
        }
      };
      let is_eof = matches!(result, Utf8Result::Eof);
      self.lookahead.push_back(result);
      if is_eof {
        break;
      }
    }
    if n < self.lookahead.len() {
      self.lookahead[n]
    } else {
      Utf8Result::Eof
    }
  }
}
