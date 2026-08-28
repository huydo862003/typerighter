mod base;
mod blob;
mod bool;
mod datetime;
mod dict;
mod existential;
mod func;
mod list;
mod literal;
mod math;
mod native_fn;
mod never;
mod null;
mod num;
mod object;
mod product;
mod schema;
mod str;
mod sum;
mod variable;
mod vault;

use std::collections::HashMap;
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
pub use existential::*;
pub use func::*;
pub use list::*;
pub use literal::*;
pub use math::*;
pub use native_fn::*;
pub use never::*;
pub use null::*;
pub use num::*;
pub use object::*;
pub use product::*;
pub use schema::*;
pub use str::*;
pub use sum::*;
pub use variable::*;
pub use vault::*;

use ambassador::Delegate;
use derive_more::From;
use enum_as_inner::EnumAsInner;
use typedown_macros::StableCompare;

use crate::db::TypedownDatabase;
use crate::db::types::{FuncSignature, InstResult, LazyType, TypeParams};
use crate::syntax::diagnostic::Diagnostic;
use typedown_incremental::Id;

// Use this instead of dyn
// The primitive types are fixed anyways
#[derive(Debug, Clone, From, Delegate, EnumAsInner, StableCompare)]
#[delegate(TdRuntimeObject)]
#[delegate(TdStaticType<'x>, generics = "'x")]
pub enum TdTypeEnum<'db> {
  TdTypeType(TdTypeType<'db>),
  TdBoolType(TdBoolType<'db>),
  TdStrType(TdStrType<'db>),
  TdNumType(TdNumType<'db>),
  TdMathType(TdMathType<'db>),
  TdFuncType(TdFuncType<'db>),
  TdListType(TdListType<'db>),
  TdDictType(TdDictType<'db>),
  TdDateTimeType(TdDateTimeType<'db>),
  TdDateType(TdDateType<'db>),
  TdTimeType(TdTimeType<'db>),
  TdProductType(TdProductType<'db>),
  TdSchemaMetaType(TdSchemaMetaType<'db>),
  TdSchemaType(TdSchemaType<'db>),
  TdBlobType(TdBlobType<'db>),
  TdNullType(TdNullType<'db>),
  TdNeverType(TdNeverType<'db>),
  TdLiteralType(TdLiteralType<'db>),
  TdSumType(TdSumType<'db>),
  TdVariableType(TdVariableType<'db>),
  TdExistentialType(TdExistentialType<'db>),
  TdObjectType(TdObjectType<'db>),
}

// Use this instead of dyn
// The primitive object kinds are fixed anyways
#[derive(Debug, Clone, From, Delegate, EnumAsInner, StableCompare)]
#[delegate(TdRuntimeObject)]
pub enum TdObjectEnum<'db> {
  // Types are objects
  TdTypeObj(TdTypeEnum<'db>),
  // Objects
  TdBoolObj(TdBoolObj<'db>),
  TdStrObj(TdStrObj<'db>),
  TdNumObj(TdNumObj<'db>),
  TdMathObj(TdMathObj<'db>),
  TdFuncObj(TdFuncObj<'db>),
  TdListObj(TdListObj<'db>),
  TdDictObj(TdDictObj<'db>),
  TdDateTimeObj(TdDateTimeObj<'db>),
  TdDateObj(TdDateObj<'db>),
  TdTimeObj(TdTimeObj<'db>),
  TdProductObj(TdProductObj<'db>),
  TdSchemaObj(TdSchemaObj<'db>),
  TdBlobObj(TdBlobObj<'db>),
  TdNullObj(TdNullObj<'db>),
  TdVaultObj(TdVaultObj<'db>),
}

// Allow converting concrete type structs directly to TdObjectEnum via TdTypeEnum
macro_rules! impl_from_type_for_obj_enum {
  ($($ty:ident),+ $(,)?) => {
    $(
      impl<'db> From<$ty<'db>> for TdObjectEnum<'db> {
        fn from(v: $ty<'db>) -> Self {
          TdObjectEnum::TdTypeObj(TdTypeEnum::from(v))
        }
      }
    )+
  };
}

impl_from_type_for_obj_enum!(
  TdTypeType,
  TdBoolType,
  TdStrType,
  TdNumType,
  TdMathType,
  TdFuncType,
  TdListType,
  TdDictType,
  TdDateTimeType,
  TdDateType,
  TdTimeType,
  TdProductType,
  TdSchemaMetaType,
  TdSchemaType,
  TdBlobType,
  TdNullType,
  TdNeverType,
  TdLiteralType,
  TdSumType,
  TdVariableType,
  TdExistentialType,
  TdObjectType,
);

impl Id for TdTypeEnum<'_> {
  fn as_id(&self) -> (usize, usize) {
    match self {
      TdTypeEnum::TdTypeType(v) => v.as_id(),
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
      TdTypeEnum::TdSchemaMetaType(v) => v.as_id(),
      TdTypeEnum::TdSchemaType(v) => v.as_id(),
      TdTypeEnum::TdBlobType(v) => v.as_id(),
      TdTypeEnum::TdNullType(v) => v.as_id(),
      TdTypeEnum::TdNeverType(v) => v.as_id(),
      TdTypeEnum::TdLiteralType(v) => v.as_id(),
      TdTypeEnum::TdSumType(v) => v.as_id(),
      TdTypeEnum::TdVariableType(v) => v.as_id(),
      TdTypeEnum::TdExistentialType(v) => v.as_id(),
      TdTypeEnum::TdObjectType(v) => v.as_id(),
    }
  }
}

impl Id for TdObjectEnum<'_> {
  fn as_id(&self) -> (usize, usize) {
    match self {
      TdObjectEnum::TdTypeObj(v) => v.as_id(),
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
      TdObjectEnum::TdSchemaObj(v) => v.as_id(),
      TdObjectEnum::TdBlobObj(v) => v.as_id(),
      TdObjectEnum::TdNullObj(v) => v.as_id(),
      TdObjectEnum::TdVaultObj(v) => v.as_id(),
    }
  }
}

impl PartialEq for TdTypeEnum<'_> {
  fn eq(&self, other: &Self) -> bool {
    self.as_id() == other.as_id()
  }
}
impl Eq for TdTypeEnum<'_> {}

impl Hash for TdTypeEnum<'_> {
  fn hash<H: Hasher>(&self, state: &mut H) {
    self.as_id().hash(state);
  }
}

impl PartialEq for TdObjectEnum<'_> {
  fn eq(&self, other: &Self) -> bool {
    self.as_id() == other.as_id()
  }
}
impl Eq for TdObjectEnum<'_> {}

impl Hash for TdObjectEnum<'_> {
  fn hash<H: Hasher>(&self, state: &mut H) {
    self.as_id().hash(state);
  }
}

impl typedown_incremental::StableHash for TdTypeEnum<'_> {
  fn stable_hash<DB: typedown_incremental::QueryDatabase + ?Sized>(
    &self,
    db: &DB,
    hasher: &mut typedown_incremental::StableHasher,
  ) {
    match self {
      TdTypeEnum::TdTypeType(v) => v.stable_hash(db, hasher),
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
      TdTypeEnum::TdSchemaMetaType(v) => v.stable_hash(db, hasher),
      TdTypeEnum::TdSchemaType(v) => v.stable_hash(db, hasher),
      TdTypeEnum::TdBlobType(v) => v.stable_hash(db, hasher),
      TdTypeEnum::TdNullType(v) => v.stable_hash(db, hasher),
      TdTypeEnum::TdNeverType(v) => v.stable_hash(db, hasher),
      TdTypeEnum::TdLiteralType(v) => v.stable_hash(db, hasher),
      TdTypeEnum::TdSumType(v) => v.stable_hash(db, hasher),
      TdTypeEnum::TdVariableType(v) => v.stable_hash(db, hasher),
      TdTypeEnum::TdExistentialType(v) => v.stable_hash(db, hasher),
      TdTypeEnum::TdObjectType(v) => v.stable_hash(db, hasher),
    }
  }
}

impl typedown_incremental::StableHash for TdObjectEnum<'_> {
  fn stable_hash<DB: typedown_incremental::QueryDatabase + ?Sized>(
    &self,
    db: &DB,
    hasher: &mut typedown_incremental::StableHasher,
  ) {
    match self {
      TdObjectEnum::TdTypeObj(v) => v.stable_hash(db, hasher),
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
      TdObjectEnum::TdSchemaObj(v) => v.stable_hash(db, hasher),
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
  Literal = 16,
  Sum = 17,
  SchemaMetaType = 18,
  Schema = 19,
  Existential = 20,
  Variable = 21,
}

#[derive(FromRepr)]
#[repr(u8)]
pub enum TdObjectKind {
  // Types (wraps TdTypeEnum)
  Type = 0,
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
  SchemaObj = 142,
}

// TdTypeEnum
impl Encodable for TdTypeEnum<'_> {
  fn encode(&self, buf: &mut Vec<u8>, encoder: &mut Encoder) {
    match self {
      TdTypeEnum::TdTypeType(v) => {
        encoder.emit_u8(buf, TdTypeKind::Type as u8);
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
      TdTypeEnum::TdSchemaMetaType(v) => {
        encoder.emit_u8(buf, TdTypeKind::SchemaMetaType as u8);
        v.encode_field(buf, encoder);
      }
      TdTypeEnum::TdSchemaType(v) => {
        encoder.emit_u8(buf, TdTypeKind::Schema as u8);
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
      TdTypeEnum::TdLiteralType(v) => {
        encoder.emit_u8(buf, TdTypeKind::Literal as u8);
        v.encode_field(buf, encoder);
      }
      TdTypeEnum::TdSumType(v) => {
        encoder.emit_u8(buf, TdTypeKind::Sum as u8);
        v.encode_field(buf, encoder);
      }
      TdTypeEnum::TdVariableType(v) => {
        encoder.emit_u8(buf, TdTypeKind::Variable as u8);
        v.encode_field(buf, encoder);
      }
      TdTypeEnum::TdExistentialType(v) => {
        encoder.emit_u8(buf, TdTypeKind::Existential as u8);
        v.encode_field(buf, encoder);
      }
      TdTypeEnum::TdObjectType(v) => {
        encoder.emit_u8(buf, TdTypeKind::Object as u8);
        v.encode_field(buf, encoder);
      }
    }
  }
}

impl Decodable for TdTypeEnum<'_> {
  fn decode(data: &mut &[u8], decoder: &Decoder) -> Self {
    let tag = decoder.read_u8(data);
    match TdTypeKind::from_repr(tag).expect("unknown TdTypeKind tag") {
      TdTypeKind::Type => TdTypeType::decode_field(data, decoder).into(),
      TdTypeKind::Str => TdStrType::decode_field(data, decoder).into(),
      TdTypeKind::Bool => TdBoolType::decode_field(data, decoder).into(),
      TdTypeKind::Num => TdNumType::decode_field(data, decoder).into(),
      TdTypeKind::Math => TdMathType::decode_field(data, decoder).into(),
      TdTypeKind::List => TdListType::decode_field(data, decoder).into(),
      TdTypeKind::Dict => TdDictType::decode_field(data, decoder).into(),
      TdTypeKind::Func => TdFuncType::decode_field(data, decoder).into(),
      TdTypeKind::Product => TdProductType::decode_field(data, decoder).into(),
      TdTypeKind::SchemaMetaType => TdSchemaMetaType::decode_field(data, decoder).into(),
      TdTypeKind::Schema => TdSchemaType::decode_field(data, decoder).into(),
      TdTypeKind::DateTime => TdDateTimeType::decode_field(data, decoder).into(),
      TdTypeKind::Date => TdDateType::decode_field(data, decoder).into(),
      TdTypeKind::Time => TdTimeType::decode_field(data, decoder).into(),
      TdTypeKind::Blob => TdBlobType::decode_field(data, decoder).into(),
      TdTypeKind::Null => TdNullType::decode_field(data, decoder).into(),
      TdTypeKind::Never => TdNeverType::decode_field(data, decoder).into(),
      TdTypeKind::Literal => TdLiteralType::decode_field(data, decoder).into(),
      TdTypeKind::Sum => TdSumType::decode_field(data, decoder).into(),
      TdTypeKind::Existential => TdExistentialType::decode_field(data, decoder).into(),
      TdTypeKind::Variable => TdVariableType::decode_field(data, decoder).into(),
      TdTypeKind::Object => TdObjectType::decode_field(data, decoder).into(),
    }
  }
}

// TdObjectEnum
impl Encodable for TdObjectEnum<'_> {
  fn encode(&self, buf: &mut Vec<u8>, encoder: &mut Encoder) {
    match self {
      TdObjectEnum::TdTypeObj(v) => {
        encoder.emit_u8(buf, TdObjectKind::Type as u8);
        v.encode(buf, encoder);
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
      TdObjectEnum::TdSchemaObj(v) => {
        encoder.emit_u8(buf, TdObjectKind::SchemaObj as u8);
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

impl Decodable for TdObjectEnum<'_> {
  fn decode(data: &mut &[u8], decoder: &Decoder) -> Self {
    let tag = decoder.read_u8(data);
    match TdObjectKind::from_repr(tag) {
      Some(TdObjectKind::Type) => TdObjectEnum::TdTypeObj(TdTypeEnum::decode(data, decoder)),
      Some(TdObjectKind::StrObj) => TdStrObj::decode_field(data, decoder).into(),
      Some(TdObjectKind::BoolObj) => TdBoolObj::decode_field(data, decoder).into(),
      Some(TdObjectKind::NumObj) => TdNumObj::decode_field(data, decoder).into(),
      Some(TdObjectKind::MathObj) => TdMathObj::decode_field(data, decoder).into(),
      Some(TdObjectKind::ListObj) => TdListObj::decode_field(data, decoder).into(),
      Some(TdObjectKind::DictObj) => TdDictObj::decode_field(data, decoder).into(),
      Some(TdObjectKind::FuncObj) => TdFuncObj::decode_field(data, decoder).into(),
      Some(TdObjectKind::ProductObj) => TdProductObj::decode_field(data, decoder).into(),
      Some(TdObjectKind::SchemaObj) => TdSchemaObj::decode_field(data, decoder).into(),
      Some(TdObjectKind::DateTimeObj) => TdDateTimeObj::decode_field(data, decoder).into(),
      Some(TdObjectKind::DateObj) => TdDateObj::decode_field(data, decoder).into(),
      Some(TdObjectKind::TimeObj) => TdTimeObj::decode_field(data, decoder).into(),
      Some(TdObjectKind::BlobObj) => TdBlobObj::decode_field(data, decoder).into(),
      Some(TdObjectKind::NullObj) => TdNullObj::decode_field(data, decoder).into(),
      Some(TdObjectKind::VaultObj) => TdVaultObj::decode_field(data, decoder).into(),
      None => panic!("unknown TdObjectKind tag: {}", tag),
    }
  }
}
