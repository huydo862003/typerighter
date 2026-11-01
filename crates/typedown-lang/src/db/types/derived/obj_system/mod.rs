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
use crate::syntax::diagnostic::TdDiagnostic;
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
macro_rules! impl_from_typ_for_obj_enum {
  ($($ty:ident),+ $(,)?) => {
    $(
      impl<'db> From<$ty<'db>> for TdObjEnum<'db> {
        fn from(value: $ty<'db>) -> Self {
          TdObjEnum::TdTypObj(TdTypEnum::from(value))
        }
      }
    )+
  };
}

impl_from_typ_for_obj_enum!(
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
      TdTypEnum::TdTypTyp(value) => value.as_id(),
      TdTypEnum::TdBoolTyp(value) => value.as_id(),
      TdTypEnum::TdStringTyp(value) => value.as_id(),
      TdTypEnum::TdNumberTyp(value) => value.as_id(),
      TdTypEnum::TdMathTyp(value) => value.as_id(),
      TdTypEnum::TdFuncTyp(value) => value.as_id(),
      TdTypEnum::TdListTyp(value) => value.as_id(),
      TdTypEnum::TdDictTyp(value) => value.as_id(),
      TdTypEnum::TdDateTimeTyp(value) => value.as_id(),
      TdTypEnum::TdDateTyp(value) => value.as_id(),
      TdTypEnum::TdTimeTyp(value) => value.as_id(),
      TdTypEnum::TdProductTyp(value) => value.as_id(),
      TdTypEnum::TdSchemaMetaTyp(value) => value.as_id(),
      TdTypEnum::TdSchemaTyp(value) => value.as_id(),
      TdTypEnum::TdBlobTyp(value) => value.as_id(),
      TdTypEnum::TdNullTyp(value) => value.as_id(),
      TdTypEnum::TdNeverTyp(value) => value.as_id(),
      TdTypEnum::TdLitTyp(value) => value.as_id(),
      TdTypEnum::TdSumTyp(value) => value.as_id(),
      TdTypEnum::TdVariableTyp(value) => value.as_id(),
      TdTypEnum::TdExistentialTyp(value) => value.as_id(),
      TdTypEnum::TdObjTyp(value) => value.as_id(),
      TdTypEnum::TdIconTyp(value) => value.as_id(),
    }
  }
}

impl Id for TdObjEnum<'_> {
  fn as_id(&self) -> DepId {
    match self {
      TdObjEnum::TdTypObj(value) => value.as_id(),
      TdObjEnum::TdBoolObj(value) => value.as_id(),
      TdObjEnum::TdStringObj(value) => value.as_id(),
      TdObjEnum::TdNumberObj(value) => value.as_id(),
      TdObjEnum::TdMathObj(value) => value.as_id(),
      TdObjEnum::TdFuncObj(value) => value.as_id(),
      TdObjEnum::TdListObj(value) => value.as_id(),
      TdObjEnum::TdDictObj(value) => value.as_id(),
      TdObjEnum::TdDateTimeObj(value) => value.as_id(),
      TdObjEnum::TdDateObj(value) => value.as_id(),
      TdObjEnum::TdTimeObj(value) => value.as_id(),
      TdObjEnum::TdProductObj(value) => value.as_id(),
      TdObjEnum::TdSchemaObj(value) => value.as_id(),
      TdObjEnum::TdBlobObj(value) => value.as_id(),
      TdObjEnum::TdNullObj(value) => value.as_id(),
      TdObjEnum::TdVaultObj(value) => value.as_id(),
      TdObjEnum::TdIconObj(value) => value.as_id(),
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
      TdTypEnum::TdTypTyp(value) => value.stable_hash(db, hasher),
      TdTypEnum::TdBoolTyp(value) => value.stable_hash(db, hasher),
      TdTypEnum::TdStringTyp(value) => value.stable_hash(db, hasher),
      TdTypEnum::TdNumberTyp(value) => value.stable_hash(db, hasher),
      TdTypEnum::TdMathTyp(value) => value.stable_hash(db, hasher),
      TdTypEnum::TdFuncTyp(value) => value.stable_hash(db, hasher),
      TdTypEnum::TdListTyp(value) => value.stable_hash(db, hasher),
      TdTypEnum::TdDictTyp(value) => value.stable_hash(db, hasher),
      TdTypEnum::TdDateTimeTyp(value) => value.stable_hash(db, hasher),
      TdTypEnum::TdDateTyp(value) => value.stable_hash(db, hasher),
      TdTypEnum::TdTimeTyp(value) => value.stable_hash(db, hasher),
      TdTypEnum::TdProductTyp(value) => value.stable_hash(db, hasher),
      TdTypEnum::TdSchemaMetaTyp(value) => value.stable_hash(db, hasher),
      TdTypEnum::TdSchemaTyp(value) => value.stable_hash(db, hasher),
      TdTypEnum::TdBlobTyp(value) => value.stable_hash(db, hasher),
      TdTypEnum::TdNullTyp(value) => value.stable_hash(db, hasher),
      TdTypEnum::TdNeverTyp(value) => value.stable_hash(db, hasher),
      TdTypEnum::TdLitTyp(value) => value.stable_hash(db, hasher),
      TdTypEnum::TdSumTyp(value) => value.stable_hash(db, hasher),
      TdTypEnum::TdVariableTyp(value) => value.stable_hash(db, hasher),
      TdTypEnum::TdExistentialTyp(value) => value.stable_hash(db, hasher),
      TdTypEnum::TdObjTyp(value) => value.stable_hash(db, hasher),
      TdTypEnum::TdIconTyp(value) => value.stable_hash(db, hasher),
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
      TdObjEnum::TdTypObj(value) => value.stable_hash(db, hasher),
      TdObjEnum::TdBoolObj(value) => value.stable_hash(db, hasher),
      TdObjEnum::TdStringObj(value) => value.stable_hash(db, hasher),
      TdObjEnum::TdNumberObj(value) => value.stable_hash(db, hasher),
      TdObjEnum::TdMathObj(value) => value.stable_hash(db, hasher),
      TdObjEnum::TdFuncObj(value) => value.stable_hash(db, hasher),
      TdObjEnum::TdListObj(value) => value.stable_hash(db, hasher),
      TdObjEnum::TdDictObj(value) => value.stable_hash(db, hasher),
      TdObjEnum::TdDateTimeObj(value) => value.stable_hash(db, hasher),
      TdObjEnum::TdDateObj(value) => value.stable_hash(db, hasher),
      TdObjEnum::TdTimeObj(value) => value.stable_hash(db, hasher),
      TdObjEnum::TdProductObj(value) => value.stable_hash(db, hasher),
      TdObjEnum::TdSchemaObj(value) => value.stable_hash(db, hasher),
      TdObjEnum::TdBlobObj(value) => value.stable_hash(db, hasher),
      TdObjEnum::TdNullObj(value) => value.stable_hash(db, hasher),
      TdObjEnum::TdVaultObj(value) => value.stable_hash(db, hasher),
      TdObjEnum::TdIconObj(value) => value.stable_hash(db, hasher),
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
  Typ = 0,
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
  fn encode(&self, buffer: &mut Vec<u8>, encoder: &mut Encoder) {
    match self {
      TdTypEnum::TdTypTyp(value) => {
        encoder.emit_u8(buffer, TdTypeKind::Typ as u8);
        value.field_encode(buffer, encoder);
      }
      TdTypEnum::TdStringTyp(value) => {
        encoder.emit_u8(buffer, TdTypeKind::String as u8);
        value.field_encode(buffer, encoder);
      }
      TdTypEnum::TdBoolTyp(value) => {
        encoder.emit_u8(buffer, TdTypeKind::Bool as u8);
        value.field_encode(buffer, encoder);
      }
      TdTypEnum::TdNumberTyp(value) => {
        encoder.emit_u8(buffer, TdTypeKind::Number as u8);
        value.field_encode(buffer, encoder);
      }
      TdTypEnum::TdMathTyp(value) => {
        encoder.emit_u8(buffer, TdTypeKind::Math as u8);
        value.field_encode(buffer, encoder);
      }
      TdTypEnum::TdListTyp(value) => {
        encoder.emit_u8(buffer, TdTypeKind::List as u8);
        value.field_encode(buffer, encoder);
      }
      TdTypEnum::TdDictTyp(value) => {
        encoder.emit_u8(buffer, TdTypeKind::Dict as u8);
        value.field_encode(buffer, encoder);
      }
      TdTypEnum::TdFuncTyp(value) => {
        encoder.emit_u8(buffer, TdTypeKind::Func as u8);
        value.field_encode(buffer, encoder);
      }
      TdTypEnum::TdProductTyp(value) => {
        encoder.emit_u8(buffer, TdTypeKind::Product as u8);
        value.field_encode(buffer, encoder);
      }
      TdTypEnum::TdSchemaMetaTyp(value) => {
        encoder.emit_u8(buffer, TdTypeKind::SchemaMetaTyp as u8);
        value.field_encode(buffer, encoder);
      }
      TdTypEnum::TdSchemaTyp(value) => {
        encoder.emit_u8(buffer, TdTypeKind::Schema as u8);
        value.field_encode(buffer, encoder);
      }
      TdTypEnum::TdDateTimeTyp(value) => {
        encoder.emit_u8(buffer, TdTypeKind::DateTime as u8);
        value.field_encode(buffer, encoder);
      }
      TdTypEnum::TdDateTyp(value) => {
        encoder.emit_u8(buffer, TdTypeKind::Date as u8);
        value.field_encode(buffer, encoder);
      }
      TdTypEnum::TdTimeTyp(value) => {
        encoder.emit_u8(buffer, TdTypeKind::Time as u8);
        value.field_encode(buffer, encoder);
      }
      TdTypEnum::TdBlobTyp(value) => {
        encoder.emit_u8(buffer, TdTypeKind::Blob as u8);
        value.field_encode(buffer, encoder);
      }
      TdTypEnum::TdNullTyp(value) => {
        encoder.emit_u8(buffer, TdTypeKind::Null as u8);
        value.field_encode(buffer, encoder);
      }
      TdTypEnum::TdNeverTyp(value) => {
        encoder.emit_u8(buffer, TdTypeKind::Never as u8);
        value.field_encode(buffer, encoder);
      }
      TdTypEnum::TdLitTyp(value) => {
        encoder.emit_u8(buffer, TdTypeKind::Lit as u8);
        value.field_encode(buffer, encoder);
      }
      TdTypEnum::TdSumTyp(value) => {
        encoder.emit_u8(buffer, TdTypeKind::Sum as u8);
        value.field_encode(buffer, encoder);
      }
      TdTypEnum::TdVariableTyp(value) => {
        encoder.emit_u8(buffer, TdTypeKind::Variable as u8);
        value.field_encode(buffer, encoder);
      }
      TdTypEnum::TdExistentialTyp(value) => {
        encoder.emit_u8(buffer, TdTypeKind::Existential as u8);
        value.field_encode(buffer, encoder);
      }
      TdTypEnum::TdObjTyp(value) => {
        encoder.emit_u8(buffer, TdTypeKind::Obj as u8);
        value.field_encode(buffer, encoder);
      }
      TdTypEnum::TdIconTyp(value) => {
        encoder.emit_u8(buffer, TdTypeKind::Icon as u8);
        value.field_encode(buffer, encoder);
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
  fn encode(&self, buffer: &mut Vec<u8>, encoder: &mut Encoder) {
    match self {
      TdObjEnum::TdTypObj(value) => {
        encoder.emit_u8(buffer, TdObjectKind::Typ as u8);
        value.encode(buffer, encoder);
      }
      // Objects
      TdObjEnum::TdStringObj(value) => {
        encoder.emit_u8(buffer, TdObjectKind::StringObj as u8);
        value.field_encode(buffer, encoder);
      }
      TdObjEnum::TdBoolObj(value) => {
        encoder.emit_u8(buffer, TdObjectKind::BoolObj as u8);
        value.field_encode(buffer, encoder);
      }
      TdObjEnum::TdNumberObj(value) => {
        encoder.emit_u8(buffer, TdObjectKind::NumObj as u8);
        value.field_encode(buffer, encoder);
      }
      TdObjEnum::TdMathObj(value) => {
        encoder.emit_u8(buffer, TdObjectKind::MathObj as u8);
        value.field_encode(buffer, encoder);
      }
      TdObjEnum::TdListObj(value) => {
        encoder.emit_u8(buffer, TdObjectKind::ListObj as u8);
        value.field_encode(buffer, encoder);
      }
      TdObjEnum::TdDictObj(value) => {
        encoder.emit_u8(buffer, TdObjectKind::DictObj as u8);
        value.field_encode(buffer, encoder);
      }
      TdObjEnum::TdFuncObj(value) => {
        encoder.emit_u8(buffer, TdObjectKind::FuncObj as u8);
        value.field_encode(buffer, encoder);
      }
      TdObjEnum::TdProductObj(value) => {
        encoder.emit_u8(buffer, TdObjectKind::ProductObj as u8);
        value.field_encode(buffer, encoder);
      }
      TdObjEnum::TdSchemaObj(value) => {
        encoder.emit_u8(buffer, TdObjectKind::SchemaObj as u8);
        value.field_encode(buffer, encoder);
      }
      TdObjEnum::TdDateTimeObj(value) => {
        encoder.emit_u8(buffer, TdObjectKind::DateTimeObj as u8);
        value.field_encode(buffer, encoder);
      }
      TdObjEnum::TdDateObj(value) => {
        encoder.emit_u8(buffer, TdObjectKind::DateObj as u8);
        value.field_encode(buffer, encoder);
      }
      TdObjEnum::TdTimeObj(value) => {
        encoder.emit_u8(buffer, TdObjectKind::TimeObj as u8);
        value.field_encode(buffer, encoder);
      }
      TdObjEnum::TdBlobObj(value) => {
        encoder.emit_u8(buffer, TdObjectKind::BlobObj as u8);
        value.field_encode(buffer, encoder);
      }
      TdObjEnum::TdNullObj(value) => {
        encoder.emit_u8(buffer, TdObjectKind::NullObj as u8);
        value.field_encode(buffer, encoder);
      }
      TdObjEnum::TdVaultObj(value) => {
        encoder.emit_u8(buffer, TdObjectKind::VaultObj as u8);
        value.field_encode(buffer, encoder);
      }
      TdObjEnum::TdIconObj(value) => {
        encoder.emit_u8(buffer, TdObjectKind::IconObj as u8);
        value.field_encode(buffer, encoder);
      }
    }
  }
}

impl Decodable for TdObjEnum<'_> {
  fn decode(data: &mut &[u8], decoder: &Decoder) -> Self {
    let tag = decoder.read_u8(data);
    match TdObjectKind::from_repr(tag) {
      Some(TdObjectKind::Typ) => TdObjEnum::TdTypObj(TdTypEnum::decode(data, decoder)),
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
