//! Macros for the salsa database layer in typedown-db

use proc_macro::TokenStream;
use quote::quote;
use syn::{ItemFn, ItemStruct};

pub fn query_db_impl(_attr: TokenStream, item: TokenStream) -> TokenStream {
  // Only a struct can be decorated
  let struct_ast = match syn::parse::<ItemStruct>(item) {
    Ok(ast) => ast,
    Err(err) => return err.to_compile_error().into(),
  };

  let struct_name = &struct_ast.ident;

  // Require the struct to have a `storage` field
  let storage_field = struct_ast
    .fields
    .iter()
    .find(|field| field.ident.as_ref().is_some_and(|name| name == "storage"));

  // Get the type the user wrote for the `storage` field
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

    // Compile-time check: the `storage` field must be `::typedown_incremental::QueryStorage`
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

pub fn query_input_impl(_attr: TokenStream, item: TokenStream) -> TokenStream {
  // Only a struct can be decorated
  let struct_ast = match syn::parse::<ItemStruct>(item) {
    Ok(ast) => ast,
    Err(err) => return err.to_compile_error().into(),
  };

  let visibility = &struct_ast.vis;
  let struct_name = &struct_ast.ident;

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
    let field_ty = &field.ty;
    // Validate that every field is Send + Sync + Clone
    output.extend::<TokenStream>(
      quote! {
        const _: () = {
          const fn assert_send<T: Send>() {}
          const fn assert_sync<T: Sync>() {}
          const fn assert_clone<T: Clone>() {}
          assert_send::<#field_ty>();
          assert_sync::<#field_ty>();
          assert_clone::<#field_ty>();

          // Validate that InputFieldIngredient is what it is supposed to be
          #[cfg(debug_assertions)]
          const _: () = <::typedown_incremental::InputFieldIngredient<#field_ty>>::__TYPEDOWN_INPUT_FIELD_INGREDIENT;

          // Validate that QueryStorage is what it is supposed to be
          #[cfg(debug_assertions)]
          const _: () = ::typedown_incremental::QueryStorage::__TYPEDOWN_QUERY_STORAGE;
        };
      }
      .into(),
    );
  }

  let field_types: Vec<_> = fields.iter().map(|f| &f.ty).collect();
  let field_names: Vec<_> = fields.iter().map(|f| f.ident.as_ref().unwrap()).collect();
  let field_indices: Vec<_> = (0..fields.len()).collect();

  let struct_name_str = struct_name.to_string();

  // Generate ingredients for all fields
  output.extend::<TokenStream>(
    quote! {
      ::inventory::submit! {
        ::typedown_incremental::Inventory {
          register: |factories| {
            let start_index = factories.len();
            #(
              factories.push(|index| {
                ::typedown_incremental::IngredientEntry {
                  ingredient: Box::new(::typedown_incremental::InputFieldIngredient::<#field_types>::new(
                    index,
                    #struct_name_str,
                    #field_indices as u8,
                    #struct_name::id_counter(),
                  )),
                  field_index: Some(#field_indices as u8),
                }
              });
            )*
            #struct_name::set_ingredient_start_index(start_index);
          },
        }
      }
    }
    .into(),
  );

  // Generate getters and setters
  let mut getter_setter_tokens = quote! {};
  for (idx, field) in fields.iter().enumerate() {
    let field_name = field.ident.as_ref().unwrap();
    let field_ty = &field.ty;
    let setter_name = quote::format_ident!("set_{}", field_name);

    getter_setter_tokens.extend(quote! {
      pub fn #field_name<DB: ::typedown_incremental::QueryDatabase + ?Sized>(&self, db: &DB) -> #field_ty {
        let storage = unsafe { db.storage() };
        let ingredient_index = Self::ingredient_start_index() + #idx;
        let ingredient = (&*storage.ingredients[ingredient_index].ingredient as &dyn ::std::any::Any)
          .downcast_ref::<::typedown_incremental::InputFieldIngredient<#field_ty>>().expect("ingredient type mismatch");
        let entry = ingredient.data.get(&self.0).expect("invalid input id");

        // Record dependency if inside a derived query
        storage.with_context(|ctx| {
          if let Some(ctx) = ctx {
            ctx.dependencies.push(::typedown_incremental::Dependency {
              ingredient_index,
              arg_id: self.0,
              changed_at: entry.changed_at,
            });
          }
        });

        entry.value.clone()
      }

      pub fn #setter_name<DB: ::typedown_incremental::QueryDatabase + ?Sized>(&self, db: &mut DB, value: #field_ty) {
        let storage = unsafe { db.storage() };
        let ingredient = (&*storage.ingredients[Self::ingredient_start_index() + #idx].ingredient as &dyn ::std::any::Any)
          .downcast_ref::<::typedown_incremental::InputFieldIngredient<#field_ty>>().expect("ingredient type mismatch");
        let mut entry = ingredient.data.get_mut(&self.0).expect("invalid input id");
        if entry.value.eq(&value) {
          return; // Old value is new value, nothing to do here
        }

        // We don't need to lock here
        // We expect that the Rust borrow checker would only allow one &mut db while no other &db is present
        // We just want a race-free revision counter here to signal "staleness" to later reads
        let new_revision = storage.revision.fetch_add(1, ::std::sync::atomic::Ordering::Release) + 1;
        let stamped = entry.value_mut();
        stamped.value = value;
        stamped.changed_at = new_revision;
      }
    });
  }

  output.extend::<TokenStream>(
    quote! {
      #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
      #visibility struct #struct_name(usize);

      impl #struct_name {
        fn ingredient_start_index_lock() -> &'static ::std::sync::OnceLock<usize> {
          static START_INDEX: ::std::sync::OnceLock<usize> = ::std::sync::OnceLock::new();
          &START_INDEX
        }

        fn ingredient_start_index() -> usize {
          *Self::ingredient_start_index_lock().get()
            .expect("ingredient not registered; was QueryStorage initialized?")
        }

        #[doc(hidden)]
        pub fn set_ingredient_start_index(index: usize) {
          let _ = Self::ingredient_start_index_lock().set(index);
        }

        fn id_counter() -> &'static ::std::sync::atomic::AtomicUsize {
          static COUNTER: ::std::sync::atomic::AtomicUsize = ::std::sync::atomic::AtomicUsize::new(0);
          &COUNTER
        }

        fn next_id() -> usize {
          Self::id_counter().fetch_add(1, ::std::sync::atomic::Ordering::Relaxed)
        }

        #[allow(clippy::too_many_arguments)]
        pub fn new<DB: ::typedown_incremental::QueryDatabase + ?Sized>(db: &DB, #(#field_names: #field_types),*) -> Self {
          let storage = unsafe { db.storage() };
          let id = Self::next_id();
          let start_index = Self::ingredient_start_index();

          let current_revision = storage.revision.load(::std::sync::atomic::Ordering::Acquire);
          #(
            {
              let ingredient = (&*storage.ingredients[start_index + #field_indices].ingredient as &dyn ::std::any::Any)
                .downcast_ref::<::typedown_incremental::InputFieldIngredient<#field_types>>().expect("ingredient type mismatch");
              ingredient.data.insert(id, ::typedown_incremental::StampedInputField {
                value: #field_names,
                changed_at: current_revision,
              });
            }
          )*

          Self(id)
        }

        #getter_setter_tokens
      }

      impl ::typedown_incremental::StableHash for #struct_name {
        fn stable_hash<DB: ::typedown_incremental::QueryDatabase + ?Sized>(&self, db: &DB, hasher: &mut ::typedown_incremental::StableHasher) {
          #(
            self.#field_names(db).stable_hash(db, hasher);
          )*
        }
      }

      impl ::typedown_incremental::StableCompare for #struct_name {
        fn stable_cmp<DB: ::typedown_incremental::QueryDatabase + ?Sized>(&self, db: &DB, other: &Self) -> ::std::cmp::Ordering {
          let _ = db;
          ::std::cmp::Ordering::Equal
          #(
            .then_with(|| self.#field_names(db).stable_cmp(db, &other.#field_names(db)))
          )*
        }
      }

      impl ::typedown_incremental::Encodable for #struct_name {
        fn encode(&self, buf: &mut Vec<u8>, encoder: &mut ::typedown_incremental::Encoder) {
          let index = encoder.add_dep_id(::typedown_incremental::Id::as_id(self));
          encoder.emit_u32(buf, index);
          #(
            ::typedown_incremental::FieldEncodable::encode_field(&self.#field_names(encoder.db()), buf, encoder);
          )*
        }
      }

      impl ::typedown_incremental::Decodable for #struct_name {
        fn decode(data: &mut &[u8], decoder: &::typedown_incremental::Decoder) -> Self {
          let index = decoder.read_u32(data);
          #(
            let _ = <#field_types as ::typedown_incremental::FieldDecodable>::decode_field(data, decoder);
          )*
          let dep_id = decoder.get_or_deserialize_dep_node_id(index)
            .expect("DepNodeIndex not found in decoder dep_id_table");
          Self::from(dep_id.1)
        }
      }

      impl ::typedown_incremental::Id for #struct_name {
        fn as_id(&self) -> (usize, usize) { (Self::ingredient_start_index(), self.0) }
      }
      impl From<usize> for #struct_name {
        fn from(id: usize) -> Self { Self(id) }
      }
      impl From<#struct_name> for usize {
        fn from(val: #struct_name) -> usize { val.0 }
      }

      impl ::typedown_incremental::InputId for #struct_name {
        fn iter<DB: ::typedown_incremental::QueryDatabase + ?Sized>(db: &DB) -> Vec<Self> {
          let storage = unsafe { db.storage() };
          let ingredient = &storage.ingredients[Self::ingredient_start_index()].ingredient;
          ingredient.entry_ids().map(Self).collect()
        }
      }

      #[cfg(debug_assertions)]
      const _: () = <#struct_name as ::typedown_incremental::InputId>::__TYPEDOWN_INPUT_ID;
    }
    .into(),
  );

  output
}

pub fn query_derived_impl(_attr: TokenStream, item: TokenStream) -> TokenStream {
  // Try parsing as a function first, then as a struct
  if let Ok(func) = syn::parse::<ItemFn>(item.clone()) {
    return query_derived_fn_impl(func);
  }

  if let Ok(struct_ast) = syn::parse::<ItemStruct>(item.clone()) {
    return query_derived_struct_impl(struct_ast);
  }
  syn::Error::new(
    proc_macro::Span::call_site().into(),
    "#[query_derived] can only be applied to a function or a struct",
  )
  .to_compile_error()
  .into()
}

fn query_derived_fn_impl(func: ItemFn) -> TokenStream {
  let visibility = &func.vis;
  let fn_name = &func.sig.ident;
  let fn_block = &func.block;
  let return_type = match &func.sig.output {
    syn::ReturnType::Type(_, ty) => ty.as_ref(),
    syn::ReturnType::Default => {
      return syn::Error::new_spanned(&func.sig, "derived query must have a return type")
        .to_compile_error()
        .into();
    }
  };

  // Extract arguments: first arg is &db, rest are keys
  let all_args: Vec<_> = func.sig.inputs.iter().collect();
  if all_args.is_empty() {
    return syn::Error::new_spanned(&func.sig, "derived query must take &db as first argument")
      .to_compile_error()
      .into();
  }

  // Skip the first arg (db), collect the rest as key params
  let key_args: Vec<_> = all_args[1..].to_vec();
  let key_names: Vec<_> = key_args
    .iter()
    .filter_map(|arg| {
      if let syn::FnArg::Typed(pat_type) = arg
        && let syn::Pat::Ident(pat_ident) = pat_type.pat.as_ref()
      {
        return Some(&pat_ident.ident);
      }
      None
    })
    .collect();
  let key_types: Vec<_> = key_args
    .iter()
    .filter_map(|arg| {
      if let syn::FnArg::Typed(pat_type) = arg {
        return Some(pat_type.ty.as_ref());
      }
      None
    })
    .collect();

  let key_tuple_ty = quote! { (#(#key_types,)*) };

  // The db argument (first arg)
  let db_arg = &all_args[0];

  // Extract the db type (e.g. `Database` from `db: &Database`)
  let db_type = if let syn::FnArg::Typed(pat_type) = db_arg {
    if let syn::Type::Reference(type_ref) = pat_type.ty.as_ref() {
      type_ref.elem.as_ref().clone()
    } else {
      return syn::Error::new_spanned(db_arg, "first argument must be a reference to a database")
        .to_compile_error()
        .into();
    }
  } else {
    return syn::Error::new_spanned(db_arg, "first argument must be a typed parameter")
      .to_compile_error()
      .into();
  };

  let mut output: TokenStream = quote! {}.into();

  // Generate marker struct with ingredient index management
  output.extend::<TokenStream>(
    quote! {
      // TIL: Originally, i used a unit struct instead of record-like struct
      // Thinking that a struct would not collide with a function with the same name, as they are
      // in different namespaces
      // However, unit structs create both a value and a type (cause you can use a unit struct name
      // to represent the singleton value)
      #[allow(non_camel_case_types)]
      #visibility struct #fn_name { private: () }

      impl #fn_name {
        fn ingredient_index_lock() -> &'static ::std::sync::OnceLock<usize> {
          static INDEX: ::std::sync::OnceLock<usize> = ::std::sync::OnceLock::new();
          &INDEX
        }

        fn ingredient_index() -> usize {
          *Self::ingredient_index_lock().get()
            .expect("derived query ingredient not registered; was QueryStorage initialized?")
        }

        #[doc(hidden)]
        pub fn set_ingredient_index(index: usize) {
          let _ = Self::ingredient_index_lock().set(index);
        }

        /// The bare query implementation
        fn #fn_name(db: &#db_type, key: #key_tuple_ty) -> #return_type {
          let (#(#key_names,)*) = key;
          #fn_block
        }
      }
    }
    .into(),
  );

  // Register derived query ingredient via inventory
  output.extend::<TokenStream>(
    quote! {
      ::inventory::submit! {
        ::typedown_incremental::Inventory {
          register: |factories| {
            let index = factories.len();
            factories.push(|index| ::typedown_incremental::IngredientEntry {
              ingredient: Box::new(
                ::typedown_incremental::DerivedQueryIngredient::<#db_type, #key_tuple_ty, #return_type>::new(index, stringify!(#fn_name), stringify!(#return_type), #return_type::id_counter(), #fn_name::#fn_name),
              ),
              field_index: None,
            });
            #fn_name::set_ingredient_index(index);
          },
        }
      }
    }
    .into(),
  );

  // Generate the public wrapper that calls execute_query
  output.extend::<TokenStream>(
    quote! {
      #visibility fn #fn_name(#db_arg, #(#key_names: #key_types),*) -> #return_type {
        let storage = unsafe { db.storage() };
        let ingredient = (&*storage.ingredients[#fn_name::ingredient_index()].ingredient as &dyn ::std::any::Any)
          .downcast_ref::<::typedown_incremental::DerivedQueryIngredient<#db_type, #key_tuple_ty, #return_type>>()
          .expect("derived ingredient type mismatch");
        ingredient.execute_query(db, (#(#key_names,)*))
      }
    }
    .into(),
  );

  output
}

fn query_derived_struct_impl(struct_ast: ItemStruct) -> TokenStream {
  let visibility = &struct_ast.vis;
  let struct_name = &struct_ast.ident;

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

  // Validate each field
  for field in &fields {
    let field_ty = &field.ty;

    // All fields must be Send + Sync + Clone
    output.extend::<TokenStream>(
      quote! {
        const _: () = {
          const fn assert_send<T: Send>() {}
          const fn assert_sync<T: Sync>() {}
          const fn assert_clone<T: Clone>() {}
          assert_send::<#field_ty>();
          assert_sync::<#field_ty>();
          assert_clone::<#field_ty>();

          #[cfg(debug_assertions)]
          const _: () = ::typedown_incremental::QueryStorage::__TYPEDOWN_QUERY_STORAGE;
        };
      }
      .into(),
    );
  }

  // Ensure at least one field ingredient exists for serialization
  let has_phantom = fields.is_empty();
  let internal_field_types: Vec<syn::Type> = if has_phantom {
    vec![syn::parse_quote! { () }]
  } else {
    fields.iter().map(|f| f.ty.clone()).collect()
  };

  let field_types: Vec<_> = fields.iter().map(|f| &f.ty).collect();
  let field_names: Vec<_> = fields.iter().map(|f| f.ident.as_ref().unwrap()).collect();
  let first_field_ty = internal_field_types.first().cloned();

  let id_fields: Vec<_> = fields
    .iter()
    .enumerate()
    .filter(|(_, field)| field.attrs.iter().any(|attr| attr.path().is_ident("id")))
    .collect();
  let id_field_tys: Vec<_> = id_fields
    .iter()
    .map(|(_, field)| field.ty.clone())
    .collect();
  let id_field_names: Vec<_> = id_fields
    .iter()
    .map(|(_, field)| field.ident.as_ref().unwrap())
    .collect();
  let identity_ty = quote! {((usize, usize), usize, (#(#id_field_tys,)*) )};

  // Register per-field ingredients via inventory
  let struct_name_str = struct_name.to_string();
  let mut register_tokens = quote! {};
  for (idx, field_ty) in internal_field_types.iter().enumerate() {
    let identity_map_expr = if idx == 0 {
      quote! {
        Some(::std::sync::Arc::new(dashmap::DashMap::<#identity_ty, usize>::new())
          as ::std::sync::Arc<dyn ::std::any::Any + Send + Sync>)
      }
    } else {
      quote! { None }
    };
    register_tokens.extend(quote! {
      factories.push(|index| {
        ::typedown_incremental::IngredientEntry {
          ingredient: Box::new(::typedown_incremental::DerivedFieldIngredient::<#field_ty>::new(
            index,
            #struct_name_str,
            #idx as u8,
            #struct_name::id_counter(),
            #identity_map_expr,
          )),
          field_index: Some(#idx as u8),
        }
      });
    });
  }
  output.extend::<TokenStream>(
    quote! {
      ::inventory::submit! {
        ::typedown_incremental::Inventory {
          register: |factories| {
            let start_index = factories.len();
            #register_tokens
            #struct_name::set_ingredient_start_index(start_index);
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

    getter_tokens.extend(quote! {
      pub fn #field_name<DB: ::typedown_incremental::QueryDatabase + ?Sized>(&self, db: &DB) -> #field_ty {
        let storage = unsafe { db.storage() };
        let ingredient_index = Self::ingredient_start_index() + #idx;
        let ingredient = (&*storage.ingredients[ingredient_index].ingredient as &dyn ::std::any::Any)
          .downcast_ref::<::typedown_incremental::DerivedFieldIngredient<#field_ty>>().expect("ingredient type mismatch");
        let entry = ingredient.data.get(&self.0).expect("invalid derived id");

        // Record dependency if inside a derived query
        storage.with_context(|ctx| {
          if let Some(ctx) = ctx {
            ctx.dependencies.push(::typedown_incremental::Dependency {
              ingredient_index,
              arg_id: self.0,
              changed_at: entry.changed_at,
            });
          }
        });

        entry.value.clone()
      }
    });
  }

  let identity_map_lookup_tokens = if let Some(fft) = first_field_ty {
    quote! {
      let first_ingredient = (&*storage.ingredients[start_index].ingredient as &dyn ::std::any::Any)
        .downcast_ref::<::typedown_incremental::DerivedFieldIngredient<#fft>>()
        .expect("ingredient type mismatch");
      let map = first_ingredient.identity_map.as_ref()
        .expect("first field ingredient must have identity_map")
        .downcast_ref::<dashmap::DashMap<#identity_ty, usize>>()
        .expect("identity_map type mismatch");

      let id = if let Some(existing) = map.get(&identity) {
        *existing
      } else {
        let new_id = Self::next_id();
        *map.entry(identity).or_insert(new_id)
      };
    }
  } else {
    unreachable!("all derived structs have at least one internal field (phantom)")
  };

  let mut new_body_tokens = quote! {};

  let phantom_encode_tokens = if has_phantom {
    quote! { ::typedown_incremental::Encodable::encode(&(), buf, encoder); }
  } else {
    quote! {}
  };
  let phantom_decode_tokens = if has_phantom {
    quote! { let _ = <() as ::typedown_incremental::Decodable>::decode(data, decoder); }
  } else {
    quote! {}
  };

  // Insert phantom field value for zero-field structs
  if has_phantom {
    new_body_tokens.extend(quote! {
      {
        let ingredient = (&*storage.ingredients[start_index].ingredient as &dyn ::std::any::Any)
          .downcast_ref::<::typedown_incremental::DerivedFieldIngredient<()>>().expect("ingredient type mismatch");
        ingredient.data.entry(id).or_insert(::typedown_incremental::StampedDerivedField {
          value: (),
          changed_at: current_revision,
        });
      }
    });
  }

  for (idx, field) in fields.iter().enumerate() {
    let field_name = field.ident.as_ref().unwrap();
    let field_ty = &field.ty;

    new_body_tokens.extend(quote! {
      {
        let ingredient = (&*storage.ingredients[start_index + #idx].ingredient as &dyn ::std::any::Any)
          .downcast_ref::<::typedown_incremental::DerivedFieldIngredient<#field_ty>>().expect("ingredient type mismatch");
        // Backdate: only update changed_at if the value actually changed
        if let Some(existing) = ingredient.data.get(&id) {
          if existing.value == #field_name.clone() {
            // Value unchanged, keep old changed_at (backdating)
          } else {
            drop(existing);
            ingredient.data.insert(id, ::typedown_incremental::StampedDerivedField {
              value: #field_name.clone(),
              changed_at: current_revision,
            });
          }
        } else {
          ingredient.data.insert(id, ::typedown_incremental::StampedDerivedField {
            value: #field_name.clone(),
            changed_at: current_revision,
          });
        }
      }
    });
  }

  output.extend::<TokenStream>(
    quote! {
      // Derived struct is bound to an immutable database
      #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
      #visibility struct #struct_name<'db>(usize, ::std::marker::PhantomData<&'db (dyn ::typedown_incremental::QueryDatabase + Sync + Send)>);

      impl<'db> #struct_name<'db> {
        fn ingredient_start_index_lock() -> &'static ::std::sync::OnceLock<usize> {
          static START_INDEX: ::std::sync::OnceLock<usize> = ::std::sync::OnceLock::new();
          &START_INDEX
        }

        fn ingredient_start_index() -> usize {
          *Self::ingredient_start_index_lock().get()
            .expect("ingredient not registered; was QueryStorage initialized?")
        }

        #[doc(hidden)]
        pub fn set_ingredient_start_index(index: usize) {
          let _ = Self::ingredient_start_index_lock().set(index);
        }

        #[doc(hidden)]
        pub fn id_counter() -> &'static ::std::sync::atomic::AtomicUsize {
          static COUNTER: ::std::sync::atomic::AtomicUsize = ::std::sync::atomic::AtomicUsize::new(0);
          &COUNTER
        }

        fn next_id() -> usize {
          Self::id_counter().fetch_add(1, ::std::sync::atomic::Ordering::Relaxed)
        }

        /// Create or update a derived struct by identity
        /// If a struct with the same identity already exists, reuses its ID and updates fields in place
        #[allow(clippy::too_many_arguments)]
        pub fn new<DB: ::typedown_incremental::QueryDatabase + ?Sized>(db: &'db DB, #(#field_names: #field_types),*) -> Self {
          let storage = unsafe { db.storage() };
          let start_index = Self::ingredient_start_index();
          let current_revision = storage.revision.load(::std::sync::atomic::Ordering::Acquire);

          // Compute disambiguator scoped to the creating query
          let identity_hash = {
            use ::std::hash::{Hash, Hasher};
            let mut hasher = std::collections::hash_map::DefaultHasher::new();
            start_index.hash(&mut hasher);
            storage.current_query_identity().hash(&mut hasher);
            #(#id_field_names.hash(&mut hasher);)*
            hasher.finish()
          };
          let disambiguator = storage.next_disambiguator(identity_hash);
          let creating_query = storage.current_query_identity();

          let identity = (creating_query, disambiguator, (#(#id_field_names.clone(),)*));
          #identity_map_lookup_tokens

          #new_body_tokens

          Self(id, ::std::marker::PhantomData)
        }

        #getter_tokens
      }

      impl<'db> ::typedown_incremental::StableHash for #struct_name<'db> {
        fn stable_hash<DB: ::typedown_incremental::QueryDatabase + ?Sized>(&self, db: &DB, hasher: &mut ::typedown_incremental::StableHasher) {
          #(
            self.#field_names(db).stable_hash(db, hasher);
          )*
        }
      }

      impl<'db> ::typedown_incremental::StableCompare for #struct_name<'db> {
        fn stable_cmp<DB: ::typedown_incremental::QueryDatabase + ?Sized>(&self, db: &DB, other: &Self) -> ::std::cmp::Ordering {
          let _ = db;
          ::std::cmp::Ordering::Equal
          #(
            .then_with(|| self.#field_names(db).stable_cmp(db, &other.#field_names(db)))
          )*
        }
      }

      impl<'db> ::typedown_incremental::Encodable for #struct_name<'db> {
        fn encode(&self, buf: &mut Vec<u8>, encoder: &mut ::typedown_incremental::Encoder) {
          let index = encoder.add_dep_id(::typedown_incremental::Id::as_id(self));
          encoder.emit_u32(buf, index);
          #(
            ::typedown_incremental::FieldEncodable::encode_field(&self.#field_names(encoder.db()), buf, encoder);
          )*
          #phantom_encode_tokens
        }
      }

      impl<'db> ::typedown_incremental::Decodable for #struct_name<'db> {
        fn decode(data: &mut &[u8], decoder: &::typedown_incremental::Decoder) -> Self {
          let index = decoder.read_u32(data);
          #(
            let _ = <#field_types as ::typedown_incremental::FieldDecodable>::decode_field(data, decoder);
          )*
          #phantom_decode_tokens
          let dep_id = decoder.get_or_deserialize_dep_node_id(index)
            .expect("DepNodeIndex not found in decoder dep_id_table");
          Self::from(dep_id.1)
        }
      }

      impl<'db> ::typedown_incremental::Id for #struct_name<'db> {
        fn as_id(&self) -> (usize, usize) { (Self::ingredient_start_index(), self.0) }
      }
      impl<'db> From<usize> for #struct_name<'db> {
        fn from(id: usize) -> Self { Self(id, ::std::marker::PhantomData) }
      }
      impl<'db> From<#struct_name<'db>> for usize {
        fn from(val: #struct_name<'db>) -> usize { val.0 }
      }

      impl<'db> ::typedown_incremental::DerivedId for #struct_name<'db> {}

      #[cfg(debug_assertions)]
      const _: () = <#struct_name<'static> as ::typedown_incremental::DerivedId>::__TYPEDOWN_DERIVED_ID;
    }
    .into(),
  );

  output
}

pub fn query_interned_impl(_attr: TokenStream, item: TokenStream) -> TokenStream {
  let struct_ast = match syn::parse::<ItemStruct>(item) {
    Ok(ast) => ast,
    Err(err) => return err.to_compile_error().into(),
  };

  let visibility = &struct_ast.vis;
  let struct_name = &struct_ast.ident;

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
    let field_ty = &field.ty;
    output.extend::<TokenStream>(
      quote! {
        const _: () = {
          const fn assert_send<T: Send>() {}
          const fn assert_sync<T: Sync>() {}
          const fn assert_clone<T: Clone>() {}
          const fn assert_hash<T: ::std::hash::Hash>() {}
          const fn assert_eq<T: Eq>() {}
          assert_send::<#field_ty>();
          assert_sync::<#field_ty>();
          assert_clone::<#field_ty>();
          assert_hash::<#field_ty>();
          assert_eq::<#field_ty>();

          #[cfg(debug_assertions)]
          const _: () = ::typedown_incremental::QueryStorage::__TYPEDOWN_QUERY_STORAGE;
        };
      }
      .into(),
    );
  }

  let field_types: Vec<_> = fields.iter().map(|f| &f.ty).collect();
  let field_names: Vec<_> = fields.iter().map(|f| f.ident.as_ref().unwrap()).collect();
  let intern_key_ty = quote! { (#(#field_types,)*) };

  // Register a single InternedIngredient for the whole struct
  output.extend::<TokenStream>(
    quote! {
      ::inventory::submit! {
        ::typedown_incremental::Inventory {
          register: |factories| {
            let index = factories.len();
            factories.push(|index| ::typedown_incremental::IngredientEntry {
              ingredient: Box::new(::typedown_incremental::InternedIngredient::<#intern_key_ty>::new(
                index,
                stringify!(#struct_name),
                #struct_name::id_counter(),
                #struct_name::intern_map(),
              )),
              field_index: None,
            });
            #struct_name::set_ingredient_index(index);
          },
        }
      }
    }
    .into(),
  );

  // Generate getters that access fields from the interned tuple
  let mut getter_tokens = quote! {};
  for (idx, field) in fields.iter().enumerate() {
    let field_name = field.ident.as_ref().unwrap();
    let field_ty = &field.ty;
    let tuple_index = syn::Index::from(idx);

    getter_tokens.extend(quote! {
      pub fn #field_name<DB: ::typedown_incremental::QueryDatabase + ?Sized>(&self, db: &DB) -> #field_ty {
        let storage = unsafe { db.storage() };
        let ingredient_index = Self::ingredient_index();
        let ingredient = (&*storage.ingredients[ingredient_index].ingredient as &dyn ::std::any::Any)
          .downcast_ref::<::typedown_incremental::InternedIngredient<#intern_key_ty>>().expect("ingredient type mismatch");
        let entry = ingredient.data.get(&self.0).expect("invalid interned id");

        entry.#tuple_index.clone()
      }
    });
  }

  output.extend::<TokenStream>(
    quote! {
      #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
      #visibility struct #struct_name(usize);

      impl #struct_name {
        fn ingredient_index_lock() -> &'static ::std::sync::OnceLock<usize> {
          static INDEX: ::std::sync::OnceLock<usize> = ::std::sync::OnceLock::new();
          &INDEX
        }

        fn ingredient_index() -> usize {
          *Self::ingredient_index_lock().get()
            .expect("ingredient not registered; was QueryStorage initialized?")
        }

        #[doc(hidden)]
        pub fn set_ingredient_index(index: usize) {
          let _ = Self::ingredient_index_lock().set(index);
        }

        fn id_counter() -> &'static ::std::sync::atomic::AtomicUsize {
          static COUNTER: ::std::sync::atomic::AtomicUsize = ::std::sync::atomic::AtomicUsize::new(0);
          &COUNTER
        }

        fn next_id() -> usize {
          Self::id_counter().fetch_add(1, ::std::sync::atomic::Ordering::Relaxed)
        }

        fn intern_map() -> &'static dashmap::DashMap<#intern_key_ty, usize> {
          static MAP: ::std::sync::OnceLock<dashmap::DashMap<#intern_key_ty, usize>> = ::std::sync::OnceLock::new();
          MAP.get_or_init(|| dashmap::DashMap::new())
        }

        #[allow(clippy::too_many_arguments)]
        pub fn new<DB: ::typedown_incremental::QueryDatabase + ?Sized>(db: &DB, #(#field_names: #field_types),*) -> Self {
          let intern_key = (#(#field_names.clone(),)*);
          let map = Self::intern_map();

          let id = if let Some(existing) = map.get(&intern_key) {
            *existing
          } else {
            let id = Self::next_id();
            *map.entry(intern_key.clone()).or_insert(id)
          };

          // Always ensure data exists in the current storage
          let storage = unsafe { db.storage() };
          let ingredient = (&*storage.ingredients[Self::ingredient_index()].ingredient as &dyn ::std::any::Any)
            .downcast_ref::<::typedown_incremental::InternedIngredient<#intern_key_ty>>().expect("ingredient type mismatch");
          ingredient.data.entry(id).or_insert(intern_key);

          Self(id)
        }

        #getter_tokens
      }

      impl ::typedown_incremental::StableHash for #struct_name {
        fn stable_hash<DB: ::typedown_incremental::QueryDatabase + ?Sized>(&self, db: &DB, hasher: &mut ::typedown_incremental::StableHasher) {
          #(
            self.#field_names(db).stable_hash(db, hasher);
          )*
        }
      }

      impl ::typedown_incremental::StableCompare for #struct_name {
        fn stable_cmp<DB: ::typedown_incremental::QueryDatabase + ?Sized>(&self, db: &DB, other: &Self) -> ::std::cmp::Ordering {
          let _ = db;
          ::std::cmp::Ordering::Equal
          #(
            .then_with(|| self.#field_names(db).stable_cmp(db, &other.#field_names(db)))
          )*
        }
      }

      impl ::typedown_incremental::Encodable for #struct_name {
        fn encode(&self, buf: &mut Vec<u8>, encoder: &mut ::typedown_incremental::Encoder) {
          let index = encoder.add_dep_id(::typedown_incremental::Id::as_id(self));
          encoder.emit_u32(buf, index);
          #(
            ::typedown_incremental::FieldEncodable::encode_field(&self.#field_names(encoder.db()), buf, encoder);
          )*
        }
      }

      impl ::typedown_incremental::Decodable for #struct_name {
        fn decode(data: &mut &[u8], decoder: &::typedown_incremental::Decoder) -> Self {
          let index = decoder.read_u32(data);
          #(
            let _ = <#field_types as ::typedown_incremental::FieldDecodable>::decode_field(data, decoder);
          )*
          let dep_id = decoder.get_or_deserialize_dep_node_id(index)
            .expect("DepNodeIndex not found in decoder dep_id_table");
          Self::from(dep_id.1)
        }
      }

      impl ::typedown_incremental::Id for #struct_name {
        fn as_id(&self) -> (usize, usize) { (Self::ingredient_index(), self.0) }
      }
      impl From<usize> for #struct_name {
        fn from(id: usize) -> Self { Self(id) }
      }
      impl From<#struct_name> for usize {
        fn from(val: #struct_name) -> usize { val.0 }
      }

      impl ::typedown_incremental::InternedId for #struct_name {
        fn iter<DB: ::typedown_incremental::QueryDatabase + ?Sized>(db: &DB) -> Vec<Self> {
          let storage = unsafe { db.storage() };
          let ingredient = &storage.ingredients[Self::ingredient_index()].ingredient;
          ingredient.entry_ids().map(Self).collect()
        }
      }

      #[cfg(debug_assertions)]
      const _: () = <#struct_name as ::typedown_incremental::InternedId>::__TYPEDOWN_INTERNED_ID;
    }
    .into(),
  );

  output
}
