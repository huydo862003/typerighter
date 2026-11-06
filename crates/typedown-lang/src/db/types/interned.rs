use std::hash::Hash;

use strum::FromRepr;
use typedown_macros::{StableCompare, query_interned};

use typedown_incremental::{
  Decodable, Decoder, Encodable, Encoder, QueryDatabase, StableHash, StableHasher,
};

use super::{TdBoundVariableTyp, TdTypEnum};
use crate::db::TypedownDatabase;
use crate::db::derived::evaluate::evaluate_typ::evaluate_typ;
use crate::db::types::Symbol;
use crate::db::types::typecheck::substitute_variables;

#[query_interned]
pub struct FuncSignature<'db> {
  pub typ_params: Option<TypParams<'db>>,
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
  fn encode(&self, buffer: &mut Vec<u8>, _encoder: &mut Encoder) {
    buffer.push(*self as u8);
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
pub struct TypParams<'db> {
  pub params: Vec<TdBoundVariableTyp<'db>>,
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

  pub fn get_param(&self, db: &TypedownDatabase, index: usize) -> Option<TdBoundVariableTyp<'_>> {
    self.params(db).get(index).copied()
  }

  pub fn get_binding(&self, db: &TypedownDatabase, index: usize) -> Option<LazyTyp<'_>> {
    self.bindings(db).get(index).cloned()
  }

  pub fn get_by_index(
    &self,
    db: &TypedownDatabase,
    index: usize,
  ) -> Option<TdBoundVariableTyp<'_>> {
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

// A type reference that may be eagerly resolved or lazily deferred to a symbol with optional pending substitution bindings
#[derive(Debug, Clone, PartialEq, Eq, Hash, StableCompare)]
pub enum LazyTyp<'db> {
  Eager(TdTypEnum<'db>),
  Lazy(Symbol, Vec<(TdBoundVariableTyp<'db>, LazyTyp<'db>)>),
}

impl<'db> LazyTyp<'db> {
  pub fn eager(typ: TdTypEnum<'db>) -> Self {
    LazyTyp::Eager(typ)
  }

  pub fn lazy(symbol: Symbol) -> Self {
    LazyTyp::Lazy(symbol, vec![])
  }

  pub fn resolve(&self, db: &'db TypedownDatabase) -> Option<TdTypEnum<'db>> {
    match self {
      LazyTyp::Eager(typ) => Some(typ.clone()),
      LazyTyp::Lazy(symbol, bindings) => {
        let base = evaluate_typ(db, *symbol).typ(db)?;
        if bindings.is_empty() {
          return Some(base);
        }
        let binding_map: hashbrown::HashMap<TdBoundVariableTyp<'db>, LazyTyp<'db>> =
          bindings.iter().cloned().collect();
        Some(substitute_variables(db, &base, &binding_map))
      }
    }
  }

  pub fn as_eager(&self) -> Option<TdTypEnum<'db>> {
    match self {
      LazyTyp::Eager(typ) => Some(typ.clone()),
      _ => None,
    }
  }
}

impl<'db> Encodable for LazyTyp<'db> {
  fn encode(&self, buffer: &mut Vec<u8>, encoder: &mut Encoder) {
    match self {
      LazyTyp::Eager(typ) => {
        encoder.emit_u8(buffer, 0);
        typ.encode(buffer, encoder);
      }
      LazyTyp::Lazy(symbol, bindings) => {
        encoder.emit_u8(buffer, 1);
        symbol.encode(buffer, encoder);
        bindings.encode(buffer, encoder);
      }
    }
  }
}

impl<'db> Decodable for LazyTyp<'db> {
  fn decode(data: &mut &[u8], decoder: &Decoder) -> Self {
    match decoder.read_u8(data) {
      0 => LazyTyp::Eager(TdTypEnum::decode(data, decoder)),
      1 => LazyTyp::Lazy(Symbol::decode(data, decoder), Vec::decode(data, decoder)),
      _ => panic!("unknown LazyTyp tag"),
    }
  }
}

impl<'db> StableHash for LazyTyp<'db> {
  fn stable_hash<DB: QueryDatabase + ?Sized>(&self, db: &DB, hasher: &mut StableHasher) {
    match self {
      LazyTyp::Eager(typ) => {
        0u8.stable_hash(db, hasher);
        typ.stable_hash(db, hasher);
      }
      LazyTyp::Lazy(symbol, bindings) => {
        1u8.stable_hash(db, hasher);
        symbol.stable_hash(db, hasher);
        bindings.stable_hash(db, hasher);
      }
    }
  }
}

/// A concrete literal value used in literal constraints
#[derive(Debug, Clone, PartialEq, Eq, Hash, StableCompare)]
pub enum LitValue<'db> {
  String(String),
  Bool(bool),
  // f64 cannot be hashed so we store in string
  Number(String),
  Typ(TdTypEnum<'db>),
}

#[derive(FromRepr)]
#[repr(u8)]
enum LitValueTag {
  String = 0,
  Bool = 1,
  Number = 2,
  Typ = 3,
}

impl Encodable for LitValue<'_> {
  fn encode(&self, buffer: &mut Vec<u8>, encoder: &mut Encoder) {
    match self {
      LitValue::String(value) => {
        encoder.emit_u8(buffer, LitValueTag::String as u8);
        value.encode(buffer, encoder);
      }
      LitValue::Bool(value) => {
        encoder.emit_u8(buffer, LitValueTag::Bool as u8);
        value.encode(buffer, encoder);
      }
      LitValue::Number(value) => {
        encoder.emit_u8(buffer, LitValueTag::Number as u8);
        value.encode(buffer, encoder);
      }
      LitValue::Typ(value) => {
        encoder.emit_u8(buffer, LitValueTag::Typ as u8);
        value.encode(buffer, encoder);
      }
    }
  }
}

impl Decodable for LitValue<'_> {
  fn decode(data: &mut &[u8], decoder: &Decoder) -> Self {
    let tag = decoder.read_u8(data);
    match LitValueTag::from_repr(tag).expect("unknown LiteralValue tag") {
      LitValueTag::String => LitValue::String(String::decode(data, decoder)),
      LitValueTag::Bool => LitValue::Bool(bool::decode(data, decoder)),
      LitValueTag::Number => LitValue::Number(String::decode(data, decoder)),
      LitValueTag::Typ => LitValue::Typ(TdTypEnum::decode(data, decoder)),
    }
  }
}

impl StableHash for LitValue<'_> {
  fn stable_hash<DB: QueryDatabase + ?Sized>(&self, db: &DB, hasher: &mut StableHasher) {
    std::mem::discriminant(self).stable_hash(db, hasher);
    match self {
      LitValue::String(value) => value.stable_hash(db, hasher),
      LitValue::Bool(value) => value.stable_hash(db, hasher),
      LitValue::Number(value) => value.stable_hash(db, hasher),
      LitValue::Typ(value) => value.stable_hash(db, hasher),
    }
  }
}
