use std::collections::BTreeMap;
use typedown_incremental::{
  Decodable, Decoder, Encodable, Encoder, QueryDatabase, StableHash, StableHasher,
};
use typedown_macros::{StableCompare, query_derived};

use super::base::{TdRuntimeObj, TdStaticTyp, TdTypTyp};
use super::null::TdNullObj;
use super::typecheck::{is_nullable, is_subtype_of};
use super::{TdObjEnum, TdTypEnum};
use crate::db::TypedownDatabase;
use crate::db::derived::evaluate::evaluate_node::evaluate_node;
use crate::db::derived::get_builtin_typs::get_obj_typ;
use crate::db::derived::name_resolver::scope::get_file_runtime_scope;
use crate::db::types::{HirValue, LazyTyp, Symbol};
use crate::db::utils::static_type::format_field_map;
use typedown_types::either::Either;

#[derive(Debug, Clone, PartialEq, Eq, StableCompare)]
pub struct PropertyDescriptor<'db> {
  pub field_typ: LazyTyp<'db>,
  pub default_value: Option<TdObjEnum<'db>>,
  pub computed_func: Option<TdObjEnum<'db>>,
}

impl<'db> StableHash for PropertyDescriptor<'db> {
  fn stable_hash<DB: QueryDatabase + ?Sized>(&self, db: &DB, hasher: &mut StableHasher) {
    self.field_typ.stable_hash(db, hasher);
    self.default_value.stable_hash(db, hasher);
    self.computed_func.stable_hash(db, hasher);
  }
}

impl<'db> Encodable for PropertyDescriptor<'db> {
  fn encode(&self, buf: &mut Vec<u8>, encoder: &mut Encoder) {
    self.field_typ.encode(buf, encoder);
    self.default_value.encode(buf, encoder);
    self.computed_func.encode(buf, encoder);
  }
}

impl<'db> Decodable for PropertyDescriptor<'db> {
  fn decode(data: &mut &[u8], decoder: &Decoder) -> Self {
    let field_typ = LazyTyp::decode(data, decoder);
    let default_value = Option::<TdObjEnum>::decode(data, decoder);
    let computed_func = Option::<TdObjEnum<'db>>::decode(data, decoder);
    PropertyDescriptor {
      field_typ,
      default_value,
      computed_func,
    }
  }
}

// Structural data bag with optional display name
#[query_derived(custom_hash)]
pub struct TdProductTyp<'db> {
  pub name: Option<String>,
  pub fields: BTreeMap<String, LazyTyp<'db>>,
}

// Hash name + field keys (not field type values)
impl<'db> StableHash for TdProductTyp<'db> {
  fn stable_hash<DB: QueryDatabase + ?Sized>(&self, db: &DB, hasher: &mut StableHasher) {
    let storage = unsafe { db.storage() };
    let cache_key = (Self::ingredient_start_index(), self.0);
    if let Some(cached) = storage.derived_fingerprints.get(&cache_key) {
      ::std::hash::Hasher::write(hasher, &cached.0);
      return;
    }
    let mut inner_hasher = StableHasher::new();
    Self::try_name(*self, db).stable_hash(db, &mut inner_hasher);
    if let Some(fields) = Self::try_fields(*self, db) {
      fields.len().stable_hash(db, &mut inner_hasher);
      for key in fields.keys() {
        key.stable_hash(db, &mut inner_hasher);
      }
    }
    let fingerprint = typedown_incremental::Fingerprint::from_hasher(inner_hasher);
    storage.derived_fingerprints.insert(cache_key, fingerprint);
    ::std::hash::Hasher::write(hasher, &fingerprint.0);
  }
}

impl<'db> TdRuntimeObj<'db> for TdProductTyp<'db> {
  fn get_typ(&self, db: &'db TypedownDatabase) -> TdTypEnum<'db> {
    TdTypTyp::get(db).into()
  }
  fn get_owned_field(&self, _db: &'db TypedownDatabase, _key: &str) -> Option<TdObjEnum<'db>> {
    None
  }
  fn get_builtin_field(&self, _db: &'db TypedownDatabase, _key: &str) -> Option<TdObjEnum<'db>> {
    None
  }
  fn source_path(&self, db: &'db TypedownDatabase) -> String {
    self.display_name(db)
  }
}

impl<'db> TdStaticTyp<'db> for TdProductTyp<'db> {
  fn display_name(&self, db: &'db TypedownDatabase) -> String {
    if let Some(name) = self.name(db) {
      return name;
    }
    format_field_map(db, &self.fields(db))
  }
  fn runtime_typ(&self, _db: &'db TypedownDatabase) -> Option<TdTypEnum<'db>> {
    None
  }
  fn parent_typ(&self, db: &'db TypedownDatabase) -> Option<TdTypEnum<'db>> {
    Some(get_obj_typ(db).into())
  }
  fn get_fields(&self, db: &'db TypedownDatabase) -> BTreeMap<String, LazyTyp<'db>> {
    self.fields(db)
  }
}

// Runtime instance of a product type, plain data bag
#[query_derived(custom_hash)]
pub struct TdProductObj<'db> {
  pub product_typ: TdTypEnum<'db>,
  pub file_symbol: Option<Symbol<'db>>,
  pub builtins: BTreeMap<String, Either<HirValue<'db>, TdObjEnum<'db>>>,
  pub fields: BTreeMap<String, Either<HirValue<'db>, TdObjEnum<'db>>>,
}

// If file_symbol is present, hash only (product_type, file_symbol) for O(1)
// Otherwise fall back to hashing all fields
impl<'db> StableHash for TdProductObj<'db> {
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
      Self::try_product_typ(*self, db).stable_hash(db, &mut inner_hasher);
      symbol.stable_hash(db, &mut inner_hasher);
    } else {
      Self::try_product_typ(*self, db).stable_hash(db, &mut inner_hasher);
      Self::try_builtins(*self, db).stable_hash(db, &mut inner_hasher);
      Self::try_fields(*self, db).stable_hash(db, &mut inner_hasher);
    }
    let fingerprint = typedown_incremental::Fingerprint::from_hasher(inner_hasher);
    storage.derived_fingerprints.insert(cache_key, fingerprint);
    ::std::hash::Hasher::write(hasher, &fingerprint.0);
  }
}

impl<'db> TdRuntimeObj<'db> for TdProductObj<'db> {
  fn get_typ(&self, db: &'db TypedownDatabase) -> TdTypEnum<'db> {
    self.product_typ(db)
  }
  fn get_owned_field(&self, db: &'db TypedownDatabase, key: &str) -> Option<TdObjEnum<'db>> {
    match self.fields(db).get(key).cloned() {
      Some(Either::Left(hir)) => {
        let file_scope = get_file_runtime_scope(db, hir.project(db), hir.node(db).owner_file);
        evaluate_node(db, hir, file_scope).value(db)
      }
      Some(Either::Right(obj)) => Some(obj),
      None => Some(TdNullObj::get(db).into()),
    }
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
    self.get_typ(db).source_path(db)
  }
}

// Check if expected fields are compatible with actual fields
pub fn fields_compatible<'db>(
  db: &'db TypedownDatabase,
  expected_fields: &BTreeMap<String, LazyTyp<'db>>,
  actual_fields: &BTreeMap<String, LazyTyp<'db>>,
) -> bool {
  expected_fields.iter().all(|(name, expected_lazy)| {
    let optional = expected_lazy
      .resolve(db)
      .is_some_and(|typ| is_nullable(db, &typ));
    match actual_fields.get(name) {
      Some(actual_lazy) => {
        let Some(expected_typ) = expected_lazy.resolve(db) else {
          return false;
        };
        let Some(actual_typ) = actual_lazy.resolve(db) else {
          return false;
        };
        is_subtype_of(db, &actual_typ, &expected_typ)
      }
      None => optional,
    }
  })
}

pub fn make_property_descriptors<'db>(
  _db: &'db TypedownDatabase,
  fields: BTreeMap<String, LazyTyp<'db>>,
) -> BTreeMap<String, PropertyDescriptor<'db>> {
  fields
    .into_iter()
    .map(|(k, v)| {
      (
        k,
        PropertyDescriptor {
          field_typ: v,
          default_value: None,
          computed_func: None,
        },
      )
    })
    .collect()
}
