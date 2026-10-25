//! Macros for the AST layer in typedown-syntax

use proc_macro::TokenStream;
use quote::quote;
use syn::{
  Ident, Token,
  parse::{Parse, ParseStream},
  punctuated::Punctuated,
};

pub fn ast_node_derive_impl(item: TokenStream) -> TokenStream {
  let item_ast: syn::DeriveInput = syn::parse(item).unwrap();

  let name = &item_ast.ident;
  let generated = quote! {
    impl AstNode for #name {
      fn cast(syntax: RedNode) -> Option<Self> {
          match syntax.kind() {
            crate::syntax::syntax_kind::SyntaxKind::#name => Some(Self(syntax)),
            _ => None,
          }
        }
      fn syntax(&self) -> &RedNode {
        &self.0
      }
    };
  };
  generated.into()
}

// Parses: SyntaxKind = [Variant1, Variant2, ...]
struct WrapperAstNodeMacroArgs {
  kinds: Punctuated<Ident, Token![,]>,
}

impl Parse for WrapperAstNodeMacroArgs {
  fn parse(input: ParseStream) -> syn::Result<Self> {
    let _: Ident = input.parse()?; // Consume SyntaxKind
    let _: Token![=] = input.parse()?; // Consume `=`

    let content;
    syn::bracketed!(content in input); // Extract the list content

    // Parse the list content and return
    let kinds = Punctuated::<Ident, Token![,]>::parse_terminated(&content)?;
    Ok(WrapperAstNodeMacroArgs { kinds })
  }
}

/// Attribute macro for wrapper AST enums that cover multiple SyntaxKind variants
/// The annotated type becomes an enum with one variant per kind
/// Pattern matching on the enum is exhaustive with no extra wrapper needed
///
/// Generates:
/// - The enum with `#[derive(Clone, PartialEq, Eq, Hash)]`
/// - `AstNode` impl: `cast()` dispatches via O(1) SyntaxKind match; `syntax()` delegates to the variant
/// - `From<Variant>` and `TryFrom<Self> for Variant` for each variant
///
/// Usage:
/// ```ignore
/// #[wrapper_ast_node(SyntaxKind = [KindA, KindB])]
/// pub enum MyNode {}
/// ```
pub fn wrapper_ast_node_impl(attr: TokenStream, item: TokenStream) -> TokenStream {
  let args: WrapperAstNodeMacroArgs = syn::parse(attr).unwrap();
  let item_ast: syn::DeriveInput = syn::parse(item).unwrap();

  let name = &item_ast.ident;
  let vis = &item_ast.vis;
  let kinds: Vec<_> = args.kinds.iter().collect();

  // cast() arms: SyntaxKind::X => Some(Name::X(X(syntax)))
  let cast_arms = kinds.iter().map(|kind| {
    quote! {
      crate::syntax::syntax_kind::SyntaxKind::#kind => Some(#name::#kind(#kind(syntax))),
    }
  });

  // syntax() arms: Name::X(n) => n.syntax()
  let syntax_arms = kinds.iter().map(|kind| {
    quote! { #name::#kind(n) => n.syntax(), }
  });

  let from_impls = kinds.iter().map(|kind| {
    quote! {
      impl From<#kind> for #name {
        fn from(node: #kind) -> Self {
          #name::#kind(node)
        }
      }

      impl TryFrom<#name> for #kind {
        type Error = ();
        fn try_from(node: #name) -> Result<Self, ()> {
          match node {
            #name::#kind(n) => Ok(n),
            _ => Err(()),
          }
        }
      }
    }
  });

  let generated = quote! {
    #[derive(Clone, PartialEq, Eq, Hash)]
    #vis enum #name {
      #(#kinds(#kinds)),*
    }

    impl AstNode for #name {
      fn cast(syntax: RedNode) -> Option<Self> {
        match syntax.kind() {
          #(#cast_arms)*
          _ => None,
        }
      }
      fn syntax(&self) -> &RedNode {
        match self {
          #(#syntax_arms)*
        }
      }
    }

    #(#from_impls)*
  };
  generated.into()
}
