use proc_macro::TokenStream;
use quote::quote;
use syn::{ItemFn, ItemStruct};

use super::{CacheModifiers, erase_db_lifetime_tokens, has_return_ref, parse_cache_modifiers};

pub fn query_derived_impl(attr: TokenStream, item: TokenStream) -> TokenStream {
  let modifiers = parse_cache_modifiers(attr);

  if let Ok(func) = syn::parse::<ItemFn>(item.clone()) {
    return query_derived_fn_impl(func, &modifiers);
  }

  if let Ok(struct_ast) = syn::parse::<ItemStruct>(item.clone()) {
    return query_derived_struct_impl(struct_ast, &modifiers);
  }
  syn::Error::new(
    proc_macro::Span::call_site().into(),
    "#[query_derived] can only be applied to a function or a struct",
  )
  .to_compile_error()
  .into()
}

fn query_derived_fn_impl(func: ItemFn, modifiers: &CacheModifiers) -> TokenStream {
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

  let return_type_segment = if let syn::Type::Path(type_path) = return_type {
    type_path.path.segments.last()
  } else {
    None
  };
  let Some(return_type_segment) = return_type_segment else {
    return syn::Error::new_spanned(return_type, "return type must be a named type")
      .to_compile_error()
      .into();
  };
  let return_type_without_lifetime = &return_type_segment.ident;
  // Check if the return type has a 'db lifetime (e.g. IdResult<'db> vs IdInput)
  let return_type_has_lifetime = matches!(
    &return_type_segment.arguments,
    syn::PathArguments::AngleBracketed(args) if args.args.iter().any(|arg| {
      matches!(arg, syn::GenericArgument::Lifetime(lt) if lt.ident == "db")
    })
  );
  // Static version of return type for storage (either Type<'static> or Type)
  let return_type_static = if return_type_has_lifetime {
    quote! { #return_type_without_lifetime<'static> }
  } else {
    quote! { #return_type_without_lifetime }
  };

  let has_db_lifetime = func
    .sig
    .generics
    .lifetimes()
    .any(|param| param.lifetime.ident == "db");
  if !has_db_lifetime {
    return syn::Error::new_spanned(
      &func.sig,
      "#[query_derived] function must be generic over 'db, e.g. fn foo<'db>(...)",
    )
    .to_compile_error()
    .into();
  }

  let all_args: Vec<_> = func.sig.inputs.iter().collect();
  if all_args.is_empty() {
    return syn::Error::new_spanned(&func.sig, "derived query must take &db as first argument")
      .to_compile_error()
      .into();
  }

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
  let key_tuple_ty_static = erase_db_lifetime_tokens(&key_tuple_ty);

  let db_arg = &all_args[0];

  let db_type = if let syn::FnArg::Typed(pat_type) = db_arg {
    if let syn::Type::Reference(type_ref) = pat_type.ty.as_ref() {
      let has_db_lifetime = type_ref
        .lifetime
        .as_ref()
        .is_some_and(|lifetime| lifetime.ident == "db");
      if !has_db_lifetime {
        return syn::Error::new_spanned(
          db_arg,
          "first argument must be &'db, e.g. db: &'db Database",
        )
        .to_compile_error()
        .into();
      }
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

  // Generate marker struct
  output.extend::<TokenStream>(
    quote! {
      #[allow(non_camel_case_types, clippy::useless_transmute)]
      #visibility struct #fn_name { private: () }

      #[allow(clippy::useless_transmute)]
      impl #fn_name {
        fn ingredient_id_lock() -> &'static ::std::sync::OnceLock<u32> {
          static INDEX: ::std::sync::OnceLock<u32> = ::std::sync::OnceLock::new();
          &INDEX
        }

        fn ingredient_id() -> u32 {
          *Self::ingredient_id_lock().get()
            .expect("derived query ingredient not registered; was QueryStorage initialized?")
        }

        #[doc(hidden)]
        pub fn set_ingredient_id(index: u32) {
          let _ = Self::ingredient_id_lock().set(index);
        }

        fn #fn_name<'db>(db: &'db #db_type, key: #key_tuple_ty_static) -> #return_type_static {
          fn __inner<'db>(db: &'db #db_type, #(#key_names: #key_types),*) -> #return_type
            #fn_block

          // Safety: transmute key from 'static to 'db, then result from 'db to 'static
          let (#(#key_names,)*): #key_tuple_ty = unsafe { ::std::mem::transmute(key) };
          unsafe { ::std::mem::transmute(__inner(db, #(#key_names),*)) }
        }
      }
    }
    .into(),
  );

  // Register derived query ingredient via QueryInventory
  let no_hash = modifiers.no_hash;
  output.extend::<TokenStream>(
    quote! {
      ::inventory::submit! {
        ::typedown_incremental::QueryInventory {
          register: |factories| {
            let index = factories.len() as u32;
            factories.push(|ingredient_id| {
              let mut ingredient = ::typedown_incremental::DerivedQueryIngredientStore::<
                #db_type,
                #key_tuple_ty_static,
                #return_type_static,
              >::new(
                ingredient_id,
                stringify!(#fn_name),
                stringify!(#return_type_without_lifetime),
                #fn_name::#fn_name,
              );
              ingredient.no_hash_flag = #no_hash;
              Box::new(ingredient)
            });
            #fn_name::set_ingredient_id(index);
          },
        }
      }
    }
    .into(),
  );

  // Generate the public wrapper that calls execute_query
  output.extend::<TokenStream>(
    quote! {
      #[allow(clippy::useless_transmute)]
      #visibility fn #fn_name<'db>(#db_arg, #(#key_names: #key_types),*) -> #return_type {
        let storage = unsafe { db.storage() };
        let ingredient = (&*storage.queries[#fn_name::ingredient_id() as usize] as &dyn ::std::any::Any)
          .downcast_ref::<::typedown_incremental::DerivedQueryIngredientStore<
            #db_type,
            #key_tuple_ty_static,
            #return_type_static,
          >>()
          .expect("derived ingredient type mismatch");
        // Safety: transmute key 'db -> 'static, then result 'static -> 'db
        let key: #key_tuple_ty_static = unsafe { ::std::mem::transmute((#(#key_names,)*)) };
        unsafe { ::std::mem::transmute(ingredient.execute_query(db, key)) }
      }
    }
    .into(),
  );

  output
}

fn query_derived_struct_impl(struct_ast: ItemStruct, modifiers: &CacheModifiers) -> TokenStream {
  let visibility = &struct_ast.vis;
  let struct_name = &struct_ast.ident;

  let has_db_lifetime = struct_ast
    .generics
    .lifetimes()
    .any(|param| param.lifetime.ident == "db");
  if !has_db_lifetime {
    return syn::Error::new_spanned(
      &struct_ast,
      "#[query_derived] struct must be generic over 'db, e.g. struct Foo<'db> { ... }",
    )
    .to_compile_error()
    .into();
  }

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
          assert_send::<#field_ty_static>();
          assert_sync::<#field_ty_static>();
          assert_clone::<#field_ty_static>();

          #[cfg(debug_assertions)]
          const _: () = ::typedown_incremental::QueryStorage::__TYPEDOWN_QUERY_STORAGE;
        };
      }
      .into(),
    );
  }

  let has_phantom = fields.is_empty();
  let internal_field_types: Vec<syn::Type> = if has_phantom {
    vec![syn::parse_quote! { () }]
  } else {
    fields.iter().map(|f| f.ty.clone()).collect()
  };
  let internal_field_types_static: Vec<proc_macro2::TokenStream> = internal_field_types
    .iter()
    .map(erase_db_lifetime_tokens)
    .collect();

  let field_types: Vec<_> = fields.iter().map(|f| &f.ty).collect();
  let field_types_static: Vec<proc_macro2::TokenStream> =
    field_types.iter().map(erase_db_lifetime_tokens).collect();
  let field_names: Vec<_> = fields.iter().map(|f| f.ident.as_ref().unwrap()).collect();
  let try_field_names: Vec<_> = field_names
    .iter()
    .map(|n| quote::format_ident!("try_{}", n))
    .collect();
  let id_fields: Vec<_> = fields
    .iter()
    .enumerate()
    .filter(|(_, field)| field.attrs.iter().any(|attr| attr.path().is_ident("id")))
    .collect();
  let id_field_tys: Vec<_> = id_fields
    .iter()
    .map(|(_, field)| field.ty.clone())
    .collect();
  let id_field_tys_static: Vec<proc_macro2::TokenStream> =
    id_field_tys.iter().map(erase_db_lifetime_tokens).collect();
  let id_field_names: Vec<_> = id_fields
    .iter()
    .map(|(_, field)| field.ident.as_ref().unwrap())
    .collect();
  let identity_ty = quote! {(u32, (#(#id_field_tys_static,)*) )};

  // Register per-field ingredients via FieldInventory
  let struct_name_str = struct_name.to_string();
  let no_hash = modifiers.no_hash;
  let mut register_tokens = quote! {};
  for (idx, field_ty) in internal_field_types_static.iter().enumerate() {
    register_tokens.extend(quote! {
      factories.push(|ingredient_id| {
        let mut ingredient = ::typedown_incremental::DerivedFieldIngredientStore::<#field_ty>::new(
          ingredient_id,
          #struct_name_str,
          #idx as u8,
          #struct_name::id_counter(),
        );
        ingredient.no_hash_flag = #no_hash;
        Box::new(ingredient)
      });
    });
  }
  output.extend::<TokenStream>(
    quote! {
      ::inventory::submit! {
        ::typedown_incremental::FieldInventory {
          register: |factories| {
            let start_index = factories.len() as u32;
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
    let field_ty_static = &field_types_static[idx];
    let try_field_name = quote::format_ident!("try_{}", field_name);
    let is_return_ref = has_return_ref(field);

    let getter = if is_return_ref {
      quote! {
        pub fn #field_name<'__db, DB: ::typedown_incremental::QueryDatabase + ?Sized>(self, db: &'__db DB) -> ::typedown_incremental::MappedRef<'__db, ::typedown_incremental::StampedDerivedField<#field_ty_static>, #field_ty> {
          let id = self.0;
          debug_assert!(id != ::typedown_incremental::TOMBSTONE_ENTRY_ID, "accessed evicted derived struct");
          let storage = unsafe { db.storage() };
          let ingredient_id = (Self::ingredient_start_index() + #idx as u32) as usize;
          let ingredient = (&*storage.fields[ingredient_id] as &dyn ::std::any::Any)
            .downcast_ref::<::typedown_incremental::DerivedFieldIngredientStore<#field_ty_static>>().expect("ingredient type mismatch");
          let entry = ingredient.data.get(&id).expect("invalid derived id");

          // Record dependency if inside a derived query
          let dep_id = ::typedown_incremental::DepId::new(
            ::typedown_incremental::IngredientKind::Field,
            ingredient_id as u32,
            id,
          );
          storage.with_context(|ctx| {
            if let Some(ctx) = ctx {
              ctx.dependencies.push(::typedown_incremental::Dependency {
                dep_id,
                changed_at: entry.changed_at,
              });
            }
          });

          // Safety: transmute 'static to 'db on the projected type (same as the clone path)
          unsafe { ::std::mem::transmute(::typedown_incremental::MappedRef::new(entry, |stamped| &stamped.value)) }
        }
      }
    } else {
      quote! {
        pub fn #field_name<DB: ::typedown_incremental::QueryDatabase + ?Sized>(self, db: &DB) -> #field_ty {
          let id = self.0;
          debug_assert!(id != ::typedown_incremental::TOMBSTONE_ENTRY_ID, "accessed evicted derived struct");
          let storage = unsafe { db.storage() };
          let ingredient_id = (Self::ingredient_start_index() + #idx as u32) as usize;
          let ingredient = (&*storage.fields[ingredient_id] as &dyn ::std::any::Any)
            .downcast_ref::<::typedown_incremental::DerivedFieldIngredientStore<#field_ty_static>>().expect("ingredient type mismatch");
          let entry = ingredient.data.get(&id).expect("invalid derived id");

          // Record dependency if inside a derived query
          let dep_id = ::typedown_incremental::DepId::new(
            ::typedown_incremental::IngredientKind::Field,
            ingredient_id as u32,
            id,
          );
          storage.with_context(|ctx| {
            if let Some(ctx) = ctx {
              ctx.dependencies.push(::typedown_incremental::Dependency {
                dep_id,
                changed_at: entry.changed_at,
              });
            }
          });

          // Safety: transmute 'static stored value to 'db at the boundary
          unsafe { ::std::mem::transmute(entry.value.clone()) }
        }
      }
    };

    getter_tokens.extend(getter);

    let try_getter = if is_return_ref {
      quote! {
        // Fallible getter for serialization paths where field data may have been cleaned up
        pub fn #try_field_name<'__db, DB: ::typedown_incremental::QueryDatabase + ?Sized>(self, db: &'__db DB) -> Option<::typedown_incremental::MappedRef<'__db, ::typedown_incremental::StampedDerivedField<#field_ty_static>, #field_ty>> {
          let id = self.0;
          if id == ::typedown_incremental::TOMBSTONE_ENTRY_ID {
            return None;
          }
          let storage = unsafe { db.storage() };
          let ingredient_id = (Self::ingredient_start_index() + #idx as u32) as usize;
          let ingredient = (&*storage.fields[ingredient_id] as &dyn ::std::any::Any)
            .downcast_ref::<::typedown_incremental::DerivedFieldIngredientStore<#field_ty_static>>().expect("ingredient type mismatch");
          let entry = ingredient.data.get(&id)?;

          // Safety: transmute 'static to 'db on the projected type (same as the clone path)
          Some(unsafe { ::std::mem::transmute(::typedown_incremental::MappedRef::new(entry, |stamped| &stamped.value)) })
        }
      }
    } else {
      quote! {
        // Fallible getter for serialization paths where field data may have been cleaned up
        pub fn #try_field_name<DB: ::typedown_incremental::QueryDatabase + ?Sized>(self, db: &DB) -> Option<#field_ty> {
          let id = self.0;
          if id == ::typedown_incremental::TOMBSTONE_ENTRY_ID {
            return None;
          }
          let storage = unsafe { db.storage() };
          let ingredient_id = (Self::ingredient_start_index() + #idx as u32) as usize;
          let ingredient = (&*storage.fields[ingredient_id] as &dyn ::std::any::Any)
            .downcast_ref::<::typedown_incremental::DerivedFieldIngredientStore<#field_ty_static>>().expect("ingredient type mismatch");
          let entry = ingredient.data.get(&id)?;

          // Safety: transmute 'static stored value to 'db at the boundary
          Some(unsafe { ::std::mem::transmute(entry.value.clone()) })
        }
      }
    };
    getter_tokens.extend(try_getter);
  }

  // Identity map lookup
  let identity_map_lookup_tokens = quote! {
    let id = storage.with_context(|ctx| {
      let ctx = ctx.as_mut()?;
      let store = ctx.identity_maps.as_ref()?;
      let parent_entry_id = ctx.query_stack.last().map(|e| e.dep_id.entry_id())?;
      let map_arc = store
        .entry((parent_entry_id, start_index))
        .or_insert_with(|| ::std::sync::Arc::new(dashmap::DashMap::<#identity_ty, u32>::new()))
        .clone();
      let map = (&*map_arc as &dyn ::std::any::Any)
        .downcast_ref::<dashmap::DashMap<#identity_ty, u32>>()
        .expect("identity_map type mismatch");
      let id = if let Some(existing) = map.get(&identity) {
        *existing
      } else {
        let new_id = Self::next_id();
        *map.entry(identity).or_insert(new_id)
      };
      ctx.created_ids.entry(start_index).or_default().insert(id);
      Some(id)
    }).unwrap_or_else(|| Self::next_id());
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

  if has_phantom {
    new_body_tokens.extend(quote! {
      {
        let ingredient = (&*storage.fields[start_index as usize] as &dyn ::std::any::Any)
          .downcast_ref::<::typedown_incremental::DerivedFieldIngredientStore<()>>().expect("ingredient type mismatch");
        ingredient.data.entry(id).or_insert(::typedown_incremental::StampedDerivedField {
          value: (),
          changed_at: current_revision,
          fingerprint: ::std::sync::OnceLock::new(),
        });
      }
    });
  }

  for (idx, field) in fields.iter().enumerate() {
    let field_name = field.ident.as_ref().unwrap();
    let field_ty_static = &field_types_static[idx];

    new_body_tokens.extend(quote! {
      {
        let ingredient = (&*storage.fields[(start_index + #idx as u32) as usize] as &dyn ::std::any::Any)
          .downcast_ref::<::typedown_incremental::DerivedFieldIngredientStore<#field_ty_static>>().expect("ingredient type mismatch");
        // Safety: transmute field value from 'db to 'static at storage boundary
        let __val: #field_ty_static = unsafe { ::std::mem::transmute(#field_name.clone()) };
        // Backdate: only update changed_at if the value actually changed
        if let Some(existing) = ingredient.data.get(&id) {
          if existing.value == __val {
            // Value unchanged, keep old changed_at (backdating)
          } else {
            drop(existing);
            ingredient.data.insert(id, ::typedown_incremental::StampedDerivedField {
              value: __val,
              changed_at: current_revision,
              fingerprint: ::std::sync::OnceLock::new(),
            });
          }
        } else {
          ingredient.data.insert(id, ::typedown_incremental::StampedDerivedField {
            value: __val,
            changed_at: current_revision,
            fingerprint: ::std::sync::OnceLock::new(),
          });
        }
      }
    });
  }

  // Skip generating StableHash for custom_hash structs (user provides their own)
  // Always use try_ for stable hash in case the value is evicted
  let stable_hash_impl = if modifiers.custom_hash {
    quote! {}
  } else {
    quote! {
      impl<'db> ::typedown_incremental::StableHash for #struct_name<'db> {
        fn stable_hash<DB: ::typedown_incremental::QueryDatabase + ?Sized>(&self, db: &DB, hasher: &mut ::typedown_incremental::StableHasher) {
          #(
            Self::#try_field_names(*self, db).stable_hash(db, hasher);
          )*
        }
      }
    }
  };

  output.extend::<TokenStream>(
    quote! {
      #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
      #visibility struct #struct_name<'db>(u32, ::std::marker::PhantomData<&'db (dyn ::typedown_incremental::QueryDatabase + Sync + Send)>);

      #[allow(clippy::useless_transmute)]
      impl<'db> #struct_name<'db> {
        fn ingredient_start_index_lock() -> &'static ::std::sync::OnceLock<u32> {
          static START_INDEX: ::std::sync::OnceLock<u32> = ::std::sync::OnceLock::new();
          &START_INDEX
        }

        fn ingredient_start_index() -> u32 {
          *Self::ingredient_start_index_lock().get()
            .expect("ingredient not registered; was QueryStorage initialized?")
        }

        #[doc(hidden)]
        pub fn set_ingredient_start_index(index: u32) {
          let _ = Self::ingredient_start_index_lock().set(index);
        }

        #[doc(hidden)]
        pub fn id_counter() -> &'static ::std::sync::atomic::AtomicU32 {
          static COUNTER: ::std::sync::atomic::AtomicU32 = ::std::sync::atomic::AtomicU32::new(0);
          &COUNTER
        }

        fn next_id() -> u32 {
          Self::id_counter().fetch_add(1, ::std::sync::atomic::Ordering::Relaxed)
        }

        /// Create or update a derived struct by identity
        #[allow(clippy::too_many_arguments)]
        pub fn new<DB: ::typedown_incremental::QueryDatabase + ?Sized>(db: &'db DB, #(#field_names: #field_types),*) -> Self {
          let storage = unsafe { db.storage() };
          debug_assert!(
            storage.is_in_query(),
            "cannot create a derived struct outside of a query function"
          );
          let start_index = Self::ingredient_start_index();
          let current_revision = storage.revision.load(::std::sync::atomic::Ordering::Acquire);

          // Compute disambiguator scoped to the creating query
          let identity_hash = {
            use ::std::hash::{Hash, Hasher};
            let mut hasher = std::collections::hash_map::DefaultHasher::new();
            start_index.hash(&mut hasher);
            storage.current_query_dep_id().hash(&mut hasher);
            #(#id_field_names.hash(&mut hasher);)*
            hasher.finish()
          };
          let disambiguator = storage.next_disambiguator(identity_hash);

          // Safety: erase 'db to 'static for the identity map key
          let identity: #identity_ty = unsafe {
            ::std::mem::transmute((disambiguator, (#(#id_field_names.clone(),)*)))
          };
          #identity_map_lookup_tokens

          #new_body_tokens

          Self(id, ::std::marker::PhantomData)
        }

        #getter_tokens
      }

      #stable_hash_impl

      impl<'db> ::typedown_incremental::StableCompare for #struct_name<'db> {
        fn stable_cmp<DB: ::typedown_incremental::QueryDatabase + ?Sized>(&self, db: &DB, other: &Self) -> ::std::cmp::Ordering {
          let _ = db;
          ::std::cmp::Ordering::Equal
          #(
            .then_with(|| Self::#try_field_names(*self, db).stable_cmp(db, &Self::#try_field_names(*other, db)))
          )*
        }
      }

      impl<'db> ::typedown_incremental::Encodable for #struct_name<'db> {
        fn encode(&self, buf: &mut Vec<u8>, encoder: &mut ::typedown_incremental::Encoder) {
          let index = encoder.add_dep_id(::typedown_incremental::Id::as_id(self));
          encoder.emit_u32(buf, index);
          #(
            ::typedown_incremental::Encodable::field_encode(&Self::#field_names(*self, encoder.db()), buf, encoder);
          )*
          #phantom_encode_tokens
        }

        fn field_encode(&self, buf: &mut Vec<u8>, encoder: &mut ::typedown_incremental::Encoder) {
          let index = encoder.add_dep_id(::typedown_incremental::Id::as_id(self));
          encoder.emit_u32(buf, index);
        }
      }

      impl<'db> ::typedown_incremental::Decodable for #struct_name<'db> {
        fn decode(data: &mut &[u8], decoder: &::typedown_incremental::Decoder) -> Self {
          let index = decoder.read_u32(data);
          #(
            let _ = <#field_types_static as ::typedown_incremental::Decodable>::field_decode(data, decoder);
          )*
          #phantom_decode_tokens
          let dep_id = decoder.get_or_deserialize_dep_node_id(index)
            .expect("DepNodeIndex not found in decoder dep_id_table");
          Self::from(dep_id.entry_id())
        }

        fn field_decode(data: &mut &[u8], decoder: &::typedown_incremental::Decoder) -> Self {
          let index = decoder.read_u32(data);
          let entry_id = decoder
            .get_or_deserialize_dep_node_id(index)
            .map(|dep_id| dep_id.entry_id())
            .unwrap_or(::typedown_incremental::TOMBSTONE_ENTRY_ID);
          Self::from(entry_id)
        }
      }

      impl<'db> ::typedown_incremental::Id for #struct_name<'db> {
        fn as_id(&self) -> ::typedown_incremental::DepId {
          ::typedown_incremental::DepId::new(
            ::typedown_incremental::IngredientKind::Field,
            Self::ingredient_start_index(),
            self.0,
          )
        }
      }
      impl<'db> From<u32> for #struct_name<'db> {
        fn from(id: u32) -> Self { Self(id, ::std::marker::PhantomData) }
      }
      impl<'db> From<#struct_name<'db>> for u32 {
        fn from(val: #struct_name<'db>) -> u32 { val.0 }
      }

      impl<'db> ::typedown_incremental::DerivedId for #struct_name<'db> {}

      #[cfg(debug_assertions)]
      const _: () = <#struct_name<'static> as ::typedown_incremental::DerivedId>::__TYPEDOWN_DERIVED_ID;
    }
    .into(),
  );

  output
}
