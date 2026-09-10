use proc_macro::TokenStream;
use quote::quote;
use syn::ItemStruct;

use super::{has_return_ref, parse_cache_modifiers};

pub fn query_input_impl(attr: TokenStream, item: TokenStream) -> TokenStream {
  let modifiers = parse_cache_modifiers(attr);
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
          assert_send::<#field_ty>();
          assert_sync::<#field_ty>();
          assert_clone::<#field_ty>();

          #[cfg(debug_assertions)]
          const _: () = <::typedown_incremental::InputIngredientStore<#field_ty>>::__TYPEDOWN_INPUT_FIELD_INGREDIENT;

          #[cfg(debug_assertions)]
          const _: () = ::typedown_incremental::QueryStorage::__TYPEDOWN_QUERY_STORAGE;
        };
      }
      .into(),
    );
  }

  let field_types: Vec<_> = fields.iter().map(|f| &f.ty).collect();
  let field_names: Vec<_> = fields.iter().map(|f| f.ident.as_ref().unwrap()).collect();
  let try_field_names: Vec<_> = field_names
    .iter()
    .map(|n| quote::format_ident!("try_{}", n))
    .collect();
  let field_indices: Vec<_> = (0..fields.len()).collect();

  let struct_name_str = struct_name.to_string();

  // Register input ingredients
  output.extend::<TokenStream>(
    quote! {
      ::inventory::submit! {
        ::typedown_incremental::InputInventory {
          register: |factories| {
            let start_index = factories.len() as u32;
            #(
              factories.push(|ingredient_id| {
                Box::new(::typedown_incremental::InputIngredientStore::<#field_types>::new(
                  ingredient_id,
                  #struct_name_str,
                  #field_indices as u8,
                  #struct_name::id_counter(),
                ))
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
    let try_field_name = quote::format_ident!("try_{}", field_name);
    let is_return_ref = has_return_ref(field);

    // Getter: returns MappedRef (deref to &T) for return_ref fields, T otherwise
    let getter = if is_return_ref {
      quote! {
        pub fn #field_name<'__db, DB: ::typedown_incremental::QueryDatabase + ?Sized>(&self, db: &'__db DB) -> ::typedown_incremental::MappedRef<'__db, ::typedown_incremental::StampedInputField<#field_ty>, #field_ty> {
          let storage = unsafe { db.storage() };
          let ingredient_id = (Self::ingredient_start_index() + #idx as u32) as usize;
          let ingredient = (&*storage.inputs[ingredient_id] as &dyn ::std::any::Any)
            .downcast_ref::<::typedown_incremental::InputIngredientStore<#field_ty>>().expect("ingredient type mismatch");
          let entry = ingredient.data.get(&self.0).expect("invalid input id");

          // Record dependency if inside a derived query
          let dep_id = ::typedown_incremental::DepId::new(
            ::typedown_incremental::IngredientKind::Input,
            ingredient_id as u32,
            self.0,
          );
          storage.with_context(|ctx| {
            if let Some(ctx) = ctx {
              ctx.dependencies.push(::typedown_incremental::Dependency {
                dep_id,
                changed_at: entry.changed_at,
              });
            }
          });

          ::typedown_incremental::MappedRef::new(entry, |stamped| &stamped.value)
        }
      }
    } else {
      quote! {
        pub fn #field_name<DB: ::typedown_incremental::QueryDatabase + ?Sized>(&self, db: &DB) -> #field_ty {
          let storage = unsafe { db.storage() };
          let ingredient_id = (Self::ingredient_start_index() + #idx as u32) as usize;
          let ingredient = (&*storage.inputs[ingredient_id] as &dyn ::std::any::Any)
            .downcast_ref::<::typedown_incremental::InputIngredientStore<#field_ty>>().expect("ingredient type mismatch");
          let entry = ingredient.data.get(&self.0).expect("invalid input id");

          // Record dependency if inside a derived query
          let dep_id = ::typedown_incremental::DepId::new(
            ::typedown_incremental::IngredientKind::Input,
            ingredient_id as u32,
            self.0,
          );
          storage.with_context(|ctx| {
            if let Some(ctx) = ctx {
              ctx.dependencies.push(::typedown_incremental::Dependency {
                dep_id,
                changed_at: entry.changed_at,
              });
            }
          });

          entry.value.clone()
        }
      }
    };

    getter_setter_tokens.extend(getter);

    let try_getter = if is_return_ref {
      quote! {
        pub fn #try_field_name<'__db, DB: ::typedown_incremental::QueryDatabase + ?Sized>(&self, db: &'__db DB) -> Option<::typedown_incremental::MappedRef<'__db, ::typedown_incremental::StampedInputField<#field_ty>, #field_ty>> {
          let storage = unsafe { db.storage() };
          let ingredient_id = (Self::ingredient_start_index() + #idx as u32) as usize;
          let ingredient = (&*storage.inputs[ingredient_id] as &dyn ::std::any::Any)
            .downcast_ref::<::typedown_incremental::InputIngredientStore<#field_ty>>().expect("ingredient type mismatch");
          Some(::typedown_incremental::MappedRef::new(ingredient.data.get(&self.0)?, |stamped| &stamped.value))
        }
      }
    } else {
      quote! {
        pub fn #try_field_name<DB: ::typedown_incremental::QueryDatabase + ?Sized>(&self, db: &DB) -> Option<#field_ty> {
          let storage = unsafe { db.storage() };
          let ingredient_id = (Self::ingredient_start_index() + #idx as u32) as usize;
          let ingredient = (&*storage.inputs[ingredient_id] as &dyn ::std::any::Any)
            .downcast_ref::<::typedown_incremental::InputIngredientStore<#field_ty>>().expect("ingredient type mismatch");
          Some(ingredient.data.get(&self.0)?.value.clone())
        }
      }
    };
    getter_setter_tokens.extend(try_getter);

    getter_setter_tokens.extend(quote! {
      pub fn #setter_name<DB: ::typedown_incremental::QueryDatabase + ?Sized>(&self, db: &mut DB, value: #field_ty) {
        let storage = unsafe { db.storage() };
        let ingredient_id = (Self::ingredient_start_index() + #idx as u32) as usize;
        let ingredient = (&*storage.inputs[ingredient_id] as &dyn ::std::any::Any)
          .downcast_ref::<::typedown_incremental::InputIngredientStore<#field_ty>>().expect("ingredient type mismatch");
        let mut entry = ingredient.data.get_mut(&self.0).expect("invalid input id");
        if entry.value.eq(&value) {
          return;
        }

        let new_revision = storage.revision.fetch_add(1, ::std::sync::atomic::Ordering::Release) + 1;
        storage.reset_for_new_revision();
        storage.input_fingerprints.remove(&(Self::ingredient_start_index(), self.0));
        let stamped = entry.value_mut();
        stamped.value = value;
        stamped.changed_at = new_revision as u32;
      }
    });
  }

  // Per-field Encodable tokens: return_ref fields access encoder.db directly to avoid borrow conflict
  let encode_field_tokens: Vec<_> = fields
    .iter()
    .map(|field| {
      let name = field.ident.as_ref().unwrap();
      if has_return_ref(field) {
        quote! {
          let __ref = self.#name(encoder.db);
          ::typedown_incremental::Encodable::field_encode(&*__ref, buf, encoder);
        }
      } else {
        quote! { ::typedown_incremental::Encodable::field_encode(&self.#name(encoder.db()), buf, encoder); }
      }
    })
    .collect();

  let stable_hash_impl = if modifiers.custom_hash {
    quote! {}
  } else {
    quote! {
      impl ::typedown_incremental::StableHash for #struct_name {
        fn stable_hash<DB: ::typedown_incremental::QueryDatabase + ?Sized>(&self, db: &DB, hasher: &mut ::typedown_incremental::StableHasher) {
          let storage = unsafe { db.storage() };
          let cache_key = (Self::ingredient_start_index(), self.0);
          if let Some(cached) = storage.input_fingerprints.get(&cache_key) {
            ::std::hash::Hasher::write(hasher, &cached.0);
            return;
          }
          let mut inner_hasher = ::typedown_incremental::StableHasher::new();
          #(
            self.#try_field_names(db).stable_hash(db, &mut inner_hasher);
          )*
          let fingerprint = ::typedown_incremental::Fingerprint::from_hasher(inner_hasher);
          storage.input_fingerprints.insert(cache_key, fingerprint);
          ::std::hash::Hasher::write(hasher, &fingerprint.0);
        }
      }
    }
  };

  output.extend::<TokenStream>(
    quote! {
      #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
      #visibility struct #struct_name(u32);

      impl #struct_name {
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

        fn id_counter() -> &'static ::std::sync::atomic::AtomicU32 {
          static COUNTER: ::std::sync::atomic::AtomicU32 = ::std::sync::atomic::AtomicU32::new(0);
          &COUNTER
        }

        fn next_id() -> u32 {
          Self::id_counter().fetch_add(1, ::std::sync::atomic::Ordering::Relaxed)
        }

        #[allow(clippy::too_many_arguments)]
        pub fn new<DB: ::typedown_incremental::QueryDatabase + ?Sized>(db: &DB, #(#field_names: #field_types),*) -> Self {
          let storage = unsafe { db.storage() };
          let id = Self::next_id();
          let start_index = Self::ingredient_start_index() as usize;

          let current_revision = storage.revision.load(::std::sync::atomic::Ordering::Acquire);
          #(
            {
              let ingredient = (&*storage.inputs[start_index + #field_indices] as &dyn ::std::any::Any)
                .downcast_ref::<::typedown_incremental::InputIngredientStore<#field_types>>().expect("ingredient type mismatch");
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

      #stable_hash_impl

      impl ::typedown_incremental::StableCompare for #struct_name {
        fn stable_cmp<DB: ::typedown_incremental::QueryDatabase + ?Sized>(&self, db: &DB, other: &Self) -> ::std::cmp::Ordering {
          let _ = db;
          ::std::cmp::Ordering::Equal
          #(
            .then_with(|| self.#try_field_names(db).stable_cmp(db, &other.#try_field_names(db)))
          )*
        }
      }

      impl ::typedown_incremental::Encodable for #struct_name {
        fn encode(&self, buf: &mut Vec<u8>, encoder: &mut ::typedown_incremental::Encoder) {
          let index = encoder.add_dep_id(::typedown_incremental::Id::as_id(self));
          encoder.emit_u32(buf, index);
          #( #encode_field_tokens )*
        }

        fn field_encode(&self, buf: &mut Vec<u8>, encoder: &mut ::typedown_incremental::Encoder) {
          let index = encoder.add_dep_id(::typedown_incremental::Id::as_id(self));
          encoder.emit_u32(buf, index);
        }
      }

      impl ::typedown_incremental::Decodable for #struct_name {
        fn decode(data: &mut &[u8], decoder: &::typedown_incremental::Decoder) -> Self {
          let index = decoder.read_u32(data);
          #(
            let _ = <#field_types as ::typedown_incremental::Decodable>::field_decode(data, decoder);
          )*
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

      impl ::typedown_incremental::Id for #struct_name {
        fn as_id(&self) -> ::typedown_incremental::DepId {
          ::typedown_incremental::DepId::new(
            ::typedown_incremental::IngredientKind::Input,
            Self::ingredient_start_index(),
            self.0,
          )
        }
      }
      impl From<u32> for #struct_name {
        fn from(id: u32) -> Self { Self(id) }
      }
      impl From<#struct_name> for u32 {
        fn from(val: #struct_name) -> u32 { val.0 }
      }

      impl ::typedown_incremental::InputId for #struct_name {
        fn iter<DB: ::typedown_incremental::QueryDatabase + ?Sized>(db: &DB) -> Vec<Self> {
          let storage = unsafe { db.storage() };
          let ingredient = &storage.inputs[Self::ingredient_start_index() as usize];
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
