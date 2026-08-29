// TIL: We use nightly `specialization` to simulate C++ `if constexpr` for compile-time type dispatch
// - FieldEncodable/FieldDecodable dispatch between query struct IDs (as DepNodeIndex) and plain types
// - The known unsoundness involves lifetime-dependent specialization, which we don't use
#![feature(specialization)]
#![allow(incomplete_features)]

/// We need this so macros can be used within this crate
extern crate self as typedown_incremental;

mod cancel;
mod db;
mod id;
mod ingredient;
mod persist;
mod storage;
#[cfg(test)]
mod tests;

pub use cancel::*;
pub use db::*;
pub use id::*;
pub use ingredient::*;
pub use persist::*;
pub use storage::*;

pub use typedown_macros::{query_db, query_derived, query_input, query_interned};
