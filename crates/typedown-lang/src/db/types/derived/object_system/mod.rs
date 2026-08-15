mod base;
mod blob;
mod bool;
mod datetime;
mod dict;
mod func;
mod list;
mod math;
mod native_fn;
mod never;
mod null;
mod num;
mod product;
mod str;
mod vault;

use std::hash::{Hash, Hasher};

use strum::FromRepr;

use typedown_incremental::{
  Decodable, Decoder, Encodable, Encoder, FieldDecodable, FieldEncodable,
};

pub use base::*;
pub use blob::*;
pub use bool::*;
pub use datetime::*;
pub use dict::*;
pub use func::*;
pub use list::*;
pub use math::*;
pub use native_fn::*;
pub use never::*;
pub use null::*;
pub use num::*;
pub use product::*;
pub use str::*;
pub use vault::*;

use ambassador::Delegate;
use derive_more::From;
use enum_as_inner::EnumAsInner;

use crate::db::types::{InstResult, LazyType};
use typedown_incremental::Id;

/// Use this instead of dyn
/// The primitive types are fixed anyways
#[derive(Debug, Clone, From, Delegate, EnumAsInner)]
#[delegate(TdObjectLike)]
#[delegate(TdTypeLike)]
pub enum TdTypeEnum {
  TdTypeType(TdTypeType),
  TdObjectType(TdObjectType),
  TdBoolType(TdBoolType),
  TdStrType(TdStrType),
  TdNumType(TdNumType),
  TdMathType(TdMathType),
  TdFuncType(TdFuncType),
  TdListType(TdListType),
  TdDictType(TdDictType),
  TdDateTimeType(TdDateTimeType),
  TdDateType(TdDateType),
  TdTimeType(TdTimeType),
  TdProductType(TdProductType),
  TdBlobType(TdBlobType),
  TdNullType(TdNullType),
  TdNeverType(TdNeverType),
}

/// Use this instead of dyn
/// The primitive object kinds are fixed anyways
#[derive(Debug, Clone, From, Delegate, EnumAsInner)]
#[delegate(TdObjectLike)]
pub enum TdObjectEnum {
  // Types are objects
  TdTypeType(TdTypeType),
  TdObjectType(TdObjectType),
  TdBoolType(TdBoolType),
  TdStrType(TdStrType),
  TdNumType(TdNumType),
  TdMathType(TdMathType),
  TdFuncType(TdFuncType),
  TdListType(TdListType),
  TdDictType(TdDictType),
  TdDateTimeType(TdDateTimeType),
  TdDateType(TdDateType),
  TdTimeType(TdTimeType),
  TdProductType(TdProductType),
  TdBlobType(TdBlobType),
  TdNullType(TdNullType),
  TdNeverType(TdNeverType),
  // Objects
  TdBoolObj(TdBoolObj),
  TdStrObj(TdStrObj),
  TdNumObj(TdNumObj),
  TdMathObj(TdMathObj),
  TdFuncObj(TdFuncObj),
  TdListObj(TdListObj),
  TdDictObj(TdDictObj),
  TdDateTimeObj(TdDateTimeObj),
  TdDateObj(TdDateObj),
  TdTimeObj(TdTimeObj),
  TdProductObj(TdProductObj),
  TdBlobObj(TdBlobObj),
  TdNullObj(TdNullObj),
  TdVaultObj(TdVaultObj),
}

impl Id for TdTypeEnum {
  fn as_id(&self) -> (usize, usize) {
    match self {
      TdTypeEnum::TdTypeType(v) => v.as_id(),
      TdTypeEnum::TdObjectType(v) => v.as_id(),
      TdTypeEnum::TdBoolType(v) => v.as_id(),
      TdTypeEnum::TdStrType(v) => v.as_id(),
      TdTypeEnum::TdNumType(v) => v.as_id(),
      TdTypeEnum::TdMathType(v) => v.as_id(),
      TdTypeEnum::TdFuncType(v) => v.as_id(),
      TdTypeEnum::TdListType(v) => v.as_id(),
      TdTypeEnum::TdDictType(v) => v.as_id(),
      TdTypeEnum::TdDateTimeType(v) => v.as_id(),
      TdTypeEnum::TdDateType(v) => v.as_id(),
      TdTypeEnum::TdTimeType(v) => v.as_id(),
      TdTypeEnum::TdProductType(v) => v.as_id(),
      TdTypeEnum::TdBlobType(v) => v.as_id(),
      TdTypeEnum::TdNullType(v) => v.as_id(),
      TdTypeEnum::TdNeverType(v) => v.as_id(),
    }
  }
}

impl Id for TdObjectEnum {
  fn as_id(&self) -> (usize, usize) {
    match self {
      TdObjectEnum::TdTypeType(v) => v.as_id(),
      TdObjectEnum::TdObjectType(v) => v.as_id(),
      TdObjectEnum::TdBoolType(v) => v.as_id(),
      TdObjectEnum::TdStrType(v) => v.as_id(),
      TdObjectEnum::TdNumType(v) => v.as_id(),
      TdObjectEnum::TdMathType(v) => v.as_id(),
      TdObjectEnum::TdFuncType(v) => v.as_id(),
      TdObjectEnum::TdListType(v) => v.as_id(),
      TdObjectEnum::TdDictType(v) => v.as_id(),
      TdObjectEnum::TdDateTimeType(v) => v.as_id(),
      TdObjectEnum::TdDateType(v) => v.as_id(),
      TdObjectEnum::TdTimeType(v) => v.as_id(),
      TdObjectEnum::TdProductType(v) => v.as_id(),
      TdObjectEnum::TdBlobType(v) => v.as_id(),
      TdObjectEnum::TdNullType(v) => v.as_id(),
      TdObjectEnum::TdNeverType(v) => v.as_id(),
      TdObjectEnum::TdBoolObj(v) => v.as_id(),
      TdObjectEnum::TdStrObj(v) => v.as_id(),
      TdObjectEnum::TdNumObj(v) => v.as_id(),
      TdObjectEnum::TdMathObj(v) => v.as_id(),
      TdObjectEnum::TdFuncObj(v) => v.as_id(),
      TdObjectEnum::TdListObj(v) => v.as_id(),
      TdObjectEnum::TdDictObj(v) => v.as_id(),
      TdObjectEnum::TdDateTimeObj(v) => v.as_id(),
      TdObjectEnum::TdDateObj(v) => v.as_id(),
      TdObjectEnum::TdTimeObj(v) => v.as_id(),
      TdObjectEnum::TdProductObj(v) => v.as_id(),
      TdObjectEnum::TdBlobObj(v) => v.as_id(),
      TdObjectEnum::TdNullObj(v) => v.as_id(),
      TdObjectEnum::TdVaultObj(v) => v.as_id(),
    }
  }
}

impl From<TdTypeEnum> for TdObjectEnum {
  fn from(ty: TdTypeEnum) -> Self {
    match ty {
      TdTypeEnum::TdTypeType(v) => TdObjectEnum::TdTypeType(v),
      TdTypeEnum::TdObjectType(v) => TdObjectEnum::TdObjectType(v),
      TdTypeEnum::TdBoolType(v) => TdObjectEnum::TdBoolType(v),
      TdTypeEnum::TdStrType(v) => TdObjectEnum::TdStrType(v),
      TdTypeEnum::TdNumType(v) => TdObjectEnum::TdNumType(v),
      TdTypeEnum::TdMathType(v) => TdObjectEnum::TdMathType(v),
      TdTypeEnum::TdFuncType(v) => TdObjectEnum::TdFuncType(v),
      TdTypeEnum::TdListType(v) => TdObjectEnum::TdListType(v),
      TdTypeEnum::TdDictType(v) => TdObjectEnum::TdDictType(v),
      TdTypeEnum::TdDateTimeType(v) => TdObjectEnum::TdDateTimeType(v),
      TdTypeEnum::TdDateType(v) => TdObjectEnum::TdDateType(v),
      TdTypeEnum::TdTimeType(v) => TdObjectEnum::TdTimeType(v),
      TdTypeEnum::TdProductType(v) => TdObjectEnum::TdProductType(v),
      TdTypeEnum::TdBlobType(v) => TdObjectEnum::TdBlobType(v),
      TdTypeEnum::TdNullType(v) => TdObjectEnum::TdNullType(v),
      TdTypeEnum::TdNeverType(v) => TdObjectEnum::TdNeverType(v),
    }
  }
}

impl TdObjectEnum {
  pub fn as_type(self) -> Option<TdTypeEnum> {
    match self {
      TdObjectEnum::TdTypeType(v) => Some(TdTypeEnum::TdTypeType(v)),
      TdObjectEnum::TdObjectType(v) => Some(TdTypeEnum::TdObjectType(v)),
      TdObjectEnum::TdBoolType(v) => Some(TdTypeEnum::TdBoolType(v)),
      TdObjectEnum::TdStrType(v) => Some(TdTypeEnum::TdStrType(v)),
      TdObjectEnum::TdNumType(v) => Some(TdTypeEnum::TdNumType(v)),
      TdObjectEnum::TdMathType(v) => Some(TdTypeEnum::TdMathType(v)),
      TdObjectEnum::TdFuncType(v) => Some(TdTypeEnum::TdFuncType(v)),
      TdObjectEnum::TdListType(v) => Some(TdTypeEnum::TdListType(v)),
      TdObjectEnum::TdDictType(v) => Some(TdTypeEnum::TdDictType(v)),
      TdObjectEnum::TdDateTimeType(v) => Some(TdTypeEnum::TdDateTimeType(v)),
      TdObjectEnum::TdDateType(v) => Some(TdTypeEnum::TdDateType(v)),
      TdObjectEnum::TdTimeType(v) => Some(TdTypeEnum::TdTimeType(v)),
      TdObjectEnum::TdProductType(v) => Some(TdTypeEnum::TdProductType(v)),
      TdObjectEnum::TdBlobType(v) => Some(TdTypeEnum::TdBlobType(v)),
      TdObjectEnum::TdNullType(v) => Some(TdTypeEnum::TdNullType(v)),
      TdObjectEnum::TdNeverType(v) => Some(TdTypeEnum::TdNeverType(v)),
      _ => None,
    }
  }
}

impl PartialEq for TdTypeEnum {
  fn eq(&self, other: &Self) -> bool {
    self.as_id() == other.as_id()
  }
}
impl Eq for TdTypeEnum {}

impl Hash for TdTypeEnum {
  fn hash<H: Hasher>(&self, state: &mut H) {
    self.as_id().hash(state);
  }
}

impl PartialEq for TdObjectEnum {
  fn eq(&self, other: &Self) -> bool {
    self.as_id() == other.as_id()
  }
}
impl Eq for TdObjectEnum {}

impl Hash for TdObjectEnum {
  fn hash<H: Hasher>(&self, state: &mut H) {
    self.as_id().hash(state);
  }
}

impl typedown_incremental::StableHash for TdTypeEnum {
  fn stable_hash<DB: typedown_incremental::QueryDatabase + ?Sized>(
    &self,
    db: &DB,
    hasher: &mut typedown_incremental::StableHasher,
  ) {
    match self {
      TdTypeEnum::TdTypeType(v) => v.stable_hash(db, hasher),
      TdTypeEnum::TdObjectType(v) => v.stable_hash(db, hasher),
      TdTypeEnum::TdBoolType(v) => v.stable_hash(db, hasher),
      TdTypeEnum::TdStrType(v) => v.stable_hash(db, hasher),
      TdTypeEnum::TdNumType(v) => v.stable_hash(db, hasher),
      TdTypeEnum::TdMathType(v) => v.stable_hash(db, hasher),
      TdTypeEnum::TdFuncType(v) => v.stable_hash(db, hasher),
      TdTypeEnum::TdListType(v) => v.stable_hash(db, hasher),
      TdTypeEnum::TdDictType(v) => v.stable_hash(db, hasher),
      TdTypeEnum::TdDateTimeType(v) => v.stable_hash(db, hasher),
      TdTypeEnum::TdDateType(v) => v.stable_hash(db, hasher),
      TdTypeEnum::TdTimeType(v) => v.stable_hash(db, hasher),
      TdTypeEnum::TdProductType(v) => v.stable_hash(db, hasher),
      TdTypeEnum::TdBlobType(v) => v.stable_hash(db, hasher),
      TdTypeEnum::TdNullType(v) => v.stable_hash(db, hasher),
      TdTypeEnum::TdNeverType(v) => v.stable_hash(db, hasher),
    }
  }
}

impl typedown_incremental::StableHash for TdObjectEnum {
  fn stable_hash<DB: typedown_incremental::QueryDatabase + ?Sized>(
    &self,
    db: &DB,
    hasher: &mut typedown_incremental::StableHasher,
  ) {
    match self {
      TdObjectEnum::TdTypeType(v) => v.stable_hash(db, hasher),
      TdObjectEnum::TdObjectType(v) => v.stable_hash(db, hasher),
      TdObjectEnum::TdBoolType(v) => v.stable_hash(db, hasher),
      TdObjectEnum::TdStrType(v) => v.stable_hash(db, hasher),
      TdObjectEnum::TdNumType(v) => v.stable_hash(db, hasher),
      TdObjectEnum::TdMathType(v) => v.stable_hash(db, hasher),
      TdObjectEnum::TdFuncType(v) => v.stable_hash(db, hasher),
      TdObjectEnum::TdListType(v) => v.stable_hash(db, hasher),
      TdObjectEnum::TdDictType(v) => v.stable_hash(db, hasher),
      TdObjectEnum::TdDateTimeType(v) => v.stable_hash(db, hasher),
      TdObjectEnum::TdDateType(v) => v.stable_hash(db, hasher),
      TdObjectEnum::TdTimeType(v) => v.stable_hash(db, hasher),
      TdObjectEnum::TdProductType(v) => v.stable_hash(db, hasher),
      TdObjectEnum::TdBlobType(v) => v.stable_hash(db, hasher),
      TdObjectEnum::TdNullType(v) => v.stable_hash(db, hasher),
      TdObjectEnum::TdNeverType(v) => v.stable_hash(db, hasher),
      TdObjectEnum::TdBoolObj(v) => v.stable_hash(db, hasher),
      TdObjectEnum::TdStrObj(v) => v.stable_hash(db, hasher),
      TdObjectEnum::TdNumObj(v) => v.stable_hash(db, hasher),
      TdObjectEnum::TdMathObj(v) => v.stable_hash(db, hasher),
      TdObjectEnum::TdFuncObj(v) => v.stable_hash(db, hasher),
      TdObjectEnum::TdListObj(v) => v.stable_hash(db, hasher),
      TdObjectEnum::TdDictObj(v) => v.stable_hash(db, hasher),
      TdObjectEnum::TdDateTimeObj(v) => v.stable_hash(db, hasher),
      TdObjectEnum::TdDateObj(v) => v.stable_hash(db, hasher),
      TdObjectEnum::TdTimeObj(v) => v.stable_hash(db, hasher),
      TdObjectEnum::TdProductObj(v) => v.stable_hash(db, hasher),
      TdObjectEnum::TdBlobObj(v) => v.stable_hash(db, hasher),
      TdObjectEnum::TdNullObj(v) => v.stable_hash(db, hasher),
      TdObjectEnum::TdVaultObj(v) => v.stable_hash(db, hasher),
    }
  }
}

#[derive(FromRepr)]
#[repr(u8)]
pub enum TdTypeKind {
  Type = 0,
  Object = 1,
  Str = 2,
  Bool = 3,
  Num = 4,
  Math = 5,
  List = 6,
  Dict = 7,
  Func = 8,
  Product = 9,
  DateTime = 10,
  Date = 11,
  Time = 12,
  Blob = 13,
  Null = 14,
  Never = 15,
}

#[derive(FromRepr)]
#[repr(u8)]
pub enum TdObjectKind {
  // Types (also objects)
  Type = 0,
  Object = 1,
  Str = 2,
  Bool = 3,
  Num = 4,
  Math = 5,
  List = 6,
  Dict = 7,
  Func = 8,
  Product = 9,
  DateTime = 10,
  Date = 11,
  Time = 12,
  Blob = 13,
  Null = 14,
  Never = 15,
  // Object-only
  StrObj = 128,
  BoolObj = 129,
  NumObj = 130,
  MathObj = 131,
  ListObj = 132,
  DictObj = 133,
  FuncObj = 134,
  ProductObj = 135,
  DateTimeObj = 136,
  DateObj = 137,
  TimeObj = 138,
  BlobObj = 139,
  VaultObj = 140,
  NullObj = 141,
}

// TdTypeEnum
impl Encodable for TdTypeEnum {
  fn encode(&self, buf: &mut Vec<u8>, encoder: &mut Encoder) {
    match self {
      TdTypeEnum::TdTypeType(v) => {
        encoder.emit_u8(buf, TdTypeKind::Type as u8);
        v.encode_field(buf, encoder);
      }
      TdTypeEnum::TdObjectType(v) => {
        encoder.emit_u8(buf, TdTypeKind::Object as u8);
        v.encode_field(buf, encoder);
      }
      TdTypeEnum::TdStrType(v) => {
        encoder.emit_u8(buf, TdTypeKind::Str as u8);
        v.encode_field(buf, encoder);
      }
      TdTypeEnum::TdBoolType(v) => {
        encoder.emit_u8(buf, TdTypeKind::Bool as u8);
        v.encode_field(buf, encoder);
      }
      TdTypeEnum::TdNumType(v) => {
        encoder.emit_u8(buf, TdTypeKind::Num as u8);
        v.encode_field(buf, encoder);
      }
      TdTypeEnum::TdMathType(v) => {
        encoder.emit_u8(buf, TdTypeKind::Math as u8);
        v.encode_field(buf, encoder);
      }
      TdTypeEnum::TdListType(v) => {
        encoder.emit_u8(buf, TdTypeKind::List as u8);
        v.encode_field(buf, encoder);
      }
      TdTypeEnum::TdDictType(v) => {
        encoder.emit_u8(buf, TdTypeKind::Dict as u8);
        v.encode_field(buf, encoder);
      }
      TdTypeEnum::TdFuncType(v) => {
        encoder.emit_u8(buf, TdTypeKind::Func as u8);
        v.encode_field(buf, encoder);
      }
      TdTypeEnum::TdProductType(v) => {
        encoder.emit_u8(buf, TdTypeKind::Product as u8);
        v.encode_field(buf, encoder);
      }
      TdTypeEnum::TdDateTimeType(v) => {
        encoder.emit_u8(buf, TdTypeKind::DateTime as u8);
        v.encode_field(buf, encoder);
      }
      TdTypeEnum::TdDateType(v) => {
        encoder.emit_u8(buf, TdTypeKind::Date as u8);
        v.encode_field(buf, encoder);
      }
      TdTypeEnum::TdTimeType(v) => {
        encoder.emit_u8(buf, TdTypeKind::Time as u8);
        v.encode_field(buf, encoder);
      }
      TdTypeEnum::TdBlobType(v) => {
        encoder.emit_u8(buf, TdTypeKind::Blob as u8);
        v.encode_field(buf, encoder);
      }
      TdTypeEnum::TdNullType(v) => {
        encoder.emit_u8(buf, TdTypeKind::Null as u8);
        v.encode_field(buf, encoder);
      }
      TdTypeEnum::TdNeverType(v) => {
        encoder.emit_u8(buf, TdTypeKind::Never as u8);
        v.encode_field(buf, encoder);
      }
    }
  }
}

impl Decodable for TdTypeEnum {
  fn decode(data: &mut &[u8], decoder: &Decoder) -> Self {
    let tag = decoder.read_u8(data);
    match TdTypeKind::from_repr(tag).expect("unknown TdTypeKind tag") {
      TdTypeKind::Type => TdTypeType::decode_field(data, decoder).into(),
      TdTypeKind::Object => TdObjectType::decode_field(data, decoder).into(),
      TdTypeKind::Str => TdStrType::decode_field(data, decoder).into(),
      TdTypeKind::Bool => TdBoolType::decode_field(data, decoder).into(),
      TdTypeKind::Num => TdNumType::decode_field(data, decoder).into(),
      TdTypeKind::Math => TdMathType::decode_field(data, decoder).into(),
      TdTypeKind::List => TdListType::decode_field(data, decoder).into(),
      TdTypeKind::Dict => TdDictType::decode_field(data, decoder).into(),
      TdTypeKind::Func => TdFuncType::decode_field(data, decoder).into(),
      TdTypeKind::Product => TdProductType::decode_field(data, decoder).into(),
      TdTypeKind::DateTime => TdDateTimeType::decode_field(data, decoder).into(),
      TdTypeKind::Date => TdDateType::decode_field(data, decoder).into(),
      TdTypeKind::Time => TdTimeType::decode_field(data, decoder).into(),
      TdTypeKind::Blob => TdBlobType::decode_field(data, decoder).into(),
      TdTypeKind::Null => TdNullType::decode_field(data, decoder).into(),
      TdTypeKind::Never => TdNeverType::decode_field(data, decoder).into(),
    }
  }
}

// TdObjectEnum
impl Encodable for TdObjectEnum {
  fn encode(&self, buf: &mut Vec<u8>, encoder: &mut Encoder) {
    match self {
      // Types
      TdObjectEnum::TdTypeType(v) => {
        encoder.emit_u8(buf, TdObjectKind::Type as u8);
        v.encode_field(buf, encoder);
      }
      TdObjectEnum::TdObjectType(v) => {
        encoder.emit_u8(buf, TdObjectKind::Object as u8);
        v.encode_field(buf, encoder);
      }
      TdObjectEnum::TdStrType(v) => {
        encoder.emit_u8(buf, TdObjectKind::Str as u8);
        v.encode_field(buf, encoder);
      }
      TdObjectEnum::TdBoolType(v) => {
        encoder.emit_u8(buf, TdObjectKind::Bool as u8);
        v.encode_field(buf, encoder);
      }
      TdObjectEnum::TdNumType(v) => {
        encoder.emit_u8(buf, TdObjectKind::Num as u8);
        v.encode_field(buf, encoder);
      }
      TdObjectEnum::TdMathType(v) => {
        encoder.emit_u8(buf, TdObjectKind::Math as u8);
        v.encode_field(buf, encoder);
      }
      TdObjectEnum::TdListType(v) => {
        encoder.emit_u8(buf, TdObjectKind::List as u8);
        v.encode_field(buf, encoder);
      }
      TdObjectEnum::TdDictType(v) => {
        encoder.emit_u8(buf, TdObjectKind::Dict as u8);
        v.encode_field(buf, encoder);
      }
      TdObjectEnum::TdFuncType(v) => {
        encoder.emit_u8(buf, TdObjectKind::Func as u8);
        v.encode_field(buf, encoder);
      }
      TdObjectEnum::TdProductType(v) => {
        encoder.emit_u8(buf, TdObjectKind::Product as u8);
        v.encode_field(buf, encoder);
      }
      TdObjectEnum::TdDateTimeType(v) => {
        encoder.emit_u8(buf, TdObjectKind::DateTime as u8);
        v.encode_field(buf, encoder);
      }
      TdObjectEnum::TdDateType(v) => {
        encoder.emit_u8(buf, TdObjectKind::Date as u8);
        v.encode_field(buf, encoder);
      }
      TdObjectEnum::TdTimeType(v) => {
        encoder.emit_u8(buf, TdObjectKind::Time as u8);
        v.encode_field(buf, encoder);
      }
      TdObjectEnum::TdBlobType(v) => {
        encoder.emit_u8(buf, TdObjectKind::Blob as u8);
        v.encode_field(buf, encoder);
      }
      TdObjectEnum::TdNullType(v) => {
        encoder.emit_u8(buf, TdObjectKind::Null as u8);
        v.encode_field(buf, encoder);
      }
      TdObjectEnum::TdNeverType(v) => {
        encoder.emit_u8(buf, TdObjectKind::Never as u8);
        v.encode_field(buf, encoder);
      }
      // Objects
      TdObjectEnum::TdStrObj(v) => {
        encoder.emit_u8(buf, TdObjectKind::StrObj as u8);
        v.encode_field(buf, encoder);
      }
      TdObjectEnum::TdBoolObj(v) => {
        encoder.emit_u8(buf, TdObjectKind::BoolObj as u8);
        v.encode_field(buf, encoder);
      }
      TdObjectEnum::TdNumObj(v) => {
        encoder.emit_u8(buf, TdObjectKind::NumObj as u8);
        v.encode_field(buf, encoder);
      }
      TdObjectEnum::TdMathObj(v) => {
        encoder.emit_u8(buf, TdObjectKind::MathObj as u8);
        v.encode_field(buf, encoder);
      }
      TdObjectEnum::TdListObj(v) => {
        encoder.emit_u8(buf, TdObjectKind::ListObj as u8);
        v.encode_field(buf, encoder);
      }
      TdObjectEnum::TdDictObj(v) => {
        encoder.emit_u8(buf, TdObjectKind::DictObj as u8);
        v.encode_field(buf, encoder);
      }
      TdObjectEnum::TdFuncObj(v) => {
        encoder.emit_u8(buf, TdObjectKind::FuncObj as u8);
        v.encode_field(buf, encoder);
      }
      TdObjectEnum::TdProductObj(v) => {
        encoder.emit_u8(buf, TdObjectKind::ProductObj as u8);
        v.encode_field(buf, encoder);
      }
      TdObjectEnum::TdDateTimeObj(v) => {
        encoder.emit_u8(buf, TdObjectKind::DateTimeObj as u8);
        v.encode_field(buf, encoder);
      }
      TdObjectEnum::TdDateObj(v) => {
        encoder.emit_u8(buf, TdObjectKind::DateObj as u8);
        v.encode_field(buf, encoder);
      }
      TdObjectEnum::TdTimeObj(v) => {
        encoder.emit_u8(buf, TdObjectKind::TimeObj as u8);
        v.encode_field(buf, encoder);
      }
      TdObjectEnum::TdBlobObj(v) => {
        encoder.emit_u8(buf, TdObjectKind::BlobObj as u8);
        v.encode_field(buf, encoder);
      }
      TdObjectEnum::TdNullObj(v) => {
        encoder.emit_u8(buf, TdObjectKind::NullObj as u8);
        v.encode_field(buf, encoder);
      }
      TdObjectEnum::TdVaultObj(v) => {
        encoder.emit_u8(buf, TdObjectKind::VaultObj as u8);
        v.encode_field(buf, encoder);
      }
    }
  }
}

impl Decodable for TdObjectEnum {
  fn decode(data: &mut &[u8], decoder: &Decoder) -> Self {
    let tag = decoder.read_u8(data);
    match TdObjectKind::from_repr(tag).expect("unknown TdObjectKind tag") {
      TdObjectKind::Type => TdTypeType::decode_field(data, decoder).into(),
      TdObjectKind::Object => TdObjectType::decode_field(data, decoder).into(),
      TdObjectKind::Str => TdStrType::decode_field(data, decoder).into(),
      TdObjectKind::Bool => TdBoolType::decode_field(data, decoder).into(),
      TdObjectKind::Num => TdNumType::decode_field(data, decoder).into(),
      TdObjectKind::Math => TdMathType::decode_field(data, decoder).into(),
      TdObjectKind::List => TdListType::decode_field(data, decoder).into(),
      TdObjectKind::Dict => TdDictType::decode_field(data, decoder).into(),
      TdObjectKind::Func => TdFuncType::decode_field(data, decoder).into(),
      TdObjectKind::Product => TdProductType::decode_field(data, decoder).into(),
      TdObjectKind::DateTime => TdDateTimeType::decode_field(data, decoder).into(),
      TdObjectKind::Date => TdDateType::decode_field(data, decoder).into(),
      TdObjectKind::Time => TdTimeType::decode_field(data, decoder).into(),
      TdObjectKind::Blob => TdBlobType::decode_field(data, decoder).into(),
      TdObjectKind::Null => TdNullType::decode_field(data, decoder).into(),
      TdObjectKind::Never => TdNeverType::decode_field(data, decoder).into(),
      TdObjectKind::StrObj => TdStrObj::decode_field(data, decoder).into(),
      TdObjectKind::BoolObj => TdBoolObj::decode_field(data, decoder).into(),
      TdObjectKind::NumObj => TdNumObj::decode_field(data, decoder).into(),
      TdObjectKind::MathObj => TdMathObj::decode_field(data, decoder).into(),
      TdObjectKind::ListObj => TdListObj::decode_field(data, decoder).into(),
      TdObjectKind::DictObj => TdDictObj::decode_field(data, decoder).into(),
      TdObjectKind::FuncObj => TdFuncObj::decode_field(data, decoder).into(),
      TdObjectKind::ProductObj => TdProductObj::decode_field(data, decoder).into(),
      TdObjectKind::DateTimeObj => TdDateTimeObj::decode_field(data, decoder).into(),
      TdObjectKind::DateObj => TdDateObj::decode_field(data, decoder).into(),
      TdObjectKind::TimeObj => TdTimeObj::decode_field(data, decoder).into(),
      TdObjectKind::BlobObj => TdBlobObj::decode_field(data, decoder).into(),
      TdObjectKind::NullObj => TdNullObj::decode_field(data, decoder).into(),
      TdObjectKind::VaultObj => TdVaultObj::decode_field(data, decoder).into(),
    }
  }
}
