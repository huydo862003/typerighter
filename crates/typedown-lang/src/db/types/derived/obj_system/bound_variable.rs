use std::collections::BTreeMap;
use std::hash::Hash;

use strum::FromRepr;
use typedown_incremental::{
  Decodable, Decoder, Encodable, Encoder, QueryDatabase, StableHash, StableHasher,
};
use typedown_macros::{StableCompare, query_interned};

use super::base::{TdRuntimeObj, TdStaticTyp, TdTypTyp};
use super::{TdObjEnum, TdTypEnum};
use crate::db::TypedownDatabase;
use crate::db::types::{
  BuiltinSchemaKind, CallTypResult, FuncSignature, LazyTyp, NativeFuncKind, ProjectRedNode,
  Variance,
};

/// Identifies the scope that created a bound variable
#[derive(Debug, Clone, PartialEq, Eq, Hash, StableCompare)]
pub enum Binder {
  BuiltinSchema(BuiltinSchemaKind),
  BuiltinFunc(NativeFuncKind),
  Closure(ProjectRedNode),
  Existential(ProjectRedNode),
}

#[derive(FromRepr)]
#[repr(u8)]
enum BinderTag {
  BuiltinSchema = 0,
  BuiltinFunc = 1,
  Closure = 2,
  Existential = 3,
}

impl StableHash for Binder {
  fn stable_hash<DB: QueryDatabase + ?Sized>(&self, _db: &DB, hasher: &mut StableHasher) {
    match self {
      Binder::BuiltinSchema(kind) => {
        0u8.hash(hasher);
        (*kind as u8).hash(hasher);
      }
      Binder::BuiltinFunc(kind) => {
        1u8.hash(hasher);
        (*kind as u8).hash(hasher);
      }
      Binder::Closure(node) => {
        2u8.hash(hasher);
        node.hash(hasher);
      }
      Binder::Existential(node) => {
        3u8.hash(hasher);
        node.hash(hasher);
      }
    }
  }
}

impl Encodable for Binder {
  fn encode(&self, buffer: &mut Vec<u8>, encoder: &mut Encoder) {
    match self {
      Binder::BuiltinSchema(kind) => {
        encoder.emit_u8(buffer, BinderTag::BuiltinSchema as u8);
        (*kind as u8).encode(buffer, encoder);
      }
      Binder::BuiltinFunc(kind) => {
        encoder.emit_u8(buffer, BinderTag::BuiltinFunc as u8);
        kind.encode(buffer, encoder);
      }
      Binder::Closure(node) => {
        encoder.emit_u8(buffer, BinderTag::Closure as u8);
        node.encode(buffer, encoder);
      }
      Binder::Existential(node) => {
        encoder.emit_u8(buffer, BinderTag::Existential as u8);
        node.encode(buffer, encoder);
      }
    }
  }
}

impl Decodable for Binder {
  fn decode(data: &mut &[u8], decoder: &Decoder) -> Self {
    match BinderTag::from_repr(decoder.read_u8(data)).expect("unknown Binder tag") {
      BinderTag::BuiltinSchema => Binder::BuiltinSchema(
        BuiltinSchemaKind::from_repr(u8::decode(data, decoder)).expect("unknown BuiltinSchemaKind"),
      ),
      BinderTag::BuiltinFunc => Binder::BuiltinFunc(NativeFuncKind::decode(data, decoder)),
      BinderTag::Closure => Binder::Closure(ProjectRedNode::decode(data, decoder)),
      BinderTag::Existential => Binder::Existential(ProjectRedNode::decode(data, decoder)),
    }
  }
}

#[query_interned]
pub struct TdBoundVariableTyp<'db> {
  pub binder: Binder,
  pub offset: usize,
  pub upper_bound: LazyTyp<'db>,
  pub variance: Variance,
}

impl<'db> TdBoundVariableTyp<'db> {
  fn resolve_bound(&self, db: &'db TypedownDatabase) -> Option<TdTypEnum<'db>> {
    Self::upper_bound(*self, db).resolve(db)
  }
}

impl<'db> TdStaticTyp<'db> for TdBoundVariableTyp<'db> {
  fn display_name(&self, db: &'db TypedownDatabase) -> String {
    let name = format!("T{}", Self::offset(*self, db));
    match self.resolve_bound(db) {
      Some(bound) if bound.as_td_obj_typ().is_none() => {
        format!("{} <: {}", name, bound.display_name(db))
      }
      _ => name,
    }
  }

  fn static_vtable(&self, db: &'db TypedownDatabase) -> BTreeMap<String, TdTypEnum<'db>> {
    self
      .resolve_bound(db)
      .map(|upper| upper.static_vtable(db))
      .unwrap_or_default()
  }

  fn get_fields(&self, db: &'db TypedownDatabase) -> BTreeMap<String, LazyTyp<'db>> {
    self
      .resolve_bound(db)
      .map(|upper| upper.get_fields(db))
      .unwrap_or_default()
  }

  fn lookup_field_typ(&self, db: &'db TypedownDatabase, name: &str) -> Option<TdTypEnum<'db>> {
    self.resolve_bound(db)?.lookup_field_typ(db, name)
  }

  fn index_typ(
    &self,
    db: &'db TypedownDatabase,
    key_typ: &TdTypEnum<'db>,
  ) -> Option<FuncSignature<'db>> {
    self.resolve_bound(db)?.index_typ(db, key_typ)
  }

  fn call_typ(
    &self,
    db: &'db TypedownDatabase,
    arg_typs: Vec<TdTypEnum<'db>>,
  ) -> CallTypResult<'db> {
    match self.resolve_bound(db) {
      Some(resolved) => resolved.call_typ(db, arg_typs),
      None => CallTypResult {
        signature: None,
        diagnostics: vec![],
      },
    }
  }
}

impl<'db> TdRuntimeObj<'db> for TdBoundVariableTyp<'db> {
  fn get_typ(&self, db: &'db TypedownDatabase) -> TdTypEnum<'db> {
    TdTypTyp::get(db).into()
  }
  fn get_owned_field(&self, _db: &'db TypedownDatabase, _key: &str) -> Option<TdObjEnum<'db>> {
    None
  }
  fn get_builtin_field(&self, _db: &'db TypedownDatabase, _key: &str) -> Option<TdObjEnum<'db>> {
    None
  }
  fn source_path(&self, db: &'db TypedownDatabase) -> String {
    self.display_name(db)
  }
}
