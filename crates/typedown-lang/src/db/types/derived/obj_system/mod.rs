mod base;
mod blob;
mod bool;
mod datetime;
mod dict;
mod existential;
mod func;
mod icon;
mod list;
mod literal;
mod math;
mod native_func;
mod never;
mod null;
mod number;
mod obj;
mod product;
mod schema;
mod string;
mod sum;
pub mod typecheck;
mod variable;
mod vault;

use std::collections::BTreeMap;
use std::hash::{Hash, Hasher};

use strum::FromRepr;

use typedown_incremental::{Decodable, Decoder, Encodable, Encoder};

pub use base::*;
pub use blob::*;
pub use bool::*;
pub use datetime::*;
pub use dict::*;
pub use existential::*;
pub use func::*;
pub use icon::*;
pub use list::*;
pub use literal::*;
pub use math::*;
pub use native_func::*;
pub use never::*;
pub use null::*;
pub use number::*;
pub use obj::*;
pub use product::*;
pub use schema::*;
pub use string::*;
pub use sum::*;
pub use variable::*;
pub use vault::*;

use ambassador::Delegate;
use derive_more::From;
use enum_as_inner::EnumAsInner;
use typedown_macros::StableCompare;

use crate::db::TypedownDatabase;
use crate::db::types::{FuncSignature, InstantiateResult, LazyTyp, TypParams};
use crate::syntax::diagnostic::Diagnostic;
use typedown_incremental::{DepId, Id};

// Use this instead of dyn
// The primitive types are fixed anyways
#[derive(Debug, Clone, From, Delegate, EnumAsInner, StableCompare)]
#[delegate(TdRuntimeObj<'x>, generics = "'x")]
#[delegate(TdStaticTyp<'x>, generics = "'x")]
pub enum TdTypEnum<'db> {
  TdTypTyp(TdTypTyp<'db>),
  TdBoolTyp(TdBoolTyp<'db>),
  TdStringTyp(TdStringTyp<'db>),
  TdNumberTyp(TdNumberTyp<'db>),
  TdMathTyp(TdMathTyp<'db>),
  TdFuncTyp(TdFuncTyp<'db>),
  TdListTyp(TdListTyp<'db>),
  TdDictTyp(TdDictTyp<'db>),
  TdDateTimeTyp(TdDateTimeTyp<'db>),
  TdDateTyp(TdDateTyp<'db>),
  TdTimeTyp(TdTimeTyp<'db>),
  TdProductTyp(TdProductTyp<'db>),
  TdSchemaMetaTyp(TdSchemaMetaTyp<'db>),
  TdSchemaTyp(TdSchemaTyp<'db>),
  TdBlobTyp(TdBlobTyp<'db>),
  TdNullTyp(TdNullTyp<'db>),
  TdNeverTyp(TdNeverTyp<'db>),
  TdLitTyp(TdLitTyp<'db>),
  TdSumTyp(TdSumTyp<'db>),
  TdVariableTyp(TdVariableTyp<'db>),
  TdExistentialTyp(TdExistentialTyp<'db>),
  TdObjTyp(TdObjTyp<'db>),
  TdIconTyp(TdIconTyp<'db>),
}

// Use this instead of dyn
// The primitive object kinds are fixed anyways
#[derive(Debug, Clone, From, Delegate, EnumAsInner, StableCompare)]
#[delegate(TdRuntimeObj<'x>, generics = "'x")]
pub enum TdObjEnum<'db> {
  // Types are objects
  TdTypObj(TdTypEnum<'db>),
  // Objects
  TdBoolObj(TdBoolObj<'db>),
  TdStringObj(TdStringObj<'db>),
  TdNumberObj(TdNumberObj<'db>),
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
  TdIconObj(TdIconObj<'db>),
}

// Allow converting concrete type structs directly to TdObjectEnum via TdTypeEnum
macro_rules! impl_from_type_for_obj_enum {
  ($($ty:ident),+ $(,)?) => {
    $(
      impl<'db> From<$ty<'db>> for TdObjEnum<'db> {
        fn from(v: $ty<'db>) -> Self {
          TdObjEnum::TdTypObj(TdTypEnum::from(v))
        }
      }
    )+
  };
}

impl_from_type_for_obj_enum!(
  TdTypTyp,
  TdBoolTyp,
  TdStringTyp,
  TdNumberTyp,
  TdMathTyp,
  TdFuncTyp,
  TdListTyp,
  TdDictTyp,
  TdDateTimeTyp,
  TdDateTyp,
  TdTimeTyp,
  TdProductTyp,
  TdSchemaMetaTyp,
  TdSchemaTyp,
  TdBlobTyp,
  TdNullTyp,
  TdNeverTyp,
  TdLitTyp,
  TdSumTyp,
  TdVariableTyp,
  TdExistentialTyp,
  TdObjTyp,
  TdIconTyp,
);

impl Id for TdTypEnum<'_> {
  fn as_id(&self) -> DepId {
    match self {
      TdTypEnum::TdTypTyp(v) => v.as_id(),
      TdTypEnum::TdBoolTyp(v) => v.as_id(),
      TdTypEnum::TdStringTyp(v) => v.as_id(),
      TdTypEnum::TdNumberTyp(v) => v.as_id(),
      TdTypEnum::TdMathTyp(v) => v.as_id(),
      TdTypEnum::TdFuncTyp(v) => v.as_id(),
      TdTypEnum::TdListTyp(v) => v.as_id(),
      TdTypEnum::TdDictTyp(v) => v.as_id(),
      TdTypEnum::TdDateTimeTyp(v) => v.as_id(),
      TdTypEnum::TdDateTyp(v) => v.as_id(),
      TdTypEnum::TdTimeTyp(v) => v.as_id(),
      TdTypEnum::TdProductTyp(v) => v.as_id(),
      TdTypEnum::TdSchemaMetaTyp(v) => v.as_id(),
      TdTypEnum::TdSchemaTyp(v) => v.as_id(),
      TdTypEnum::TdBlobTyp(v) => v.as_id(),
      TdTypEnum::TdNullTyp(v) => v.as_id(),
      TdTypEnum::TdNeverTyp(v) => v.as_id(),
      TdTypEnum::TdLitTyp(v) => v.as_id(),
      TdTypEnum::TdSumTyp(v) => v.as_id(),
      TdTypEnum::TdVariableTyp(v) => v.as_id(),
      TdTypEnum::TdExistentialTyp(v) => v.as_id(),
      TdTypEnum::TdObjTyp(v) => v.as_id(),
      TdTypEnum::TdIconTyp(v) => v.as_id(),
    }
  }
}

impl Id for TdObjEnum<'_> {
  fn as_id(&self) -> DepId {
    match self {
      TdObjEnum::TdTypObj(v) => v.as_id(),
      TdObjEnum::TdBoolObj(v) => v.as_id(),
      TdObjEnum::TdStringObj(v) => v.as_id(),
      TdObjEnum::TdNumberObj(v) => v.as_id(),
      TdObjEnum::TdMathObj(v) => v.as_id(),
      TdObjEnum::TdFuncObj(v) => v.as_id(),
      TdObjEnum::TdListObj(v) => v.as_id(),
      TdObjEnum::TdDictObj(v) => v.as_id(),
      TdObjEnum::TdDateTimeObj(v) => v.as_id(),
      TdObjEnum::TdDateObj(v) => v.as_id(),
      TdObjEnum::TdTimeObj(v) => v.as_id(),
      TdObjEnum::TdProductObj(v) => v.as_id(),
      TdObjEnum::TdSchemaObj(v) => v.as_id(),
      TdObjEnum::TdBlobObj(v) => v.as_id(),
      TdObjEnum::TdNullObj(v) => v.as_id(),
      TdObjEnum::TdVaultObj(v) => v.as_id(),
      TdObjEnum::TdIconObj(v) => v.as_id(),
    }
  }
}

impl PartialEq for TdTypEnum<'_> {
  fn eq(&self, other: &Self) -> bool {
    self.as_id() == other.as_id()
  }
}
impl Eq for TdTypEnum<'_> {}

impl Hash for TdTypEnum<'_> {
  fn hash<H: Hasher>(&self, state: &mut H) {
    self.as_id().hash(state);
  }
}

impl PartialEq for TdObjEnum<'_> {
  fn eq(&self, other: &Self) -> bool {
    self.as_id() == other.as_id()
  }
}
impl Eq for TdObjEnum<'_> {}

impl Hash for TdObjEnum<'_> {
  fn hash<H: Hasher>(&self, state: &mut H) {
    self.as_id().hash(state);
  }
}

impl typedown_incremental::StableHash for TdTypEnum<'_> {
  fn stable_hash<DB: typedown_incremental::QueryDatabase + ?Sized>(
    &self,
    db: &DB,
    hasher: &mut typedown_incremental::StableHasher,
  ) {
    std::mem::discriminant(self).hash(hasher);
    match self {
      TdTypEnum::TdTypTyp(v) => v.stable_hash(db, hasher),
      TdTypEnum::TdBoolTyp(v) => v.stable_hash(db, hasher),
      TdTypEnum::TdStringTyp(v) => v.stable_hash(db, hasher),
      TdTypEnum::TdNumberTyp(v) => v.stable_hash(db, hasher),
      TdTypEnum::TdMathTyp(v) => v.stable_hash(db, hasher),
      TdTypEnum::TdFuncTyp(v) => v.stable_hash(db, hasher),
      TdTypEnum::TdListTyp(v) => v.stable_hash(db, hasher),
      TdTypEnum::TdDictTyp(v) => v.stable_hash(db, hasher),
      TdTypEnum::TdDateTimeTyp(v) => v.stable_hash(db, hasher),
      TdTypEnum::TdDateTyp(v) => v.stable_hash(db, hasher),
      TdTypEnum::TdTimeTyp(v) => v.stable_hash(db, hasher),
      TdTypEnum::TdProductTyp(v) => v.stable_hash(db, hasher),
      TdTypEnum::TdSchemaMetaTyp(v) => v.stable_hash(db, hasher),
      TdTypEnum::TdSchemaTyp(v) => v.stable_hash(db, hasher),
      TdTypEnum::TdBlobTyp(v) => v.stable_hash(db, hasher),
      TdTypEnum::TdNullTyp(v) => v.stable_hash(db, hasher),
      TdTypEnum::TdNeverTyp(v) => v.stable_hash(db, hasher),
      TdTypEnum::TdLitTyp(v) => v.stable_hash(db, hasher),
      TdTypEnum::TdSumTyp(v) => v.stable_hash(db, hasher),
      TdTypEnum::TdVariableTyp(v) => v.stable_hash(db, hasher),
      TdTypEnum::TdExistentialTyp(v) => v.stable_hash(db, hasher),
      TdTypEnum::TdObjTyp(v) => v.stable_hash(db, hasher),
      TdTypEnum::TdIconTyp(v) => v.stable_hash(db, hasher),
    }
  }
}

impl typedown_incremental::StableHash for TdObjEnum<'_> {
  fn stable_hash<DB: typedown_incremental::QueryDatabase + ?Sized>(
    &self,
    db: &DB,
    hasher: &mut typedown_incremental::StableHasher,
  ) {
    std::mem::discriminant(self).hash(hasher);
    match self {
      TdObjEnum::TdTypObj(v) => v.stable_hash(db, hasher),
      TdObjEnum::TdBoolObj(v) => v.stable_hash(db, hasher),
      TdObjEnum::TdStringObj(v) => v.stable_hash(db, hasher),
      TdObjEnum::TdNumberObj(v) => v.stable_hash(db, hasher),
      TdObjEnum::TdMathObj(v) => v.stable_hash(db, hasher),
      TdObjEnum::TdFuncObj(v) => v.stable_hash(db, hasher),
      TdObjEnum::TdListObj(v) => v.stable_hash(db, hasher),
      TdObjEnum::TdDictObj(v) => v.stable_hash(db, hasher),
      TdObjEnum::TdDateTimeObj(v) => v.stable_hash(db, hasher),
      TdObjEnum::TdDateObj(v) => v.stable_hash(db, hasher),
      TdObjEnum::TdTimeObj(v) => v.stable_hash(db, hasher),
      TdObjEnum::TdProductObj(v) => v.stable_hash(db, hasher),
      TdObjEnum::TdSchemaObj(v) => v.stable_hash(db, hasher),
      TdObjEnum::TdBlobObj(v) => v.stable_hash(db, hasher),
      TdObjEnum::TdNullObj(v) => v.stable_hash(db, hasher),
      TdObjEnum::TdVaultObj(v) => v.stable_hash(db, hasher),
      TdObjEnum::TdIconObj(v) => v.stable_hash(db, hasher),
    }
  }
}

#[derive(FromRepr)]
#[repr(u8)]
pub enum TdTypeKind {
  Typ = 0,
  Obj = 1,
  String = 2,
  Bool = 3,
  Number = 4,
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
  Lit = 16,
  Sum = 17,
  SchemaMetaTyp = 18,
  Schema = 19,
  Existential = 20,
  Variable = 21,
  Icon = 22,
}

#[derive(FromRepr)]
#[repr(u8)]
pub enum TdObjectKind {
  // Types (wraps TdTypeEnum)
  Type = 0,
  // Object-only
  StringObj = 128,
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
  IconObj = 143,
}

// TdTypeEnum
impl Encodable for TdTypEnum<'_> {
  fn encode(&self, buf: &mut Vec<u8>, encoder: &mut Encoder) {
    match self {
      TdTypEnum::TdTypTyp(v) => {
        encoder.emit_u8(buf, TdTypeKind::Typ as u8);
        v.field_encode(buf, encoder);
      }
      TdTypEnum::TdStringTyp(v) => {
        encoder.emit_u8(buf, TdTypeKind::String as u8);
        v.field_encode(buf, encoder);
      }
      TdTypEnum::TdBoolTyp(v) => {
        encoder.emit_u8(buf, TdTypeKind::Bool as u8);
        v.field_encode(buf, encoder);
      }
      TdTypEnum::TdNumberTyp(v) => {
        encoder.emit_u8(buf, TdTypeKind::Number as u8);
        v.field_encode(buf, encoder);
      }
      TdTypEnum::TdMathTyp(v) => {
        encoder.emit_u8(buf, TdTypeKind::Math as u8);
        v.field_encode(buf, encoder);
      }
      TdTypEnum::TdListTyp(v) => {
        encoder.emit_u8(buf, TdTypeKind::List as u8);
        v.field_encode(buf, encoder);
      }
      TdTypEnum::TdDictTyp(v) => {
        encoder.emit_u8(buf, TdTypeKind::Dict as u8);
        v.field_encode(buf, encoder);
      }
      TdTypEnum::TdFuncTyp(v) => {
        encoder.emit_u8(buf, TdTypeKind::Func as u8);
        v.field_encode(buf, encoder);
      }
      TdTypEnum::TdProductTyp(v) => {
        encoder.emit_u8(buf, TdTypeKind::Product as u8);
        v.field_encode(buf, encoder);
      }
      TdTypEnum::TdSchemaMetaTyp(v) => {
        encoder.emit_u8(buf, TdTypeKind::SchemaMetaTyp as u8);
        v.field_encode(buf, encoder);
      }
      TdTypEnum::TdSchemaTyp(v) => {
        encoder.emit_u8(buf, TdTypeKind::Schema as u8);
        v.field_encode(buf, encoder);
      }
      TdTypEnum::TdDateTimeTyp(v) => {
        encoder.emit_u8(buf, TdTypeKind::DateTime as u8);
        v.field_encode(buf, encoder);
      }
      TdTypEnum::TdDateTyp(v) => {
        encoder.emit_u8(buf, TdTypeKind::Date as u8);
        v.field_encode(buf, encoder);
      }
      TdTypEnum::TdTimeTyp(v) => {
        encoder.emit_u8(buf, TdTypeKind::Time as u8);
        v.field_encode(buf, encoder);
      }
      TdTypEnum::TdBlobTyp(v) => {
        encoder.emit_u8(buf, TdTypeKind::Blob as u8);
        v.field_encode(buf, encoder);
      }
      TdTypEnum::TdNullTyp(v) => {
        encoder.emit_u8(buf, TdTypeKind::Null as u8);
        v.field_encode(buf, encoder);
      }
      TdTypEnum::TdNeverTyp(v) => {
        encoder.emit_u8(buf, TdTypeKind::Never as u8);
        v.field_encode(buf, encoder);
      }
      TdTypEnum::TdLitTyp(v) => {
        encoder.emit_u8(buf, TdTypeKind::Lit as u8);
        v.field_encode(buf, encoder);
      }
      TdTypEnum::TdSumTyp(v) => {
        encoder.emit_u8(buf, TdTypeKind::Sum as u8);
        v.field_encode(buf, encoder);
      }
      TdTypEnum::TdVariableTyp(v) => {
        encoder.emit_u8(buf, TdTypeKind::Variable as u8);
        v.field_encode(buf, encoder);
      }
      TdTypEnum::TdExistentialTyp(v) => {
        encoder.emit_u8(buf, TdTypeKind::Existential as u8);
        v.field_encode(buf, encoder);
      }
      TdTypEnum::TdObjTyp(v) => {
        encoder.emit_u8(buf, TdTypeKind::Obj as u8);
        v.field_encode(buf, encoder);
      }
      TdTypEnum::TdIconTyp(v) => {
        encoder.emit_u8(buf, TdTypeKind::Icon as u8);
        v.field_encode(buf, encoder);
      }
    }
  }
}

impl Decodable for TdTypEnum<'_> {
  fn decode(data: &mut &[u8], decoder: &Decoder) -> Self {
    let tag = decoder.read_u8(data);
    match TdTypeKind::from_repr(tag).expect("unknown TdTypeKind tag") {
      TdTypeKind::Typ => TdTypTyp::field_decode(data, decoder).into(),
      TdTypeKind::String => TdStringTyp::field_decode(data, decoder).into(),
      TdTypeKind::Bool => TdBoolTyp::field_decode(data, decoder).into(),
      TdTypeKind::Number => TdNumberTyp::field_decode(data, decoder).into(),
      TdTypeKind::Math => TdMathTyp::field_decode(data, decoder).into(),
      TdTypeKind::List => TdListTyp::field_decode(data, decoder).into(),
      TdTypeKind::Dict => TdDictTyp::field_decode(data, decoder).into(),
      TdTypeKind::Func => TdFuncTyp::field_decode(data, decoder).into(),
      TdTypeKind::Product => TdProductTyp::field_decode(data, decoder).into(),
      TdTypeKind::SchemaMetaTyp => TdSchemaMetaTyp::field_decode(data, decoder).into(),
      TdTypeKind::Schema => TdSchemaTyp::field_decode(data, decoder).into(),
      TdTypeKind::DateTime => TdDateTimeTyp::field_decode(data, decoder).into(),
      TdTypeKind::Date => TdDateTyp::field_decode(data, decoder).into(),
      TdTypeKind::Time => TdTimeTyp::field_decode(data, decoder).into(),
      TdTypeKind::Blob => TdBlobTyp::field_decode(data, decoder).into(),
      TdTypeKind::Null => TdNullTyp::field_decode(data, decoder).into(),
      TdTypeKind::Never => TdNeverTyp::field_decode(data, decoder).into(),
      TdTypeKind::Lit => TdLitTyp::field_decode(data, decoder).into(),
      TdTypeKind::Sum => TdSumTyp::field_decode(data, decoder).into(),
      TdTypeKind::Existential => TdExistentialTyp::field_decode(data, decoder).into(),
      TdTypeKind::Variable => TdVariableTyp::field_decode(data, decoder).into(),
      TdTypeKind::Obj => TdObjTyp::field_decode(data, decoder).into(),
      TdTypeKind::Icon => TdIconTyp::field_decode(data, decoder).into(),
    }
  }
}

// TdObjectEnum
impl Encodable for TdObjEnum<'_> {
  fn encode(&self, buf: &mut Vec<u8>, encoder: &mut Encoder) {
    match self {
      TdObjEnum::TdTypObj(v) => {
        encoder.emit_u8(buf, TdObjectKind::Type as u8);
        v.encode(buf, encoder);
      }
      // Objects
      TdObjEnum::TdStringObj(v) => {
        encoder.emit_u8(buf, TdObjectKind::StringObj as u8);
        v.field_encode(buf, encoder);
      }
      TdObjEnum::TdBoolObj(v) => {
        encoder.emit_u8(buf, TdObjectKind::BoolObj as u8);
        v.field_encode(buf, encoder);
      }
      TdObjEnum::TdNumberObj(v) => {
        encoder.emit_u8(buf, TdObjectKind::NumObj as u8);
        v.field_encode(buf, encoder);
      }
      TdObjEnum::TdMathObj(v) => {
        encoder.emit_u8(buf, TdObjectKind::MathObj as u8);
        v.field_encode(buf, encoder);
      }
      TdObjEnum::TdListObj(v) => {
        encoder.emit_u8(buf, TdObjectKind::ListObj as u8);
        v.field_encode(buf, encoder);
      }
      TdObjEnum::TdDictObj(v) => {
        encoder.emit_u8(buf, TdObjectKind::DictObj as u8);
        v.field_encode(buf, encoder);
      }
      TdObjEnum::TdFuncObj(v) => {
        encoder.emit_u8(buf, TdObjectKind::FuncObj as u8);
        v.field_encode(buf, encoder);
      }
      TdObjEnum::TdProductObj(v) => {
        encoder.emit_u8(buf, TdObjectKind::ProductObj as u8);
        v.field_encode(buf, encoder);
      }
      TdObjEnum::TdSchemaObj(v) => {
        encoder.emit_u8(buf, TdObjectKind::SchemaObj as u8);
        v.field_encode(buf, encoder);
      }
      TdObjEnum::TdDateTimeObj(v) => {
        encoder.emit_u8(buf, TdObjectKind::DateTimeObj as u8);
        v.field_encode(buf, encoder);
      }
      TdObjEnum::TdDateObj(v) => {
        encoder.emit_u8(buf, TdObjectKind::DateObj as u8);
        v.field_encode(buf, encoder);
      }
      TdObjEnum::TdTimeObj(v) => {
        encoder.emit_u8(buf, TdObjectKind::TimeObj as u8);
        v.field_encode(buf, encoder);
      }
      TdObjEnum::TdBlobObj(v) => {
        encoder.emit_u8(buf, TdObjectKind::BlobObj as u8);
        v.field_encode(buf, encoder);
      }
      TdObjEnum::TdNullObj(v) => {
        encoder.emit_u8(buf, TdObjectKind::NullObj as u8);
        v.field_encode(buf, encoder);
      }
      TdObjEnum::TdVaultObj(v) => {
        encoder.emit_u8(buf, TdObjectKind::VaultObj as u8);
        v.field_encode(buf, encoder);
      }
      TdObjEnum::TdIconObj(v) => {
        encoder.emit_u8(buf, TdObjectKind::IconObj as u8);
        v.field_encode(buf, encoder);
      }
    }
  }
}

impl Decodable for TdObjEnum<'_> {
  fn decode(data: &mut &[u8], decoder: &Decoder) -> Self {
    let tag = decoder.read_u8(data);
    match TdObjectKind::from_repr(tag) {
      Some(TdObjectKind::Type) => TdObjEnum::TdTypObj(TdTypEnum::decode(data, decoder)),
      Some(TdObjectKind::StringObj) => TdStringObj::field_decode(data, decoder).into(),
      Some(TdObjectKind::BoolObj) => TdBoolObj::field_decode(data, decoder).into(),
      Some(TdObjectKind::NumObj) => TdNumberObj::field_decode(data, decoder).into(),
      Some(TdObjectKind::MathObj) => TdMathObj::field_decode(data, decoder).into(),
      Some(TdObjectKind::ListObj) => TdListObj::field_decode(data, decoder).into(),
      Some(TdObjectKind::DictObj) => TdDictObj::field_decode(data, decoder).into(),
      Some(TdObjectKind::FuncObj) => TdFuncObj::field_decode(data, decoder).into(),
      Some(TdObjectKind::ProductObj) => TdProductObj::field_decode(data, decoder).into(),
      Some(TdObjectKind::SchemaObj) => TdSchemaObj::field_decode(data, decoder).into(),
      Some(TdObjectKind::DateTimeObj) => TdDateTimeObj::field_decode(data, decoder).into(),
      Some(TdObjectKind::DateObj) => TdDateObj::field_decode(data, decoder).into(),
      Some(TdObjectKind::TimeObj) => TdTimeObj::field_decode(data, decoder).into(),
      Some(TdObjectKind::BlobObj) => TdBlobObj::field_decode(data, decoder).into(),
      Some(TdObjectKind::NullObj) => TdNullObj::field_decode(data, decoder).into(),
      Some(TdObjectKind::VaultObj) => TdVaultObj::field_decode(data, decoder).into(),
      Some(TdObjectKind::IconObj) => TdIconObj::field_decode(data, decoder).into(),
      None => panic!("unknown TdObjectKind tag: {}", tag),
    }
  }
}
