use std::hash::Hash;

use strum::FromRepr;
use typedown_macros::{StableCompare, query_interned};

use typedown_incremental::{
  Decodable, Decoder, Encodable, Encoder, QueryDatabase, StableHash, StableHasher,
};

use typedown_types::either::Either;

use super::TdTypEnum;
use crate::db::TypedownDatabase;
use crate::db::derived::evaluate::evaluate_typ::evaluate_typ;
use crate::db::derived::get_builtin_typs::get_obj_typ;
use crate::db::types::Symbol;

#[query_interned]
pub struct FuncSignature<'db> {
  pub params: Vec<TdTypEnum<'db>>,
  pub ret: TdTypEnum<'db>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, FromRepr, StableCompare)]
#[repr(u8)]
#[derive(Default)]
pub enum Variance {
  #[default]
  Covariant = 0,
  Contravariant = 1,
  Invariant = 2,
}

impl StableHash for Variance {
  fn stable_hash<DB: QueryDatabase + ?Sized>(&self, _db: &DB, hasher: &mut StableHasher) {
    (*self as u8).hash(hasher);
  }
}

impl Encodable for Variance {
  fn encode(&self, buf: &mut Vec<u8>, _encoder: &mut Encoder) {
    buf.push(*self as u8);
  }
}

impl Decodable for Variance {
  fn decode(data: &mut &[u8], _decoder: &Decoder) -> Self {
    let tag = data[0];
    *data = &data[1..];
    Variance::from_repr(tag).unwrap_or(Variance::Covariant)
  }
}

#[query_interned]
pub struct TypVariable<'db> {
  pub upper_bound: LazyTyp<'db>,
  pub variance: Variance, // Existential type variables always have INVARIANCE because variance is irrelevant
}

impl<'db> TypVariable<'db> {
  pub fn get(db: &'db TypedownDatabase, upper_bound: Option<LazyTyp<'db>>) -> Self {
    let upper_bound = upper_bound.unwrap_or_else(|| LazyTyp::eager(get_obj_typ(db).into()));
    TypVariable::new(db, upper_bound, Variance::Covariant)
  }

  pub fn get_with_variance(
    db: &'db TypedownDatabase,
    upper_bound: Option<LazyTyp<'db>>,
    variance: Variance,
  ) -> Self {
    let upper_bound = upper_bound.unwrap_or_else(|| LazyTyp::eager(get_obj_typ(db).into()));
    TypVariable::new(db, upper_bound, variance)
  }
}

#[query_interned]
pub struct TypParams<'db> {
  pub params: Vec<TypVariable<'db>>,
  pub bindings: Vec<LazyTyp<'db>>,
}

impl<'db> TypParams<'db> {
  pub fn instantiate(
    &self,
    db: &'db TypedownDatabase,
    args: Vec<LazyTyp<'db>>,
  ) -> Option<TypParams<'db>> {
    let params = self.params(db);
    if params.len() != args.len() {
      return None;
    }
    Some(TypParams::new(db, params, args))
  }

  pub fn bind(
    &self,
    db: &'db TypedownDatabase,
    index: usize,
    arg: LazyTyp<'db>,
  ) -> Option<TypParams<'db>> {
    let params = self.params(db);
    let mut bindings = self.bindings(db);
    if index >= params.len() {
      return None;
    }
    if bindings.len() <= index {
      bindings.resize(index + 1, arg.clone());
    }
    bindings[index] = arg;
    Some(TypParams::new(db, params, bindings))
  }

  pub fn get_param(&self, db: &TypedownDatabase, index: usize) -> Option<TypVariable<'_>> {
    self.params(db).get(index).copied()
  }

  pub fn get_binding(&self, db: &TypedownDatabase, index: usize) -> Option<LazyTyp<'_>> {
    self.bindings(db).get(index).cloned()
  }

  pub fn get_by_index(&self, db: &TypedownDatabase, index: usize) -> Option<TypVariable<'_>> {
    self.params(db).get(index).copied()
  }

  pub fn is_instantiated(&self, db: &TypedownDatabase) -> bool {
    let params = self.params(db);
    let bindings = self.bindings(db);
    !params.is_empty() && params.len() == bindings.len()
  }

  pub fn arity(&self, db: &TypedownDatabase) -> usize {
    let params = self.params(db).len();
    let bound = self.bindings(db).len();
    params.saturating_sub(bound)
  }

  pub fn len(&self, db: &TypedownDatabase) -> usize {
    self.params(db).len()
  }

  pub fn is_empty(&self, db: &TypedownDatabase) -> bool {
    self.params(db).is_empty()
  }
}

// A type reference that may be eagerly resolved or lazily deferred to a symbol
#[derive(Debug, Clone, PartialEq, Eq, Hash, StableCompare)]
pub struct LazyTyp<'db>(Either<TdTypEnum<'db>, Symbol<'db>>);

impl<'db> LazyTyp<'db> {
  pub fn eager(typ: TdTypEnum<'db>) -> Self {
    LazyTyp(Either::Left(typ))
  }

  pub fn lazy(symbol: Symbol<'db>) -> Self {
    LazyTyp(Either::Right(symbol))
  }

  pub fn resolve(&self, db: &'db TypedownDatabase) -> Option<TdTypEnum<'db>> {
    match &self.0 {
      Either::Left(typ) => Some(typ.clone()),
      Either::Right(symbol) => evaluate_typ(db, *symbol).typ(db),
    }
  }

  pub fn as_eager(&self) -> Option<TdTypEnum<'db>> {
    match &self.0 {
      Either::Left(typ) => Some(typ.clone()),
      Either::Right(_) => None,
    }
  }
}

impl<'db> Encodable for LazyTyp<'db> {
  fn encode(&self, buf: &mut Vec<u8>, encoder: &mut Encoder) {
    self.0.encode(buf, encoder);
  }
}

impl<'db> Decodable for LazyTyp<'db> {
  fn decode(data: &mut &[u8], decoder: &Decoder) -> Self {
    LazyTyp(Either::<TdTypEnum, Symbol>::decode(data, decoder))
  }
}

impl<'db> StableHash for LazyTyp<'db> {
  fn stable_hash<DB: QueryDatabase + ?Sized>(&self, db: &DB, hasher: &mut StableHasher) {
    self.0.stable_hash(db, hasher);
  }
}

/// A concrete literal value used in literal constraints
#[derive(Debug, Clone, PartialEq, Eq, Hash, StableCompare)]
pub enum LitValue {
  String(String),
  Bool(bool),
  // f64 cannot be hashed so we store in string
  Number(String),
}

#[derive(FromRepr)]
#[repr(u8)]
enum LitValueTag {
  String = 0,
  Bool = 1,
  Number = 2,
}

impl Encodable for LitValue {
  fn encode(&self, buf: &mut Vec<u8>, encoder: &mut Encoder) {
    match self {
      LitValue::String(val) => {
        encoder.emit_u8(buf, LitValueTag::String as u8);
        val.encode(buf, encoder);
      }
      LitValue::Bool(val) => {
        encoder.emit_u8(buf, LitValueTag::Bool as u8);
        val.encode(buf, encoder);
      }
      LitValue::Number(val) => {
        encoder.emit_u8(buf, LitValueTag::Number as u8);
        val.encode(buf, encoder);
      }
    }
  }
}

impl Decodable for LitValue {
  fn decode(data: &mut &[u8], decoder: &Decoder) -> Self {
    let tag = decoder.read_u8(data);
    match LitValueTag::from_repr(tag).expect("unknown LiteralValue tag") {
      LitValueTag::String => LitValue::String(String::decode(data, decoder)),
      LitValueTag::Bool => LitValue::Bool(bool::decode(data, decoder)),
      LitValueTag::Number => LitValue::Number(String::decode(data, decoder)),
    }
  }
}

impl StableHash for LitValue {
  fn stable_hash<DB: QueryDatabase + ?Sized>(&self, db: &DB, hasher: &mut StableHasher) {
    std::mem::discriminant(self).stable_hash(db, hasher);
    match self {
      LitValue::String(value) => value.stable_hash(db, hasher),
      LitValue::Bool(value) => value.stable_hash(db, hasher),
      LitValue::Number(value) => value.stable_hash(db, hasher),
    }
  }
}
