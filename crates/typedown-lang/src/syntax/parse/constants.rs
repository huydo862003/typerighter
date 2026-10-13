pub(in crate::syntax::parse) const SKIP_NONE: usize = 0;

pub(in crate::syntax::parse) const SKIP_WS: usize = 1 << 0;

pub(in crate::syntax::parse) const SKIP_COMMENT: usize = 1 << 1;
pub(in crate::syntax::parse) const SKIP_NEWLINE: usize = 1 << 2;
pub(in crate::syntax::parse) const SKIP_INDENT: usize = 1 << 3;

pub(in crate::syntax::parse) const SKIP_WC: usize = SKIP_WS | SKIP_COMMENT;
pub(in crate::syntax::parse) const SKIP_WCN: usize = SKIP_WS | SKIP_COMMENT | SKIP_NEWLINE;
pub(in crate::syntax::parse) const SKIP_ALL_TRIVIA: usize =
  SKIP_WS | SKIP_COMMENT | SKIP_NEWLINE | SKIP_INDENT;
