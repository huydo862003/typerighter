use std::hash::Hash;

use strum::FromRepr;
use typedown_macros::{StableCompare, query_interned};

use typedown_incremental::{
  Decodable, Decoder, Encodable, Encoder, QueryDatabase, StableHash, StableHasher,
};

use typedown_types::either::Either;

use super::derived::object_system::{TdDictType, TdListType};
use super::{TdTypeEnum, TdVariableType};
use crate::db::TypedownDatabase;
use crate::db::derived::evaluate::evaluate_type::evaluate_type;
use crate::db::derived::get_builtin_types::{
  get_func_type, get_literal_type, get_sum_type,
};
use crate::db::types::Symbol;
use typedown_incremental::Id;

#[query_interned]
pub struct FuncSignature<'db> {
  pub type_params: Vec<TdVariableType<'db>>,
  pub params: Vec<TdTypeEnum<'db>>,
  pub ret: TdTypeEnum<'db>,
}

impl<'db> FuncSignature<'db> {
  pub fn instantiate(
    &self,
    db: &'db TypedownDatabase,
    arg: TdTypeEnum<'db>,
  ) -> Option<FuncSignature<'db>> {
    let mut type_params = self.type_params(db);
    if type_params.is_empty() {
      return None;
    }
    let variable = type_params.remove(0);
    let params = self
      .params(db)
      .into_iter()
      .map(|p| substitute_variable(db, &p, variable, &arg))
      .collect();
    let ret = substitute_variable(db, &self.ret(db), variable, &arg);
    Some(FuncSignature::new(db, type_params, params, ret))
  }
}

/// Replace all occurrences of a type variable in a type with a concrete type
pub fn substitute_variable<'db>(
  db: &'db TypedownDatabase,
  typ: &TdTypeEnum<'db>,
  variable: TdVariableType<'db>,
  replacement: &TdTypeEnum<'db>,
) -> TdTypeEnum<'db> {
  let substitute = |lazy: LazyType<'db>| -> LazyType<'db> {
    lazy
      .resolve(db)
      .map(|t| LazyType::eager(substitute_variable(db, &t, variable, replacement)))
      .unwrap_or(lazy)
  };

  match typ {
    TdTypeEnum::TdVariableType(v) if v.as_id() == variable.as_id() => replacement.clone(),
    TdTypeEnum::TdListType(list) => TdListType::new(db, list.elem(db).map(substitute)).into(),
    TdTypeEnum::TdDictType(dict) => TdDictType::new(
      db,
      dict.key(db).map(substitute),
      dict.value(db).map(substitute),
    )
    .into(),
    TdTypeEnum::TdSumType(sum) => {
      let members: Vec<LazyType> = sum.members(db).into_iter().map(substitute).collect();
      get_sum_type(db, members).into()
    }
    TdTypeEnum::TdFuncType(func) => {
      let sig = func.signature(db);
      let params = sig
        .params(db)
        .into_iter()
        .map(|p| substitute_variable(db, &p, variable, replacement))
        .collect();
      let ret = substitute_variable(db, &sig.ret(db), variable, replacement);
      let type_params = sig.type_params(db);
      get_func_type(db, FuncSignature::new(db, type_params, params, ret)).into()
    }
    TdTypeEnum::TdLiteralType(lit) => match lit.value(db) {
      LiteralValue::Type(t) => {
        let subst = substitute_variable(db, &t, variable, replacement);
        get_literal_type(db, LiteralValue::Type(subst)).into()
      }
      LiteralValue::Str(_) | LiteralValue::Bool(_) | LiteralValue::Num(_) => typ.clone(),
    },
    // Leaf types: no nested type to substitute
    TdTypeEnum::TdVariableType(_)
    | TdTypeEnum::TdTypeType(_)
    | TdTypeEnum::TdBoolType(_)
    | TdTypeEnum::TdStrType(_)
    | TdTypeEnum::TdNumType(_)
    | TdTypeEnum::TdMathType(_)
    | TdTypeEnum::TdDateTimeType(_)
    | TdTypeEnum::TdDateType(_)
    | TdTypeEnum::TdTimeType(_)
    | TdTypeEnum::TdBlobType(_)
    | TdTypeEnum::TdNullType(_)
    | TdTypeEnum::TdNeverType(_)
    | TdTypeEnum::TdObjectType(_)
    | TdTypeEnum::TdIconType(_)
    | TdTypeEnum::TdProductType(_)
    | TdTypeEnum::TdSchemaType(_)
    | TdTypeEnum::TdSchemaMetaType(_)
    | TdTypeEnum::TdExistentialType(_) => typ.clone(),
  }
}

#[query_interned]
pub struct TypeParams<'db> {
  pub params: Vec<TdVariableType<'db>>,
  pub bindings: Vec<LazyType<'db>>,
}

impl<'db> TypeParams<'db> {
  pub fn instantiate(
    &self,
    db: &'db TypedownDatabase,
    args: Vec<LazyType<'db>>,
  ) -> Option<TypeParams<'db>> {
    let params = self.params(db);
    if params.len() != args.len() {
      return None;
    }
    Some(TypeParams::new(db, params, args))
  }

  pub fn bind(
    &self,
    db: &'db TypedownDatabase,
    index: usize,
    arg: LazyType<'db>,
  ) -> Option<TypeParams<'db>> {
    let params = self.params(db);
    let mut bindings = self.bindings(db);
    if index >= params.len() {
      return None;
    }
    if bindings.len() <= index {
      bindings.resize(index + 1, arg.clone());
    }
    bindings[index] = arg;
    Some(TypeParams::new(db, params, bindings))
  }

  pub fn get_param(&self, db: &TypedownDatabase, index: usize) -> Option<TdVariableType<'_>> {
    self.params(db).get(index).copied()
  }

  pub fn get_binding(&self, db: &TypedownDatabase, index: usize) -> Option<LazyType<'_>> {
    self.bindings(db).get(index).cloned()
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
pub struct LazyType<'db>(Either<TdTypeEnum<'db>, Symbol<'db>>);

impl<'db> LazyType<'db> {
  pub fn eager(typ: TdTypeEnum<'db>) -> Self {
    LazyType(Either::Left(typ))
  }

  pub fn lazy(symbol: Symbol<'db>) -> Self {
    LazyType(Either::Right(symbol))
  }

  pub fn resolve(&self, db: &'db TypedownDatabase) -> Option<TdTypeEnum<'db>> {
    match &self.0 {
      Either::Left(typ) => Some(typ.clone()),
      Either::Right(symbol) => evaluate_type(db, *symbol).typ(db),
    }
  }

  pub fn as_eager(&self) -> Option<TdTypeEnum<'db>> {
    match &self.0 {
      Either::Left(typ) => Some(typ.clone()),
      Either::Right(_) => None,
    }
  }
}

impl<'db> Encodable for LazyType<'db> {
  fn encode(&self, buf: &mut Vec<u8>, encoder: &mut Encoder) {
    self.0.encode(buf, encoder);
  }
}

impl<'db> Decodable for LazyType<'db> {
  fn decode(data: &mut &[u8], decoder: &Decoder) -> Self {
    LazyType(Either::<TdTypeEnum, Symbol>::decode(data, decoder))
  }
}

impl<'db> StableHash for LazyType<'db> {
  fn stable_hash<DB: QueryDatabase + ?Sized>(&self, db: &DB, hasher: &mut StableHasher) {
    self.0.stable_hash(db, hasher);
  }
}

/// A concrete literal value used in literal type constraints
#[derive(Debug, Clone, PartialEq, Eq, Hash, StableCompare)]
pub enum LiteralValue<'db> {
  Str(String),
  Bool(bool),
  // f64 cannot be hashed so we store in string
  Num(String),
  // A type used as a value (e.g. Task in vault.query(Task))
  Type(super::derived::object_system::TdTypeEnum<'db>),
}

#[derive(FromRepr)]
#[repr(u8)]
enum LiteralValueTag {
  Str = 0,
  Bool = 1,
  Num = 2,
  Type = 3,
}

impl Encodable for LiteralValue<'_> {
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
      LiteralValue::Type(val) => {
        encoder.emit_u8(buf, LiteralValueTag::Type as u8);
        val.encode(buf, encoder);
      }
    }
  }
}

impl Decodable for LiteralValue<'_> {
  fn decode(data: &mut &[u8], decoder: &Decoder) -> Self {
    let tag = decoder.read_u8(data);
    match LiteralValueTag::from_repr(tag).expect("unknown LiteralValue tag") {
      LiteralValueTag::Str => LiteralValue::Str(String::decode(data, decoder)),
      LiteralValueTag::Bool => LiteralValue::Bool(bool::decode(data, decoder)),
      LiteralValueTag::Num => LiteralValue::Num(String::decode(data, decoder)),
      LiteralValueTag::Type => LiteralValue::Type(
        super::derived::object_system::TdTypeEnum::decode(data, decoder),
      ),
    }
  }
}

impl StableHash for LiteralValue<'_> {
  fn stable_hash<DB: QueryDatabase + ?Sized>(&self, db: &DB, hasher: &mut StableHasher) {
    std::mem::discriminant(self).stable_hash(db, hasher);
    match self {
      LiteralValue::Str(value) => value.stable_hash(db, hasher),
      LiteralValue::Bool(value) => value.stable_hash(db, hasher),
      LiteralValue::Num(value) => value.stable_hash(db, hasher),
      LiteralValue::Type(value) => value.stable_hash(db, hasher),
    }
  }
}
