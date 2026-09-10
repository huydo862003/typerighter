use proc_macro::TokenStream;
use quote::quote;
use syn::ItemStruct;

use super::erase_db_lifetime_tokens;

pub fn query_interned_impl(_attr: TokenStream, item: TokenStream) -> TokenStream {
  let struct_ast = match syn::parse::<ItemStruct>(item) {
    Ok(ast) => ast,
    Err(err) => return err.to_compile_error().into(),
  };

  let visibility = &struct_ast.vis;
  let struct_name = &struct_ast.ident;
  let has_db_lifetime = struct_ast
    .generics
    .lifetimes()
    .any(|param| param.lifetime.ident == "db");

  let fields: Vec<_> = match &struct_ast.fields {
    syn::Fields::Named(fields) => &fields.named,
    _ => {
      return syn::Error::new_spanned(&struct_ast, "expected a struct with named fields")
        .to_compile_error()
        .into();
    }
  }
  .iter()
  .collect();

  let mut output: TokenStream = quote! {}.into();

  for field in &fields {
    let field_ty_static = erase_db_lifetime_tokens(&field.ty);
    output.extend::<TokenStream>(
      quote! {
        const _: () = {
          const fn assert_send<T: Send>() {}
          const fn assert_sync<T: Sync>() {}
          const fn assert_clone<T: Clone>() {}
          const fn assert_hash<T: ::std::hash::Hash>() {}
          const fn assert_eq<T: Eq>() {}
          assert_send::<#field_ty_static>();
          assert_sync::<#field_ty_static>();
          assert_clone::<#field_ty_static>();
          assert_hash::<#field_ty_static>();
          assert_eq::<#field_ty_static>();

          #[cfg(debug_assertions)]
          const _: () = ::typedown_incremental::QueryStorage::__TYPEDOWN_QUERY_STORAGE;
        };
      }
      .into(),
    );
  }

  let field_types: Vec<_> = fields.iter().map(|f| &f.ty).collect();
  let field_types_static: Vec<proc_macro2::TokenStream> =
    field_types.iter().map(erase_db_lifetime_tokens).collect();
  let field_names: Vec<_> = fields.iter().map(|f| f.ident.as_ref().unwrap()).collect();
  let intern_key_ty = quote! { (#(#field_types_static,)*) };

  // Register via InternedInventory
  output.extend::<TokenStream>(
    quote! {
      ::inventory::submit! {
        ::typedown_incremental::InternedInventory {
          register: |factories| {
            let index = factories.len() as u32;
            factories.push(|ingredient_id| {
              Box::new(::typedown_incremental::InternedIngredientStore::<#intern_key_ty>::new(
                ingredient_id,
                stringify!(#struct_name),
                #struct_name::id_counter(),
                #struct_name::intern_map(),
              ))
            });
            #struct_name::set_ingredient_id(index);
          },
        }
      }
    }
    .into(),
  );

  // Generate getters
  let mut getter_tokens = quote! {};
  for (idx, field) in fields.iter().enumerate() {
    let field_name = field.ident.as_ref().unwrap();
    let field_ty = &field.ty;
    let tuple_index = syn::Index::from(idx);
    let try_field_name = quote::format_ident!("try_{}", field_name);

    getter_tokens.extend(quote! {
      pub fn #field_name<DB: ::typedown_incremental::QueryDatabase + ?Sized>(self, db: &DB) -> #field_ty {
        let id = self.0;
        let storage = unsafe { db.storage() };
        let ingredient_id = Self::ingredient_id() as usize;
        let ingredient = (&*storage.interned[ingredient_id] as &dyn ::std::any::Any)
          .downcast_ref::<::typedown_incremental::InternedIngredientStore<#intern_key_ty>>().expect("ingredient type mismatch");
        let entry = ingredient.data.get(&id).expect("invalid interned id");

        // Safety: transmute 'static stored value to 'db at the boundary
        unsafe { ::std::mem::transmute(entry.value.#tuple_index.clone()) }
      }

      pub fn #try_field_name<DB: ::typedown_incremental::QueryDatabase + ?Sized>(self, db: &DB) -> Option<#field_ty> {
        let id = self.0;
        let storage = unsafe { db.storage() };
        let ingredient_id = Self::ingredient_id() as usize;
        let ingredient = (&*storage.interned[ingredient_id] as &dyn ::std::any::Any)
          .downcast_ref::<::typedown_incremental::InternedIngredientStore<#intern_key_ty>>().expect("ingredient type mismatch");
        let entry = ingredient.data.get(&id)?;

        // Safety: transmute 'static stored value to 'db at the boundary
        Some(unsafe { ::std::mem::transmute(entry.value.#tuple_index.clone()) })
      }
    });
  }

  // Conditionally generate struct and impls with or without 'db
  let (struct_def, self_constructor, iter_map, impl_trait_prefix, impl_trait_for, debug_check_ty) =
    if has_db_lifetime {
      (
        quote! { #visibility struct #struct_name<'db>(u32, ::std::marker::PhantomData<&'db (dyn ::typedown_incremental::QueryDatabase + Sync + Send)>) },
        quote! { Self(id, ::std::marker::PhantomData) },
        quote! { ingredient.entry_ids().map(|id| Self(id, ::std::marker::PhantomData)).collect() },
        quote! { impl<'db> },
        quote! { #struct_name<'db> },
        quote! { #struct_name<'static> },
      )
    } else {
      (
        quote! { #visibility struct #struct_name(u32) },
        quote! { Self(id) },
        quote! { ingredient.entry_ids().map(Self).collect() },
        quote! { impl },
        quote! { #struct_name },
        quote! { #struct_name },
      )
    };
  let impl_header = quote! { #impl_trait_prefix #impl_trait_for };

  output.extend::<TokenStream>(
    quote! {
      #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
      #struct_def;

      #[allow(clippy::useless_transmute)]
      #impl_header {
        fn ingredient_id_lock() -> &'static ::std::sync::OnceLock<u32> {
          static INDEX: ::std::sync::OnceLock<u32> = ::std::sync::OnceLock::new();
          &INDEX
        }

        fn ingredient_id() -> u32 {
          *Self::ingredient_id_lock().get()
            .expect("ingredient not registered; was QueryStorage initialized?")
        }

        #[doc(hidden)]
        pub fn set_ingredient_id(index: u32) {
          let _ = Self::ingredient_id_lock().set(index);
        }

        fn id_counter() -> &'static ::std::sync::atomic::AtomicU32 {
          static COUNTER: ::std::sync::atomic::AtomicU32 = ::std::sync::atomic::AtomicU32::new(0);
          &COUNTER
        }

        fn next_id() -> u32 {
          Self::id_counter().fetch_add(1, ::std::sync::atomic::Ordering::Relaxed)
        }

        fn intern_map() -> &'static dashmap::DashMap<#intern_key_ty, u32> {
          static MAP: ::std::sync::OnceLock<dashmap::DashMap<#intern_key_ty, u32>> = ::std::sync::OnceLock::new();
          MAP.get_or_init(|| dashmap::DashMap::new())
        }

        #[allow(clippy::too_many_arguments)]
        pub fn new<DB: ::typedown_incremental::QueryDatabase + ?Sized>(db: &DB, #(#field_names: #field_types),*) -> Self {
          // Safety: transmute field values from 'db to 'static for storage
          let intern_key: #intern_key_ty = unsafe {
            ::std::mem::transmute((#(#field_names.clone(),)*))
          };
          let map = Self::intern_map();

          let id = if let Some(existing) = map.get(&intern_key) {
            *existing
          } else {
            let id = Self::next_id();
            *map.entry(intern_key.clone()).or_insert(id)
          };

          let storage = unsafe { db.storage() };
          let ingredient = (&*storage.interned[Self::ingredient_id() as usize] as &dyn ::std::any::Any)
            .downcast_ref::<::typedown_incremental::InternedIngredientStore<#intern_key_ty>>().expect("ingredient type mismatch");
          ingredient.data.entry(id).or_insert(::typedown_incremental::StampedInternedValue {
            value: intern_key,
            fingerprint: ::std::sync::OnceLock::new(),
          });

          #self_constructor
        }

        #getter_tokens
      }

      #impl_trait_prefix ::typedown_incremental::StableHash for #impl_trait_for {
        fn stable_hash<DB: ::typedown_incremental::QueryDatabase + ?Sized>(&self, db: &DB, hasher: &mut ::typedown_incremental::StableHasher) {
          #(
            Self::#field_names(*self, db).stable_hash(db, hasher);
          )*
        }
      }

      #impl_trait_prefix ::typedown_incremental::StableCompare for #impl_trait_for {
        fn stable_cmp<DB: ::typedown_incremental::QueryDatabase + ?Sized>(&self, db: &DB, other: &Self) -> ::std::cmp::Ordering {
          let _ = db;
          ::std::cmp::Ordering::Equal
          #(
            .then_with(|| Self::#field_names(*self, db).stable_cmp(db, &Self::#field_names(*other, db)))
          )*
        }
      }

      #impl_trait_prefix ::typedown_incremental::Encodable for #impl_trait_for {
        fn encode(&self, buf: &mut Vec<u8>, encoder: &mut ::typedown_incremental::Encoder) {
          let index = encoder.add_dep_id(::typedown_incremental::Id::as_id(self));
          encoder.emit_u32(buf, index);
          #(
            ::typedown_incremental::Encodable::field_encode(&Self::#field_names(*self, encoder.db()), buf, encoder);
          )*
        }
      }

      #impl_trait_prefix ::typedown_incremental::Decodable for #impl_trait_for {
        fn decode(data: &mut &[u8], decoder: &::typedown_incremental::Decoder) -> Self {
          let index = decoder.read_u32(data);
          #(
            let _ = <#field_types_static as ::typedown_incremental::Decodable>::field_decode(data, decoder);
          )*
          let dep_id = decoder.get_or_deserialize_dep_node_id(index)
            .expect("DepNodeIndex not found in decoder dep_id_table");
          Self::from(dep_id.entry_id())
        }
      }

      #impl_trait_prefix ::typedown_incremental::Id for #impl_trait_for {
        fn as_id(&self) -> ::typedown_incremental::DepId {
          ::typedown_incremental::DepId::new(
            ::typedown_incremental::IngredientKind::Interned,
            Self::ingredient_id(),
            self.0,
          )
        }
      }
      #impl_trait_prefix From<u32> for #impl_trait_for {
        fn from(id: u32) -> Self { #self_constructor }
      }
      #impl_trait_prefix From<#impl_trait_for> for u32 {
        fn from(val: #impl_trait_for) -> u32 { val.0 }
      }

      #impl_trait_prefix ::typedown_incremental::InternedId for #impl_trait_for {
        fn iter<DB: ::typedown_incremental::QueryDatabase + ?Sized>(db: &DB) -> Vec<Self> {
          let storage = unsafe { db.storage() };
          let ingredient = &storage.interned[Self::ingredient_id() as usize];
          #iter_map
        }
      }

      #[cfg(debug_assertions)]
      const _: () = <#debug_check_ty as ::typedown_incremental::InternedId>::__TYPEDOWN_INTERNED_ID;
    }
    .into(),
  );

  output
}
