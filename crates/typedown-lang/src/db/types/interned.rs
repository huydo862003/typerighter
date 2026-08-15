use std::collections::HashMap;
use std::hash::{Hash, Hasher};

use strum::FromRepr;
use typedown_macros::query_interned;

use typedown_incremental::{
  Decodable, Decoder, Encodable, Encoder, QueryDatabase, StableHash, StableHasher,
};

use typedown_types::either::Either;

use super::{Symbol, TdTypeEnum};
use crate::db::TypedownDatabase;
use crate::db::derived::evaluate::evaluate_type::evaluate_type;

#[query_interned]
pub struct FuncSignature {
  pub params: Vec<TdTypeEnum>,
  pub ret: TdTypeEnum,
}

bitflags::bitflags! {
  #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
  pub struct TypeMemberDescriptors: u8 {
  }
}

impl StableHash for TypeMemberDescriptors {
  fn stable_hash<DB: QueryDatabase + ?Sized>(&self, _db: &DB, hasher: &mut StableHasher) {
    hasher.write_u8(self.bits());
  }
}

// A type reference that may be eagerly resolved or lazily deferred to a symbol
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
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

/// The type of a type member field
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MemberType {
  /// A reference to a type, either eagerly resolved or lazily deferred
  Simple(LazyType),
  /// A union or enum type: each arm is itself a `TypeMember` (a type ref)
  Sum(Vec<TypeMember>),
  /// A literal value constraint (e.g. `"foo"`, `42`, `true`)
  Literal(LiteralValue),
  /// A list whose members are of the sum type
  ListOfSum(Vec<TypeMember>),
  /// A dict whose values are of the sum type
  DictOfSum(Vec<TypeMember>),
  // Anonymous field map for typechecking only, never exists at runtime
  Structural(HashMap<String, TypeMember>),
  /// The bottom type: no value can be assigned to this field
  Never,
}

impl MemberType {
  // Check if this type includes null (i.e. is T? or Sum containing null)
  // Null is always eagerly resolved so no query cycles can occur
  pub fn is_nullable(&self, db: &TypedownDatabase) -> bool {
    match self {
      MemberType::Simple(lazy) => lazy
        .as_eager()
        .is_some_and(|t| t.as_td_null_type().is_some()),
      MemberType::Sum(arms) => arms.iter().any(|arm| arm.typ(db).is_nullable(db)),
      _ => false,
    }
  }
}

impl Hash for MemberType {
  fn hash<H: Hasher>(&self, state: &mut H) {
    std::mem::discriminant(self).hash(state);
    match self {
      MemberType::Simple(v) => v.hash(state),
      MemberType::Sum(v) => v.hash(state),
      MemberType::Literal(v) => v.hash(state),
      MemberType::ListOfSum(v) => v.hash(state),
      MemberType::DictOfSum(v) => v.hash(state),
      MemberType::Structural(fields) => {
        let mut entries: Vec<_> = fields.iter().collect();
        entries.sort_by_key(|(k, _)| *k);
        for (k, v) in entries {
          k.hash(state);
          v.hash(state);
        }
      }
      MemberType::Never => {}
    }
  }
}

/// A concrete literal value used in literal constraints
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
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

#[derive(FromRepr)]
#[repr(u8)]
enum MemberTypeTag {
  Simple = 0,
  Sum = 1,
  Literal = 2,
  ListOfSum = 3,
  DictOfSum = 4,
  Structural = 5,
  Never = 6,
}

impl Encodable for MemberType {
  fn encode(&self, buf: &mut Vec<u8>, encoder: &mut Encoder) {
    match self {
      MemberType::Simple(typ) => {
        encoder.emit_u8(buf, MemberTypeTag::Simple as u8);
        typ.encode(buf, encoder);
      }
      MemberType::Sum(members) => {
        encoder.emit_u8(buf, MemberTypeTag::Sum as u8);
        members.encode(buf, encoder);
      }
      MemberType::Literal(value) => {
        encoder.emit_u8(buf, MemberTypeTag::Literal as u8);
        value.encode(buf, encoder);
      }
      MemberType::ListOfSum(value) => {
        encoder.emit_u8(buf, MemberTypeTag::ListOfSum as u8);
        value.encode(buf, encoder);
      }
      MemberType::DictOfSum(value) => {
        encoder.emit_u8(buf, MemberTypeTag::DictOfSum as u8);
        value.encode(buf, encoder);
      }
      MemberType::Structural(fields) => {
        encoder.emit_u8(buf, MemberTypeTag::Structural as u8);
        fields.encode(buf, encoder);
      }
      MemberType::Never => {
        encoder.emit_u8(buf, MemberTypeTag::Never as u8);
      }
    }
  }
}

impl Decodable for MemberType {
  fn decode(data: &mut &[u8], decoder: &Decoder) -> Self {
    let tag = decoder.read_u8(data);
    match MemberTypeTag::from_repr(tag).expect("unknown MemberType tag") {
      MemberTypeTag::Simple => MemberType::Simple(LazyType::decode(data, decoder)),
      MemberTypeTag::Sum => MemberType::Sum(Vec::decode(data, decoder)),
      MemberTypeTag::Literal => MemberType::Literal(LiteralValue::decode(data, decoder)),
      MemberTypeTag::ListOfSum => MemberType::ListOfSum(Vec::decode(data, decoder)),
      MemberTypeTag::DictOfSum => MemberType::DictOfSum(Vec::decode(data, decoder)),
      MemberTypeTag::Structural => MemberType::Structural(HashMap::decode(data, decoder)),
      MemberTypeTag::Never => MemberType::Never,
    }
  }
}

impl StableHash for MemberType {
  fn stable_hash<DB: QueryDatabase + ?Sized>(&self, db: &DB, hasher: &mut StableHasher) {
    std::mem::discriminant(self).stable_hash(db, hasher);
    match self {
      MemberType::Simple(typ) => typ.stable_hash(db, hasher),
      MemberType::Sum(members) => members.stable_hash(db, hasher),
      MemberType::Literal(value) => value.stable_hash(db, hasher),
      MemberType::ListOfSum(members) => members.stable_hash(db, hasher),
      MemberType::DictOfSum(members) => members.stable_hash(db, hasher),
      MemberType::Structural(fields) => fields.stable_hash(db, hasher),
      MemberType::Never => {}
    }
  }
}

#[query_interned]
pub struct TypeMember {
  pub typ: MemberType,
  pub descriptors: TypeMemberDescriptors,
}

impl Encodable for TypeMemberDescriptors {
  fn encode(&self, buf: &mut Vec<u8>, encoder: &mut Encoder) {
    encoder.emit_u8(buf, self.bits());
  }
}

impl Decodable for TypeMemberDescriptors {
  fn decode(data: &mut &[u8], decoder: &Decoder) -> Self {
    TypeMemberDescriptors::from_bits_truncate(decoder.read_u8(data))
  }
}
