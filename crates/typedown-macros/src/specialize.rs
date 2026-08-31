// TIL: Nice technique - Stable specialization via autoderef-based method resolution
// Based on: https://lukaskalbertodt.github.io/2019/12/05/generalized-autoref-based-specialization.html
//
// specialize!(ident {
//   where Bound => ident.method();
//   default => fallback;
// })
//
// specialize!(type Type {
//   where Bound => expr;
//   default => fallback;
// })

use proc_macro::TokenStream;
use quote::{format_ident, quote};
use syn::parse::{Parse, ParseStream};
use syn::{Expr, Ident, Token, Type, braced};

struct Arm {
  bound: Option<syn::Path>, // None = default
  body: Expr,
}

// ident { ... }  OR  type Type { ... }
struct Input {
  binding: Option<Ident>, // Some for value dispatch, None for type dispatch
  ty: Type,               // typeof(ident) or explicit Type
  arms: Vec<Arm>,
}

impl Parse for Input {
  fn parse(input: ParseStream) -> syn::Result<Self> {
    // type Type OR ident
    let (binding, ty) = if input.peek(Token![type]) {
      input.parse::<Token![type]>()?;
      let ty: Type = input.parse()?;
      (None, ty)
    } else {
      let ident: Ident = input.parse()?;
      (Some(ident), syn::parse_quote! { _ })
    };

    // { where Bound => body; default => body; }
    let body;
    braced!(body in input);
    let mut arms = Vec::new();
    while !body.is_empty() {
      if body.peek(Token![where]) {
        // Bounded
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
        // Unbounded
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

    if arms.is_empty() {
      return Err(body.error("specialize! requires at least one arm"));
    }

    Ok(Input { binding, ty, arms })
  }
}

pub fn specialize_impl(input: TokenStream) -> TokenStream {
  let Input { binding, ty, arms } = syn::parse_macro_input!(input as Input);

  let n = arms.len();

  // One trait corresponds to an arm, higher priority trait gets more `&` layers on `Self`
  // NOTE: __dispatch_arm_index returns an arm index, arm bodies run in the outer scope where captures work
  let trait_impls: Vec<_> = arms
    .iter()
    .enumerate()
    .map(|(i, arm)| {
      let trait_name = format_ident!("__Arm{}", i);
      let refs = n - 1 - i;
      // Add `refs` &
      let self_ty = (0..refs).fold(quote! { __TypeHolder<__T> }, |inner, _| quote! { &#inner });
      let bound_clause = arm.bound.as_ref().map(|b| quote! { : #b });

      quote! {
        trait #trait_name {
          fn __dispatch_arm_index(&self) -> usize;
        }
        impl<__T #bound_clause> #trait_name for #self_ty {
          fn __dispatch_arm_index(&self) -> usize { #i }
        }
      }
    })
    .collect();

  // Build match arms: index => body
  let match_arms: Vec<_> = arms
    .iter()
    .enumerate()
    .map(|(i, arm)| {
      let body = &arm.body;
      quote! { #i => { #body } }
    })
    .collect();

  // TIL: Rust has no typeof, but a generic helper fn infers T from a value
  // __phantom(&val) produces __TypeHolder<TypeOfVal> without naming the type
  let make_wrapper = match &binding {
    Some(ident) => quote! { __phantom(&#ident) },
    None => quote! { __TypeHolder::<#ty>(::std::marker::PhantomData) },
  };
  let wrapped = (0..n).fold(make_wrapper, |inner, _| quote! { &#inner });

  // Traits select arm index, match runs the body in outer scope so variables are available
  quote! {
    {
      struct __TypeHolder<__T>(::std::marker::PhantomData<__T>);
      fn __phantom<__T>(_: &__T) -> __TypeHolder<__T> { __TypeHolder(::std::marker::PhantomData) }
      #(#trait_impls)*
      match (#wrapped).__dispatch_arm_index() {
        #(#match_arms)*
        _ => unreachable!()
      }
    }
  }
  .into()
}
