use strum::FromRepr;

use super::TdObjectEnum;
use super::base::TdObjectLike;
use super::str::TdStrObj;
use crate::db::TypedownDatabase;
use typedown_incremental::{
  Decodable, Decoder, Encodable, Encoder, QueryDatabase, StableHash, StableHasher,
};

type NativeFn = fn(&TypedownDatabase, TdObjectEnum, Vec<TdObjectEnum>) -> Option<TdObjectEnum>;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, FromRepr)]
#[repr(u8)]
pub enum NativeFnKind {
  StrToString = 0,
  NumToString = 1,
  BoolToString = 2,
  MathToString = 3,
  ObjectToString = 4,
  FuncToString = 5,
  DateTimeToString = 6,
  DateToString = 7,
  TimeToString = 8,
}

impl StableHash for NativeFnKind {
  fn stable_hash<DB: QueryDatabase + ?Sized>(&self, db: &DB, hasher: &mut StableHasher) {
    (*self as u8).stable_hash(db, hasher);
  }
}

impl Encodable for NativeFnKind {
  fn encode(&self, buf: &mut Vec<u8>, encoder: &mut Encoder) {
    encoder.emit_u8(buf, *self as u8);
  }
}

impl Decodable for NativeFnKind {
  fn decode(data: &mut &[u8], decoder: &Decoder) -> Self {
    let tag = decoder.read_u8(data);
    NativeFnKind::from_repr(tag).expect("unknown NativeFnKind tag")
  }
}

impl NativeFnKind {
  pub fn resolve(self) -> NativeFn {
    match self {
      NativeFnKind::StrToString => str_to_string,
      NativeFnKind::NumToString => num_to_string,
      NativeFnKind::BoolToString => bool_to_string,
      NativeFnKind::MathToString => math_to_string,
      NativeFnKind::ObjectToString => object_to_string,
      NativeFnKind::FuncToString => func_to_string,
      NativeFnKind::DateTimeToString => datetime_to_string,
      NativeFnKind::DateToString => date_to_string,
      NativeFnKind::TimeToString => time_to_string,
    }
  }
}

fn str_to_string(
  db: &TypedownDatabase,
  this: TdObjectEnum,
  _args: Vec<TdObjectEnum>,
) -> Option<TdObjectEnum> {
  let obj = this.as_td_str_obj()?;
  Some(TdStrObj::new(db, obj.value(db)).into())
}

fn num_to_string(
  db: &TypedownDatabase,
  this: TdObjectEnum,
  _args: Vec<TdObjectEnum>,
) -> Option<TdObjectEnum> {
  let obj = this.as_td_num_obj()?;
  Some(TdStrObj::new(db, obj.value(db).to_string()).into())
}

fn bool_to_string(
  db: &TypedownDatabase,
  this: TdObjectEnum,
  _args: Vec<TdObjectEnum>,
) -> Option<TdObjectEnum> {
  let obj = this.as_td_bool_obj()?;
  Some(TdStrObj::new(db, obj.value(db).to_string()).into())
}

fn math_to_string(
  db: &TypedownDatabase,
  this: TdObjectEnum,
  _args: Vec<TdObjectEnum>,
) -> Option<TdObjectEnum> {
  let obj = this.as_td_math_obj()?;
  Some(TdStrObj::new(db, format!("${}$", obj.value(db))).into())
}

fn object_to_string(
  db: &TypedownDatabase,
  this: TdObjectEnum,
  _args: Vec<TdObjectEnum>,
) -> Option<TdObjectEnum> {
  Some(TdStrObj::new(db, this.source_path(db)).into())
}

fn func_to_string(
  db: &TypedownDatabase,
  this: TdObjectEnum,
  _args: Vec<TdObjectEnum>,
) -> Option<TdObjectEnum> {
  let func = this.as_td_func_obj()?;
  Some(TdStrObj::new(db, func.name(db)).into())
}

fn datetime_to_string(
  db: &TypedownDatabase,
  this: TdObjectEnum,
  _args: Vec<TdObjectEnum>,
) -> Option<TdObjectEnum> {
  let obj = this.as_td_date_time_obj()?;
  Some(TdStrObj::new(db, obj.value(db)).into())
}

fn date_to_string(
  db: &TypedownDatabase,
  this: TdObjectEnum,
  _args: Vec<TdObjectEnum>,
) -> Option<TdObjectEnum> {
  let obj = this.as_td_date_obj()?;
  Some(TdStrObj::new(db, obj.value(db)).into())
}

fn time_to_string(
  db: &TypedownDatabase,
  this: TdObjectEnum,
  _args: Vec<TdObjectEnum>,
) -> Option<TdObjectEnum> {
  let obj = this.as_td_time_obj()?;
  Some(TdStrObj::new(db, obj.value(db)).into())
}
