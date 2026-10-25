use proc_macro::TokenStream;
use quote::quote;
use syn::{Data, DeriveInput, Fields, parse_macro_input};

pub fn decodable_derive_impl(item: TokenStream) -> TokenStream {
  let input = parse_macro_input!(item as DeriveInput);
  let name = &input.ident;
  let (impl_generics, typ_generics, where_clause) = input.generics.split_for_impl();

  let body = match &input.data {
    Data::Struct(data) => decode_struct_body(name, &data.fields),
    Data::Enum(data) => {
      let arms: Vec<_> = data
        .variants
        .iter()
        .enumerate()
        .map(|(index, variant)| {
          let vname = &variant.ident;
          let tag = index as u8;
          match &variant.fields {
            Fields::Named(fields) => {
              let field_decodes: Vec<_> = fields
                .named
                .iter()
                .map(|field| {
                  let field_name = &field.ident;
                  let field_typ = &field.ty;
                  quote! { #field_name: <#field_typ as ::typedown_incremental::Decodable>::decode(data, decoder) }
                })
                .collect();
              quote! {
                #tag => #name::#vname { #(#field_decodes),* },
              }
            }
            Fields::Unnamed(fields) => {
              let field_decodes: Vec<_> = fields
                .unnamed
                .iter()
                .map(|field| {
                  let field_typ = &field.ty;
                  quote! { <#field_typ as ::typedown_incremental::Decodable>::decode(data, decoder) }
                })
                .collect();
              quote! {
                #tag => #name::#vname(#(#field_decodes),*),
              }
            }
            Fields::Unit => {
              quote! {
                #tag => #name::#vname,
              }
            }
          }
        })
        .collect();

      let name_string = name.to_string();
      quote! {
        let __tag = decoder.read_u8(data);
        match __tag {
          #(#arms)*
          _ => panic!("unknown {} variant tag: {}", #name_string, __tag),
        }
      }
    }
    Data::Union(_) => {
      return syn::Error::new_spanned(&input.ident, "Decodable cannot be derived for unions")
        .to_compile_error()
        .into();
    }
  };

  quote! {
    impl #impl_generics ::typedown_incremental::Decodable for #name #typ_generics #where_clause {
      fn decode(data: &mut &[u8], decoder: &::typedown_incremental::Decoder) -> Self {
        #body
      }
    }
  }
  .into()
}

fn decode_struct_body(name: &syn::Ident, fields: &Fields) -> proc_macro2::TokenStream {
  match fields {
    Fields::Named(fields) => {
      let field_decodes: Vec<_> = fields
        .named
        .iter()
        .map(|field| {
          let field_name = &field.ident;
          let field_typ = &field.ty;
          quote! { #field_name: <#field_typ as ::typedown_incremental::Decodable>::decode(data, decoder) }
        })
        .collect();
      quote! { #name { #(#field_decodes),* } }
    }
    Fields::Unnamed(fields) => {
      let field_decodes: Vec<_> = fields
        .unnamed
        .iter()
        .map(|field| {
          let field_typ = &field.ty;
          quote! { <#field_typ as ::typedown_incremental::Decodable>::decode(data, decoder) }
        })
        .collect();
      quote! { #name(#(#field_decodes),*) }
    }
    Fields::Unit => quote! { #name },
  }
}
