use std::hash::Hash;

use strum::FromRepr;
use typedown_macros::{StableCompare, query_interned};

use typedown_incremental::{
  Decodable, Decoder, Encodable, Encoder, QueryDatabase, StableHash, StableHasher,
};

use typedown_types::either::Either;

use super::TdTypeEnum;
use crate::db::TypedownDatabase;
use crate::db::derived::evaluate::evaluate_type::evaluate_type;
use crate::db::types::Symbol;

#[query_interned]
pub struct FuncSignature {
  pub params: Vec<TdTypeEnum>,
  pub ret: TdTypeEnum,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, StableCompare)]
pub struct TypeVariable {
  pub bound: Option<LazyType>,
}

impl StableHash for TypeVariable {
  fn stable_hash<DB: QueryDatabase + ?Sized>(&self, db: &DB, hasher: &mut StableHasher) {
    self.bound.stable_hash(db, hasher);
  }
}

impl Encodable for TypeVariable {
  fn encode(&self, buf: &mut Vec<u8>, encoder: &mut Encoder) {
    self.bound.encode(buf, encoder);
  }
}

impl Decodable for TypeVariable {
  fn decode(data: &mut &[u8], decoder: &Decoder) -> Self {
    Self {
      bound: Option::<LazyType>::decode(data, decoder),
    }
  }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, StableCompare)]
pub struct TypeParamPair {
  pub name: String,
  pub var: TypeVariable,
  pub value: Option<LazyType>,
}

impl StableHash for TypeParamPair {
  fn stable_hash<DB: QueryDatabase + ?Sized>(&self, db: &DB, hasher: &mut StableHasher) {
    self.name.stable_hash(db, hasher);
    self.var.stable_hash(db, hasher);
    self.value.stable_hash(db, hasher);
  }
}

impl Encodable for TypeParamPair {
  fn encode(&self, buf: &mut Vec<u8>, encoder: &mut Encoder) {
    self.name.encode(buf, encoder);
    self.var.encode(buf, encoder);
    self.value.encode(buf, encoder);
  }
}

impl Decodable for TypeParamPair {
  fn decode(data: &mut &[u8], decoder: &Decoder) -> Self {
    Self {
      name: String::decode(data, decoder),
      var: TypeVariable::decode(data, decoder),
      value: Option::<LazyType>::decode(data, decoder),
    }
  }
}

#[query_interned]
pub struct TypeParams {
  pub params: Vec<TypeParamPair>,
}

impl TypeParams {
  pub fn get_by_index(&self, db: &TypedownDatabase, index: usize) -> Option<TypeParamPair> {
    self.params(db).get(index).cloned()
  }

  pub fn get_by_name(&self, db: &TypedownDatabase, name: &str) -> Option<(usize, TypeVariable)> {
    self
      .params(db)
      .iter()
      .enumerate()
      .find(|(_, p)| p.name == name)
      .map(|(idx, p)| (idx, p.var.clone()))
  }

  pub fn get_index_of(&self, db: &TypedownDatabase, name: &str) -> Option<usize> {
    self.params(db).iter().position(|p| p.name == name)
  }

  pub fn instantiate(&self, db: &TypedownDatabase, args: Vec<LazyType>) -> Option<TypeParams> {
    let current_params = self.params(db);
    if current_params.len() != args.len() {
      return None;
    }
    let new_params = current_params
      .iter()
      .zip(args)
      .map(|(p, arg)| TypeParamPair {
        name: p.name.clone(),
        var: p.var.clone(),
        value: Some(arg),
      })
      .collect();
    Some(TypeParams::new(db, new_params))
  }

  pub fn bind(&self, db: &TypedownDatabase, name: &str, arg: LazyType) -> Option<TypeParams> {
    let current_params = self.params(db);
    let mut updated = false;
    let new_params = current_params
      .iter()
      .map(|p| {
        if p.name == name {
          updated = true;
          TypeParamPair {
            name: p.name.clone(),
            var: p.var.clone(),
            value: Some(arg.clone()),
          }
        } else {
          p.clone()
        }
      })
      .collect();
    if updated {
      Some(TypeParams::new(db, new_params))
    } else {
      None
    }
  }

  pub fn is_instantiated(&self, db: &TypedownDatabase) -> bool {
    let current = self.params(db);
    !current.is_empty() && current.iter().all(|p| p.value.is_some())
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
pub struct LazyType(Either<TdTypeEnum, Symbol>);

impl LazyType {
  pub fn eager(typ: TdTypeEnum) -> Self {
    LazyType(Either::Left(typ))
  }

  pub fn lazy(symbol: Symbol) -> Self {
    LazyType(Either::Right(symbol))
  }

  pub fn resolve(&self, db: &TypedownDatabase) -> Option<TdTypeEnum> {
    match &self.0 {
      Either::Left(typ) => Some(typ.clone()),
      Either::Right(symbol) => evaluate_type(db, *symbol).typ(db),
    }
  }

  pub fn as_eager(&self) -> Option<&TdTypeEnum> {
    match &self.0 {
      Either::Left(typ) => Some(typ),
      Either::Right(_) => None,
    }
  }
}

impl Encodable for LazyType {
  fn encode(&self, buf: &mut Vec<u8>, encoder: &mut Encoder) {
    self.0.encode(buf, encoder);
  }
}

impl Decodable for LazyType {
  fn decode(data: &mut &[u8], decoder: &Decoder) -> Self {
    LazyType(Either::<TdTypeEnum, Symbol>::decode(data, decoder))
  }
}

impl StableHash for LazyType {
  fn stable_hash<DB: QueryDatabase + ?Sized>(&self, db: &DB, hasher: &mut StableHasher) {
    self.0.stable_hash(db, hasher);
  }
}

/// A concrete literal value used in literal constraints
#[derive(Debug, Clone, PartialEq, Eq, Hash, StableCompare)]
pub enum LiteralValue {
  Str(String),
  Bool(bool),
  // f64 cannot be hashed so we store in string
  Num(String),
}

#[derive(FromRepr)]
#[repr(u8)]
enum LiteralValueTag {
  Str = 0,
  Bool = 1,
  Num = 2,
}

impl Encodable for LiteralValue {
  fn encode(&self, buf: &mut Vec<u8>, encoder: &mut Encoder) {
    match self {
      LiteralValue::Str(val) => {
        encoder.emit_u8(buf, LiteralValueTag::Str as u8);
        val.encode(buf, encoder);
      }
      LiteralValue::Bool(val) => {
        encoder.emit_u8(buf, LiteralValueTag::Bool as u8);
        val.encode(buf, encoder);
      }
      LiteralValue::Num(val) => {
        encoder.emit_u8(buf, LiteralValueTag::Num as u8);
        val.encode(buf, encoder);
      }
    }
  }
}

impl Decodable for LiteralValue {
  fn decode(data: &mut &[u8], decoder: &Decoder) -> Self {
    let tag = decoder.read_u8(data);
    match LiteralValueTag::from_repr(tag).expect("unknown LiteralValue tag") {
      LiteralValueTag::Str => LiteralValue::Str(String::decode(data, decoder)),
      LiteralValueTag::Bool => LiteralValue::Bool(bool::decode(data, decoder)),
      LiteralValueTag::Num => LiteralValue::Num(String::decode(data, decoder)),
    }
  }
}

impl StableHash for LiteralValue {
  fn stable_hash<DB: QueryDatabase + ?Sized>(&self, db: &DB, hasher: &mut StableHasher) {
    std::mem::discriminant(self).stable_hash(db, hasher);
    match self {
      LiteralValue::Str(value) => value.stable_hash(db, hasher),
      LiteralValue::Bool(value) => value.stable_hash(db, hasher),
      LiteralValue::Num(value) => value.stable_hash(db, hasher),
    }
  }
}
