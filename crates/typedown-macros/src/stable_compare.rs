use proc_macro::TokenStream;
use quote::quote;
use syn::{Data, DeriveInput, Fields, parse_macro_input};

pub fn stable_compare_derive_impl(item: TokenStream) -> TokenStream {
  let input = parse_macro_input!(item as DeriveInput);
  let name = &input.ident;

  let body = match &input.data {
    Data::Struct(data) => match &data.fields {
      Fields::Named(fields) => {
        let field_names: Vec<_> = fields.named.iter().map(|f| &f.ident).collect();
        quote! {
          ::std::cmp::Ordering::Equal
          #(
            .then_with(|| self.#field_names.stable_cmp(db, &other.#field_names))
          )*
        }
      }
      Fields::Unnamed(fields) => {
        let indices: Vec<syn::Index> = (0..fields.unnamed.len()).map(syn::Index::from).collect();
        quote! {
          ::std::cmp::Ordering::Equal
          #(
            .then_with(|| self.#indices.stable_cmp(db, &other.#indices))
          )*
        }
      }
      Fields::Unit => {
        quote! { let _ = db; ::std::cmp::Ordering::Equal }
      }
    },
    Data::Enum(data) => {
      let disc_arms: Vec<_> = data
        .variants
        .iter()
        .enumerate()
        .map(|(i, variant)| {
          let vname = &variant.ident;
          match &variant.fields {
            Fields::Named(_) => quote! { #name::#vname { .. } => #i, },
            Fields::Unnamed(_) => quote! { #name::#vname(..) => #i, },
            Fields::Unit => quote! { #name::#vname => #i, },
          }
        })
        .collect();

      let cmp_arms: Vec<_> = data
        .variants
        .iter()
        .map(|variant| {
          let vname = &variant.ident;
          match &variant.fields {
            Fields::Named(fields) => {
              let field_names: Vec<_> = fields.named.iter().map(|f| &f.ident).collect();
              let self_vars: Vec<_> = (0..field_names.len())
                .map(|i| syn::Ident::new(&format!("__self_{i}"), proc_macro2::Span::call_site()))
                .collect();
              let other_vars: Vec<_> = (0..field_names.len())
                .map(|i| syn::Ident::new(&format!("__other_{i}"), proc_macro2::Span::call_site()))
                .collect();
              let self_bindings: Vec<_> = field_names
                .iter()
                .zip(self_vars.iter())
                .map(|(name, var)| quote! { #name: #var })
                .collect();
              let other_bindings: Vec<_> = field_names
                .iter()
                .zip(other_vars.iter())
                .map(|(name, var)| quote! { #name: #var })
                .collect();
              quote! {
                (#name::#vname { #(#self_bindings),* }, #name::#vname { #(#other_bindings),* }) => {
                  ::std::cmp::Ordering::Equal
                  #(
                    .then_with(|| #self_vars.stable_cmp(db, #other_vars))
                  )*
                }
              }
            }
            Fields::Unnamed(fields) => {
              let self_vars: Vec<_> = (0..fields.unnamed.len())
                .map(|i| syn::Ident::new(&format!("__self_{i}"), proc_macro2::Span::call_site()))
                .collect();
              let other_vars: Vec<_> = (0..fields.unnamed.len())
                .map(|i| syn::Ident::new(&format!("__other_{i}"), proc_macro2::Span::call_site()))
                .collect();
              quote! {
                (#name::#vname(#(#self_vars),*), #name::#vname(#(#other_vars),*)) => {
                  ::std::cmp::Ordering::Equal
                  #(
                    .then_with(|| #self_vars.stable_cmp(db, #other_vars))
                  )*
                }
              }
            }
            Fields::Unit => {
              quote! {
                (#name::#vname, #name::#vname) => ::std::cmp::Ordering::Equal,
              }
            }
          }
        })
        .collect();

      quote! {
        {
          fn __disc_idx(v: &#name) -> usize {
            match v {
              #(#disc_arms)*
            }
          }
          let _ = db;
          match (self, other) {
            #(#cmp_arms)*
            _ => __disc_idx(self).cmp(&__disc_idx(other)),
          }
        }
      }
    }
    Data::Union(_) => {
      return syn::Error::new_spanned(&input.ident, "StableCompare cannot be derived for unions")
        .to_compile_error()
        .into();
    }
  };

  quote! {
    impl ::typedown_incremental::StableCompare for #name {
      fn stable_cmp<DB: ::typedown_incremental::QueryDatabase + ?Sized>(&self, db: &DB, other: &Self) -> ::std::cmp::Ordering {
        #body
      }
    }
  }
  .into()
}
