use proc_macro::TokenStream;
use quote::quote;
use syn::ItemStruct;

pub fn query_db_impl(_attr: TokenStream, item: TokenStream) -> TokenStream {
  let struct_ast = match syn::parse::<ItemStruct>(item) {
    Ok(ast) => ast,
    Err(err) => return err.to_compile_error().into(),
  };

  let struct_name = &struct_ast.ident;

  let storage_field = struct_ast
    .fields
    .iter()
    .find(|field| field.ident.as_ref().is_some_and(|name| name == "storage"));

  let storage_ty = match storage_field {
    Some(field) => &field.ty,
    None => {
      return syn::Error::new_spanned(&struct_ast, "expected a `storage: QueryStorage` field")
        .to_compile_error()
        .into();
    }
  };

  quote! {
    #struct_ast

    #[cfg(debug_assertions)]
    const _: () = <#storage_ty>::__TYPEDOWN_QUERY_STORAGE;

    impl ::typedown_incremental::QueryDatabase for #struct_name {
      unsafe fn storage(&self) -> &::typedown_incremental::QueryStorage {
        &self.storage
      }

      unsafe fn storage_mut(&mut self) -> &mut ::typedown_incremental::QueryStorage {
        &mut self.storage
      }
    }

    impl ::typedown_incremental::SerializableQueryDatabase for #struct_name {}
  }
  .into()
}
