use proc_macro::TokenStream;

mod ast;
mod db;
mod specialize;
mod stable_compare;

#[proc_macro_derive(AstNode)]
pub fn ast_node_derive(item: TokenStream) -> TokenStream {
  ast::ast_node_derive_impl(item)
}

/// Attribute macro for synthetic AST nodes that wrap multiple SyntaxKind variants.
/// Usage:
/// ```ignore
/// #[wrapper_ast_node(SyntaxKind = [KindA, KindB])]
/// pub struct MyNode(RedNode);
/// ```
#[proc_macro_attribute]
pub fn wrapper_ast_node(attr: TokenStream, item: TokenStream) -> TokenStream {
  ast::wrapper_ast_node_impl(attr, item)
}

/// Attribute macro for annotating an incremental database.
/// Usage:
/// ```ignore
/// #[query_db]
/// pub struct Database {
///   storage: Storage,
/// }
/// ```
#[proc_macro_attribute]
pub fn query_db(attr: TokenStream, item: TokenStream) -> TokenStream {
  db::query_db_impl(attr, item)
}

/// Attribute macro for annotating an incremental engine's input state.
/// Usage:
/// ```ignore
/// #[query_input]
/// pub struct Input {
/// }
/// ```
#[proc_macro_attribute]
pub fn query_input(attr: TokenStream, item: TokenStream) -> TokenStream {
  db::query_input_impl(attr, item)
}

/// Attribute macro for annotating a derived (tracked/memoized) query function.
/// Usage:
/// ```ignore
/// #[query_derived]
/// fn parse_file(db: &TypedownDatabase, file: File) -> GreenNode {
///   // ...
/// }
/// ```
#[proc_macro_attribute]
pub fn query_derived(attr: TokenStream, item: TokenStream) -> TokenStream {
  db::query_derived_impl(attr, item)
}

/// Attribute macro for annotating an interned struct.
/// Usage:
/// ```ignore
/// #[query_interned]
/// pub struct Symbol {
///   name: String,
/// }
/// ```
#[proc_macro_attribute]
pub fn query_interned(attr: TokenStream, item: TokenStream) -> TokenStream {
  db::query_interned_impl(attr, item)
}

#[proc_macro_derive(StableCompare)]
pub fn stable_compare_derive(item: TokenStream) -> TokenStream {
  stable_compare::stable_compare_derive_impl(item)
}

// Ad-hoc overloading via autoderef-based specialization (stable)
// specialize!(ident {
//   where Bound => ident.method();
//   default => fallback;
// })
#[proc_macro]
pub fn specialize(input: TokenStream) -> TokenStream {
  specialize::specialize_impl(input)
}
