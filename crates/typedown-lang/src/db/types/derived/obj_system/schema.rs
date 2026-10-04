use std::collections::BTreeMap;
use typedown_incremental::{QueryDatabase, StableHash, StableHasher};
use typedown_macros::query_derived;

use super::base::{
  BUILTIN_TO_STRING, PROTOCOL_CALL, PROTOCOL_INDEX, TdRuntimeObj, TdStaticTyp, TdTypTyp,
};
use super::func::TdFuncObj;
use super::null::TdNullObj;
use super::product::{PropertyDescriptor, TdProductTyp};
use super::{TdObjEnum, TdTypEnum};
use crate::db::TypedownDatabase;
use crate::db::derived::evaluate::evaluate_node::evaluate_node;
use crate::db::derived::get_builtin_typs::{
  get_dict_typ, get_func_typ, get_icon_typ, get_null_typ, get_obj_typ, get_schema_meta_typ,
  get_string_typ, get_sum_typ,
};
use crate::db::derived::name_resolver::scope::get_file_runtime_scope;
use crate::db::derived::schema_prop::get_schema_prop_typ;
use crate::db::types::{
  FuncKind, FuncSignature, HirValue, LazyTyp, NativeFuncKind, Project, Symbol,
};
use crate::syntax::diagnostic::Diagnostic;
use typedown_types::either::Either;

// The metatype of all schema types
// schema is to TdSchemaType as type is to TdTypeType
#[query_derived]
pub struct TdSchemaMetaTyp<'db> {}

impl<'db> TdRuntimeObj<'db> for TdSchemaMetaTyp<'db> {
  fn get_typ(&self, db: &'db TypedownDatabase) -> TdTypEnum<'db> {
    TdTypTyp::get(db).into()
  }
  fn get_owned_field(&self, _db: &'db TypedownDatabase, _key: &str) -> Option<TdObjEnum<'db>> {
    None
  }
  fn get_builtin_field(&self, _db: &'db TypedownDatabase, _key: &str) -> Option<TdObjEnum<'db>> {
    None
  }
  fn source_path(&self, _db: &'db TypedownDatabase) -> String {
    "schema".to_string()
  }
}

impl<'db> TdStaticTyp<'db> for TdSchemaMetaTyp<'db> {
  fn display_name(&self, _db: &'db TypedownDatabase) -> String {
    "schema".to_string()
  }

  fn parent_typ(&self, db: &'db TypedownDatabase) -> Option<TdTypEnum<'db>> {
    Some(TdTypTyp::get(db).into())
  }

  fn get_fields(&self, db: &'db TypedownDatabase) -> BTreeMap<String, LazyTyp<'db>> {
    let properties_type = get_sum_typ(
      db,
      vec![
        LazyTyp::eager(
          get_dict_typ(db)
            .instantiate(
              db,
              vec![
                LazyTyp::eager(get_string_typ(db).into()),
                LazyTyp::eager(get_schema_prop_typ(db).into()),
              ],
            )
            .typ(db),
        ),
        LazyTyp::eager(TdTypEnum::TdNullTyp(get_null_typ(db))),
      ],
    );
    BTreeMap::from([(
      "properties".to_string(),
      LazyTyp::eager(TdTypEnum::TdSumTyp(properties_type)),
    )])
  }

  fn is_typ(&self, _db: &'db TypedownDatabase) -> bool {
    true
  }

  fn runtime_typ(&self, _db: &'db TypedownDatabase) -> Option<TdTypEnum<'db>> {
    Some((*self).into())
  }
}

// Named opaque type with methods, construction, and nominal subtyping
// Analogous to a class in JS
#[query_derived(custom_hash)]
pub struct TdSchemaTyp<'db> {
  pub name: String,
  pub builtins: BTreeMap<String, Either<HirValue<'db>, TdObjEnum<'db>>>,
  pub fields: BTreeMap<String, PropertyDescriptor<'db>>,
  pub vtable: BTreeMap<String, TdFuncObj<'db>>,
  pub parent: Option<TdTypEnum<'db>>,
}

// Schema types are uniquely identified by name
impl<'db> StableHash for TdSchemaTyp<'db> {
  fn stable_hash<DB: QueryDatabase + ?Sized>(&self, db: &DB, hasher: &mut StableHasher) {
    let storage = unsafe { db.storage() };
    let cache_key = (Self::ingredient_start_index(), self.0);
    if let Some(cached) = storage.derived_fingerprints.get(&cache_key) {
      ::std::hash::Hasher::write(hasher, &cached.0);
      return;
    }
    let mut inner_hasher = StableHasher::new();
    Self::try_name(*self, db).stable_hash(db, &mut inner_hasher);
    let fingerprint = typedown_incremental::Fingerprint::from_hasher(inner_hasher);
    storage.derived_fingerprints.insert(cache_key, fingerprint);
    ::std::hash::Hasher::write(hasher, &fingerprint.0);
  }
}

impl<'db> TdRuntimeObj<'db> for TdSchemaTyp<'db> {
  fn get_typ(&self, db: &'db TypedownDatabase) -> TdTypEnum<'db> {
    get_schema_meta_typ(db).into()
  }
  fn get_owned_field(&self, _db: &'db TypedownDatabase, _key: &str) -> Option<TdObjEnum<'db>> {
    None
  }
  fn get_builtin_field(&self, db: &'db TypedownDatabase, key: &str) -> Option<TdObjEnum<'db>> {
    match self.builtins(db).get(key).cloned() {
      Some(Either::Left(hir)) => {
        let file_scope = get_file_runtime_scope(db, hir.project(db), hir.node(db).owner_file);
        evaluate_node(db, hir, file_scope).value(db)
      }
      Some(Either::Right(obj)) => Some(obj),
      None => None,
    }
  }
  fn source_path(&self, db: &'db TypedownDatabase) -> String {
    self.display_name(db)
  }
}

impl<'db> TdStaticTyp<'db> for TdSchemaTyp<'db> {
  fn display_name(&self, db: &'db TypedownDatabase) -> String {
    self.name(db)
  }
  fn runtime_typ(&self, _db: &'db TypedownDatabase) -> Option<TdTypEnum<'db>> {
    Some((*self).into())
  }
  fn parent_typ(&self, db: &'db TypedownDatabase) -> Option<TdTypEnum<'db>> {
    self.parent(db).or_else(|| Some(get_obj_typ(db).into()))
  }
  // Construct a schema instance from a product object
  fn construct(
    &self,
    db: &'db TypedownDatabase,
    project: Project,
    args: Vec<TdObjEnum<'db>>,
  ) -> Option<TdObjEnum<'db>> {
    let arg = args.into_iter().next()?;
    let product = arg.as_td_product_obj()?;
    let builtins = product.builtins(db);
    let fields = product.fields(db);
    Some(TdSchemaObj::new(db, (*self).into(), project, None, builtins, fields).into())
  }
  fn runtime_vtable(&self, db: &'db TypedownDatabase) -> BTreeMap<String, TdFuncObj<'db>> {
    let mut result = self
      .parent_typ(db)
      .map(|p| p.runtime_vtable(db))
      .unwrap_or_default();
    let signature = FuncSignature::new(db, vec![], get_string_typ(db).into());
    let to_string_func = TdFuncObj::new(
      db,
      BUILTIN_TO_STRING.to_string(),
      signature,
      FuncKind::Native(NativeFuncKind::ToStringMethod),
    );
    result
      .entry(BUILTIN_TO_STRING.to_string())
      .or_insert(to_string_func);
    result.extend(self.vtable(db));
    result
  }
  fn static_vtable(&self, db: &'db TypedownDatabase) -> BTreeMap<String, TdTypEnum<'db>> {
    let mut result = self
      .parent_typ(db)
      .map(|p| p.static_vtable(db))
      .unwrap_or_default();
    let signature = FuncSignature::new(db, vec![], get_string_typ(db).into());
    let func_typ = get_func_typ(db, signature).into();
    result
      .entry(BUILTIN_TO_STRING.to_string())
      .or_insert(func_typ);
    for (name, func_obj) in self.vtable(db) {
      result.insert(name, get_func_typ(db, func_obj.signature(db)).into());
    }
    result
  }
  fn get_fields(&self, db: &'db TypedownDatabase) -> BTreeMap<String, LazyTyp<'db>> {
    // Include inherited fields from parent schema
    let mut result = self
      .parent(db)
      .map(|p| p.get_fields(db))
      .unwrap_or_default();
    result.extend(
      self
        .fields(db)
        .into_iter()
        .map(|(name, descriptor)| (name, descriptor.field_typ)),
    );
    result
  }
  fn is_typ(&self, _db: &'db TypedownDatabase) -> bool {
    true
  }
}

impl<'db> TdSchemaTyp<'db> {
  // The argument type for construct: product | self
  pub fn construct_arg_typ(&self, db: &'db TypedownDatabase) -> TdTypEnum<'db> {
    let product: TdTypEnum = TdProductTyp::new(db, None, self.get_fields(db)).into();
    let schema: TdTypEnum = (*self).into();
    get_sum_typ(db, vec![LazyTyp::eager(product), LazyTyp::eager(schema)]).into()
  }

  // Resolve the declared type of a built-in underscore-prefixed field
  pub fn builtin_field_typ(db: &'db TypedownDatabase, name: &str) -> Option<TdTypEnum<'db>> {
    let null_typ = LazyTyp::eager(get_null_typ(db).into());
    match name {
      "_label" => {
        let string_typ = LazyTyp::eager(get_string_typ(db).into());
        Some(get_sum_typ(db, vec![string_typ, null_typ]).into())
      }
      "_icon" => {
        let icon_typ = LazyTyp::eager(get_icon_typ(db).into());
        Some(get_sum_typ(db, vec![icon_typ, null_typ]).into())
      }
      "_content" => Some(get_string_typ(db).into()),
      "_type" => Some(get_schema_meta_typ(db).into()),
      "_extends" => Some(get_schema_meta_typ(db).into()),
      "_meta" => {
        let optional_string = get_sum_typ(
          db,
          vec![
            LazyTyp::eager(get_string_typ(db).into()),
            LazyTyp::eager(get_null_typ(db).into()),
          ],
        );
        let meta_fields = BTreeMap::from([
          ("title".to_string(), LazyTyp::eager(optional_string.into())),
          (
            "description".to_string(),
            LazyTyp::eager(optional_string.into()),
          ),
          ("image".to_string(), LazyTyp::eager(optional_string.into())),
        ]);
        let meta_typ =
          LazyTyp::eager(TdProductTyp::new(db, Some("_meta".to_string()), meta_fields).into());
        Some(get_sum_typ(db, vec![meta_typ, null_typ]).into())
      }
      _ => None,
    }
  }
}

// Runtime instance of a schema type, with computed fields, defaults, and methods
#[query_derived(custom_hash)]
pub struct TdSchemaObj<'db> {
  pub schema: TdTypEnum<'db>,
  pub project: Project,
  pub file_symbol: Option<Symbol<'db>>,
  pub builtins: BTreeMap<String, Either<HirValue<'db>, TdObjEnum<'db>>>,
  pub fields: BTreeMap<String, Either<HirValue<'db>, TdObjEnum<'db>>>,
}

// If file_symbol is present, hash only (schema, file_symbol) for O(1)
// Otherwise fall back to hashing all fields
impl<'db> StableHash for TdSchemaObj<'db> {
  fn stable_hash<DB: QueryDatabase + ?Sized>(&self, db: &DB, hasher: &mut StableHasher) {
    let storage = unsafe { db.storage() };
    let cache_key = (Self::ingredient_start_index(), self.0);
    if let Some(cached) = storage.derived_fingerprints.get(&cache_key) {
      ::std::hash::Hasher::write(hasher, &cached.0);
      return;
    }
    let mut inner_hasher = StableHasher::new();
    let file_symbol = Self::try_file_symbol(*self, db);
    if let Some(Some(symbol)) = &file_symbol {
      Self::try_schema(*self, db).stable_hash(db, &mut inner_hasher);
      symbol.stable_hash(db, &mut inner_hasher);
    } else {
      Self::try_schema(*self, db).stable_hash(db, &mut inner_hasher);
      Self::try_project(*self, db).stable_hash(db, &mut inner_hasher);
      Self::try_builtins(*self, db).stable_hash(db, &mut inner_hasher);
      Self::try_fields(*self, db).stable_hash(db, &mut inner_hasher);
    }
    let fingerprint = typedown_incremental::Fingerprint::from_hasher(inner_hasher);
    storage.derived_fingerprints.insert(cache_key, fingerprint);
    ::std::hash::Hasher::write(hasher, &fingerprint.0);
  }
}

impl<'db> TdRuntimeObj<'db> for TdSchemaObj<'db> {
  fn get_typ(&self, db: &'db TypedownDatabase) -> TdTypEnum<'db> {
    self.schema(db)
  }
  fn get_owned_field(&self, db: &'db TypedownDatabase, key: &str) -> Option<TdObjEnum<'db>> {
    match self.fields(db).get(key).cloned() {
      Some(Either::Left(hir)) => {
        let file_scope = get_file_runtime_scope(db, hir.project(db), hir.node(db).owner_file);
        evaluate_node(db, hir, file_scope).value(db)
      }
      Some(Either::Right(obj)) => Some(obj),
      // Missing fields: check schema for computed/default, then null
      None => {
        if let Some(schema_type) = self.schema(db).as_td_schema_typ()
          && let Some(prop_desc) = schema_type.fields(db).get(key)
        {
          if let Some(ref computed_enum) = prop_desc.computed_func
            && let Some(func_obj) = computed_enum.as_td_func_obj()
            && let Ok(res_val) = func_obj.call(db, self.project(db), None, vec![(*self).into()])
          {
            return Some(res_val);
          }
          if let Some(ref def_obj) = prop_desc.default_value {
            return Some(def_obj.clone());
          }
        }
        Some(TdNullObj::get(db).into())
      }
    }
  }
  fn get_builtin_field(&self, db: &'db TypedownDatabase, key: &str) -> Option<TdObjEnum<'db>> {
    // Check instance builtins first
    if let Some(entry) = self.builtins(db).get(key).cloned() {
      return match entry {
        Either::Left(hir) => {
          let file_scope = get_file_runtime_scope(db, hir.project(db), hir.node(db).owner_file);
          evaluate_node(db, hir, file_scope).value(db)
        }
        Either::Right(obj) => Some(obj),
      };
    }
    // Fall back to schema type for _icon default
    if key == "_icon" {
      return self.schema(db).get_builtin_field(db, key);
    }
    None
  }
  fn source_path(&self, db: &'db TypedownDatabase) -> String {
    self.get_typ(db).source_path(db)
  }
  fn index(&self, db: &'db TypedownDatabase, key: &TdObjEnum<'db>) -> Option<TdObjEnum<'db>> {
    let this: TdObjEnum = (*self).into();
    self
      .lookup_method(db, PROTOCOL_INDEX)?
      .call(db, self.project(db), Some(this), vec![key.clone()])
      .ok()
  }
  fn call(
    &self,
    db: &'db TypedownDatabase,
    project: Project,
    this: Option<TdObjEnum<'db>>,
    args: Vec<TdObjEnum<'db>>,
  ) -> Result<TdObjEnum<'db>, Vec<Diagnostic>> {
    let Some(func) = self.lookup_method(db, PROTOCOL_CALL) else {
      return Err(vec![]);
    };
    func.call(db, project, this, args)
  }
}
