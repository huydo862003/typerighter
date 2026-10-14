use proc_macro::TokenStream;
use quote::quote;
use syn::{Data, DeriveInput, Fields, parse_macro_input};

pub fn encodable_derive_impl(item: TokenStream) -> TokenStream {
  let input = parse_macro_input!(item as DeriveInput);
  let name = &input.ident;
  let (impl_generics, ty_generics, where_clause) = input.generics.split_for_impl();

  let body = match &input.data {
    Data::Struct(data) => encode_fields_body(&data.fields),
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
              let encodes = field_names.iter().map(|f| {
                quote! { #f.encode(buf, encoder); }
              });
              quote! {
                #name::#vname { #(#field_names),* } => {
                  encoder.emit_u8(buf, #tag);
                  #(#encodes)*
                }
              }
            }
            Fields::Unnamed(fields) => {
              let vars: Vec<_> = (0..fields.unnamed.len())
                .map(|i| syn::Ident::new(&format!("__f{i}"), proc_macro2::Span::call_site()))
                .collect();
              let encodes = vars.iter().map(|v| {
                quote! { #v.encode(buf, encoder); }
              });
              quote! {
                #name::#vname(#(#vars),*) => {
                  encoder.emit_u8(buf, #tag);
                  #(#encodes)*
                }
              }
            }
            Fields::Unit => {
              quote! {
                #name::#vname => {
                  encoder.emit_u8(buf, #tag);
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
      return syn::Error::new_spanned(&input.ident, "Encodable cannot be derived for unions")
        .to_compile_error()
        .into();
    }
  };

  quote! {
    impl #impl_generics ::typedown_incremental::Encodable for #name #ty_generics #where_clause {
      fn encode(&self, buf: &mut Vec<u8>, encoder: &mut ::typedown_incremental::Encoder) {
        #body
      }
    }
  }
  .into()
}

fn encode_fields_body(fields: &Fields) -> proc_macro2::TokenStream {
  match fields {
    Fields::Named(fields) => {
      let encodes = fields.named.iter().map(|f| {
        let name = &f.ident;
        quote! { self.#name.encode(buf, encoder); }
      });
      quote! { #(#encodes)* }
    }
    Fields::Unnamed(fields) => {
      let encodes = (0..fields.unnamed.len()).map(|i| {
        let idx = syn::Index::from(i);
        quote! { self.#idx.encode(buf, encoder); }
      });
      quote! { #(#encodes)* }
    }
    Fields::Unit => quote! {},
  }
}
