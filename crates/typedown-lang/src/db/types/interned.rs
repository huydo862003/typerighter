use std::hash::Hasher;

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
    const OPTIONAL = 0b0000_0001;
  }
}

impl StableHash for TypeMemberDescriptors {
  fn stable_hash<DB: QueryDatabase + ?Sized>(&self, _db: &DB, hasher: &mut StableHasher) {
    hasher.write_u8(self.bits());
  }
}

/// The type of a type member field
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum MemberType {
  /// A reference to a type: either an evaluated type or a lazy schema symbol
  Simple(Either<TdTypeEnum, Symbol>),
  /// A union or enum type: each arm is itself a `TypeMember` (a type ref)
  Sum(Vec<TypeMember>),
  /// A literal value constraint (e.g. `"foo"`, `42`, `true`)
  Literal(LiteralValue),
  /// A list whose members are of the sum type
  ListOfSum(Vec<TypeMember>),
  /// A dict whose values are of the sum type
  DictOfSum(Vec<TypeMember>),
  /// The bottom type: no value can be assigned to this field
  Never,
}

impl MemberType {
  pub fn eager_simple(typ: TdTypeEnum) -> Self {
    MemberType::Simple(Either::Left(typ))
  }

  pub fn lazy_simple(symbol: Symbol) -> Self {
    MemberType::Simple(Either::Right(symbol))
  }

  pub fn evaluate_simple(&self, db: &TypedownDatabase) -> Option<TdTypeEnum> {
    match self {
      MemberType::Simple(Either::Left(typ)) => Some(typ.clone()),
      MemberType::Simple(Either::Right(symbol)) => evaluate_type(db, *symbol).typ(db),
      _ => None,
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
  Never = 5,
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
      MemberTypeTag::Simple => {
        MemberType::Simple(Either::<TdTypeEnum, Symbol>::decode(data, decoder))
      }
      MemberTypeTag::Sum => MemberType::Sum(Vec::decode(data, decoder)),
      MemberTypeTag::Literal => MemberType::Literal(LiteralValue::decode(data, decoder)),
      MemberTypeTag::ListOfSum => MemberType::ListOfSum(Vec::decode(data, decoder)),
      MemberTypeTag::DictOfSum => MemberType::DictOfSum(Vec::decode(data, decoder)),
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
