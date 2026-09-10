//! Macros for the salsa database layer in typedown-db

mod derived;
mod input;
mod interned;
mod query_db;

pub use derived::query_derived_impl;
pub use input::query_input_impl;
pub use interned::query_interned_impl;
pub use query_db::query_db_impl;

use quote::quote;

// Replace 'db with 'static in a token stream for storage contexts
pub(crate) fn erase_db_lifetime_tokens(ty: &impl quote::ToTokens) -> proc_macro2::TokenStream {
  quote!(#ty)
    .to_string()
    .replace("'db", "'static")
    .parse()
    .expect("failed to parse type with erased lifetime")
}

// Modifiers parsed from #[query_derived(no_hash, custom_hash)]
pub(crate) struct CacheModifiers {
  pub no_hash: bool,
  pub custom_hash: bool,
}

pub(crate) fn parse_cache_modifiers(attr: proc_macro::TokenStream) -> CacheModifiers {
  let attr_str = attr.to_string();
  CacheModifiers {
    no_hash: attr_str.contains("no_hash"),
    custom_hash: attr_str.contains("custom_hash"),
  }
}
