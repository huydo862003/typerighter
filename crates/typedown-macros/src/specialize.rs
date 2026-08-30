use proc_macro::TokenStream;
use syn::parse::{Parse, ParseStream};
use syn::{Expr, Ident, Token, braced};

// default => expr;
// where Bound => expr;
// ...
struct Arm {
  bound: Option<syn::Path>,
  body: Expr,
}

// Input of the macro
struct Input {
  sig: syn::Signature, // fn name<T>(val: &T, ...) -> Ret
  arms: Vec<Arm>,      // default + specialized arms
}

impl Parse for Input {
  fn parse(input: ParseStream) -> syn::Result<Self> {
    // Signature fn<T>(val: &T, ...) -> Ret
    let sig: syn::Signature = input.parse()?;

    // Body of function
    let body;
    braced!(body in input);
    let mut arms = Vec::new();

    while !body.is_empty() {
      if body.peek(Token![where]) {
        // with bound
        body.parse::<Token![where]>()?;
        let bound: syn::Path = body.parse()?;
        body.parse::<Token![=>]>()?;
        let expr: Expr = body.parse()?;
        body.parse::<Token![;]>()?;
        arms.push(Arm {
          bound: Some(bound),
          body: expr,
        });
      } else {
        // without bound
        let ident: Ident = body.parse()?;
        if ident != "default" {
          return Err(syn::Error::new(
            ident.span(),
            "expected `default` or `where`",
          ));
        }
        body.parse::<Token![=>]>()?;
        let expr: Expr = body.parse()?;
        body.parse::<Token![;]>()?;
        arms.push(Arm {
          bound: None,
          body: expr,
        });
      }
    }
    Ok(Input { sig, arms })
  }
}

pub fn specialize_impl(input: TokenStream) -> TokenStream {
  let spec = syn::parse_macro_input!(input as Input);
  // TODO: codegen
  todo!()
}
