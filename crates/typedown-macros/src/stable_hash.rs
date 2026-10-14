use proc_macro::TokenStream;
use quote::quote;
use syn::{Data, DeriveInput, Fields, parse_macro_input};

pub fn stable_hash_derive_impl(item: TokenStream) -> TokenStream {
  let input = parse_macro_input!(item as DeriveInput);
  let name = &input.ident;
  let (impl_generics, ty_generics, where_clause) = input.generics.split_for_impl();

  let body = match &input.data {
    Data::Struct(data) => hash_fields_body(&data.fields),
    Data::Enum(data) => {
      let arms: Vec<_> = data
        .variants
        .iter()
        .enumerate()
        .map(|(i, variant)| {
          let vname = &variant.ident;
          let tag = i as u8;
          match &variant.fields {
            Fields::Named(fields) => {
              let field_names: Vec<_> = fields.named.iter().map(|f| &f.ident).collect();
              let hashes = field_names.iter().map(|f| {
                quote! { #f.stable_hash(db, hasher); }
              });
              quote! {
                #name::#vname { #(#field_names),* } => {
                  #tag.stable_hash(db, hasher);
                  #(#hashes)*
                }
              }
            }
            Fields::Unnamed(fields) => {
              let vars: Vec<_> = (0..fields.unnamed.len())
                .map(|i| syn::Ident::new(&format!("__f{i}"), proc_macro2::Span::call_site()))
                .collect();
              let hashes = vars.iter().map(|v| {
                quote! { #v.stable_hash(db, hasher); }
              });
              quote! {
                #name::#vname(#(#vars),*) => {
                  #tag.stable_hash(db, hasher);
                  #(#hashes)*
                }
              }
            }
            Fields::Unit => {
              quote! {
                #name::#vname => {
                  #tag.stable_hash(db, hasher);
                }
              }
            }
          }
        })
        .collect();

      quote! {
        match self {
          #(#arms)*
        }
      }
    }
    Data::Union(_) => {
      return syn::Error::new_spanned(&input.ident, "StableHash cannot be derived for unions")
        .to_compile_error()
        .into();
    }
  };

  quote! {
    impl #impl_generics ::typedown_incremental::StableHash for #name #ty_generics #where_clause {
      fn stable_hash<DB: ::typedown_incremental::QueryDatabase + ?Sized>(
        &self,
        db: &DB,
        hasher: &mut ::typedown_incremental::StableHasher,
      ) {
        #body
      }
    }
  }
  .into()
}

fn hash_fields_body(fields: &Fields) -> proc_macro2::TokenStream {
  match fields {
    Fields::Named(fields) => {
      let hashes = fields.named.iter().map(|f| {
        let name = &f.ident;
        quote! { self.#name.stable_hash(db, hasher); }
      });
      let suppress_db = if fields.named.is_empty() {
        quote! { let _ = (db, hasher); }
      } else {
        quote! {}
      };
      quote! { #suppress_db #(#hashes)* }
    }
    Fields::Unnamed(fields) => {
      let hashes = (0..fields.unnamed.len()).map(|i| {
        let idx = syn::Index::from(i);
        quote! { self.#idx.stable_hash(db, hasher); }
      });
      quote! { #(#hashes)* }
    }
    Fields::Unit => quote! { let _ = (db, hasher); },
  }
}
