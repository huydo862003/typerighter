mod projection;
mod unwrap;

use std::collections::BTreeMap;

use std::hash::Hash;
use strum::FromRepr;
use typedown_incremental::{
  Decodable, Decoder, Encodable, Encoder, QueryDatabase, StableHash, StableHasher,
};
use typedown_macros::{StableCompare, query_derived};

use super::base::{TdRuntimeObj, TdStaticTyp, TdTypTyp};
use super::func::TdFuncObj;
use super::{TdObjEnum, TdTypEnum};
use crate::db::TypedownDatabase;
use crate::db::types::{FuncSignature, LazyTyp};

use projection::resolve_projection;
use unwrap::resolve_unwrap;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, FromRepr, StableCompare)]
#[repr(u8)]
pub enum TypFn {
  // Projection<T, K>: field type of key K on type T
  Projection = 0,
  // Unwrap<T>: strip nullable and list wrappers to get the underlying schema
  Unwrap = 1,
}

impl StableHash for TypFn {
  fn stable_hash<DB: QueryDatabase + ?Sized>(&self, _db: &DB, hasher: &mut StableHasher) {
    (*self as u8).hash(hasher);
  }
}

impl Encodable for TypFn {
  fn encode(&self, buffer: &mut Vec<u8>, encoder: &mut Encoder) {
    encoder.emit_u8(buffer, *self as u8);
  }
}

impl Decodable for TypFn {
  fn decode(data: &mut &[u8], decoder: &Decoder) -> Self {
    TypFn::from_repr(decoder.read_u8(data)).expect("unknown TypFn tag")
  }
}

/// Type-level computation that resolves lazily
/// try_resolve returns Some when all args are concrete, None if variables remain
#[query_derived]
pub struct TdComputedTyp<'db> {
  pub func: TypFn,
  pub args: Vec<TdTypEnum<'db>>,
}

impl<'db> TdComputedTyp<'db> {
  pub fn try_resolve(&self, db: &'db TypedownDatabase) -> Option<TdTypEnum<'db>> {
    match self.func(db) {
      TypFn::Projection => {
        let args = self.args(db);
        resolve_projection(db, args.first()?, args.get(1)?)
      }
      TypFn::Unwrap => {
        let args = self.args(db);
        resolve_unwrap(db, args.first()?)
      }
    }
  }

  pub fn simplify(&self, db: &'db TypedownDatabase) -> TdTypEnum<'db> {
    self.try_resolve(db).unwrap_or_else(|| (*self).into())
  }
}

impl<'db> TdRuntimeObj<'db> for TdComputedTyp<'db> {
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

impl<'db> TdStaticTyp<'db> for TdComputedTyp<'db> {
  fn display_name(&self, db: &'db TypedownDatabase) -> String {
    match self.try_resolve(db) {
      Some(resolved) => resolved.display_name(db),
      None => {
        let func_name = match self.func(db) {
          TypFn::Projection => "Projection",
          TypFn::Unwrap => "Unwrap",
        };
        let args: Vec<String> = self.args(db).iter().map(|a| a.display_name(db)).collect();
        format!("{}<{}>", func_name, args.join(", "))
      }
    }
  }

  fn lookup_field_typ(&self, db: &'db TypedownDatabase, name: &str) -> Option<TdTypEnum<'db>> {
    self.try_resolve(db)?.lookup_field_typ(db, name)
  }

  fn get_fields(&self, db: &'db TypedownDatabase) -> BTreeMap<String, LazyTyp<'db>> {
    self
      .try_resolve(db)
      .map(|typ| typ.get_fields(db))
      .unwrap_or_default()
  }

  fn runtime_vtable(&self, db: &'db TypedownDatabase) -> BTreeMap<String, TdFuncObj<'db>> {
    self
      .try_resolve(db)
      .map(|typ| typ.runtime_vtable(db))
      .unwrap_or_default()
  }

  fn static_vtable(&self, db: &'db TypedownDatabase) -> BTreeMap<String, TdTypEnum<'db>> {
    self
      .try_resolve(db)
      .map(|typ| typ.static_vtable(db))
      .unwrap_or_default()
  }

  fn index_typ(
    &self,
    db: &'db TypedownDatabase,
    key_typ: &TdTypEnum<'db>,
  ) -> Option<FuncSignature<'db>> {
    self.try_resolve(db)?.index_typ(db, key_typ)
  }

  fn runtime_typ(&self, db: &'db TypedownDatabase) -> Option<TdTypEnum<'db>> {
    self.try_resolve(db)
  }
}
