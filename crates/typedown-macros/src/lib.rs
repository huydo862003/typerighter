use proc_macro::TokenStream;

mod ast;
mod db;
mod decodable;
mod diagnostic;
mod encodable;
mod specialize;
mod stable_compare;
mod stable_hash;

#[proc_macro_derive(AstNode)]
pub fn ast_node_derive(item: TokenStream) -> TokenStream {
  ast::ast_node_derive_impl(item)
}

/// Attribute macro for synthetic AST nodes that wrap multiple SyntaxKind variants
#[proc_macro_attribute]
pub fn wrapper_ast_node(attr: TokenStream, item: TokenStream) -> TokenStream {
  ast::wrapper_ast_node_impl(attr, item)
}

#[proc_macro_attribute]
pub fn query_db(attr: TokenStream, item: TokenStream) -> TokenStream {
  db::query_db_impl(attr, item)
}

#[proc_macro_attribute]
pub fn query_input(attr: TokenStream, item: TokenStream) -> TokenStream {
  db::query_input_impl(attr, item)
}

#[proc_macro_attribute]
pub fn query_derived(attr: TokenStream, item: TokenStream) -> TokenStream {
  db::query_derived_impl(attr, item)
}

#[proc_macro_attribute]
pub fn query_interned(attr: TokenStream, item: TokenStream) -> TokenStream {
  db::query_interned_impl(attr, item)
}

#[proc_macro_derive(Encodable)]
pub fn encodable_derive(item: TokenStream) -> TokenStream {
  encodable::encodable_derive_impl(item)
}

#[proc_macro_derive(Decodable)]
pub fn decodable_derive(item: TokenStream) -> TokenStream {
  decodable::decodable_derive_impl(item)
}

#[proc_macro_derive(StableHash)]
pub fn stable_hash_derive(item: TokenStream) -> TokenStream {
  stable_hash::stable_hash_derive_impl(item)
}

#[proc_macro_derive(StableCompare)]
pub fn stable_compare_derive(item: TokenStream) -> TokenStream {
  stable_compare::stable_compare_derive_impl(item)
}

#[proc_macro_attribute]
pub fn typedown_diagnostic_kind(attr: TokenStream, item: TokenStream) -> TokenStream {
  diagnostic::typedown_diagnostic_kind_impl(attr, item)
}

#[proc_macro]
pub fn specialize(input: TokenStream) -> TokenStream {
  specialize::specialize_impl(input)
}
