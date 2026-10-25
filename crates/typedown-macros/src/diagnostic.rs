use proc_macro::TokenStream;
use proc_macro2::TokenStream as TokenStream2;
use quote::quote;
use syn::{
  Data, DeriveInput, Expr, Fields, Lit, Meta, Token, parse_macro_input, punctuated::Punctuated,
};

/// Attribute macro for the TdDiagnosticKind enum
/// Every variant must have a `#[message(...)]` attribute
pub fn typedown_diagnostic_kind_impl(_attr: TokenStream, item: TokenStream) -> TokenStream {
  let mut input = parse_macro_input!(item as DeriveInput);
  let name = &input.ident;

  let data = match &input.data {
    Data::Enum(data) => data,
    _ => {
      return syn::Error::new_spanned(name, "typedown_diagnostic_kind can only be used on enums")
        .to_compile_error()
        .into();
    }
  };

  let mut message_arms = Vec::new();
  let mut code_str_arms = Vec::new();

  for variant in &data.variants {
    let variant_name = &variant.ident;
    let kebab = pascal_to_kebab(&variant_name.to_string());

    let message_attribute = variant
      .attrs
      .iter()
      .find(|attr| attr.path().is_ident("message"));

    let Some(attr) = message_attribute else {
      return syn::Error::new_spanned(variant_name, "missing #[message(\"...\")] attribute")
        .to_compile_error()
        .into();
    };

    let message_body = parse_message_attr(attr, &variant.fields);

    // code_str() arm
    match &variant.fields {
      Fields::Named(_) => code_str_arms.push(quote! { #name::#variant_name { .. } => #kebab }),
      Fields::Unnamed(_) => code_str_arms.push(quote! { #name::#variant_name(..) => #kebab }),
      Fields::Unit => code_str_arms.push(quote! { #name::#variant_name => #kebab }),
    }

    // message() arm
    match &variant.fields {
      Fields::Named(fields) => {
        let field_names: Vec<_> = fields.named.iter().map(|field| &field.ident).collect();
        message_arms.push(quote! {
          #name::#variant_name { #(#field_names,)* } => { #message_body }
        });
      }
      Fields::Unit => {
        message_arms.push(quote! { #name::#variant_name => { #message_body } });
      }
      Fields::Unnamed(_) => {
        message_arms.push(quote! { #name::#variant_name(..) => { #message_body } });
      }
    }
  }

  // Strip #[message] attributes from variants
  if let Data::Enum(ref mut data) = input.data {
    for variant in &mut data.variants {
      variant
        .attrs
        .retain(|attr| !attr.path().is_ident("message"));
    }
  }

  let attrs = &input.attrs;
  let visibility = &input.vis;
  let variants = match &input.data {
    Data::Enum(data) => &data.variants,
    _ => unreachable!(),
  };

  let output = quote! {
    #(#attrs)*
    #[derive(Debug, Clone, PartialEq, Eq, Encodable, Decodable, StableHash, StableCompare)]
    #visibility enum #name {
      #variants
    }

    impl #name {
      pub fn code_str(&self) -> &'static str {
        match self {
          #(#code_str_arms,)*
        }
      }

      #[allow(unused_variables)]
      pub fn message(&self) -> String {
        match self {
          #(#message_arms)*
        }
      }

      pub fn into_error(self, start_offset: usize, end_offset: usize) -> ::typedown_lang::syntax::diagnostic::TdDiagnostic {
        ::typedown_lang::syntax::diagnostic::TdDiagnostic {
          kind: self,
          severity: ::typedown_lang::syntax::diagnostic::Severity::Error,
          start_offset,
          end_offset,
        }
      }

      pub fn into_warning(self, start_offset: usize, end_offset: usize) -> ::typedown_lang::syntax::diagnostic::TdDiagnostic {
        ::typedown_lang::syntax::diagnostic::TdDiagnostic {
          kind: self,
          severity: ::typedown_lang::syntax::diagnostic::Severity::Warning,
          start_offset,
          end_offset,
        }
      }
    }
  };

  output.into()
}

/// Parse `#[message("fmt", arg1, arg2)]`
fn parse_message_attr(attr: &syn::Attribute, fields: &Fields) -> TokenStream2 {
  let Meta::List(list) = &attr.meta else {
    return quote! { format!("") };
  };

  let args: Punctuated<Expr, Token![,]> = match list.parse_args_with(Punctuated::parse_terminated) {
    Ok(args) => args,
    Err(e) => return e.to_compile_error(),
  };

  let mut iter = args.iter();
  let Some(first) = iter.next() else {
    return quote! { format!("") };
  };

  let Expr::Lit(lit) = first else {
    return quote! { format!("") };
  };
  let Lit::Str(s) = &lit.lit else {
    return quote! { format!("") };
  };
  let template = s.value();

  let extra_args: Vec<_> = iter.collect();

  // Has extra format args: format!("...", arg1, arg2)
  if !extra_args.is_empty() {
    return quote! { format!(#template, #(#extra_args),*) };
  }

  // Named field interpolation: only pass fields referenced as {name} in the template
  match fields {
    Fields::Named(named) => {
      let used_fields: Vec<_> = named
        .named
        .iter()
        .filter(|field| {
          let name = field.ident.as_ref().unwrap().to_string();
          template.contains(&format!("{{{name}}}")) || template.contains(&format!("{{{name}:"))
        })
        .map(|field| &field.ident)
        .collect();

      if used_fields.is_empty() {
        quote! { format!(#template) }
      } else {
        quote! { format!(#template, #(#used_fields = #used_fields),*) }
      }
    }
    _ => quote! { format!(#template) },
  }
}

fn pascal_to_kebab(name: &str) -> String {
  let mut result = String::new();
  let mut chars = name.chars().peekable();
  while let Some(char) = chars.next() {
    if char.is_uppercase() {
      if !result.is_empty() {
        let next_is_lower = chars.peek().is_some_and(|name| name.is_lowercase());
        let prev_is_lower_or_digit =
          result.ends_with(|c: char| c.is_ascii_lowercase() || c.is_ascii_digit());
        if prev_is_lower_or_digit || next_is_lower {
          result.push('-');
        }
      }
      result.push(char.to_ascii_lowercase());
    } else {
      result.push(char);
    }
  }
  result
}
